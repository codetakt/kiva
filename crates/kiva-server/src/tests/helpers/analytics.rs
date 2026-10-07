#![allow(clippy::wildcard_imports)]

use super::*;
use crate::surfaces::analytics::handlers::{
    get_analytics_product, get_analytics_product_aggregate, get_analytics_products,
};
use time::format_description::well_known::Rfc3339;

fn require_rfc3339(value: &Value, pointer: &str) -> Result<(), String> {
    let timestamp = value
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{pointer} should be an RFC3339 string"))?;
    OffsetDateTime::parse(timestamp, &Rfc3339)
        .map(|_parsed| ())
        .map_err(|error| format!("{pointer} should parse as RFC3339: {error:?}"))
}

async fn analytics_products(
    state: &super::AppState,
    headers: HeaderMap,
    query: BTreeMap<String, String>,
) -> axum::response::Response {
    get_analytics_products(analytics_state(state), headers, Query(query)).await
}

async fn analytics_product(
    state: &super::AppState,
    token: &'static str,
    data_product_id: Uuid,
) -> axum::response::Response {
    get_analytics_product(
        analytics_state(state),
        Path(data_product_id),
        confidential_headers_for(token),
        Query(BTreeMap::new()),
    )
    .await
}

async fn analytics_aggregate(
    state: &super::AppState,
    token: &'static str,
    data_product_id: Uuid,
) -> axum::response::Response {
    get_analytics_product_aggregate(
        analytics_state(state),
        Path(data_product_id),
        confidential_headers_for(token),
        Query(BTreeMap::new()),
    )
    .await
}

fn data_product_lifecycle_case_request(
    case_id: Uuid,
    status: super::PdsLifecycleCaseStatus,
    data_product_id: Uuid,
    resolution: Option<&str>,
) -> super::PdsLifecycleCaseRequest {
    let resource_id = data_product_id.to_string();
    let mut request = pds_lifecycle_case_request(
        case_id,
        super::PdsLifecycleCaseType::Tombstone,
        status,
        &resource_id,
        None,
        resolution,
        Some(super::PdsTombstonePublicationRule::MetadataOnly),
    );
    request.resource_kind = super::PdsLifecycleResourceKind::DataProduct;
    request
}

pub(in crate::tests) async fn assert_governed_analytics_boundary(
    state: &super::AppState,
) -> Result<(), String> {
    compute_confidential_aggregate_result(state).await?;
    let budget = create_differential_privacy_budget_for(state, "learner-a").await?;
    assert_eq!(budget.status(), StatusCode::NO_CONTENT);
    let release = release_differential_privacy_aggregate(
        state,
        "learner-a",
        differential_privacy_release_request(),
    )
    .await?;
    assert_eq!(release.status(), StatusCode::OK);

    let first_id = pds_uuid(201)?;
    let second_id = pds_uuid(202)?;
    for (data_product_id, title) in [
        (first_id, "Attendance trend"),
        (second_id, "Completion trend"),
    ] {
        let response = create_pds_data_product(
            state,
            "learner-a",
            data_product_id,
            super::PdsDataProductSourceKind::DifferentialPrivacyRelease,
            transcript(61),
            title,
            "Institutional aggregate reporting",
        )
        .await?;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    let unauthorized = analytics_products(state, HeaderMap::new(), BTreeMap::new()).await;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
    let forbidden =
        analytics_products(state, confidential_headers_for("reader"), BTreeMap::new()).await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let mut first_query = BTreeMap::new();
    first_query.insert("pageSize".to_owned(), "1".to_owned());
    let first_page =
        analytics_products(state, confidential_headers_for("learner-a"), first_query).await;
    assert_eq!(first_page.status(), StatusCode::OK);
    let first_body = response_json(first_page).await?;
    assert_eq!(
        first_body.pointer("/products/0/dataProductId"),
        Some(&json!(first_id))
    );
    require_rfc3339(&first_body, "/products/0/approvedAt")?;
    let cursor = first_body
        .pointer("/nextCursor")
        .and_then(Value::as_str)
        .ok_or_else(|| "analytics first page should provide a cursor".to_owned())?;

    let mut second_query = BTreeMap::new();
    second_query.insert("pageSize".to_owned(), "1".to_owned());
    second_query.insert("cursor".to_owned(), cursor.to_owned());
    let second_page =
        analytics_products(state, confidential_headers_for("learner-a"), second_query).await;
    assert_eq!(second_page.status(), StatusCode::OK);
    let second_body = response_json(second_page).await?;
    assert_eq!(
        second_body.pointer("/products/0/dataProductId"),
        Some(&json!(second_id))
    );
    assert!(second_body.pointer("/nextCursor").is_none());

    let detail = analytics_product(state, "learner-a", first_id).await;
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body = response_json(detail).await?;
    require_rfc3339(&detail_body, "/approvedAt")?;
    assert!(detail_body.pointer("/ownerSubject").is_none());
    assert!(detail_body.pointer("/statement").is_none());

    let aggregate = analytics_aggregate(state, "learner-a", first_id).await;
    assert_eq!(aggregate.status(), StatusCode::OK);
    let aggregate_body = response_json(aggregate).await?;
    require_rfc3339(&aggregate_body, "/releasedAt")?;
    assert!(
        aggregate_body
            .pointer("/value")
            .and_then(Value::as_i64)
            .is_some()
    );
    assert!(aggregate_body.pointer("/aggregateValue").is_none());
    assert!(aggregate_body.pointer("/noiseValue").is_none());

    let foreign = analytics_product(state, "learner-b", first_id).await;
    assert_eq!(foreign.status(), StatusCode::NOT_FOUND);
    let missing = analytics_product(state, "learner-a", pds_uuid(203)?).await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);

    let withdrawal_case_id = pds_uuid(204)?;
    for (status, resolution) in [
        (super::PdsLifecycleCaseStatus::Open, None),
        (super::PdsLifecycleCaseStatus::UnderReview, None),
        (
            super::PdsLifecycleCaseStatus::Tombstoned,
            Some("Aggregate product withdrawn from analytics serving"),
        ),
    ] {
        let response = create_pds_lifecycle_case(
            state,
            "learner-a",
            data_product_lifecycle_case_request(withdrawal_case_id, status, second_id, resolution),
        )
        .await?;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
    let withdrawn = analytics_product(state, "learner-a", second_id).await;
    assert_eq!(withdrawn.status(), StatusCode::NOT_FOUND);
    let visible_after_withdrawal = analytics_products(
        state,
        confidential_headers_for("learner-a"),
        BTreeMap::new(),
    )
    .await;
    assert_eq!(visible_after_withdrawal.status(), StatusCode::OK);
    let visible_body = response_json(visible_after_withdrawal).await?;
    assert_eq!(
        visible_body.pointer("/products/0/dataProductId"),
        Some(&json!(first_id))
    );
    assert_eq!(
        visible_body
            .pointer("/products")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );

    let mut oversized_query = BTreeMap::new();
    oversized_query.insert("pageSize".to_owned(), "101".to_owned());
    let oversized = analytics_products(
        state,
        confidential_headers_for("learner-a"),
        oversized_query,
    )
    .await;
    assert_eq!(oversized.status(), StatusCode::BAD_REQUEST);

    let audit = pds_access_audit_events(state, "learner-a", None).await;
    assert_eq!(audit.status(), StatusCode::OK);
    let audit_body = response_json(audit).await?;
    let events = audit_body
        .pointer("/events")
        .and_then(Value::as_array)
        .ok_or_else(|| "analytics reads should produce access audit events".to_owned())?;
    assert!(events.len() >= 6);
    assert!(events.iter().all(|event| {
        event.get("role") == Some(&json!("analyst"))
            && event.get("resource") == Some(&json!("aggregate-insight"))
            && event.get("action") == Some(&json!("read"))
    }));
    assert!(
        events
            .iter()
            .any(|event| event.get("decision") == Some(&json!("deny")))
    );
    Ok(())
}

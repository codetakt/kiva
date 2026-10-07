#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn pds_data_product_registry_exposes_metadata_without_release_payload() -> Result<(), String>
{
    let state = super::AppState::default();
    compute_confidential_aggregate_result(&state).await?;
    let budget = create_differential_privacy_budget_for(&state, "learner-a").await?;
    assert_eq!(budget.status(), StatusCode::NO_CONTENT);
    let release = release_differential_privacy_aggregate(
        &state,
        "learner-a",
        differential_privacy_release_request(),
    )
    .await?;
    assert_eq!(release.status(), StatusCode::OK);

    let data_product_id = pds_uuid(5)?;
    let response = create_pds_data_product(
        &state,
        "learner-a",
        data_product_id,
        super::PdsDataProductSourceKind::DifferentialPrivacyRelease,
        transcript(61),
        "Attendance trend",
        "School-level BI dashboard",
    )
    .await;
    assert_eq!(response?.status(), StatusCode::NO_CONTENT);

    let mut query = BTreeMap::new();
    query.insert("dataProductId".to_owned(), data_product_id.to_string());
    let list = super::get_pds_data_products(
        pds_state(&state),
        confidential_headers_for("learner-a"),
        Query(query),
    )
    .await;
    assert_eq!(list.status(), StatusCode::OK);
    let body = response_json(list).await?;
    assert_eq!(
        body.pointer("/dataProducts/0/sourceKind"),
        Some(&json!("differential-privacy-release"))
    );
    assert!(body.pointer("/dataProducts/0/aggregate").is_none());
    assert!(body.pointer("/dataProducts/0/aggregateValue").is_none());
    assert!(body.pointer("/dataProducts/0/noiseValue").is_none());
    assert!(body.pointer("/dataProducts/0/share").is_none());
    assert!(body.pointer("/dataProducts/0/signature").is_none());
    Ok(())
}

#[tokio::test]
async fn pds_data_product_access_requires_owner_scoped_grant() -> Result<(), String> {
    let state = super::AppState::default();
    compute_confidential_aggregate_result(&state).await?;
    let budget = create_differential_privacy_budget_for(&state, "learner-a").await?;
    assert_eq!(budget.status(), StatusCode::NO_CONTENT);
    let release = release_differential_privacy_aggregate(
        &state,
        "learner-a",
        differential_privacy_release_request(),
    )
    .await?;
    assert_eq!(release.status(), StatusCode::OK);

    let data_product_id = pds_uuid(16)?;
    let product = create_pds_data_product(
        &state,
        "learner-a",
        data_product_id,
        super::PdsDataProductSourceKind::DifferentialPrivacyRelease,
        transcript(61),
        "Attendance trend",
        "School-level BI dashboard",
    )
    .await?;
    assert_eq!(product.status(), StatusCode::NO_CONTENT);

    let grant_id = pds_uuid(17)?;
    let grant = create_pds_consent_grant(
        &state,
        "learner-a",
        grant_id,
        "analyst-a",
        super::PdsConsentScope::AggregateInsights,
    )
    .await?;
    assert_eq!(grant.status(), StatusCode::NO_CONTENT);

    assert_pds_data_product_access_decision(
        &state,
        "analyst-a",
        grant_id,
        data_product_id,
        "allow",
        None,
    )
    .await?;

    let audit = pds_access_audit_events(&state, "analyst-a", None).await;
    assert_eq!(audit.status(), StatusCode::OK);
    let audit_body = response_json(audit).await?;
    assert_eq!(
        audit_body.pointer("/events/0/dataProductId"),
        Some(&json!(data_product_id.to_string()))
    );

    assert_pds_data_product_access_decision(
        &state,
        "analyst-a",
        grant_id,
        pds_uuid(18)?,
        "deny",
        Some("dataProductUnavailable"),
    )
    .await?;

    let foreign_grant_id = pds_uuid(19)?;
    let foreign_grant = create_pds_consent_grant(
        &state,
        "learner-b",
        foreign_grant_id,
        "analyst-a",
        super::PdsConsentScope::AggregateInsights,
    )
    .await?;
    assert_eq!(foreign_grant.status(), StatusCode::NO_CONTENT);
    assert_pds_data_product_access_decision(
        &state,
        "analyst-a",
        foreign_grant_id,
        data_product_id,
        "deny",
        Some("dataProductUnavailable"),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn pds_data_product_requires_available_confidential_release() -> Result<(), String> {
    let state = super::AppState::default();
    let missing = create_pds_data_product(
        &state,
        "learner-a",
        pds_uuid(6)?,
        super::PdsDataProductSourceKind::DifferentialPrivacyRelease,
        transcript(61),
        "Missing DP release",
        "Should be rejected",
    )
    .await?;
    assert_eq!(missing.status(), StatusCode::CONFLICT);

    compute_confidential_aggregate_result(&state).await?;
    let request = create_threshold_release_request_for(&state, "learner-a").await?;
    let pending_threshold = create_pds_data_product(
        &state,
        "learner-a",
        pds_uuid(7)?,
        super::PdsDataProductSourceKind::ThresholdRelease,
        request.release_id,
        "Pending threshold release",
        "Should be rejected until both authorities release it",
    )
    .await?;
    assert_eq!(pending_threshold.status(), StatusCode::CONFLICT);

    let left = approve_threshold_release(&state, "policy-authority-left").await?;
    assert_eq!(left.status(), StatusCode::NO_CONTENT);
    let right = approve_threshold_release(&state, "policy-authority-right").await?;
    assert_eq!(right.status(), StatusCode::NO_CONTENT);
    let released = release_threshold_aggregate(&state, "policy-authority-right").await?;
    assert_eq!(released.status(), StatusCode::OK);

    let available_threshold = create_pds_data_product(
        &state,
        "learner-a",
        pds_uuid(8)?,
        super::PdsDataProductSourceKind::ThresholdRelease,
        request.release_id,
        "Released threshold aggregate",
        "School-level BI dashboard",
    )
    .await?;
    assert_eq!(available_threshold.status(), StatusCode::NO_CONTENT);

    let foreign_subject = create_pds_data_product(
        &state,
        "analyst-a",
        pds_uuid(9)?,
        super::PdsDataProductSourceKind::ThresholdRelease,
        request.release_id,
        "Foreign threshold aggregate",
        "Should be rejected for another subject",
    )
    .await?;
    assert_eq!(foreign_subject.status(), StatusCode::CONFLICT);

    assert_pds_shamir_reconstruction_data_product_admission(
        &state,
        pds_uuid(10)?,
        pds_uuid(11)?,
        pds_uuid(12)?,
        pds_uuid(13)?,
    )
    .await?;
    Ok(())
}

#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn pds_record_inventory_exposes_subject_statement_metadata_only() -> Result<(), String> {
    let state = super::AppState::default();
    let statement_id = pds_uuid(20)?;
    store_pds_subject_statement(&state, "learner-a", statement_id).await?;
    store_pds_subject_statement(&state, "learner-b", pds_uuid(21)?).await?;

    let inventory = pds_record_inventory(&state, "learner-a").await;
    assert_eq!(inventory.status(), StatusCode::OK);
    let body = response_json(inventory).await?;
    assert_eq!(body.pointer("/dataSubject"), Some(&json!("learner-a")));
    assert_eq!(
        body.pointer("/actorIfi"),
        Some(&json!(super::wire::pds::subject_actor_ifi("learner-a")))
    );
    assert_eq!(
        body.pointer("/statementRecords")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        body.pointer("/statementRecords/0/statementId"),
        Some(&json!(statement_id.to_string()))
    );
    assert_eq!(
        body.pointer("/statementRecords/0/verbId"),
        Some(&json!("https://example.org/xapi/verbs/completed"))
    );
    assert_no_pds_raw_payload_fields(&body);
    Ok(())
}

#[tokio::test]
async fn pds_metadata_export_includes_lifecycle_metadata_without_raw_payload() -> Result<(), String>
{
    let state = super::AppState::default();
    let statement_id = pds_uuid(22)?;
    store_pds_subject_statement(&state, "learner-a", statement_id).await?;
    let grant_id = pds_uuid(23)?;
    let grant = create_pds_consent_grant(
        &state,
        "learner-a",
        grant_id,
        "analyst-a",
        super::PdsConsentScope::AggregateInsights,
    )
    .await?;
    assert_eq!(grant.status(), StatusCode::NO_CONTENT);

    let lifecycle_event_id = pds_uuid(24)?;
    let lifecycle_event = create_pds_lifecycle_event(
        &state,
        "learner-a",
        super::PdsLifecycleEventRequest {
            event_id: lifecycle_event_id,
            event_type: super::PdsLifecycleEventType::Exported,
            resource_kind: super::PdsLifecycleResourceKind::MetadataExport,
            resource_id: "metadata-export-2025-01-01".to_owned(),
            reason: "Subject requested portable metadata export".to_owned(),
        },
    )
    .await?;
    assert_eq!(lifecycle_event.status(), StatusCode::NO_CONTENT);

    let export = pds_metadata_exports(&state, "learner-a").await;
    assert_eq!(export.status(), StatusCode::OK);
    let body = response_json(export).await?;
    assert_eq!(body.pointer("/dataSubject"), Some(&json!("learner-a")));
    assert_eq!(
        body.pointer("/recordInventory/statementRecords/0/statementId"),
        Some(&json!(statement_id.to_string()))
    );
    assert_eq!(
        body.pointer("/consentGrants/0/legalBasis"),
        Some(&json!("consent"))
    );
    assert_eq!(
        body.pointer("/consentGrants/0/purpose"),
        Some(&json!("subject delegated access"))
    );
    assert_eq!(
        body.pointer("/lifecycleEvents/0/eventType"),
        Some(&json!("exported"))
    );
    assert_eq!(
        body.pointer("/lifecycleEvents/0/eventId"),
        Some(&json!(lifecycle_event_id.to_string()))
    );
    assert_no_pds_raw_payload_fields(&body);
    Ok(())
}

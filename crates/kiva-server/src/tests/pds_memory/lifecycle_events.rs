#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn pds_lifecycle_events_are_subject_scoped_and_idempotent() -> Result<(), String> {
    let state = super::AppState::default();
    let event_id = pds_uuid(25)?;
    let request = super::PdsLifecycleEventRequest {
        event_id,
        event_type: super::PdsLifecycleEventType::CorrectionRequested,
        resource_kind: super::PdsLifecycleResourceKind::XapiStatement,
        resource_id: pds_uuid(26)?.to_string(),
        reason: "Learner disputes completion timestamp".to_owned(),
    };

    let first = create_pds_lifecycle_event(&state, "learner-a", request.clone()).await?;
    assert_eq!(first.status(), StatusCode::NO_CONTENT);
    let duplicate = create_pds_lifecycle_event(&state, "learner-a", request).await?;
    assert_eq!(duplicate.status(), StatusCode::NO_CONTENT);

    let owner_events = pds_lifecycle_events(&state, "learner-a", Some(event_id)).await;
    assert_eq!(owner_events.status(), StatusCode::OK);
    let owner_body = response_json(owner_events).await?;
    assert_eq!(
        owner_body
            .pointer("/events")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        owner_body.pointer("/events/0/resourceKind"),
        Some(&json!("xapi-statement"))
    );

    let foreign_events = pds_lifecycle_events(&state, "learner-b", Some(event_id)).await;
    assert_eq!(foreign_events.status(), StatusCode::OK);
    let foreign_body = response_json(foreign_events).await?;
    assert_eq!(
        foreign_body
            .pointer("/events")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0)
    );
    Ok(())
}

#[tokio::test]
async fn pds_retention_policies_are_subject_scoped_and_metadata_only() -> Result<(), String> {
    let state = super::AppState::default();
    let policy_id = pds_uuid(52)?;
    let request = pds_retention_policy_request(policy_id);

    let first = create_pds_retention_policy(&state, "learner-a", request.clone()).await?;
    assert_eq!(first.status(), StatusCode::NO_CONTENT);
    let duplicate = create_pds_retention_policy(&state, "learner-a", request).await?;
    assert_eq!(duplicate.status(), StatusCode::NO_CONTENT);

    let owner_policies = pds_retention_policies(&state, "learner-a", Some(policy_id)).await;
    assert_eq!(owner_policies.status(), StatusCode::OK);
    let owner_body = response_json(owner_policies).await?;
    assert_eq!(
        owner_body
            .pointer("/policies")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        owner_body.pointer("/policies/0/tombstonePublication"),
        Some(&json!("metadata-only"))
    );
    assert_eq!(
        owner_body.pointer("/policies/0/retentionPeriodDays"),
        Some(&json!(365))
    );
    assert_no_pds_raw_payload_fields(&owner_body);

    let foreign_policies = pds_retention_policies(&state, "learner-b", Some(policy_id)).await;
    assert_eq!(foreign_policies.status(), StatusCode::OK);
    let foreign_body = response_json(foreign_policies).await?;
    assert_eq!(
        foreign_body
            .pointer("/policies")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0)
    );

    let mut invalid_days = pds_retention_policy_request(pds_uuid(53)?);
    invalid_days.retention_period_days = 0;
    let invalid = create_pds_retention_policy(&state, "learner-a", invalid_days).await?;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

    let mut invalid_tombstone = pds_retention_policy_request(pds_uuid(54)?);
    invalid_tombstone.tombstone_after_days = Some(30);
    let invalid = create_pds_retention_policy(&state, "learner-a", invalid_tombstone).await?;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

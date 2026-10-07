#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn pds_lifecycle_cases_enforce_strict_transitions() -> Result<(), String> {
    let state = super::AppState::default();
    let policy_id = pds_uuid(55)?;
    let policy =
        create_pds_retention_policy(&state, "learner-a", pds_retention_policy_request(policy_id))
            .await?;
    assert_eq!(policy.status(), StatusCode::NO_CONTENT);

    let case_id = pds_uuid(56)?;
    let resource_id = pds_uuid(57)?.to_string();
    let open_case = pds_lifecycle_case_request(
        case_id,
        super::PdsLifecycleCaseType::Correction,
        super::PdsLifecycleCaseStatus::Open,
        resource_id.as_str(),
        Some(policy_id),
        None,
        None,
    );
    let first = create_pds_lifecycle_case(&state, "learner-a", open_case.clone()).await?;
    assert_eq!(first.status(), StatusCode::NO_CONTENT);
    let duplicate = create_pds_lifecycle_case(&state, "learner-a", open_case).await?;
    assert_eq!(duplicate.status(), StatusCode::NO_CONTENT);

    let early_resolution = create_pds_lifecycle_case(
        &state,
        "learner-a",
        pds_lifecycle_case_request(
            pds_uuid(58)?,
            super::PdsLifecycleCaseType::Dispute,
            super::PdsLifecycleCaseStatus::Resolved,
            pds_uuid(59)?.to_string().as_str(),
            None,
            Some("Resolved before review"),
            None,
        ),
    )
    .await?;
    assert_eq!(early_resolution.status(), StatusCode::BAD_REQUEST);

    let review = create_pds_lifecycle_case(
        &state,
        "learner-a",
        pds_lifecycle_case_request(
            case_id,
            super::PdsLifecycleCaseType::Correction,
            super::PdsLifecycleCaseStatus::UnderReview,
            resource_id.as_str(),
            Some(policy_id),
            None,
            None,
        ),
    )
    .await?;
    assert_eq!(review.status(), StatusCode::NO_CONTENT);

    let resolved = create_pds_lifecycle_case(
        &state,
        "learner-a",
        pds_lifecycle_case_request(
            case_id,
            super::PdsLifecycleCaseType::Correction,
            super::PdsLifecycleCaseStatus::Resolved,
            resource_id.as_str(),
            Some(policy_id),
            Some("Corrected by appending a superseding statement reference"),
            None,
        ),
    )
    .await?;
    assert_eq!(resolved.status(), StatusCode::NO_CONTENT);
    let terminal_rewrite = create_pds_lifecycle_case(
        &state,
        "learner-a",
        pds_lifecycle_case_request(
            case_id,
            super::PdsLifecycleCaseType::Correction,
            super::PdsLifecycleCaseStatus::Resolved,
            resource_id.as_str(),
            Some(policy_id),
            Some("Changed terminal resolution"),
            None,
        ),
    )
    .await?;
    assert_eq!(terminal_rewrite.status(), StatusCode::CONFLICT);
    Ok(())
}

#[tokio::test]
async fn pds_lifecycle_tombstone_cases_require_publication_rules() -> Result<(), String> {
    let state = super::AppState::default();
    let missing_tombstone_rule = create_pds_lifecycle_case(
        &state,
        "learner-a",
        pds_lifecycle_case_request(
            pds_uuid(60)?,
            super::PdsLifecycleCaseType::Tombstone,
            super::PdsLifecycleCaseStatus::Open,
            pds_uuid(61)?.to_string().as_str(),
            None,
            None,
            None,
        ),
    )
    .await?;
    assert_eq!(missing_tombstone_rule.status(), StatusCode::BAD_REQUEST);

    let non_tombstone_terminal = create_pds_lifecycle_case(
        &state,
        "learner-a",
        pds_lifecycle_case_request(
            pds_uuid(67)?,
            super::PdsLifecycleCaseType::Correction,
            super::PdsLifecycleCaseStatus::Tombstoned,
            pds_uuid(68)?.to_string().as_str(),
            None,
            Some("Tombstoned status cannot be used for correction cases"),
            Some(super::PdsTombstonePublicationRule::MetadataOnly),
        ),
    )
    .await?;
    assert_eq!(non_tombstone_terminal.status(), StatusCode::BAD_REQUEST);

    let tombstone_case_id = pds_uuid(62)?;
    let tombstone_resource_id = pds_uuid(63)?.to_string();
    let tombstone_open = create_pds_lifecycle_case(
        &state,
        "learner-a",
        pds_lifecycle_case_request(
            tombstone_case_id,
            super::PdsLifecycleCaseType::Tombstone,
            super::PdsLifecycleCaseStatus::Open,
            tombstone_resource_id.as_str(),
            None,
            None,
            Some(super::PdsTombstonePublicationRule::SubjectVisible),
        ),
    )
    .await?;
    assert_eq!(tombstone_open.status(), StatusCode::NO_CONTENT);
    let tombstone_review = create_pds_lifecycle_case(
        &state,
        "learner-a",
        pds_lifecycle_case_request(
            tombstone_case_id,
            super::PdsLifecycleCaseType::Tombstone,
            super::PdsLifecycleCaseStatus::UnderReview,
            tombstone_resource_id.as_str(),
            None,
            None,
            Some(super::PdsTombstonePublicationRule::SubjectVisible),
        ),
    )
    .await?;
    assert_eq!(tombstone_review.status(), StatusCode::NO_CONTENT);
    let tombstoned = create_pds_lifecycle_case(
        &state,
        "learner-a",
        pds_lifecycle_case_request(
            tombstone_case_id,
            super::PdsLifecycleCaseType::Tombstone,
            super::PdsLifecycleCaseStatus::Tombstoned,
            tombstone_resource_id.as_str(),
            None,
            Some("Subject-visible tombstone metadata recorded"),
            Some(super::PdsTombstonePublicationRule::SubjectVisible),
        ),
    )
    .await?;
    assert_eq!(tombstoned.status(), StatusCode::NO_CONTENT);

    let cases = pds_lifecycle_cases(&state, "learner-a", Some(tombstone_case_id)).await;
    assert_eq!(cases.status(), StatusCode::OK);
    let body = response_json(cases).await?;
    assert_eq!(body.pointer("/cases/0/status"), Some(&json!("tombstoned")));
    assert_eq!(
        body.pointer("/cases/0/tombstonePublication"),
        Some(&json!("subject-visible"))
    );
    assert_no_pds_raw_payload_fields(&body);
    Ok(())
}

#[tokio::test]
async fn pds_metadata_export_includes_operational_lifecycle_workflows_without_raw_payload()
-> Result<(), String> {
    let state = super::AppState::default();
    let policy_id = pds_uuid(64)?;
    let policy =
        create_pds_retention_policy(&state, "learner-a", pds_retention_policy_request(policy_id))
            .await?;
    assert_eq!(policy.status(), StatusCode::NO_CONTENT);
    let case_id = pds_uuid(65)?;
    let case = create_pds_lifecycle_case(
        &state,
        "learner-a",
        pds_lifecycle_case_request(
            case_id,
            super::PdsLifecycleCaseType::Dispute,
            super::PdsLifecycleCaseStatus::Open,
            pds_uuid(66)?.to_string().as_str(),
            Some(policy_id),
            None,
            None,
        ),
    )
    .await?;
    assert_eq!(case.status(), StatusCode::NO_CONTENT);

    let export = pds_metadata_exports(&state, "learner-a").await;
    assert_eq!(export.status(), StatusCode::OK);
    let body = response_json(export).await?;
    assert_eq!(
        body.pointer("/retentionPolicies/0/policyId"),
        Some(&json!(policy_id.to_string()))
    );
    assert_eq!(
        body.pointer("/lifecycleCases/0/caseId"),
        Some(&json!(case_id.to_string()))
    );
    assert_eq!(
        body.pointer("/lifecycleCases/0/status"),
        Some(&json!("open"))
    );
    assert_no_pds_raw_payload_fields(&body);
    Ok(())
}

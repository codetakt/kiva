#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_pds_learner_portability_export_is_subject_bound_paged_and_audited()
-> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or_else(|| "KIVA_POSTGRES_TEST_DATABASE_URL must be set".to_owned())?;
    super::pds_memory::portability::assert_portability_export_scenario(&state).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_confidential_threshold_release_is_idempotent_under_retries() -> Result<(), String>
{
    let state = postgres_test_state()
        .await?
        .ok_or_else(|| "KIVA_POSTGRES_TEST_DATABASE_URL must be set".to_owned())?;
    compute_confidential_aggregate_result(&state).await?;
    let request = create_threshold_release_request(&state).await?;
    let duplicate_request = create_threshold_release_request(&state).await?;
    assert_eq!(duplicate_request.release_id, request.release_id);

    let (left_first, left_duplicate) = tokio::join!(
        approve_threshold_release(&state, "policy-authority-left"),
        approve_threshold_release(&state, "policy-authority-left")
    );
    assert_eq!(left_first?.status(), StatusCode::NO_CONTENT);
    assert_eq!(left_duplicate?.status(), StatusCode::NO_CONTENT);

    let early_release = release_threshold_aggregate(&state, "policy-authority-left").await?;
    assert_eq!(early_release.status(), StatusCode::CONFLICT);

    let right_response = approve_threshold_release(&state, "policy-authority-right").await?;
    assert_eq!(right_response.status(), StatusCode::NO_CONTENT);

    let final_response = release_threshold_aggregate(&state, "policy-authority-right").await?;
    assert_eq!(final_response.status(), StatusCode::OK);
    let final_body = response_json(final_response).await?;
    assert_eq!(final_body.pointer("/aggregate"), Some(&json!(42)));
    assert_eq!(
        final_body.pointer("/release/status"),
        Some(&json!("released"))
    );

    let repeated_release = release_threshold_aggregate(&state, "policy-authority-left").await?;
    assert_eq!(repeated_release.status(), StatusCode::OK);
    let repeated_body = response_json(repeated_release).await?;
    assert_eq!(repeated_body.pointer("/aggregate"), Some(&json!(42)));
    assert_eq!(
        repeated_body.pointer("/release/status"),
        Some(&json!("released"))
    );

    let audit_response = threshold_release_metadata(&state, request.release_id).await;
    assert_eq!(audit_response.status(), StatusCode::OK);
    let audit_body = response_json(audit_response).await?;
    assert_eq!(
        audit_body.pointer("/releases/0/status"),
        Some(&json!("released"))
    );
    assert_eq!(
        audit_body
            .pointer("/releases/0/approvedAuthoritySubjects")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(2)
    );
    assert!(audit_body.pointer("/releases/0/aggregate").is_none());
    Ok(())
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_confidential_differential_privacy_budget_spend_is_serialized()
-> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or_else(|| "KIVA_POSTGRES_TEST_DATABASE_URL must be set".to_owned())?;
    let response = create_differential_privacy_budget(&state).await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let (first, second) = tokio::join!(
        spend_differential_privacy_budget(&state, "confidential-coordinator", 750_000),
        spend_differential_privacy_budget(&state, "confidential-coordinator", 750_000),
    );
    let statuses = [first?.status(), second?.status()];
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::OK)
            .count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::CONFLICT)
            .count(),
        1
    );

    let audit = differential_privacy_budget_metadata(&state, transcript(51)).await;
    assert_eq!(audit.status(), StatusCode::OK);
    let audit_body = response_json(audit).await?;
    assert_eq!(
        audit_body.pointer("/budgets/0/spentEpsilonMicrounits"),
        Some(&json!(750_000))
    );
    assert_eq!(
        audit_body.pointer("/budgets/0/remainingEpsilonMicrounits"),
        Some(&json!(250_000))
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_confidential_differential_privacy_release_is_idempotent_under_retries()
-> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or_else(|| "KIVA_POSTGRES_TEST_DATABASE_URL must be set".to_owned())?;
    compute_confidential_aggregate_result(&state).await?;
    let budget = create_differential_privacy_budget(&state).await?;
    assert_eq!(budget.status(), StatusCode::NO_CONTENT);

    let (first, second) = tokio::join!(
        release_differential_privacy_aggregate(
            &state,
            "confidential-coordinator",
            differential_privacy_release_request()
        ),
        release_differential_privacy_aggregate(
            &state,
            "confidential-coordinator",
            differential_privacy_release_request()
        ),
    );
    assert_eq!(first?.status(), StatusCode::OK);
    assert_eq!(second?.status(), StatusCode::OK);

    let budget_audit = differential_privacy_budget_metadata(&state, transcript(51)).await;
    assert_eq!(budget_audit.status(), StatusCode::OK);
    let budget_body = response_json(budget_audit).await?;
    assert_eq!(
        budget_body.pointer("/budgets/0/spentEpsilonMicrounits"),
        Some(&json!(250_000))
    );

    let release_audit = differential_privacy_release_metadata(&state, transcript(61)).await;
    assert_eq!(release_audit.status(), StatusCode::OK);
    let release_body = response_json(release_audit).await?;
    assert_eq!(
        release_body
            .pointer("/releases")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        release_body.pointer("/releases/0/noisedValue"),
        Some(&json!(37))
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_confidential_differential_privacy_sampled_release_is_idempotent_under_retries()
-> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or_else(|| "KIVA_POSTGRES_TEST_DATABASE_URL must be set".to_owned())?;
    compute_confidential_aggregate_result(&state).await?;
    let budget = create_differential_privacy_budget(&state).await?;
    assert_eq!(budget.status(), StatusCode::NO_CONTENT);

    let (first, second) = tokio::join!(
        release_sampled_differential_privacy_aggregate(
            &state,
            "confidential-coordinator",
            differential_privacy_sampled_release_request()
        ),
        release_sampled_differential_privacy_aggregate(
            &state,
            "confidential-coordinator",
            differential_privacy_sampled_release_request()
        ),
    );
    assert_eq!(first?.status(), StatusCode::OK);
    assert_eq!(second?.status(), StatusCode::OK);

    let budget_audit = differential_privacy_budget_metadata(&state, transcript(51)).await;
    assert_eq!(budget_audit.status(), StatusCode::OK);
    let budget_body = response_json(budget_audit).await?;
    assert_eq!(
        budget_body.pointer("/budgets/0/spentEpsilonMicrounits"),
        Some(&json!(250_000))
    );

    let release_audit = differential_privacy_release_metadata(&state, transcript(63)).await;
    assert_eq!(release_audit.status(), StatusCode::OK);
    let release_body = response_json(release_audit).await?;
    assert_eq!(
        release_body
            .pointer("/releases")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert!(
        release_body
            .pointer("/releases/0/noisedValue")
            .and_then(Value::as_i64)
            .is_some()
    );
    assert!(release_body.pointer("/releases/0/aggregate").is_none());
    assert!(release_body.pointer("/releases/0/noiseValue").is_none());
    Ok(())
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_confidential_job_lifecycle_is_idempotent_and_audited() -> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or_else(|| "KIVA_POSTGRES_TEST_DATABASE_URL must be set".to_owned())?;
    let job_id = transcript(94);
    let request = confidential_job_request(job_id);

    let (first, second) = tokio::join!(
        create_confidential_job(&state, request.clone()),
        create_confidential_job(&state, request),
    );
    assert_eq!(first?.status(), StatusCode::OK);
    assert_eq!(second?.status(), StatusCode::OK);

    transition_confidential_job(&state, job_id, ConfidentialJobStatus::Admitted).await?;
    transition_confidential_job(&state, job_id, ConfidentialJobStatus::Running).await?;
    transition_confidential_job(&state, job_id, ConfidentialJobStatus::AwaitingRelease).await?;
    transition_confidential_job(&state, job_id, ConfidentialJobStatus::Released).await?;

    let metadata = confidential_job_metadata(&state, job_id).await;
    assert_eq!(metadata.status(), StatusCode::OK);
    let metadata_body = response_json(metadata).await?;
    assert_eq!(
        metadata_body.pointer("/jobs/0/status"),
        Some(&json!("released"))
    );
    assert_eq!(metadata_body.pointer("/jobs/0/retryCount"), Some(&json!(1)));
    assert!(metadata_body.pointer("/jobs/0/share").is_none());
    assert!(metadata_body.pointer("/jobs/0/signature").is_none());
    assert!(metadata_body.pointer("/jobs/0/plaintext").is_none());

    let audit = confidential_job_audit_events(&state, job_id).await;
    assert_eq!(audit.status(), StatusCode::OK);
    let audit_body = response_json(audit).await?;
    assert_eq!(
        audit_body
            .pointer("/events")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(5)
    );
    assert!(audit_body.pointer("/events/0/share").is_none());
    assert!(audit_body.pointer("/events/0/signature").is_none());
    assert!(audit_body.pointer("/events/0/noise").is_none());
    Ok(())
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_confidential_participant_lifecycle_is_idempotent_and_audited()
-> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or_else(|| "KIVA_POSTGRES_TEST_DATABASE_URL must be set".to_owned())?;

    register_active_confidential_participant(
        &state,
        "confidential-coordinator",
        "agency-a",
        1,
        vec!["data-custodian".to_owned()],
    )
    .await?;
    register_active_confidential_participant(
        &state,
        "confidential-coordinator",
        "agency-a",
        1,
        vec!["data-custodian".to_owned()],
    )
    .await?;

    let mut query = BTreeMap::new();
    query.insert("participantSubject".to_owned(), "agency-a".to_owned());
    let participants = super::get_confidential_participants(
        confidential_state(&state),
        confidential_headers(),
        Query(query),
    )
    .await;
    assert_eq!(participants.status(), StatusCode::OK);
    let participants_body = response_json(participants).await?;
    assert_eq!(
        participants_body
            .pointer("/participants")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        participants_body.pointer("/participants/0/status"),
        Some(&json!("active"))
    );

    let mut query = BTreeMap::new();
    query.insert("participantSubject".to_owned(), "agency-a".to_owned());
    let audit = super::get_confidential_participant_audit_events(
        confidential_state(&state),
        confidential_headers(),
        Query(query),
    )
    .await;
    assert_eq!(audit.status(), StatusCode::OK);
    let audit_body = response_json(audit).await?;
    assert_eq!(
        audit_body
            .pointer("/events")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(2)
    );

    let missing_key = super::post_confidential_share_signing_key(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_signing_key_body(&confidential_signing_key_request(21))?,
    )
    .await;
    assert_eq!(missing_key.status(), StatusCode::CONFLICT);
    register_active_confidential_participant(
        &state,
        "confidential-coordinator",
        "confidential-coordinator",
        1,
        vec!["additive-share-contributor".to_owned()],
    )
    .await?;
    let active_key = super::post_confidential_share_signing_key(
        confidential_state(&state),
        confidential_headers(),
        Query(BTreeMap::new()),
        confidential_signing_key_body(&confidential_signing_key_request(21))?,
    )
    .await;
    assert_eq!(active_key.status(), StatusCode::NO_CONTENT);
    Ok(())
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_confidential_differential_privacy_composition_summary_tracks_release_and_manual_spend()
-> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or_else(|| "KIVA_POSTGRES_TEST_DATABASE_URL must be set".to_owned())?;
    compute_confidential_aggregate_result(&state).await?;
    let budget = create_differential_privacy_budget(&state).await?;
    assert_eq!(budget.status(), StatusCode::NO_CONTENT);
    let manual_spend =
        spend_differential_privacy_budget(&state, "confidential-coordinator", 300_000).await?;
    assert_eq!(manual_spend.status(), StatusCode::OK);
    let release = release_differential_privacy_aggregate(
        &state,
        "confidential-coordinator",
        differential_privacy_release_request(),
    )
    .await?;
    assert_eq!(release.status(), StatusCode::OK);

    let summaries = differential_privacy_composition_summaries(&state, transcript(51)).await;
    assert_eq!(summaries.status(), StatusCode::OK);
    let body = response_json(summaries).await?;
    assert_eq!(
        body.pointer("/summaries")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        body.pointer("/summaries/0/budgetSpentEpsilonMicrounits"),
        Some(&json!(550_000))
    );
    assert_eq!(
        body.pointer("/summaries/0/releaseSpentEpsilonMicrounits"),
        Some(&json!(250_000))
    );
    assert_eq!(
        body.pointer("/summaries/0/unattributedSpentEpsilonMicrounits"),
        Some(&json!(300_000))
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_pds_consent_revocation_and_access_audit_are_persistent() -> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or_else(|| "KIVA_POSTGRES_TEST_DATABASE_URL must be set".to_owned())?;
    let grant_id = pds_uuid(10)?;

    let first = create_pds_consent_grant(
        &state,
        "learner-a",
        grant_id,
        "analyst-a",
        super::PdsConsentScope::AggregateInsights,
    )
    .await?;
    assert_eq!(first.status(), StatusCode::NO_CONTENT);
    let duplicate = create_pds_consent_grant(
        &state,
        "learner-a",
        grant_id,
        "analyst-a",
        super::PdsConsentScope::AggregateInsights,
    )
    .await?;
    assert_eq!(duplicate.status(), StatusCode::NO_CONTENT);

    let (first_revocation, second_revocation) = tokio::join!(
        revoke_pds_consent_grant(&state, "learner-a", grant_id),
        revoke_pds_consent_grant(&state, "learner-a", grant_id),
    );
    assert_eq!(first_revocation?.status(), StatusCode::NO_CONTENT);
    assert_eq!(second_revocation?.status(), StatusCode::NO_CONTENT);

    let denied = pds_access_check(
        &state,
        "analyst-a",
        super::PdsAccessCheckRequest {
            role: "analyst".to_owned(),
            resource: "aggregate-insight".to_owned(),
            action: "read".to_owned(),
            grant_id: Some(grant_id),
            data_product_id: None,
        },
    )
    .await?;
    assert_eq!(denied.status(), StatusCode::OK);
    let denied_body = response_json(denied).await?;
    assert_eq!(denied_body.pointer("/decision"), Some(&json!("deny")));
    assert_eq!(
        denied_body.pointer("/denyReason"),
        Some(&json!("consentRevoked"))
    );

    let audit = pds_access_audit_events(&state, "analyst-a", None).await;
    assert_eq!(audit.status(), StatusCode::OK);
    let audit_body = response_json(audit).await?;
    assert_eq!(
        audit_body.pointer("/events/0/decision"),
        Some(&json!("deny"))
    );
    assert_eq!(
        audit_body.pointer("/events/0/denyReason"),
        Some(&json!("consentRevoked"))
    );
    assert_eq!(
        audit_body.pointer("/events/0/grantId"),
        Some(&json!(grant_id.to_string()))
    );
    assert!(audit_body.pointer("/events/0/statement").is_none());
    assert!(audit_body.pointer("/events/0/aggregateValue").is_none());
    Ok(())
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_learner_principal_token_exercises_subject_bound_pds_flows() -> Result<(), String>
{
    let state = postgres_test_state_with_auth(learner_principal_auth_config())
        .await?
        .ok_or_else(|| "KIVA_POSTGRES_TEST_DATABASE_URL must be set".to_owned())?;
    assert_learner_principal_subject_bound_flows(&state).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_pds_data_products_require_available_release_and_owner_scope() -> Result<(), String>
{
    let state = postgres_test_state()
        .await?
        .ok_or_else(|| "KIVA_POSTGRES_TEST_DATABASE_URL must be set".to_owned())?;
    let missing = create_pds_data_product(
        &state,
        "learner-a",
        pds_uuid(11)?,
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
        pds_uuid(12)?,
        super::PdsDataProductSourceKind::ThresholdRelease,
        request.release_id,
        "Pending threshold release",
        "Should be rejected until release",
    )
    .await?;
    assert_eq!(pending_threshold.status(), StatusCode::CONFLICT);

    let left = approve_threshold_release(&state, "policy-authority-left").await?;
    assert_eq!(left.status(), StatusCode::NO_CONTENT);
    let right = approve_threshold_release(&state, "policy-authority-right").await?;
    assert_eq!(right.status(), StatusCode::NO_CONTENT);
    let released = release_threshold_aggregate(&state, "policy-authority-right").await?;
    assert_eq!(released.status(), StatusCode::OK);

    let threshold_product_id = pds_uuid(13)?;
    let registered_threshold = create_pds_data_product(
        &state,
        "learner-a",
        threshold_product_id,
        super::PdsDataProductSourceKind::ThresholdRelease,
        request.release_id,
        "Released threshold aggregate",
        "School-level BI dashboard",
    )
    .await?;
    assert_eq!(registered_threshold.status(), StatusCode::NO_CONTENT);
    let duplicate_threshold = create_pds_data_product(
        &state,
        "learner-a",
        threshold_product_id,
        super::PdsDataProductSourceKind::ThresholdRelease,
        request.release_id,
        "Released threshold aggregate",
        "School-level BI dashboard",
    )
    .await?;
    assert_eq!(duplicate_threshold.status(), StatusCode::NO_CONTENT);

    let foreign_subject = create_pds_data_product(
        &state,
        "analyst-a",
        pds_uuid(14)?,
        super::PdsDataProductSourceKind::ThresholdRelease,
        request.release_id,
        "Foreign threshold aggregate",
        "Should be rejected for another subject",
    )
    .await?;
    assert_eq!(foreign_subject.status(), StatusCode::CONFLICT);

    let shamir_product_id = pds_uuid(15)?;
    assert_pds_shamir_reconstruction_data_product_admission(
        &state,
        pds_uuid(16)?,
        shamir_product_id,
        pds_uuid(17)?,
        pds_uuid(18)?,
    )
    .await?;

    assert_pds_data_products(
        &state,
        "learner-a",
        threshold_product_id,
        1,
        Some("threshold-release"),
    )
    .await?;
    assert_pds_data_products(&state, "analyst-a", threshold_product_id, 0, None).await?;

    assert_pds_differential_privacy_data_product_access_owner_scope(&state).await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_analytics_surface_is_governed_and_audited() -> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or_else(|| "KIVA_POSTGRES_TEST_DATABASE_URL must be set".to_owned())?;
    assert_governed_analytics_boundary(&state).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_pds_governance_profiles_are_strict_and_persistent() -> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or_else(|| "KIVA_POSTGRES_TEST_DATABASE_URL must be set".to_owned())?;
    let profile_id = pds_uuid(31)?;
    let request = pds_governance_profile_request(profile_id);

    let first = create_pds_governance_profile(&state, "public-admin", request.clone()).await?;
    assert_eq!(first.status(), StatusCode::NO_CONTENT);
    let duplicate = create_pds_governance_profile(&state, "public-admin", request).await?;
    assert_eq!(duplicate.status(), StatusCode::NO_CONTENT);

    let profiles = pds_governance_profiles(&state, "public-admin", Some(profile_id)).await;
    assert_eq!(profiles.status(), StatusCode::OK);
    let body = response_json(profiles).await?;
    assert_eq!(
        body.pointer("/profiles/0/profileId"),
        Some(&json!(profile_id.to_string()))
    );
    assert_eq!(
        body.pointer("/profiles/0/auditorSubject"),
        Some(&json!("public-auditor"))
    );

    let mut overlapping_identity = pds_governance_profile_request(pds_uuid(32)?);
    overlapping_identity.identity_authority_subject =
        Some(overlapping_identity.data_custodian_subject.clone());
    let rejected =
        create_pds_governance_profile(&state, "public-admin", overlapping_identity).await?;
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

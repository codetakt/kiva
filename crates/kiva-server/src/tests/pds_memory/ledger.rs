#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn pds_mutations_are_projected_into_internal_ledger() -> Result<(), String> {
    let state = super::AppState::default();
    let grant_id = pds_uuid(70)?;
    let grant = create_pds_consent_grant(
        &state,
        "learner-a",
        grant_id,
        "analyst-a",
        super::PdsConsentScope::AggregateInsights,
    )
    .await?;
    assert_eq!(grant.status(), StatusCode::NO_CONTENT);
    assert_pds_ledger_event(
        &state,
        "consent-grant-registered",
        "learner-a",
        "consent-grant",
        &grant_id.to_string(),
    )
    .await?;

    let access = pds_access_check(
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
    assert_eq!(access.status(), StatusCode::OK);
    let access_body = response_json(access).await?;
    let access_event_id = access_body
        .pointer("/eventId")
        .and_then(Value::as_str)
        .ok_or_else(|| "PDS access check should expose its audit event id".to_owned())?;
    assert_pds_ledger_event(
        &state,
        "access-decision-recorded",
        "analyst-a",
        "access-audit-event",
        access_event_id,
    )
    .await?;

    let revocation = revoke_pds_consent_grant(&state, "learner-a", grant_id).await?;
    assert_eq!(revocation.status(), StatusCode::NO_CONTENT);
    assert_pds_ledger_event(
        &state,
        "consent-grant-revoked",
        "learner-a",
        "consent-grant",
        &grant_id.to_string(),
    )
    .await?;

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

    let data_product_id = pds_uuid(71)?;
    let data_product = create_pds_data_product(
        &state,
        "learner-a",
        data_product_id,
        super::PdsDataProductSourceKind::DifferentialPrivacyRelease,
        transcript(61),
        "Attendance trend",
        "School-level BI dashboard",
    )
    .await?;
    assert_eq!(data_product.status(), StatusCode::NO_CONTENT);
    assert_pds_ledger_event(
        &state,
        "data-product-registered",
        "learner-a",
        "data-product",
        &data_product_id.to_string(),
    )
    .await?;

    let lifecycle_event_id = pds_uuid(72)?;
    let lifecycle_event = create_pds_lifecycle_event(
        &state,
        "learner-a",
        super::PdsLifecycleEventRequest {
            event_id: lifecycle_event_id,
            event_type: super::PdsLifecycleEventType::CorrectionRequested,
            resource_kind: super::PdsLifecycleResourceKind::XapiStatement,
            resource_id: pds_uuid(73)?.to_string(),
            reason: "Learner disputes completion timestamp".to_owned(),
        },
    )
    .await?;
    assert_eq!(lifecycle_event.status(), StatusCode::NO_CONTENT);
    assert_pds_ledger_event(
        &state,
        "lifecycle-event-registered",
        "learner-a",
        "lifecycle-event",
        &lifecycle_event_id.to_string(),
    )
    .await?;

    let policy_id = pds_uuid(74)?;
    let policy =
        create_pds_retention_policy(&state, "learner-a", pds_retention_policy_request(policy_id))
            .await?;
    assert_eq!(policy.status(), StatusCode::NO_CONTENT);
    assert_pds_ledger_event(
        &state,
        "retention-policy-registered",
        "learner-a",
        "retention-policy",
        &policy_id.to_string(),
    )
    .await?;

    let case_id = pds_uuid(75)?;
    let case_resource_id = pds_uuid(76)?.to_string();
    let opened_case = create_pds_lifecycle_case(
        &state,
        "learner-a",
        pds_lifecycle_case_request(
            case_id,
            super::PdsLifecycleCaseType::Correction,
            super::PdsLifecycleCaseStatus::Open,
            &case_resource_id,
            Some(policy_id),
            None,
            None,
        ),
    )
    .await?;
    assert_eq!(opened_case.status(), StatusCode::NO_CONTENT);
    assert_pds_ledger_event(
        &state,
        "lifecycle-case-opened",
        "learner-a",
        "lifecycle-case",
        &case_id.to_string(),
    )
    .await?;
    let transitioned_case = create_pds_lifecycle_case(
        &state,
        "learner-a",
        pds_lifecycle_case_request(
            case_id,
            super::PdsLifecycleCaseType::Correction,
            super::PdsLifecycleCaseStatus::UnderReview,
            &case_resource_id,
            Some(policy_id),
            None,
            None,
        ),
    )
    .await?;
    assert_eq!(transitioned_case.status(), StatusCode::NO_CONTENT);
    assert_pds_ledger_event(
        &state,
        "lifecycle-case-transitioned",
        "learner-a",
        "lifecycle-case",
        &case_id.to_string(),
    )
    .await?;

    let profile_id = pds_uuid(77)?;
    let profile = create_pds_governance_profile(
        &state,
        "public-admin",
        pds_governance_profile_request(profile_id),
    )
    .await?;
    assert_eq!(profile.status(), StatusCode::NO_CONTENT);
    assert_pds_ledger_event(
        &state,
        "governance-profile-registered",
        "public-admin",
        "governance-profile",
        &profile_id.to_string(),
    )
    .await?;
    assert_pds_ledger_event_count(&state, 9).await?;
    Ok(())
}

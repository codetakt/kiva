#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn pds_consent_grant_is_subject_owned_and_visible_to_grantee() -> Result<(), String> {
    let state = super::AppState::default();
    let grant_id = pds_uuid(1)?;
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

    let owner_list = pds_consent_grants(&state, "learner-a", None).await;
    assert_eq!(owner_list.status(), StatusCode::OK);
    let owner_body = response_json(owner_list).await?;
    assert_eq!(
        owner_body
            .pointer("/grants")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        owner_body.pointer("/grants/0/dataSubject"),
        Some(&json!("learner-a"))
    );

    let grantee_list = pds_consent_grants(&state, "analyst-a", Some(grant_id)).await;
    assert_eq!(grantee_list.status(), StatusCode::OK);
    let grantee_body = response_json(grantee_list).await?;
    assert_eq!(
        grantee_body.pointer("/grants/0/granteeSubject"),
        Some(&json!("analyst-a"))
    );
    Ok(())
}

#[tokio::test]
async fn pds_aggregate_access_uses_active_delegation_and_persists_audit() -> Result<(), String> {
    let state = super::AppState::default();
    let scenario = PdsDelegatedAccessScenario::aggregate_insights(pds_uuid(2)?);
    scenario.create_grant(&state).await?;

    let response = scenario
        .access_check(&state, "aggregate-insight", "read", None)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await?;
    assert_eq!(body.pointer("/decision"), Some(&json!("allow")));
    assert!(body.pointer("/denyReason").is_none());

    let audit = pds_access_audit_events(&state, "analyst-a", None).await;
    assert_eq!(audit.status(), StatusCode::OK);
    let audit_body = response_json(audit).await?;
    assert_eq!(
        audit_body
            .pointer("/events")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        audit_body.pointer("/events/0/decision"),
        Some(&json!("allow"))
    );
    assert_eq!(
        audit_body.pointer("/events/0/grantId"),
        Some(&json!(scenario.grant_id.to_string()))
    );
    assert!(audit_body.pointer("/events/0/statement").is_none());
    assert!(audit_body.pointer("/events/0/share").is_none());
    assert!(audit_body.pointer("/events/0/signature").is_none());
    assert!(audit_body.pointer("/events/0/aggregateValue").is_none());
    Ok(())
}

#[tokio::test]
async fn pds_revoked_grant_denies_delegated_access() -> Result<(), String> {
    let state = super::AppState::default();
    let scenario = PdsDelegatedAccessScenario::aggregate_insights(pds_uuid(3)?);
    scenario.create_grant(&state).await?;

    let foreign_revocation =
        revoke_pds_consent_grant(&state, scenario.grantee_token, scenario.grant_id).await?;
    assert_eq!(foreign_revocation.status(), StatusCode::FORBIDDEN);

    let revocation =
        revoke_pds_consent_grant(&state, scenario.owner_token, scenario.grant_id).await?;
    assert_eq!(revocation.status(), StatusCode::NO_CONTENT);

    let response = scenario
        .access_check(&state, "aggregate-insight", "read", None)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await?;
    assert_eq!(body.pointer("/decision"), Some(&json!("deny")));
    assert_eq!(body.pointer("/denyReason"), Some(&json!("consentRevoked")));
    Ok(())
}

#[tokio::test]
async fn pds_policy_still_blocks_analyst_raw_record_reads() -> Result<(), String> {
    let state = super::AppState::default();
    let scenario = PdsDelegatedAccessScenario::raw_records(pds_uuid(4)?);
    scenario.create_grant(&state).await?;

    let response = scenario
        .access_check(&state, "raw-learning-record", "read", None)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await?;
    assert_eq!(body.pointer("/decision"), Some(&json!("deny")));
    assert_eq!(
        body.pointer("/denyReason"),
        Some(&json!("rawRecordBoundary"))
    );
    Ok(())
}

#[tokio::test]
async fn pds_expired_consent_grant_denies_delegated_access() -> Result<(), String> {
    let state = super::AppState::default();
    let grant_id = pds_uuid(27)?;
    insert_expired_consent_grant(
        &state,
        grant_id,
        "learner-a",
        "analyst-a",
        super::PdsConsentScope::AggregateInsights,
    )
    .await?;

    let response = pds_access_check(
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
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await?;
    assert_eq!(body.pointer("/decision"), Some(&json!("deny")));
    assert_eq!(body.pointer("/denyReason"), Some(&json!("consentExpired")));
    Ok(())
}

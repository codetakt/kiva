#![allow(clippy::wildcard_imports)]

use super::*;

const LEARNER_PRINCIPAL_TOKEN: &str = "opaque-learner-principal-token";
const OTHER_LEARNER_TOKEN: &str = "opaque-other-learner-token";
const LEARNER_READ_ONLY_TOKEN: &str = "opaque-learner-read-only-token";
const LEARNER_GRANTEE_TOKEN: &str = "opaque-learner-grantee-token";

pub(in crate::tests) fn learner_principal_auth_config() -> super::AuthConfig {
    let mut auth = super::AuthConfig::default();
    auth.register_bearer_subject(LEARNER_PRINCIPAL_TOKEN, "learner-a");
    auth.register_bearer_scopes(
        LEARNER_PRINCIPAL_TOKEN,
        [
            OAuthScope::PdsRead,
            OAuthScope::PdsWrite,
            OAuthScope::PdsDelegate,
            OAuthScope::PdsRevoke,
        ],
    );
    auth.register_bearer_subject(OTHER_LEARNER_TOKEN, "learner-b");
    auth.register_bearer_scopes(
        OTHER_LEARNER_TOKEN,
        [
            OAuthScope::PdsRead,
            OAuthScope::PdsWrite,
            OAuthScope::PdsDelegate,
            OAuthScope::PdsRevoke,
        ],
    );
    auth.register_bearer_subject(LEARNER_READ_ONLY_TOKEN, "learner-a");
    auth.register_bearer_scopes(LEARNER_READ_ONLY_TOKEN, [OAuthScope::PdsRead]);
    auth.register_bearer_subject(LEARNER_GRANTEE_TOKEN, "analyst-a");
    auth.register_bearer_scopes(LEARNER_GRANTEE_TOKEN, [OAuthScope::PdsRead]);
    auth
}

pub(in crate::tests) async fn assert_learner_principal_subject_bound_flows(
    state: &super::AppState,
) -> Result<(), String> {
    let own_statement_id = pds_uuid(80)?;
    let foreign_statement_id = pds_uuid(81)?;
    store_pds_subject_statement(state, "learner-a", own_statement_id).await?;
    store_pds_subject_statement(state, "learner-b", foreign_statement_id).await?;

    let own_grant_id = pds_uuid(82)?;
    let own_grant = create_pds_consent_grant(
        state,
        LEARNER_PRINCIPAL_TOKEN,
        own_grant_id,
        "analyst-a",
        super::PdsConsentScope::AggregateInsights,
    )
    .await?;
    assert_eq!(own_grant.status(), StatusCode::NO_CONTENT);

    let foreign_grant_id = pds_uuid(83)?;
    let foreign_grant = create_pds_consent_grant(
        state,
        OTHER_LEARNER_TOKEN,
        foreign_grant_id,
        "analyst-a",
        super::PdsConsentScope::AggregateInsights,
    )
    .await?;
    assert_eq!(foreign_grant.status(), StatusCode::NO_CONTENT);

    let foreign_revocation =
        revoke_pds_consent_grant(state, LEARNER_PRINCIPAL_TOKEN, foreign_grant_id).await?;
    assert_eq!(foreign_revocation.status(), StatusCode::FORBIDDEN);

    let own_revocation =
        revoke_pds_consent_grant(state, LEARNER_PRINCIPAL_TOKEN, own_grant_id).await?;
    assert_eq!(own_revocation.status(), StatusCode::NO_CONTENT);

    let inventory = pds_record_inventory(state, LEARNER_PRINCIPAL_TOKEN).await;
    assert_eq!(inventory.status(), StatusCode::OK);
    let inventory_body = response_json(inventory).await?;
    assert_eq!(
        inventory_body.pointer("/dataSubject"),
        Some(&json!("learner-a"))
    );
    assert_eq!(
        inventory_body.pointer("/statementRecords/0/statementId"),
        Some(&json!(own_statement_id.to_string()))
    );
    assert_eq!(
        inventory_body
            .pointer("/statementRecords")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );

    let export = pds_metadata_exports(state, LEARNER_PRINCIPAL_TOKEN).await;
    assert_eq!(export.status(), StatusCode::OK);
    let export_body = response_json(export).await?;
    assert_eq!(
        export_body.pointer("/dataSubject"),
        Some(&json!("learner-a"))
    );
    assert_eq!(
        export_body.pointer("/consentGrants/0/grantId"),
        Some(&json!(own_grant_id.to_string()))
    );
    assert_eq!(
        export_body
            .pointer("/consentGrants")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_no_pds_raw_payload_fields(&export_body);

    let insufficient_scope = create_pds_consent_grant(
        state,
        LEARNER_READ_ONLY_TOKEN,
        pds_uuid(84)?,
        "analyst-a",
        super::PdsConsentScope::AggregateInsights,
    )
    .await?;
    assert_eq!(insufficient_scope.status(), StatusCode::FORBIDDEN);

    let missing_token = super::get_pds_record_inventory(
        pds_application_state(state),
        HeaderMap::new(),
        Query(BTreeMap::new()),
    )
    .await;
    assert_eq!(missing_token.status(), StatusCode::UNAUTHORIZED);

    let denied = pds_access_check(
        state,
        LEARNER_GRANTEE_TOKEN,
        super::PdsAccessCheckRequest {
            role: "analyst".to_owned(),
            resource: "aggregate-insight".to_owned(),
            action: "read".to_owned(),
            grant_id: Some(own_grant_id),
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
    Ok(())
}

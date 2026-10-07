#![allow(clippy::wildcard_imports)]

use super::*;

pub(in crate::tests) async fn assert_portability_export_scenario(
    state: &super::AppState,
) -> Result<(), String> {
    let first_id = pds_uuid(90)?;
    let second_id = pds_uuid(91)?;
    let foreign_id = pds_uuid(92)?;
    store_pds_subject_statement(state, "learner-a", first_id).await?;
    store_pds_subject_statement(state, "learner-a", second_id).await?;
    store_pds_subject_statement(state, "learner-b", foreign_id).await?;

    let first = pds_portability_exports(state, "learner-a", None, 1).await;
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = response_json(first).await?;
    assert_eq!(
        first_body.pointer("/dataSubject"),
        Some(&json!("learner-a"))
    );
    assert_eq!(
        first_body.pointer("/statements/0/statementId"),
        Some(&json!(first_id.to_string()))
    );
    assert_eq!(
        first_body.pointer("/statements/0/statement/result/score/raw"),
        Some(&json!(99))
    );
    let stored = state
        .xapi
        .statements
        .get(first_id)
        .await
        .map_err(|error| format!("stored Statement should be readable: {error:?}"))?
        .ok_or_else(|| "stored Statement should exist".to_owned())?;
    assert_eq!(
        first_body.pointer("/statements/0/statement"),
        Some(&stored.statement)
    );
    assert!(
        !first_body.to_string().contains(&foreign_id.to_string()),
        "foreign Statement identifiers must not cross the subject partition"
    );
    let first_audit = first_body
        .pointer("/auditReference")
        .and_then(Value::as_str)
        .ok_or_else(|| "first export should return an audit reference".to_owned())?;
    assert_pds_ledger_event(
        state,
        "portability-export-completed",
        "learner-a",
        "data-portability-export",
        first_audit,
    )
    .await?;

    let cursor = first_body
        .pointer("/nextCursor")
        .and_then(Value::as_str)
        .ok_or_else(|| "first export should return a continuation cursor".to_owned())?;
    let second = pds_portability_exports(state, "learner-a", Some(cursor), 1).await;
    assert_eq!(second.status(), StatusCode::OK);
    let second_body = response_json(second).await?;
    assert_eq!(
        second_body.pointer("/statements/0/statementId"),
        Some(&json!(second_id.to_string()))
    );
    assert!(second_body.pointer("/nextCursor").is_none());

    let voiding_id = pds_uuid(93)?;
    let stored_at = fixed_time() + Duration::seconds(1);
    let voiding = super::normalized_statement(
        json!({
            "actor": super::wire::pds::subject_agent("learner-a"),
            "verb": {"id": "http://adlnet.gov/expapi/verbs/voided"},
            "object": {
                "objectType": "StatementRef",
                "id": first_id,
            },
        }),
        voiding_id,
        stored_at,
        &super::authority_agent("pds-test"),
    )
    .map_err(|error| error.message)?;
    state
        .xapi
        .statements
        .put(
            voiding_id,
            super::StoredStatement::new(voiding, stored_at, BTreeMap::new()),
        )
        .await
        .map_err(|error| format!("voiding statement should store: {error:?}"))?;
    let with_voiding = pds_portability_exports(state, "learner-a", None, 100).await;
    assert_eq!(with_voiding.status(), StatusCode::OK);
    let with_voiding_body = response_json(with_voiding).await?;
    assert_eq!(
        with_voiding_body.pointer("/statements/0/voided"),
        Some(&json!(true))
    );
    assert_eq!(
        with_voiding_body.pointer("/statements/2/statementId"),
        Some(&json!(voiding_id.to_string()))
    );

    let rejected = pds_portability_exports(state, "learner-a", Some("invalid-cursor"), 1).await;
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        state
            .pds_repository()
            .ledger_event_kind_count_for_test("portability-export-rejected", "learner-a")
            .await?,
        1
    );
    assert_pds_ledger_event_count_at_least(state, 4).await?;
    Ok(())
}

#[tokio::test]
async fn learner_portability_export_is_subject_bound_paged_preserved_and_audited()
-> Result<(), String> {
    let state = super::AppState::default();
    assert_portability_export_scenario(&state).await
}

#[tokio::test]
async fn learner_portability_export_requires_exact_pds_read_scope() -> Result<(), String> {
    let mut auth = super::AuthConfig::default();
    auth.register_bearer_subject("portability-read", "learner-a");
    auth.register_bearer_scopes("portability-read", [OAuthScope::PdsRead]);
    auth.register_bearer_subject("portability-write-only", "learner-a");
    auth.register_bearer_scopes("portability-write-only", [OAuthScope::PdsWrite]);
    let state = memory_test_state_with_auth(auth);

    let missing = super::get_pds_data_portability_exports(
        pds_application_state(&state),
        HeaderMap::new(),
        Query(BTreeMap::new()),
    )
    .await;
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);

    let insufficient = pds_portability_exports(&state, "portability-write-only", None, 20).await;
    assert_eq!(insufficient.status(), StatusCode::FORBIDDEN);

    let allowed = pds_portability_exports(&state, "portability-read", None, 20).await;
    assert_eq!(allowed.status(), StatusCode::OK);
    Ok(())
}

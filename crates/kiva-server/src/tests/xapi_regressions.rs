#![allow(clippy::wildcard_imports)]

use super::*;

#[path = "xapi_regressions/admission.rs"]
mod admission;
#[path = "xapi_regressions/array_boundaries.rs"]
mod array_boundaries;
#[path = "xapi_regressions/attachment_boundaries.rs"]
mod attachment_boundaries;
#[path = "xapi_regressions/duration_boundaries.rs"]
mod duration_boundaries;
#[path = "xapi_regressions/evidence.rs"]
mod evidence;
#[path = "xapi_regressions/interactions.rs"]
mod interactions;
#[path = "xapi_regressions/locators.rs"]
mod locators;
#[path = "xapi_regressions/mime_boundaries.rs"]
mod mime_boundaries;
#[path = "xapi_regressions/numeric_boundaries.rs"]
mod numeric_boundaries;
#[path = "xapi_regressions/opaque_values.rs"]
mod opaque_values;
#[path = "xapi_regressions/pagination.rs"]
mod pagination;
#[path = "xapi_regressions/person.rs"]
mod person;
#[path = "xapi_regressions/representation.rs"]
mod representation;
#[path = "xapi_regressions/rsa_provider.rs"]
mod rsa_provider;
#[path = "xapi_regressions/signatures.rs"]
mod signatures;
#[path = "xapi_regressions/substatement_time.rs"]
mod substatement_time;
#[path = "xapi_regressions/versions.rs"]
mod versions;

#[tokio::test]
async fn memory_xapi_preserves_nul_strings_and_literal_escapes() -> Result<(), String> {
    assert_json_strings(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_preserves_nul_strings_and_literal_escapes() -> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or("PostgreSQL URL must be set")?;
    assert_json_strings(&state).await
}

async fn assert_json_strings(state: &AppState) -> Result<(), String> {
    for mode in 0..3 {
        let id = Uuid::new_v4();
        let mut value = statement(id);
        value["actor"]["name"] = json!("before\u{0000}after");
        if mode > 0 {
            value["actor"]
                .as_object_mut()
                .ok_or("actor")?
                .remove("mbox");
            value["actor"]["account"] = json!({"homePage": "https://example.org/accounts", "name": "account\u{0000}\\u0000"});
        }
        value["result"] = json!({"response": "\u{0000}\\u0000\u{007f}\u{0800}\u{10000}\u{10ffff}"});
        let submitted = if mode == 2 {
            json!([value.clone()])
        } else {
            value.clone()
        };
        let status = if mode == 0 {
            StatusCode::NO_CONTENT
        } else {
            StatusCode::OK
        };
        expect_status(
            submit(state, &submitted, mode != 0).await?,
            status,
            "NUL admission",
        )?;
        expect_status(
            submit(state, &submitted, mode != 0).await?,
            status,
            "identical NUL retry",
        )?;
        let stored = state
            .xapi
            .statements
            .get(id)
            .await
            .map_err(|e| format!("{e:?}"))?
            .ok_or("stored NUL Statement")?;
        assert_eq!(stored.statement["actor"]["name"], value["actor"]["name"]);
        assert_eq!(stored.statement["result"], value["result"]);
        let ifi = kiva_xapi::canonical_actor_ifi(&value["actor"]).ok_or("IFI")?;
        let metadata = state
            .xapi
            .statements
            .list_metadata_by_actor_ifi(&ifi)
            .await
            .map_err(|error| format!("{error:?}"))?;
        assert!(
            metadata
                .iter()
                .any(|row| row.statement_id == id && row.actor_ifi == ifi)
        );
        let exported = state
            .xapi
            .statements
            .list_portability_page_by_actor_ifi(&ifi, None, 100)
            .await
            .map_err(|error| format!("{error:?}"))?
            .ok_or("portability page")?;
        assert!(exported.iter().any(|row| row.statement_id == id));
        let listed = state
            .xapi
            .statements
            .list(None)
            .await
            .map_err(|e| format!("{e:?}"))?;
        assert!(listed.iter().any(|item| item == &stored.statement));
        value["result"]["response"] = json!("\\u0000");
        expect_status(
            submit(state, &value, false).await?,
            StatusCode::CONFLICT,
            "NUL differs from literal escape",
        )?;
    }
    Ok(())
}

#[tokio::test]
async fn memory_xapi_substatement_retries_preserve_stored_timestamps() -> Result<(), String> {
    admission::assert_retries(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
async fn memory_xapi_continuations_survive_collection_changes() -> Result<(), String> {
    pagination::assert_continuations(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_substatement_retries_preserve_stored_timestamps() -> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or("PostgreSQL URL must be set")?;
    admission::assert_retries(&state).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_continuations_survive_collection_changes() -> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or("PostgreSQL URL must be set")?;
    pagination::assert_continuations(&state).await
}

fn statement(id: Uuid) -> Value {
    json!({
        "id": id,
        "actor": {"mbox": "mailto:retry@example.org"},
        "verb": {"id": "https://example.org/experienced"},
        "object": {"id": "https://example.org/activity"}
    })
}

async fn submit(
    state: &AppState,
    value: &Value,
    post: bool,
) -> Result<axum::response::Response, String> {
    let context = State(state.xapi_http_context());
    let headers = confidential_headers_for("statement-writer-a");
    let body = json_body(value, "Statement")?;
    Ok(if post {
        post_statements(context, headers, empty_query(), body).await
    } else {
        let query = BTreeMap::from([(
            "statementId".to_owned(),
            value["id"].as_str().ok_or("missing ID")?.to_owned(),
        )]);
        put_statement(context, headers, Query(query), body).await
    })
}

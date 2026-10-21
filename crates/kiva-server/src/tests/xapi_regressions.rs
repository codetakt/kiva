#![allow(clippy::wildcard_imports)]

use super::*;

#[path = "xapi_regressions/admission.rs"]
mod admission;
#[path = "xapi_regressions/pagination.rs"]
mod pagination;

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

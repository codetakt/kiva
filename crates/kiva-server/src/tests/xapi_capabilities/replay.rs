//! A successful immutable replay must not introduce metadata work.
#![allow(clippy::wildcard_imports)]
use super::*;
use axum::{Router, routing::get};
use reqwest::Method;

#[tokio::test]
async fn memory_xapi_replay_does_not_schedule_metadata() -> Result<(), String> {
    verify(memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_replay_does_not_schedule_metadata() -> Result<(), String> {
    verify(
        postgres_test_state()
            .await?
            .ok_or("PostgreSQL fixture required")?,
    )
    .await
}

async fn verify(state: AppState) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let origin = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let host = Router::new().route(
        "/{activity}",
        get(|| async { axum::Json(json!({"name":{"en":"Owner metadata"}})) }),
    );
    let host_task = tokio::spawn(async move { axum::serve(listener, host).await });

    // First admission predates fetching being enabled. Replaying an existing
    // record after the configuration change still cannot mutate another Object.
    let original_server = Server::start(state).await?;
    let mut original = statement(Uuid::new_v4());
    original["object"] = json!({"id":format!("{origin}/existing")});
    original_server
        .receive(
            original_server
                .request(Method::POST, "/statements")
                .json(&original),
            StatusCode::OK,
        )
        .await?;
    let before = original_server
        .json(
            original_server
                .request(Method::GET, "/statements")
                .query(&[("statementId", original["id"].as_str().unwrap())]),
        )
        .await?;
    let server = Server::options(
        original_server.state.clone(),
        json!({"metadata_origins":[origin]}),
    )
    .await?;
    for (method, batch) in [
        (Method::PUT, false),
        (Method::POST, false),
        (Method::POST, true),
    ] {
        let request = server.request(method.clone(), "/statements");
        let request = if method == Method::PUT {
            request.query(&[("statementId", original["id"].as_str().unwrap())])
        } else {
            request
        };
        let body = if batch {
            json!([original])
        } else {
            original.clone()
        };
        let status = if method == Method::PUT {
            StatusCode::NO_CONTENT
        } else {
            StatusCode::OK
        };
        server.receive(request.json(&body), status).await?;
        assert!(
            server
                .state
                .xapi
                .capabilities
                .store
                .list("fetch:")
                .await
                .map_err(|e| e.message)?
                .is_empty(),
            "{method} replay created metadata work"
        );
        assert!(
            server
                .state
                .xapi
                .capabilities
                .store
                .list("metadata:")
                .await
                .map_err(|e| e.message)?
                .is_empty()
        );
        let after = server
            .json(
                server
                    .request(Method::GET, "/statements")
                    .query(&[("statementId", original["id"].as_str().unwrap())]),
            )
            .await?;
        assert_eq!(after, before);
    }

    // Rejected batches enqueue nothing; successful mixed batches enqueue only
    // newly inserted IDs, while retaining the response IDs for every input.
    let mut fresh = statement(Uuid::new_v4());
    fresh["object"] = json!({"id":format!("{origin}/new")});
    let mut conflict = original.clone();
    conflict["verb"]["id"] = json!("https://example.org/conflicting");
    server
        .receive(
            server
                .request(Method::POST, "/statements")
                .json(&json!([fresh, conflict])),
            StatusCode::CONFLICT,
        )
        .await?;
    assert!(
        server
            .state
            .xapi
            .capabilities
            .store
            .list("fetch:")
            .await
            .map_err(|e| e.message)?
            .is_empty()
    );
    server
        .receive(
            server
                .request(Method::GET, "/statements")
                .query(&[("statementId", fresh["id"].as_str().unwrap())]),
            StatusCode::NOT_FOUND,
        )
        .await?;
    let ids = server
        .json(
            server
                .request(Method::POST, "/statements")
                .json(&json!([original, fresh])),
        )
        .await?;
    assert_eq!(ids, json!([original["id"], fresh["id"]]));
    let jobs = server
        .state
        .xapi
        .capabilities
        .store
        .list("fetch:")
        .await
        .map_err(|e| e.message)?;
    assert_eq!(
        jobs.keys().cloned().collect::<Vec<_>>(),
        vec![format!("fetch:{origin}/new")]
    );
    let mut put_new = statement(Uuid::new_v4());
    put_new["object"] = json!({"id":format!("{origin}/put-new")});
    server
        .receive(
            server
                .request(Method::PUT, "/statements")
                .query(&[("statementId", put_new["id"].as_str().unwrap())])
                .json(&put_new),
            StatusCode::NO_CONTENT,
        )
        .await?;
    let jobs = server
        .state
        .xapi
        .capabilities
        .store
        .list("fetch:")
        .await
        .map_err(|e| e.message)?;
    assert_eq!(
        jobs.keys().cloned().collect::<Vec<_>>(),
        vec![
            format!("fetch:{origin}/new"),
            format!("fetch:{origin}/put-new")
        ]
    );
    evidence::observation(
        "product-capability",
        &json!({
            "capabilities":["resolvable-metadata"],"enabled":true,
            "replay_methods":["PUT","POST object","POST array"],
            "unchanged_exact_statement":true,"rejected_batch_has_no_work":true,
            "new_admission_jobs":jobs.keys().collect::<Vec<_>>()
        }),
    );
    host_task.abort();
    Ok(())
}

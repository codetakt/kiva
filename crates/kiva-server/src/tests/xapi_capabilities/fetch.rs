//! Independently controlled metadata host and actual consumer HTTP requests.
#![allow(clippy::wildcard_imports)]
use super::*;
use axum::{Router, response::IntoResponse, routing::get};
use reqwest::Method;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[tokio::test]
async fn memory_xapi_product_metadata_fetch() -> Result<(), String> {
    verify(memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_product_metadata_fetch() -> Result<(), String> {
    verify(
        postgres_test_state()
            .await?
            .ok_or("PostgreSQL fixture required")?,
    )
    .await
}

#[allow(clippy::literal_string_with_formatting_args)]
async fn verify(state: AppState) -> Result<(), String> {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let ready = Arc::new(AtomicBool::new(false));
    let forbidden_called = Arc::new(AtomicBool::new(false));
    let forbidden_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let forbidden_url = format!(
        "http://{}/private",
        forbidden_listener.local_addr().map_err(|e| e.to_string())?
    );
    let forbidden_observed = Arc::clone(&forbidden_called);
    let forbidden_host = Router::new().route(
        "/private",
        get(move || {
            forbidden_observed.store(true, Ordering::SeqCst);
            async { StatusCode::OK }
        }),
    );
    let forbidden_task =
        tokio::spawn(async move { axum::serve(forbidden_listener, forbidden_host).await });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let origin = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let logs = Arc::clone(&requests);
    let available = Arc::clone(&ready);
    let host=Router::new().route("/{kind}",get(move |axum::extract::Path(kind):axum::extract::Path<String>,headers:axum::http::HeaderMap| {
        let logs=Arc::clone(&logs);let available=Arc::clone(&available);
        let forbidden_url=forbidden_url.clone();
        async move {
            logs.lock().unwrap().push(json!({"path":kind,"accept":headers.get("accept").and_then(|h|h.to_str().ok()),"has_authorization":headers.contains_key("authorization")}));
            let definition=r#"{"name":{"en":"Owner name"},"extensions":{"https://example.org/owner":{"precise":9007199254740993,"nul":"\u0000"}}}"#;
            match kind.as_str() {
                "json" => (StatusCode::OK,[("content-type","application/json")],definition.to_owned()).into_response(),
                "html" => (StatusCode::OK,[("content-type","text/html")],format!("<script data-xapi-activity-definition type='application/json'>{definition}</script>")).into_response(),
                "alternate" => (StatusCode::OK,[("content-type","text/html")],"<link rel='alternate' type='application/json' href='/json'>").into_response(),
                "redirect" => (StatusCode::FOUND,[("location","/json")]).into_response(),
                "forbidden-redirect" => (StatusCode::FOUND,[("location",forbidden_url)]).into_response(),
                "oversize" => (StatusCode::OK,[("content-type","application/json")],format!("{{\"name\":{{\"en\":\"{}\"}}}}", "X".repeat(1000))).into_response(),
                "timeout" => {
                    tokio::time::sleep(std::time::Duration::from_millis(900)).await;
                    (StatusCode::OK,[("content-type","application/json")],definition.to_owned()).into_response()
                },
                "retry" if available.load(Ordering::SeqCst) => (StatusCode::OK,[("content-type","application/json")],definition.to_owned()).into_response(),
                "invalid" => (StatusCode::OK,[("content-type","application/json")],r#"{"name":17}"#).into_response(),
                _ => StatusCode::SERVICE_UNAVAILABLE.into_response(),
            }
        }
    }));
    let task = tokio::spawn(async move { axum::serve(listener, host).await });
    let server = Server::options(
        state,
        json!({"metadata_origins":[origin],"metadata_retry_seconds":1,"metadata_timeout_ms":500,"metadata_max_bytes":512}),
    )
    .await?;
    let mut originals = Vec::new();
    for kind in [
        "json",
        "html",
        "alternate",
        "redirect",
        "retry",
        "invalid",
        "forbidden-redirect",
        "oversize",
        "timeout",
    ] {
        let mut item = statement(Uuid::new_v4());
        item["object"] = json!({"id":format!("{origin}/{kind}"),"definition":{"name":{"en":"Submitted name","ja":"保持"},"description":{"en":"preserved description"}}});
        server
            .receive(
                server.request(Method::POST, "/statements").json(&item),
                StatusCode::OK,
            )
            .await?;
        originals.push(item);
    }
    for item in originals.iter().take(4) {
        assert_owner(&server, item).await?;
    }
    for item in originals.iter().skip(4) {
        let activity = server
            .json(
                server
                    .request(Method::GET, "/activities")
                    .query(&[("activityId", item["object"]["id"].as_str().unwrap())]),
            )
            .await?;
        assert_eq!(activity["definition"]["name"]["en"], "Submitted name");
    }
    ready.store(true, Ordering::SeqCst);
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    assert_owner(&server, &originals[4]).await?;
    // The serial worker has passed each failing source before a successful retry.
    // None may replace accepted metadata or reach the forbidden redirect target.
    for item in originals.iter().skip(5) {
        let activity = server
            .json(
                server
                    .request(Method::GET, "/activities")
                    .query(&[("activityId", item["object"]["id"].as_str().unwrap())]),
            )
            .await?;
        assert_eq!(activity["definition"]["name"]["en"], "Submitted name");
    }
    assert!(!forbidden_called.load(Ordering::SeqCst));
    let logs = requests.lock().unwrap().clone();
    assert!(
        logs.iter()
            .all(|r| r["accept"] == "application/json, */*" && r["has_authorization"] == false)
    );
    assert!(logs.iter().filter(|r| r["path"] == "retry").count() >= 2);
    for kind in ["invalid", "forbidden-redirect", "oversize", "timeout"] {
        assert!(
            logs.iter().any(|r| r["path"] == kind),
            "missing attempted source: {kind}"
        );
    }
    let reopened = Server::start(super::durability::reopen(&server.state).await?).await?;
    assert_owner(&reopened, &originals[0]).await?;
    for original in originals {
        let exact = server
            .json(
                server
                    .request(Method::GET, "/statements")
                    .query(&[("statementId", original["id"].as_str().unwrap())]),
            )
            .await?;
        assert_eq!(exact["object"], original["object"]);
    }
    evidence::observation(
        "product-capability",
        &json!({"capabilities":["resolvable-metadata"],"enabled":true,"host_requests":logs,"formats":["JSON","HTML embedded JSON","HTML alternate JSON"],"failed_fetch_does_not_reject_statement":true}),
    );
    task.abort();
    forbidden_task.abort();
    Ok(())
}

async fn assert_owner(server: &Server, original: &Value) -> Result<(), String> {
    for _ in 0..150 {
        let activity = server
            .json(
                server
                    .request(Method::GET, "/activities")
                    .query(&[("activityId", original["object"]["id"].as_str().unwrap())]),
            )
            .await?;
        if activity["definition"]["name"]["en"] == "Owner name" {
            assert_eq!(activity["definition"]["name"]["ja"], "保持");
            assert_eq!(
                activity["definition"]["description"]["en"],
                "preserved description"
            );
            assert_eq!(
                activity["definition"]["extensions"]["https://example.org/owner"]["precise"]
                    .as_u64(),
                Some(9_007_199_254_740_993)
            );
            assert_eq!(
                activity["definition"]["extensions"]["https://example.org/owner"]["nul"],
                "\0"
            );
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    Err(format!(
        "owner metadata was not incorporated for {}",
        original["object"]["id"]
    ))
}

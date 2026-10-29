//! Conditional deployment scope, separate from xAPI response obligations.
#![allow(clippy::wildcard_imports)]
use super::recording::{instant, json, watermark};
use super::*;
use sha2::{Digest, Sha256};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[tokio::test]
async fn memory_xapi_extension_hosting_and_retention_scope() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_extension_hosting_and_retention_scope() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

async fn scoped_response(
    client: &reqwest::Client,
    request: reqwest::RequestBuilder,
    role: &str,
) -> Result<(StatusCode, Vec<u8>), String> {
    let request = request.build().map_err(|e| e.to_string())?;
    let method = request.method().to_string();
    let url = request.url().to_string();
    let reply = client.execute(request).await.map_err(|e| e.to_string())?;
    let status = reply.status();
    let headers=reply.headers().iter().map(|(k,v)|json!({"name":k.as_str(),"value_hex":crate::core::response::hex_lower(v.as_bytes())})).collect::<Vec<_>>();
    let bytes = reply.bytes().await.map_err(|e| e.to_string())?.to_vec();
    evidence::observation(
        "conditional-scope-response",
        &json!({"role":role,"method":method,"url":url,"status":status.as_u16(),"headers":headers,"body_hex":crate::core::response::hex_lower(&bytes),"grading":"configured scope observation, not an invented IEEE status for an undefined operation or external host"}),
    );
    Ok((status, bytes))
}

async fn xapi(request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let reply = evidence::received(
        request
            .bearer_auth("authority-a")
            .header("X-Experience-API-Version", "2.0.0")
            .send()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await?;
    watermark(&reply)?;
    Ok(reply)
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    let mut state = state.clone();
    let verifier_calls = Arc::new(Mutex::new(Vec::new()));
    state.auth = AuthConfig::with_token_verifier(
        true,
        Arc::new(authority::Verifier {
            calls: Arc::clone(&verifier_calls),
        }),
    );
    let source = include_str!("../../core/paths.rs");
    let paths=source.split('"').filter(|part|part.starts_with('/')).map(|path|json!({"path":path,"first_segment":path.split('/').nth(1),"uses_extensions_prefix":path.starts_with("/extensions/"),
        "reason":"existing non-xAPI application surfaces and local conformance diagnostic retain their separate documented namespaces; prefix recommendation is advisory"})).collect::<Vec<_>>();
    let policy = json!({"route_inventory":paths,"paths_source_sha256":format!("{:x}",Sha256::digest(source.as_bytes())),
        "authentication":"external local verifier; subject-a is an authorized writer; no required universal authentication scheme",
        "retention":{"automatic_statement_worker":false,"configured_event":null,"conditional_event_disposition":"not applicable: this deployment configures no Statement deletion/retention event; no unlimited future retention promise"},
        "attachment_hosting":{"kiva_fileurl_host":false,"fixture":"independent local external host; allowed credential before expiry200, refused403, after declared expiry410","host_grant_sha256":format!("{:x}",Sha256::digest(b"fixture-host-grant"))}});
    evidence::observation(
        "deployment-scope-policy",
        &json!({"sha256":format!("{:x}",Sha256::digest(policy.to_string().as_bytes())),"policy":policy}),
    );
    // Scope-only calls use the real router with original response headers. They
    // are separately captured so an undefined path is not graded as an xAPI
    // operation by a collector that checks every standard resource response.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let scope_base = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let router = crate::router_with_state(&state);
    let scope_server = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    for (method, path, role) in [
        (
            reqwest::Method::GET,
            "/__kiva/conformance/rate-limit",
            "documented diagnostic extension",
        ),
        (
            reqwest::Method::GET,
            "/__kiva/undefined-scope-probe",
            "undefined resource",
        ),
        (reqwest::Method::PATCH, "/statements", "undefined method"),
    ] {
        let (_, body) = scoped_response(
            &client,
            client
                .request(method, format!("{scope_base}{path}"))
                .bearer_auth("authority-a")
                .header("X-Experience-API-Version", "2.0.0"),
            role,
        )
        .await?;
        if role == "documented diagnostic extension" {
            assert_eq!(
                serde_json::from_slice::<Value>(&body).map_err(|e| e.to_string())?["fixture"],
                "kiva-conformance-rate-limit"
            );
        }
    }
    let expired = Arc::new(AtomicBool::new(false));
    let hits = Arc::new(AtomicUsize::new(0));
    let host = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let file_url = format!(
        "http://{}/attachment",
        host.local_addr().map_err(|e| e.to_string())?
    );
    let bytes = b"controlled attachment\0\r\n";
    let host_router = axum::Router::new().route(
        "/attachment",
        axum::routing::get({
            let expired = Arc::clone(&expired);
            let hits = Arc::clone(&hits);
            move |headers: HeaderMap| {
                let expired = Arc::clone(&expired);
                let hits = Arc::clone(&hits);
                async move {
                    hits.fetch_add(1, Ordering::SeqCst);
                    if expired.load(Ordering::SeqCst) {
                        (StatusCode::GONE, Vec::new())
                    } else if headers.get("authorization").and_then(|v| v.to_str().ok())
                        != Some("Bearer fixture-host-grant")
                    {
                        (StatusCode::FORBIDDEN, Vec::new())
                    } else {
                        (StatusCode::OK, bytes.to_vec())
                    }
                }
            }
        }),
    );
    let host_server = tokio::spawn(async move { axum::serve(host, host_router).await });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let base = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let router = evidence::router(&state);
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let id = Uuid::new_v4();
    let mut value = statement(id);
    value["attachments"] = json!([{"usageType":"urn:example:scope-file","display":{"en":"external host"},"contentType":"application/octet-stream","length":bytes.len(),"sha2":format!("{:x}",Sha256::digest(bytes)),"fileUrl":file_url}]);
    state
        .xapi
        .statements
        .clock()
        .set(instant("2020-01-01T00:00:00Z")?);
    assert_eq!(
        xapi(
            client
                .put(format!("{base}/statements?statementId={id}"))
                .json(&value)
        )
        .await?
        .status(),
        StatusCode::NO_CONTENT
    );
    let reply =
        xapi(client.get(format!("{base}/statements?statementId={id}&format=exact"))).await?;
    assert_eq!(reply.status(), StatusCode::OK);
    let original = json(reply).await?;
    assert_eq!(original["attachments"], value["attachments"]);
    assert_eq!(
        hits.load(Ordering::SeqCst),
        0,
        "admission/readback do not require external hosting or fetching"
    );
    let baseline = state
        .xapi
        .statements
        .settled_snapshot()
        .await
        .map_err(|e| format!("{e:?}"))?;
    for expiry in [false, true] {
        expired.store(expiry, Ordering::SeqCst);
        evidence::observation(
            "host-expiry-event",
            &json!({"expired":expiry,"clock":"controlled external host policy switch; not Statement.timestamp or an LRS retention event"}),
        );
        for credential in [
            None,
            Some("refused-host-credential"),
            Some("fixture-host-grant"),
        ] {
            let mut request = client.get(&file_url);
            if let Some(credential) = credential {
                request = request.bearer_auth(credential);
            }
            let (status, body) = scoped_response(
                &client,
                request,
                "independently controlled external fileUrl host",
            )
            .await?;
            assert_eq!(
                status,
                if expiry {
                    StatusCode::GONE
                } else if credential == Some("fixture-host-grant") {
                    StatusCode::OK
                } else {
                    StatusCode::FORBIDDEN
                }
            );
            if status == StatusCode::OK {
                assert_eq!(body, bytes);
            }
        }
    }
    state
        .xapi
        .statements
        .clock()
        .set(instant("2030-01-01T00:00:00Z")?);
    let previous_hits = hits.load(Ordering::SeqCst);
    let reply =
        xapi(client.get(format!("{base}/statements?statementId={id}&format=exact"))).await?;
    assert_eq!(reply.status(), StatusCode::OK);
    assert_eq!(
        json(reply).await?,
        original,
        "external URL expiry is not Statement deletion, replay mutation or voiding"
    );
    assert_eq!(
        state
            .xapi
            .statements
            .settled_snapshot()
            .await
            .map_err(|e| format!("{e:?}"))?,
        baseline
    );
    assert_eq!(hits.load(Ordering::SeqCst), previous_hits);
    evidence::observation(
        "scope-authentication-calls",
        &json!(*verifier_calls.lock().map_err(|e| e.to_string())?),
    );
    host_server.abort();
    scope_server.abort();
    server.abort();
    Ok(())
}

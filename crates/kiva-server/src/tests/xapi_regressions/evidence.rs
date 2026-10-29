//! Optional native-control trace. This layer is compiled only in the test binary.

use crate::core::{response::hex_lower, state::AppState};
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::Request,
    middleware::{self, Next},
    response::Response,
};
use reqwest::ResponseBuilderExt;
use serde_json::json;
use sha2::{Digest, Sha256};

const TRACE_ID: &str = "x-kiva-native-observation-id";

pub(super) fn observation(kind: &str, value: &serde_json::Value) {
    let Some(directory) = std::env::var_os("KIVA_NATIVE_HTTP_TRACE") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory).join("controls");
    std::fs::create_dir_all(&directory).expect("native control directory");
    let record = json!({"test":std::thread::current().name(),"kind":kind,"observation":value});
    std::fs::write(
        directory.join(format!("{}.json", uuid::Uuid::new_v4())),
        serde_json::to_vec(&record).expect("native control record"),
    )
    .expect("retain native control");
}

pub(super) fn router(state: &AppState) -> Router {
    let router = crate::router_with_state(state);
    let Some(directory) = std::env::var_os("KIVA_NATIVE_HTTP_TRACE") else {
        return router;
    };
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).expect("native trace directory");
    let test = std::thread::current()
        .name()
        .unwrap_or("unnamed-native-test")
        .to_owned();
    router.layer(middleware::from_fn(move |request: Request, next: Next| {
        let directory = directory.clone();
        let test = test.clone();
        async move { capture(request, next, directory, test).await }
    }))
}

fn headers(values: &axum::http::HeaderMap) -> Vec<serde_json::Value> {
    values.iter().map(|(name,value)| if name == axum::http::header::AUTHORIZATION {
        json!({"name":name.as_str(),"credential_sha256":hex_lower(&Sha256::digest(value.as_bytes()))})
    } else { json!({"name":name.as_str(),"value_hex":hex_lower(value.as_bytes())}) }).collect()
}

async fn capture(
    request: Request,
    next: Next,
    directory: std::path::PathBuf,
    test: String,
) -> Response {
    let (parts, body) = request.into_parts();
    let body = to_bytes(body, usize::MAX)
        .await
        .expect("native request body");
    let record = json!({"test":test,"boundary":"server application; HTTP framing is observed separately by the client","request":{"method":parts.method.as_str(),"uri":parts.uri.to_string(),
        "headers":headers(&parts.headers),"body_hex":hex_lower(&body)}});
    let identifier = uuid::Uuid::new_v4().to_string();
    let path = directory.join(format!("{identifier}.json"));
    std::fs::write(
        &path,
        serde_json::to_vec(&record).expect("native request record"),
    )
    .expect("retain native request");
    let response = next.run(Request::from_parts(parts, Body::from(body))).await;
    let (mut parts, body) = response.into_parts();
    let body = to_bytes(body, usize::MAX)
        .await
        .expect("native response body");
    let mut record = record;
    record["response"] = json!({"status":parts.status.as_u16(),"headers":headers(&parts.headers),"body_hex":hex_lower(&body)});
    std::fs::write(
        &path,
        serde_json::to_vec(&record).expect("native response record"),
    )
    .expect("retain native response");
    // Correlation only: no xAPI/status/body behavior is changed. The client
    // helper removes this test-only header before assertions/snapshots.
    parts
        .headers
        .insert(TRACE_ID, identifier.parse().expect("trace UUID header"));
    Response::from_parts(parts, Body::from(body))
}

pub(super) async fn received(mut reply: reqwest::Response) -> Result<reqwest::Response, String> {
    let Some(directory) = std::env::var_os("KIVA_NATIVE_HTTP_TRACE") else {
        return Ok(reply);
    };
    let identifier = reply
        .headers_mut()
        .remove(TRACE_ID)
        .ok_or("native response lacks correlation")?;
    let identifier = uuid::Uuid::parse_str(identifier.to_str().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let path = std::path::PathBuf::from(directory).join(format!("{identifier}.json"));
    let mut record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let status = reply.status();
    let version = reply.version();
    let url = reply.url().clone();
    let received_headers = reply.headers().clone();
    record["received_response"] = json!({"boundary":"reqwest HTTP client, after HTTP framing","url":url.as_str(),"status":status.as_u16(),"headers":headers(&received_headers)});
    std::fs::write(
        &path,
        serde_json::to_vec(&record).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let body = reply.bytes().await.map_err(|e| e.to_string())?;
    record["received_response"]["body_hex"] = json!(hex_lower(&body));
    std::fs::write(
        &path,
        serde_json::to_vec(&record).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut builder = axum::http::Response::builder()
        .status(status)
        .version(version)
        .url(url);
    *builder.headers_mut().ok_or("response builder headers")? = received_headers;
    Ok(builder.body(body).map_err(|e| e.to_string())?.into())
}

/// Retain an independently parsed raw socket response before any HTTP client
/// discards a HEAD payload or decodes a content coding.
pub(super) fn received_raw(
    url: &str,
    status: u16,
    values: &axum::http::HeaderMap,
    body: &[u8],
    wire: &[u8],
) -> Result<(), String> {
    let Some(directory) = std::env::var_os("KIVA_NATIVE_HTTP_TRACE") else {
        return Ok(());
    };
    let id = uuid::Uuid::parse_str(
        values
            .get(TRACE_ID)
            .ok_or("raw trace ID")?
            .to_str()
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let path = std::path::PathBuf::from(directory).join(format!("{id}.json"));
    let mut record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let mut received_headers = values.clone();
    received_headers.remove(TRACE_ID);
    record["received_response"] = json!({"boundary":"raw TCP HTTP/1.1 bytes, before content decoding; HEAD body is the actual trailing wire bytes", "url":url,"status":status,"headers":headers(&received_headers),"body_hex":hex_lower(body),"wire_hex":hex_lower(wire)});
    std::fs::write(
        path,
        serde_json::to_vec(&record).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

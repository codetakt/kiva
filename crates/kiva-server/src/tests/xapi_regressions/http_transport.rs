//! Raw HTTP boundary controls, independent of reqwest's HEAD/body handling.
#![allow(clippy::wildcard_imports)]
use super::*;
use std::fmt::Write as _;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Reply {
    status: u16,
    headers: HeaderMap,
    body: Vec<u8>,
}

fn mandatory_head(reply: &Reply, statements: bool) -> bool {
    reply.body.is_empty()
        && reply
            .headers
            .get("x-experience-api-version")
            .is_some_and(|v| v == "2.0.0")
        && (!statements
            || reply
                .headers
                .get("x-experience-api-consistent-through")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| recording::instant(v).is_ok()))
}

async fn transfer(address: std::net::SocketAddr, request: &[u8]) -> Result<Vec<u8>, String> {
    let mut socket = tokio::net::TcpStream::connect(address)
        .await
        .map_err(|e| e.to_string())?;
    socket.write_all(request).await.map_err(|e| e.to_string())?;
    let mut response = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        socket.read_to_end(&mut response),
    )
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    Ok(response)
}

fn parse(wire: &[u8], head: bool) -> Result<Reply, String> {
    let end = wire
        .windows(4)
        .position(|v| v == b"\r\n\r\n")
        .ok_or("HTTP header terminator")?;
    let text = std::str::from_utf8(&wire[..end]).map_err(|e| e.to_string())?;
    let mut lines = text.split("\r\n");
    let status = lines
        .next()
        .ok_or("status line")?
        .strip_prefix("HTTP/1.1 ")
        .ok_or("HTTP/1.1")?
        .split(' ')
        .next()
        .ok_or("status")?
        .parse::<u16>()
        .map_err(|e| e.to_string())?;
    let mut headers = HeaderMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').ok_or("header colon")?;
        headers.append(
            axum::http::HeaderName::from_bytes(name.as_bytes()).map_err(|e| e.to_string())?,
            axum::http::HeaderValue::from_str(value.trim()).map_err(|e| e.to_string())?,
        );
    }
    let tail = &wire[end + 4..];
    let body = if head {
        tail.to_vec()
    } else if headers
        .get("transfer-encoding")
        .is_some_and(|v| v == "chunked")
    {
        assert!(!headers.contains_key("content-length"));
        let mut rest = tail;
        let mut result = Vec::new();
        loop {
            let end = rest
                .windows(2)
                .position(|v| v == b"\r\n")
                .ok_or("chunk line")?;
            let line = std::str::from_utf8(&rest[..end]).map_err(|e| e.to_string())?;
            let size = usize::from_str_radix(line.split(';').next().ok_or("chunk size")?, 16)
                .map_err(|e| e.to_string())?;
            rest = &rest[end + 2..];
            if size == 0 {
                assert_eq!(rest, b"\r\n", "no trailers in this fixture");
                break;
            }
            let payload = rest.get(..size).ok_or("truncated chunk")?;
            result.extend_from_slice(payload);
            rest = rest
                .get(size..)
                .and_then(|v| v.strip_prefix(b"\r\n"))
                .ok_or("chunk delimiter")?;
        }
        result
    } else {
        if let Some(length) = headers.get("content-length") {
            assert_eq!(
                length
                    .to_str()
                    .map_err(|e| e.to_string())?
                    .parse::<usize>()
                    .map_err(|e| e.to_string())?,
                tail.len()
            );
        }
        tail.to_vec()
    };
    Ok(Reply {
        status,
        headers,
        body,
    })
}

async fn exchange(
    address: std::net::SocketAddr,
    method: &str,
    path: &str,
    extra: &str,
    body: &[u8],
    chunked: bool,
) -> Result<Reply, String> {
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nx-eXpErIeNcE-aPi-vErSiOn: 2.0.0\r\n{extra}"
    );
    if chunked {
        request.push_str("Transfer-Encoding: chunked\r\n\r\n");
    } else {
        write!(request, "Content-Length: {}\r\n\r\n", body.len()).map_err(|e| e.to_string())?;
    }
    let mut wire = request.into_bytes();
    if chunked {
        // Splits UTF-8 characters across chunks; chunk sizes count octets.
        for chunk in body.chunks(7) {
            wire.extend_from_slice(format!("{:X}\r\n", chunk.len()).as_bytes());
            wire.extend_from_slice(chunk);
            wire.extend_from_slice(b"\r\n");
        }
        wire.extend_from_slice(b"0\r\n\r\n");
    } else {
        wire.extend_from_slice(body);
    }
    evidence::observation(
        "raw-http-request",
        &json!({"method":method,"path":path,"wire_hex":crate::core::response::hex_lower(&wire),"body_octets":body.len(),"chunked":chunked}),
    );
    let response = transfer(address, &wire).await?;
    let reply = parse(&response, method == "HEAD")?;
    evidence::received_raw(
        &format!("http://{address}{path}"),
        reply.status,
        &reply.headers,
        &reply.body,
        &response,
    )?;
    assert_eq!(reply.headers["x-experience-api-version"], "2.0.0");
    if path.starts_with("/statements") {
        recording::instant(
            reply.headers["x-experience-api-consistent-through"]
                .to_str()
                .map_err(|e| e.to_string())?,
        )?;
    }
    if method == "HEAD" {
        assert!(
            mandatory_head(&reply, path.starts_with("/statements")),
            "mandatory raw HEAD obligations"
        );
    }
    Ok(reply)
}

#[tokio::test]
async fn raw_head_oracle_keeps_metadata_advisory_and_body_headers_mandatory() -> Result<(), String>
{
    for path in [
        "/statements",
        "/agents",
        "/activities",
        "/about",
        "/agents/profile",
        "/activities/profile",
        "/activities/state",
    ] {
        for contrast in [
            "matching",
            "omitted-metadata",
            "different-metadata",
            "body",
            "version",
            "watermark",
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .map_err(|e| e.to_string())?;
            let address = listener.local_addr().map_err(|e| e.to_string())?;
            let peer = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.expect("oracle peer");
                let mut request = Vec::new();
                loop {
                    let mut byte = [0];
                    socket.read_exact(&mut byte).await.expect("HEAD request");
                    request.push(byte[0]);
                    if request.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                assert!(request.starts_with(b"HEAD "));
                let mut wire =
                    "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 123\r\n".to_owned();
                if contrast != "version" {
                    wire.push_str("X-Experience-API-Version: 2.0.0\r\n");
                }
                if path == "/statements" && contrast != "watermark" {
                    wire.push_str("X-Experience-API-Consistent-Through: 2020-01-01T00:00:00Z\r\n");
                }
                if contrast == "matching" {
                    wire.push_str("ETag: \"known-get-tag\"\r\nLast-Modified: Wed, 01 Jan 2020 00:00:00 GMT\r\nContent-Type: application/json\r\n");
                }
                if contrast == "different-metadata" {
                    wire.push_str("ETag: \"different-tag\"\r\nLast-Modified: Thu, 02 Jan 2020 00:00:00 GMT\r\nContent-Type: text/plain\r\n");
                }
                wire.push_str("\r\n");
                if contrast == "body" {
                    wire.push_str("forbidden HEAD body");
                }
                socket
                    .write_all(wire.as_bytes())
                    .await
                    .expect("peer response");
            });
            let request =
                format!("HEAD {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
            let reply = parse(&transfer(address, request.as_bytes()).await?, true)?;
            assert_eq!(
                mandatory_head(&reply, path == "/statements"),
                !matches!(contrast, "body" | "version")
                    && (contrast != "watermark" || path != "/statements"),
                "{path}/{contrast}"
            );
            peer.await.map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tokio::test]
async fn memory_xapi_raw_http_framing_and_header_contexts() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_raw_http_framing_and_header_contexts() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let address = listener.local_addr().map_err(|e| e.to_string())?;
    let router = evidence::router(state);
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let mut statement_path = String::new();
    for chunked in [false, true] {
        let mut value = statement(Uuid::new_v4());
        value["actor"]["name"] = json!("日本語の名前 <b>literal</b>");
        value["verb"]["display"] = json!({"ja":"学習した","en":"learned"});
        value["object"]["definition"] =
            json!({"name":{"en":"English only"},"description":{"ja":"説明","en":"description"}});
        let path = format!(
            "/statements?statementId={}",
            value["id"].as_str().ok_or("id")?
        );
        assert_eq!(
            exchange(
                address,
                "PUT",
                &path,
                "cOnTeNt-TyPe: application/json; charset=UTF-8\r\n",
                value.to_string().as_bytes(),
                chunked
            )
            .await?
            .status,
            204
        );
        for accept in [
            "",
            "Accept: application/json\r\nAccept-Encoding: identity\r\n",
            "Accept-Encoding: gzip, identity;q=0.5\r\n",
        ] {
            let reply = exchange(
                address,
                "GET",
                &format!("{path}&format=exact"),
                accept,
                b"",
                false,
            )
            .await?;
            assert_eq!(reply.status, 200);
            assert!(
                !reply.headers.contains_key("content-encoding"),
                "configured Kiva has identity coding only"
            );
            let result: Value = serde_json::from_slice(&reply.body).map_err(|e| e.to_string())?;
            assert_eq!(result["actor"], value["actor"]);
            assert_eq!(result["object"], value["object"]);
        }
        let reply = exchange(
            address,
            "GET",
            &format!("{path}&format=canonical"),
            "aCcEpT-lAnGuAgE: ja, en;q=0.5\r\n",
            b"",
            false,
        )
        .await?;
        let result: Value = serde_json::from_slice(&reply.body).map_err(|e| e.to_string())?;
        assert_eq!(result["verb"]["display"], json!({"ja":"学習した"}));
        assert_eq!(
            result["object"]["definition"]["name"],
            json!({"en":"English only"})
        );
        assert_eq!(
            result["object"]["definition"]["description"],
            json!({"ja":"説明"})
        );
        statement_path = path;
    }
    let agent = json!({"mbox":"mailto:transport@example.org"}).to_string();
    let mut paths = vec![statement_path, "/about".to_owned()];
    for (resource, query) in [
        ("/agents", vec![("agent", agent.as_str())]),
        (
            "/activities",
            vec![("activityId", "https://example.org/http")],
        ),
        (
            "/agents/profile",
            vec![("agent", agent.as_str()), ("profileId", "wire")],
        ),
        (
            "/activities/profile",
            vec![
                ("activityId", "https://example.org/http"),
                ("profileId", "wire"),
            ],
        ),
        (
            "/activities/state",
            vec![
                ("activityId", "https://example.org/http"),
                ("agent", agent.as_str()),
                ("stateId", "wire"),
            ],
        ),
    ] {
        let url = reqwest::Url::parse_with_params(&format!("http://{address}{resource}"), query)
            .map_err(|e| e.to_string())?;
        let path = format!("{}?{}", url.path(), url.query().ok_or("query")?);
        if resource.contains("profile") || resource.ends_with("state") {
            assert_eq!(
                exchange(
                    address,
                    "PUT",
                    &path,
                    "Content-Type: application/json; charset=UTF-8\r\n",
                    br#"{"value":"document"}"#,
                    true
                )
                .await?
                .status,
                204
            );
        }
        paths.push(path);
    }
    assert_eq!(paths.len(), 7);
    for path in paths {
        let get = exchange(address, "GET", &path, "", b"", false).await?;
        let head = exchange(address, "HEAD", &path, "", b"", false).await?;
        assert_eq!(get.status, 200);
        assert_eq!(head.status, 200);
        evidence::observation(
            "head-entity-metadata-advisory",
            &json!({"path":path,"strength":"SHOULD","headers":(["etag","last-modified","content-type","content-length"].map(|name|json!({"name":name,"equal":get.headers.get(name)==head.headers.get(name),"get":get.headers.get(name).and_then(|v|v.to_str().ok()),"head":head.headers.get(name).and_then(|v|v.to_str().ok())}))),"mandatory":"version/applicable Consistent-Through plus actual zero body independently checked"}),
        );
    }
    evidence::observation(
        "http-coding-policy",
        &json!({"selected":"identity only","gzip_offer":"identity fallback remains accepted","unsupported_optional_compression":"not required","boundary":"raw socket response retained before decoding; no automatic HEAD discard"}),
    );
    server.abort();
    Ok(())
}

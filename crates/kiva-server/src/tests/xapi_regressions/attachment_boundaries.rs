#![allow(clippy::wildcard_imports)]
use super::*;
use sha2::{Digest, Sha224, Sha256, Sha384, Sha512};

#[tokio::test]
async fn memory_xapi_sha2_attachment_partitions() -> Result<(), String> {
    verify_sha2(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_sha2_attachment_partitions() -> Result<(), String> {
    verify_sha2(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

fn unhex(value: &str) -> Result<Vec<u8>, String> {
    (0..value.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&value[at..at + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

fn hash(algorithm: &str, bytes: &[u8]) -> String {
    use crate::core::response::hex_lower;
    match algorithm {
        "sha224" => hex_lower(&Sha224::digest(bytes)),
        "sha256" => hex_lower(&Sha256::digest(bytes)),
        "sha384" => hex_lower(&Sha384::digest(bytes)),
        "sha512" => hex_lower(&Sha512::digest(bytes)),
        _ => panic!("unrecognized fixed test algorithm"),
    }
}

pub(super) fn encoded(value: &Value, bytes: &[u8], digest: &str, post: bool) -> Vec<u8> {
    let value = if post { json!([value]) } else { value.clone() };
    let mut body = format!("--sha2-control\r\nContent-Type: application/json\r\n\r\n{value}\r\n--sha2-control\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: binary\r\nX-Experience-API-Hash: {digest}\r\n\r\n").into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n--sha2-control--\r\n");
    body
}

// Independent, bounded output parser for these two-part controls. It does not
// call the production MIME decoder, trim payload bytes, or assume text data.
pub(super) async fn decoded(reply: reqwest::Response) -> Result<Vec<(String, Vec<u8>)>, String> {
    let media = reply.headers()["content-type"]
        .to_str()
        .map_err(|e| e.to_string())?;
    assert!(media.starts_with("multipart/mixed;"));
    let boundary = media
        .split("boundary=")
        .nth(1)
        .ok_or("boundary")?
        .trim_matches('"');
    let delimiter = format!("\r\n--{boundary}").into_bytes();
    let first = format!("--{boundary}\r\n").into_bytes();
    let body = reply.bytes().await.map_err(|e| e.to_string())?;
    let mut rest = body
        .strip_prefix(first.as_slice())
        .ok_or("opening delimiter")?;
    let mut parts = Vec::new();
    loop {
        let end = rest
            .windows(4)
            .position(|s| s == b"\r\n\r\n")
            .ok_or("part header terminator")?;
        let headers = String::from_utf8(rest[..end].to_vec()).map_err(|e| e.to_string())?;
        rest = &rest[end + 4..];
        let end = rest
            .windows(delimiter.len())
            .position(|s| s == delimiter)
            .ok_or("part boundary")?;
        parts.push((headers, rest[..end].to_vec()));
        rest = &rest[end + delimiter.len()..];
        if rest == b"--\r\n" {
            break;
        }
        rest = rest.strip_prefix(b"\r\n").ok_or("delimiter line ending")?;
    }
    Ok(parts)
}

#[allow(clippy::too_many_lines)]
async fn verify_sha2(state: &AppState) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let base = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let router = evidence::router(state);
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../tests/conformance/fixtures/sha2-integration.json"
    ))
    .map_err(|e| e.to_string())?;
    for vector in fixture["vectors"].as_array().ok_or("vectors")? {
        let algorithm = vector["algorithm"].as_str().ok_or("algorithm")?;
        let bytes = unhex(vector["bytes_hex"].as_str().ok_or("bytes")?)?;
        let digest = vector["digest"].as_str().ok_or("digest")?;
        assert_eq!(
            hash(algorithm, &bytes),
            digest,
            "independent hashlib known answer"
        );
        assert_eq!(json!(bytes.len()), vector["length"]);
        for post in [false, true] {
            for partition in ["file-url", "raw", "wrong-raw", "wrong-hash", "wrong-length"] {
                let id = Uuid::new_v4();
                let mut value = statement(id);
                let mut declared = digest.to_owned();
                let mut raw = bytes.clone();
                if partition == "wrong-hash" {
                    declared.replace_range(..1, if &digest[..1] == "0" { "1" } else { "0" });
                }
                if partition == "wrong-raw" {
                    raw[0] ^= 1;
                }
                value["attachments"] = json!([{"usageType":"https://example.org/sha2","display":{"en":"independent digest"},"description":{"en":"original"},"contentType":"application/octet-stream","length":bytes.len()+usize::from(partition=="wrong-length"),"sha2":declared}]);
                let request = if post {
                    client.post(format!("{base}/statements"))
                } else {
                    client.put(format!("{base}/statements?statementId={id}"))
                };
                let request = if partition == "file-url" {
                    value["attachments"][0]["fileUrl"] =
                        json!("https://example.invalid/not-fetched");
                    request.json(&if post { json!([value]) } else { value.clone() })
                } else {
                    request
                        .header("Content-Type", "multipart/mixed; boundary=sha2-control")
                        .body(encoded(&value, &raw, &declared, post))
                };
                let reply = receive(request).await?;
                if partition.starts_with("wrong-") {
                    assert_eq!(
                        reply.status(),
                        StatusCode::BAD_REQUEST,
                        "{algorithm}/{partition}/POST={post}"
                    );
                    let missing =
                        receive(client.get(format!("{base}/statements?statementId={id}"))).await?;
                    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
                    assert!(
                        state
                            .xapi
                            .statements
                            .get(id)
                            .await
                            .map_err(|e| format!("{e:?}"))?
                            .is_none()
                    );
                    continue;
                }
                assert_eq!(
                    reply.status(),
                    if post {
                        StatusCode::OK
                    } else {
                        StatusCode::NO_CONTENT
                    }
                );
                if post {
                    assert_eq!(
                        reply.json::<Value>().await.map_err(|e| e.to_string())?,
                        json!([id])
                    );
                }
                let exact = receive(client.get(format!(
                    "{base}/statements?statementId={id}&format=exact&attachments=false"
                )))
                .await?;
                assert_eq!(exact.status(), StatusCode::OK);
                assert_eq!(
                    exact.json::<Value>().await.map_err(|e| e.to_string())?["attachments"],
                    value["attachments"]
                );
                if partition == "raw" {
                    let reply = receive(client.get(format!(
                        "{base}/statements?statementId={id}&attachments=true"
                    )))
                    .await?;
                    assert_eq!(reply.status(), StatusCode::OK);
                    let parts = decoded(reply).await?;
                    assert_eq!(parts.len(), 2);
                    assert_eq!(
                        serde_json::from_slice::<Value>(&parts[0].1).map_err(|e| e.to_string())?["attachments"],
                        value["attachments"]
                    );
                    assert_eq!(parts[1].1, bytes);
                    assert_eq!(hash(algorithm, &parts[1].1), digest);
                    let headers = parts[1].0.to_ascii_lowercase();
                    for expected in [
                        "content-type: application/octet-stream".to_owned(),
                        "content-transfer-encoding: binary".to_owned(),
                        format!("x-experience-api-hash: {digest}"),
                    ] {
                        assert!(
                            headers.split("\r\n").any(|line| line == expected),
                            "{expected}"
                        );
                    }
                }
            }
        }
    }
    server.abort();
    Ok(())
}

#[tokio::test]
async fn memory_xapi_preserves_empty_attachments() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_preserves_empty_attachments() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

// Empty-message known answers obtained independently using Python hashlib;
// the target's Rust digest implementation never computes the test expectation.
const EMPTY_DIGESTS: [&str; 4] = [
    "d14a028c2a3a2bc9476102bb288234c415a2b01f828ea62ac5b3e42f",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b",
    "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
];

async fn receive(request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let reply = evidence::received(
        request
            .header("X-Experience-API-Version", "2.0.0")
            .send()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await?;
    assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
    OffsetDateTime::parse(
        reply.headers()["x-experience-api-consistent-through"]
            .to_str()
            .map_err(|e| e.to_string())?,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| e.to_string())?;
    Ok(reply)
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let base = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let router = evidence::router(state);
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    for digest in EMPTY_DIGESTS {
        for post in [false, true] {
            for length in [0, 1] {
                let id = Uuid::new_v4();
                let mut value = statement(id);
                value["attachments"] = json!([{"usageType":"https://example.org/empty-file","display":{"en":"empty file"},"contentType":"application/octet-stream","length":length,"sha2":digest}]);
                let payload = if post { json!([value]) } else { value };
                let body = format!(
                    "--empty-file\r\nContent-Type: application/json\r\n\r\n{payload}\r\n--empty-file\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: binary\r\nContent-Length: 0\r\nX-Experience-API-Hash: {digest}\r\n\r\n\r\n--empty-file--\r\n"
                );
                let request = if post {
                    client.post(format!("{base}/statements"))
                } else {
                    client.put(format!("{base}/statements?statementId={id}"))
                };
                let reply = receive(
                    request
                        .header("Content-Type", "multipart/mixed; boundary=empty-file")
                        .body(body),
                )
                .await?;
                let stored = state
                    .xapi
                    .statements
                    .get(id)
                    .await
                    .map_err(|e| format!("{e:?}"))?;
                if length != 0 {
                    assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
                    assert!(stored.is_none());
                    continue;
                }
                assert_eq!(
                    reply.status(),
                    if post {
                        StatusCode::OK
                    } else {
                        StatusCode::NO_CONTENT
                    }
                );
                if post {
                    assert_eq!(
                        reply.json::<Value>().await.map_err(|e| e.to_string())?,
                        json!([id])
                    );
                }
                let stored = stored.ok_or("empty attachment Statement")?;
                assert_eq!(stored.attachments.len(), 1);
                let attachment = stored.attachments.get(digest).ok_or("empty attachment")?;
                assert_eq!(attachment.content_type, "application/octet-stream");
                assert!(attachment.body.is_empty());
                let reply = receive(client.get(format!(
                    "{base}/statements?statementId={id}&attachments=true"
                )))
                .await?;
                assert_eq!(reply.status(), StatusCode::OK);
                let content_type = reply.headers()["content-type"]
                    .to_str()
                    .map_err(|e| e.to_string())?;
                let boundary = content_type
                    .split("boundary=")
                    .nth(1)
                    .ok_or("multipart boundary")?
                    .trim_matches('"')
                    .to_owned();
                let body = reply.text().await.map_err(|e| e.to_string())?;
                assert!(body.contains(digest));
                let parts = body.split(&format!("--{boundary}")).collect::<Vec<_>>();
                assert_eq!(
                    parts.len(),
                    4,
                    "JSON part, empty attachment, closing boundary"
                );
                let (_, attachment_body) = parts[2]
                    .split_once("\r\n\r\n")
                    .ok_or("empty attachment part body")?;
                assert_eq!(attachment_body, "\r\n");
                assert_eq!(parts[3], "--\r\n");
            }
        }
    }
    server.abort();
    Ok(())
}

#[tokio::test]
async fn memory_xapi_attachment_metadata_order_and_raw_association() -> Result<(), String> {
    verify_attachment_order(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_attachment_metadata_order_and_raw_association() -> Result<(), String> {
    verify_attachment_order(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

async fn received_order(
    request: reqwest::RequestBuilder,
    status: StatusCode,
) -> Result<reqwest::Response, String> {
    let reply = evidence::received(
        request
            .header("X-Experience-API-Version", "2.0.0")
            .send()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await?;
    assert_eq!(reply.status(), status);
    assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
    OffsetDateTime::parse(
        reply.headers()["x-experience-api-consistent-through"]
            .to_str()
            .map_err(|e| e.to_string())?,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| e.to_string())?;
    Ok(reply)
}

#[allow(clippy::too_many_lines)]
async fn verify_attachment_order(state: &AppState) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let base = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let router = evidence::router(state);
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let known: Value = serde_json::from_str(include_str!(
        "../../../../../tests/conformance/fixtures/sha2-integration.json"
    ))
    .map_err(|e| e.to_string())?;
    let answers = known["vectors"]
        .as_array()
        .ok_or("vectors")?
        .iter()
        .filter(|v| v["algorithm"] == "sha256")
        .collect::<Vec<_>>();
    assert_eq!(answers.len(), 2);
    let raw = [
        unhex(answers[0]["bytes_hex"].as_str().ok_or("bytes")?)?,
        unhex(answers[1]["bytes_hex"].as_str().ok_or("bytes")?)?,
    ];
    let hashes = [
        answers[0]["digest"].as_str().ok_or("hash")?,
        answers[1]["digest"].as_str().ok_or("hash")?,
    ];
    evidence::observation(
        "attachment-order-policy",
        &json!({"selected":"D-09=A","metadata":"Preserve [A,B,A] at root and legal SubStatement positions, including multiplicity and values","raw_parts":"Association by hash; either part order permitted; one raw part may serve repeated metadata","known_hash_basis":"bound Python hashlib SHA256 vectors","source":"IEEE metadata preservation exemption and Kiva order policy are graded separately"}),
    );
    for nested in [false, true] {
        for post in [false, true] {
            for file_url in [false, true] {
                for reverse in [false, true] {
                    let id = Uuid::new_v4();
                    let make = |n: usize| {
                        let mut item = json!({"usageType":format!("https://example.org/attachment/{n}"),"display":{"en":format!("item{n}")},"description":{"en":format!("description{n}")},"contentType":"application/octet-stream","length":raw[n].len(),"sha2":hashes[n]});
                        if file_url {
                            item["fileUrl"] =
                                json!(format!("https://unresolvable.invalid/raw/{n}"));
                        }
                        item
                    };
                    let metadata = json!([make(0), make(1), make(0)]);
                    let mut value = statement(id);
                    if nested {
                        value["object"] = json!({"objectType":"SubStatement","actor":value["actor"],"verb":value["verb"],"object":{"id":"https://example.org/sub-attachment"},"attachments":metadata});
                    } else {
                        value["attachments"] = metadata.clone();
                    }
                    let request = if post {
                        client.post(format!("{base}/statements"))
                    } else {
                        client.put(format!("{base}/statements?statementId={id}"))
                    };
                    let entity = if post { json!([value]) } else { value.clone() };
                    let request = if file_url {
                        request.json(&entity)
                    } else {
                        let mut mime = format!(
                            "--order-control\r\nContent-Type: application/json\r\n\r\n{entity}\r\n"
                        )
                        .into_bytes();
                        for n in if reverse { [1, 0] } else { [0, 1] } {
                            mime.extend_from_slice(format!("--order-control\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: binary\r\nX-Experience-API-Hash: {}\r\n\r\n",hashes[n]).as_bytes());
                            mime.extend_from_slice(&raw[n]);
                            mime.extend_from_slice(b"\r\n");
                        }
                        mime.extend_from_slice(b"--order-control--\r\n");
                        request
                            .header("Content-Type", "multipart/mixed; boundary=order-control")
                            .body(mime)
                    };
                    let reply = received_order(
                        request,
                        if post {
                            StatusCode::OK
                        } else {
                            StatusCode::NO_CONTENT
                        },
                    )
                    .await?;
                    if post {
                        assert_eq!(
                            reply.json::<Value>().await.map_err(|e| e.to_string())?,
                            json!([id])
                        );
                    }
                    for format in [None, Some("exact"), Some("canonical"), Some("ids")] {
                        for include_raw in [false, true] {
                            let mut url = reqwest::Url::parse(&format!("{base}/statements"))
                                .map_err(|e| e.to_string())?;
                            url.query_pairs_mut()
                                .append_pair("statementId", &id.to_string())
                                .append_pair(
                                    "attachments",
                                    if include_raw { "true" } else { "false" },
                                );
                            if let Some(format) = format {
                                url.query_pairs_mut().append_pair("format", format);
                            }
                            let reply = received_order(client.get(url), StatusCode::OK).await?;
                            let returned = if include_raw {
                                let parts = decoded(reply).await?;
                                assert_eq!(parts.len(), if file_url { 1 } else { 3 });
                                if !file_url {
                                    let mut seen = std::collections::BTreeSet::new();
                                    for (headers, bytes) in &parts[1..] {
                                        let digest = headers
                                            .lines()
                                            .find_map(|line| {
                                                line.strip_prefix("X-Experience-API-Hash: ")
                                            })
                                            .ok_or("hash header")?;
                                        let n = hashes
                                            .iter()
                                            .position(|v| *v == digest)
                                            .ok_or("unexpected part")?;
                                        assert!(seen.insert(n));
                                        assert_eq!(bytes, &raw[n]);
                                        assert!(
                                            headers.contains("Content-Transfer-Encoding: binary")
                                        );
                                        assert!(
                                            headers
                                                .contains("Content-Type: application/octet-stream")
                                        );
                                    }
                                    assert_eq!(seen.len(), 2);
                                }
                                serde_json::from_slice::<Value>(&parts[0].1)
                                    .map_err(|e| e.to_string())?
                            } else {
                                reply.json::<Value>().await.map_err(|e| e.to_string())?
                            };
                            assert_eq!(
                                returned.pointer(if nested {
                                    "/object/attachments"
                                } else {
                                    "/attachments"
                                }),
                                Some(&metadata),
                                "{format:?} / raw={include_raw}"
                            );
                        }
                    }
                }
            }
        }
    }
    server.abort();
    Ok(())
}

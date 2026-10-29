//! Request-correlated completion and actual repository quiescence controls.
#![allow(clippy::wildcard_imports)]
use super::*;
use crate::core::response::hex_lower;
use axum::{
    body::{Body, to_bytes},
    extract::Request,
    middleware::{self, Next},
    response::Response,
};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

const CORRELATION: &str = "x-kiva-settlement-control";
type Completions = Arc<Mutex<BTreeMap<Uuid, oneshot::Sender<Value>>>>;

#[tokio::test]
async fn memory_xapi_batch_settlement_and_leak_controls() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_batch_settlement_and_leak_controls() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

async fn terminal(request: Request, next: Next, completions: Completions) -> Response {
    let identifier = request
        .headers()
        .get(CORRELATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| Uuid::parse_str(h).ok());
    let Some(identifier) = identifier else {
        return next.run(request).await;
    };
    let (parts, body) = request.into_parts();
    let bytes = to_bytes(body, usize::MAX).await.expect("control body");
    let hash = hex_lower(&Sha256::digest(&bytes));
    let response = next
        .run(Request::from_parts(parts, Body::from(bytes)))
        .await;
    // This observation is after the ACTUAL production handler future, which
    // awaits its repository transaction. Kiva has no separate ingestion or
    // visibility worker. Registered negative-fixture jobs are drained below.
    let sender = completions
        .lock()
        .expect("completion lock")
        .remove(&identifier);
    if let Some(sender) = sender {
        let _=sender.send(json!({"request_id":identifier,"request_sha256":hash,"handler_returned":true,"status":response.status().as_u16()}));
    }
    response
}

async fn receive(request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let response = evidence::received(
        request
            .header("X-Experience-API-Version", "2.0.0")
            .send()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await?;
    assert_eq!(response.headers()["x-experience-api-version"], "2.0.0");
    OffsetDateTime::parse(
        response.headers()["x-experience-api-consistent-through"]
            .to_str()
            .map_err(|e| e.to_string())?,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| e.to_string())?;
    Ok(response)
}

struct Pending {
    identifier: Uuid,
    hash: String,
    terminal: oneshot::Receiver<Value>,
    jobs: Vec<tokio::task::JoinHandle<Result<(), String>>>,
}

async fn issue(
    client: &reqwest::Client,
    base: &str,
    completions: &Completions,
    body: Vec<u8>,
    mime: bool,
    jobs: Vec<tokio::task::JoinHandle<Result<(), String>>>,
) -> Result<(reqwest::Response, Pending), String> {
    let identifier = Uuid::new_v4();
    let hash = hex_lower(&Sha256::digest(&body));
    let (send, terminal) = oneshot::channel();
    completions
        .lock()
        .map_err(|e| e.to_string())?
        .insert(identifier, send);
    let response = receive(
        client
            .post(format!("{base}/statements"))
            .header(CORRELATION, identifier.to_string())
            .header(
                "Content-Type",
                if mime {
                    "multipart/mixed; boundary=settlement"
                } else {
                    "application/json"
                },
            )
            .body(body),
    )
    .await?;
    Ok((
        response,
        Pending {
            identifier,
            hash,
            terminal,
            jobs,
        },
    ))
}

async fn settle(state: &AppState, pending: Pending) -> Result<Value, String> {
    let mut completion = pending.terminal.await.map_err(|e| e.to_string())?;
    assert_eq!(completion["request_id"], json!(pending.identifier));
    assert_eq!(completion["request_sha256"], json!(pending.hash));
    let jobs = pending.jobs.len();
    for job in pending.jobs {
        job.await.map_err(|e| e.to_string())??;
    }
    let snapshot = state
        .xapi
        .statements
        .settled_snapshot()
        .await
        .map_err(|e| format!("{e:?}"))?;
    completion["snapshot"] = snapshot;
    completion["registered_jobs_drained"] = json!(jobs);
    completion["adapter"] = json!(
        "Kiva inline awaited admission; repository writer lock and committed snapshot; registered control jobs joined"
    );
    evidence::observation("request-settlement", &completion);
    Ok(completion)
}

#[derive(Debug, PartialEq, Eq)]
enum Grade {
    Pass,
    Fail,
    NotRun,
}

fn grade(
    expected: StatusCode,
    actual: StatusCode,
    baseline: &Value,
    receipt: Option<&Value>,
    ids: &[Uuid],
    accepted: bool,
    observed_leak: bool,
) -> Grade {
    if actual != expected || observed_leak {
        return Grade::Fail;
    }
    let Some(snapshot) = receipt
        .and_then(|r| r.get("snapshot"))
        .and_then(Value::as_array)
    else {
        return Grade::NotRun;
    };
    let baseline = baseline.as_array().expect("snapshot array");
    let unchanged = baseline.iter().all(|old| snapshot.contains(old));
    let present = ids
        .iter()
        .filter(|id| snapshot.iter().any(|r| r["statement"]["id"] == json!(id)))
        .count();
    if accepted {
        if unchanged && present == ids.len() && snapshot.len() == baseline.len() + ids.len() {
            Grade::Pass
        } else {
            Grade::Fail
        }
    } else if unchanged && snapshot.len() == baseline.len() && present == 0 {
        Grade::Pass
    } else {
        Grade::Fail
    }
}

fn batch(kind: &str) -> (Vec<Uuid>, Vec<u8>, bool) {
    let ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    let mut values = ids.iter().map(|id| statement(*id)).collect::<Vec<_>>();
    match kind {
        "invalid-middle" => values[1]["verb"]["id"] = json!(42),
        "duplicate-identical" => values[1] = values[0].clone(),
        "duplicate-different" => {
            values[1]["id"] = values[0]["id"].clone();
            values[1]["verb"]["id"] = json!("https://example.org/different");
        }
        _ => {}
    }
    let mime = kind.starts_with("mime-");
    if !mime {
        return (ids, serde_json::to_vec(&values).expect("batch JSON"), false);
    }
    let data = [
        b"first".as_slice(),
        b"middle".as_slice(),
        b"last".as_slice(),
    ];
    for (value, bytes) in values.iter_mut().zip(data) {
        value["attachments"] = json!([{"usageType":"https://example.org/file","display":{"en":"part"},"contentType":"application/octet-stream","length":bytes.len(),"sha2":hex_lower(&Sha256::digest(bytes))}]);
    }
    let mut body = format!(
        "--settlement\r\nContent-Type: application/json\r\n\r\n{}\r\n",
        json!(values)
    )
    .into_bytes();
    for (index, bytes) in data.iter().enumerate() {
        if kind == "mime-missing-middle" && index == 1 {
            continue;
        }
        body.extend_from_slice(format!("--settlement\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: binary\r\nX-Experience-API-Hash: {}\r\n\r\n",hex_lower(&Sha256::digest(bytes))).as_bytes());
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(b"--settlement--\r\n");
    (ids, body, true)
}

async fn leak(client: reqwest::Client, base: String, id: Uuid) -> Result<(), String> {
    let response = receive(
        client
            .put(format!("{base}/statements?statementId={id}"))
            .json(&statement(id)),
    )
    .await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    Ok(())
}

async fn sha2_batches(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
    completions: &Completions,
) -> Result<(), String> {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../tests/conformance/fixtures/sha2-integration.json"
    ))
    .map_err(|e| e.to_string())?;
    for vector in fixture["vectors"].as_array().ok_or("SHA-2 vectors")? {
        for cause in ["valid", "wrong-raw", "wrong-hash", "wrong-length"] {
            let baseline = state
                .xapi
                .statements
                .settled_snapshot()
                .await
                .map_err(|e| format!("{e:?}"))?;
            let ids = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
            let mut values = ids.map(statement);
            let hex = vector["bytes_hex"].as_str().ok_or("hex")?;
            let mut bytes = (0..hex.len())
                .step_by(2)
                .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).map_err(|e| e.to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            let mut digest = vector["digest"].as_str().ok_or("digest")?.to_owned();
            if cause == "wrong-raw" {
                bytes[0] ^= 1;
            }
            if cause == "wrong-hash" {
                digest.replace_range(..1, if digest.starts_with('0') { "1" } else { "0" });
            }
            values[1]["attachments"] = json!([{"usageType":"https://example.org/sha2-settlement","display":{"en":"binary"},"contentType":"application/octet-stream","length":bytes.len()+usize::from(cause=="wrong-length"),"sha2":digest}]);
            let mut body=format!("--settlement\r\nContent-Type: application/json\r\n\r\n{}\r\n--settlement\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: binary\r\nX-Experience-API-Hash: {digest}\r\n\r\n",json!(values)).into_bytes();
            body.extend_from_slice(&bytes);
            body.extend_from_slice(b"\r\n--settlement--\r\n");
            let (reply, pending) = issue(client, base, completions, body, true, vec![]).await?;
            let accepted = cause == "valid";
            let expected = if accepted {
                StatusCode::OK
            } else {
                StatusCode::BAD_REQUEST
            };
            let actual = reply.status();
            assert_eq!(actual, expected, "{}/{cause}", vector["algorithm"]);
            let receipt = settle(state, pending).await?;
            assert_eq!(
                grade(
                    expected,
                    actual,
                    &baseline,
                    Some(&receipt),
                    &ids,
                    accepted,
                    false
                ),
                Grade::Pass,
                "{}/{cause}",
                vector["algorithm"]
            );
            for id in ids {
                let read =
                    receive(client.get(format!("{base}/statements?statementId={id}"))).await?;
                assert_eq!(
                    read.status(),
                    if accepted {
                        StatusCode::OK
                    } else {
                        StatusCode::NOT_FOUND
                    }
                );
            }
        }
    }
    Ok(())
}

async fn put_array_policy(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
    completions: &Completions,
) -> Result<(), String> {
    for mime in [false, true] {
        for shape in ["object", "empty", "single", "multiple"] {
            let ids = [Uuid::new_v4(), Uuid::new_v4()];
            let values = ids.map(statement);
            let input = match shape {
                "object" => values[0].clone(),
                "empty" => json!([]),
                "single" => json!([values[0]]),
                _ => json!(values),
            };
            let body = if mime {
                format!("--settlement\r\nContent-Type: application/json\r\n\r\n{input}\r\n--settlement--\r\n").into_bytes()
            } else {
                input.to_string().into_bytes()
            };
            let baseline = state
                .xapi
                .statements
                .settled_snapshot()
                .await
                .map_err(|e| format!("{e:?}"))?;
            let identifier = Uuid::new_v4();
            let hash = hex_lower(&Sha256::digest(&body));
            let (sender, terminal) = oneshot::channel();
            completions
                .lock()
                .map_err(|e| e.to_string())?
                .insert(identifier, sender);
            let reply = receive(
                client
                    .put(format!("{base}/statements?statementId={}", ids[0]))
                    .header(CORRELATION, identifier.to_string())
                    .header(
                        "Content-Type",
                        if mime {
                            "multipart/mixed; boundary=settlement"
                        } else {
                            "application/json"
                        },
                    )
                    .body(body),
            )
            .await?;
            let accepted = shape == "object";
            let expected = if accepted {
                StatusCode::NO_CONTENT
            } else {
                StatusCode::BAD_REQUEST
            };
            let actual = reply.status();
            assert_eq!(actual, expected, "D-01=A/{shape}/{mime}");
            let receipt = settle(
                state,
                Pending {
                    identifier,
                    hash,
                    terminal,
                    jobs: vec![],
                },
            )
            .await?;
            assert_eq!(
                grade(
                    expected,
                    actual,
                    &baseline,
                    Some(&receipt),
                    if accepted { &ids[..1] } else { &ids },
                    accepted,
                    false
                ),
                Grade::Pass
            );
            evidence::observation(
                "put-array-policy",
                &json!({"policy":"D-01=A","source_disputed_array_put":"INDETERMINATE","shape":shape,"multipart":mime,"request_id":receipt["request_id"],"policy_grade":"PASS"}),
            );
        }
        for array in [false, true] {
            let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
            let ids = if array { ids } else { ids[..1].to_vec() };
            let values = ids.iter().map(|id| statement(*id)).collect::<Vec<_>>();
            let input = if array {
                json!(values)
            } else {
                values[0].clone()
            };
            let body = if mime {
                format!("--settlement\r\nContent-Type: application/json\r\n\r\n{input}\r\n--settlement--\r\n").into_bytes()
            } else {
                input.to_string().into_bytes()
            };
            let baseline = state
                .xapi
                .statements
                .settled_snapshot()
                .await
                .map_err(|e| format!("{e:?}"))?;
            let (reply, pending) =
                issue(client, base, completions, body.clone(), mime, vec![]).await?;
            assert_eq!(reply.status(), StatusCode::OK);
            assert_eq!(
                reply.json::<Value>().await.map_err(|e| e.to_string())?,
                json!(ids)
            );
            let receipt = settle(state, pending).await?;
            assert_eq!(
                grade(
                    StatusCode::OK,
                    StatusCode::OK,
                    &baseline,
                    Some(&receipt),
                    &ids,
                    true,
                    false
                ),
                Grade::Pass
            );
            let (reply, pending) = issue(client, base, completions, body, mime, vec![]).await?;
            assert_eq!(reply.status(), StatusCode::OK);
            assert_eq!(
                reply.json::<Value>().await.map_err(|e| e.to_string())?,
                json!(ids)
            );
            let retransmission = settle(state, pending).await?;
            assert_eq!(retransmission["snapshot"], receipt["snapshot"]);
        }
    }
    Ok(())
}

async fn verify(state: &AppState) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let base = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let completions: Completions = Arc::default();
    let shared = completions.clone();
    let router =
        evidence::router(state).layer(middleware::from_fn(move |request: Request, next: Next| {
            terminal(request, next, shared.clone())
        }));
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    put_array_policy(state, &client, &base, &completions).await?;
    sha2_batches(state, &client, &base, &completions).await?;
    for kind in [
        "valid",
        "mime-valid",
        "invalid-middle",
        "duplicate-identical",
        "duplicate-different",
        "mime-missing-middle",
    ] {
        let baseline = state
            .xapi
            .statements
            .settled_snapshot()
            .await
            .map_err(|e| format!("{e:?}"))?;
        let (ids, body, mime) = batch(kind);
        let (reply, pending) = issue(&client, &base, &completions, body, mime, vec![]).await?;
        let accepted = matches!(kind, "valid" | "mime-valid");
        let expected = if accepted {
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        };
        let actual = reply.status();
        assert_eq!(actual, expected, "{kind}");
        let receipt = settle(state, pending).await?;
        assert_eq!(
            grade(
                expected,
                actual,
                &baseline,
                Some(&receipt),
                &ids,
                accepted,
                false
            ),
            Grade::Pass,
            "{kind}"
        );
        // All-valid controls must expose an ineffective always-empty fixture.
        if accepted {
            assert_eq!(
                grade(
                    expected,
                    actual,
                    &baseline,
                    Some(&json!({"snapshot":[]})),
                    &ids,
                    true,
                    false
                ),
                Grade::Fail
            );
        }
        for id in ids {
            let read = receive(client.get(format!("{base}/statements?statementId={id}"))).await?;
            assert_eq!(
                read.status(),
                if accepted {
                    StatusCode::OK
                } else {
                    StatusCode::NOT_FOUND
                }
            );
        }
    }
    for queued in [false, true] {
        let baseline = state
            .xapi
            .statements
            .settled_snapshot()
            .await
            .map_err(|e| format!("{e:?}"))?;
        let (ids, body, mime) = batch("invalid-middle");
        let (release, resume) = oneshot::channel();
        let mut jobs = Vec::new();
        if queued {
            let client = client.clone();
            let base = base.clone();
            let id = ids[0];
            jobs.push(tokio::spawn(async move {
                resume.await.map_err(|e| e.to_string())?;
                leak(client, base, id).await
            }));
        } else {
            leak(client.clone(), base.clone(), ids[0]).await?;
        }
        let (reply, pending) = issue(&client, &base, &completions, body, mime, jobs).await?;
        assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
        let immediate =
            receive(client.get(format!("{base}/statements?statementId={}", ids[0]))).await?;
        assert_eq!(
            immediate.status(),
            if queued {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::OK
            }
        );
        assert_eq!(
            grade(
                StatusCode::BAD_REQUEST,
                reply.status(),
                &baseline,
                None,
                &ids,
                false,
                !queued
            ),
            if queued { Grade::NotRun } else { Grade::Fail }
        );
        let completion = settle(state, pending);
        tokio::pin!(completion);
        if queued {
            // A completion wait really remains pending while the registered
            // request job is blocked. This is a harness budget, not an IEEE
            // latency requirement; absence at this point is NOT-RUN.
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(20), &mut completion)
                    .await
                    .is_err()
            );
            assert_eq!(
                grade(
                    StatusCode::BAD_REQUEST,
                    reply.status(),
                    &baseline,
                    None,
                    &ids,
                    false,
                    false
                ),
                Grade::NotRun
            );
        }
        let _ = release.send(());
        let receipt = completion.await?;
        assert_eq!(
            receipt["registered_jobs_drained"],
            json!(usize::from(queued))
        );
        assert_eq!(
            grade(
                StatusCode::BAD_REQUEST,
                reply.status(),
                &baseline,
                Some(&receipt),
                &ids,
                false,
                false
            ),
            Grade::Fail
        );
        let leaked =
            receive(client.get(format!("{base}/statements?statementId={}", ids[0]))).await?;
        assert_eq!(leaked.status(), StatusCode::OK);
        evidence::observation(
            "settlement-negative-fixture",
            &json!({"queued_after_400":queued,"request_id":receipt["request_id"],"immediate_status":immediate.status().as_u16(),"settled_grade":"FAIL","leaked_id":ids[0]}),
        );
    }
    // An unavailable completion never passes, and cannot erase a known error.
    assert_eq!(
        grade(
            StatusCode::BAD_REQUEST,
            StatusCode::BAD_REQUEST,
            &json!([]),
            None,
            &[],
            false,
            false
        ),
        Grade::NotRun
    );
    assert_eq!(
        grade(
            StatusCode::BAD_REQUEST,
            StatusCode::OK,
            &json!([]),
            None,
            &[],
            false,
            false
        ),
        Grade::Fail
    );
    server.abort();
    Ok(())
}

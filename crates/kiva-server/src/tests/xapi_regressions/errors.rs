//! Actual HTTP errors with independently established causes and one-shot faults.
#![allow(clippy::wildcard_imports)]
use super::*;
use crate::core::{response::hex_lower, runtime::xapi_limits::XapiLimits};
use sha2::{Digest, Sha256};

#[tokio::test]
async fn memory_xapi_size_and_rate_cause_boundaries() -> Result<(), String> {
    verify_limits(memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_size_and_rate_cause_boundaries() -> Result<(), String> {
    verify_limits(
        postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

fn document_url(base: &str, family: &str, id: Uuid) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(&format!("{base}/{family}")).map_err(|e| e.to_string())?;
    {
        let mut query = url.query_pairs_mut();
        if family != "agents/profile" {
            query.append_pair("activityId", "https://example.org/limits");
        }
        if family != "activities/profile" {
            query.append_pair("agent", r#"{"mbox":"mailto:limits@example.org"}"#);
        }
        query.append_pair(
            if family == "activities/state" {
                "stateId"
            } else {
                "profileId"
            },
            &id.to_string(),
        );
    }
    Ok(url)
}

async fn serve(
    state: &AppState,
) -> Result<(String, tokio::task::JoinHandle<Result<(), std::io::Error>>), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let base = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let router = evidence::router(state);
    Ok((
        base,
        tokio::spawn(async move { axum::serve(listener, router).await }),
    ))
}

#[allow(clippy::too_many_lines)]
async fn verify_limits(mut state: AppState) -> Result<(), String> {
    // These are deployable production configuration values, installed before
    // starting the actual router. Neither expected status nor the response is
    // used to determine the accounting units or the active credential budget.
    let configuration = json!({"default":{"statement_body_bytes":4096,"document_body_bytes":64,"attachment_bytes":32},"subjects":{"larger-writer":{"statement_body_bytes":8192,"document_body_bytes":96,"attachment_bytes":48}}});
    state.limits.xapi =
        XapiLimits::from_json(&configuration.to_string()).map_err(|e| e.to_string())?;
    state
        .auth
        .register_bearer_subject("larger-writer-token", "larger-writer");
    state.auth.register_bearer_scopes(
        "larger-writer-token",
        [
            kiva_identity::OAuthScope::XapiRead,
            kiva_identity::OAuthScope::XapiWrite,
        ],
    );
    let (base, server) = serve(&state).await?;
    let client = reqwest::Client::new();
    evidence::observation(
        "configured-byte-limits",
        &json!({"configuration":configuration,"statement_unit":"entire HTTP body octets including MIME framing","document_unit":"HTTP entity octets","attachment_unit":"each raw MIME part body","ceiling":2_097_152,"subject_resolution":"after authorization; alternate token maps to larger-writer"}),
    );
    for (credential, statement_limit, document_limit, attachment_limit) in [
        ("statement-writer-a", 4096, 64, 32),
        ("larger-writer-token", 8192, 96, 48),
    ] {
        for post in [false, true] {
            for accept in ["text/plain", "text/html", "application/json"] {
                for size in [statement_limit - 1, statement_limit, statement_limit + 1] {
                    let id = Uuid::new_v4();
                    let mut body = statement(id).to_string().into_bytes();
                    body.resize(size, b' ');
                    let url = if post {
                        format!("{base}/statements")
                    } else {
                        format!("{base}/statements?statementId={id}")
                    };
                    let request = if post {
                        client.post(url)
                    } else {
                        client.put(url)
                    };
                    let request = request
                        .bearer_auth(credential)
                        .header("Content-Type", "application/json")
                        .body(body);
                    if size > statement_limit {
                        explained(
                            request,
                            StatusCode::PAYLOAD_TOO_LARGE,
                            "configured Statement byte budget",
                            accept,
                            &[],
                        )
                        .await?;
                        assert!(
                            state
                                .xapi
                                .statements
                                .get(id)
                                .await
                                .map_err(|e| format!("{e:?}"))?
                                .is_none()
                        );
                    } else {
                        receive(
                            request,
                            if post {
                                StatusCode::OK
                            } else {
                                StatusCode::NO_CONTENT
                            },
                        )
                        .await?;
                        assert!(
                            state
                                .xapi
                                .statements
                                .get(id)
                                .await
                                .map_err(|e| format!("{e:?}"))?
                                .is_some()
                        );
                    }
                }
                for family in ["activities/state", "activities/profile", "agents/profile"] {
                    for size in [document_limit - 1, document_limit, document_limit + 1] {
                        let url = document_url(&base, family, Uuid::new_v4())?;
                        let mut body = b"{}".to_vec();
                        body.resize(size, b' ');
                        let request = if post {
                            client.post(url.clone())
                        } else {
                            client.put(url.clone())
                        };
                        let request = request
                            .bearer_auth(credential)
                            .header("Content-Type", "application/json")
                            .body(body);
                        if size > document_limit {
                            explained(
                                request,
                                StatusCode::PAYLOAD_TOO_LARGE,
                                "configured Document byte budget",
                                accept,
                                &[],
                            )
                            .await?;
                            receive(client.get(url), StatusCode::NOT_FOUND).await?;
                        } else {
                            receive(request, StatusCode::NO_CONTENT).await?;
                            let read: Value = receive(client.get(url), StatusCode::OK)
                                .await?
                                .json()
                                .await
                                .map_err(|e| e.to_string())?;
                            assert_eq!(read, json!({}));
                        }
                    }
                }
                for size in [attachment_limit - 1, attachment_limit, attachment_limit + 1] {
                    let id = Uuid::new_v4();
                    let bytes = vec![0x42; size];
                    let hash = hex_lower(&Sha256::digest(&bytes));
                    let mut value = statement(id);
                    value["attachments"] = json!([{"usageType":"https://example.org/attachment","display":{"en":"bytes"},"contentType":"application/octet-stream","length":size,"sha2":hash}]);
                    let mut body=format!("--limits\r\nContent-Type: application/json\r\n\r\n{value}\r\n--limits\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: binary\r\nX-Experience-API-Hash: {hash}\r\n\r\n").into_bytes();
                    body.extend_from_slice(&bytes);
                    body.extend_from_slice(b"\r\n--limits--\r\n");
                    assert!(body.len() < statement_limit);
                    let url = if post {
                        format!("{base}/statements")
                    } else {
                        format!("{base}/statements?statementId={id}")
                    };
                    let request = if post {
                        client.post(url)
                    } else {
                        client.put(url)
                    };
                    let request = request
                        .bearer_auth(credential)
                        .header("Content-Type", "multipart/mixed; boundary=limits")
                        .body(body);
                    if size > attachment_limit {
                        explained(
                            request,
                            StatusCode::PAYLOAD_TOO_LARGE,
                            "configured raw attachment byte budget",
                            accept,
                            &[],
                        )
                        .await?;
                        assert!(
                            state
                                .xapi
                                .statements
                                .get(id)
                                .await
                                .map_err(|e| format!("{e:?}"))?
                                .is_none()
                        );
                    } else {
                        receive(
                            request,
                            if post {
                                StatusCode::OK
                            } else {
                                StatusCode::NO_CONTENT
                            },
                        )
                        .await?;
                        let record = state
                            .xapi
                            .statements
                            .get(id)
                            .await
                            .map_err(|e| format!("{e:?}"))?
                            .ok_or("saved attachment")?;
                        assert_eq!(record.attachments[&hash].body, bytes);
                    }
                }
            }
        }
    }
    server.abort();
    // Reset configuration before examining the independent extractor ceiling.
    state.limits.xapi = XapiLimits::default();
    let (base, server) = serve(&state).await?;
    for document in [false, true] {
        let id = Uuid::new_v4();
        let mut body = if document {
            b"{}".to_vec()
        } else {
            statement(id).to_string().into_bytes()
        };
        body.resize(2 * 1024 * 1024 + 1, b' ');
        let url = if document {
            document_url(&base, "activities/state", id)?
        } else {
            reqwest::Url::parse(&format!("{base}/statements?statementId={id}"))
                .map_err(|e| e.to_string())?
        };
        explained(
            client
                .put(url.clone())
                .header("Content-Type", "application/json")
                .body(body),
            StatusCode::PAYLOAD_TOO_LARGE,
            "transport extractor ceiling",
            "application/json",
            &[],
        )
        .await?;
        receive(client.get(url), StatusCode::NOT_FOUND).await?;
    }
    server.abort();
    let instant = OffsetDateTime::parse(
        "2026-10-20T12:00:00Z",
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| e.to_string())?;
    for accept in ["text/plain", "text/html", "application/json"] {
        let configuration = json!({"default":{"rate":{"accepted":2,"window_seconds":60}},"subjects":{"larger-writer":{"rate":{"accepted":3,"window_seconds":60}}}});
        state.limits.xapi =
            XapiLimits::from_json(&configuration.to_string()).map_err(|e| e.to_string())?;
        state.limits.xapi.clock().set(instant);
        evidence::observation(
            "configured-rate-limits",
            &json!({"configuration":configuration,"clock":instant.unix_timestamp(),"unit":"authorized xAPI attempts, shared across routes, independent per subject; denied attempts do not extend the window"}),
        );
        let (base, server) = serve(&state).await?;
        for (credential, n) in [("statement-writer-a", 2), ("larger-writer-token", 3)] {
            // Refused credentials never consume an admitted subject's budget.
            receive(
                client
                    .get(format!("{base}/about"))
                    .bearer_auth("refused-credential"),
                StatusCode::UNAUTHORIZED,
            )
            .await?;
            for index in 0..n {
                let path = if index == 0 { "about" } else { "statements" };
                receive(
                    client.get(format!("{base}/{path}")).bearer_auth(credential),
                    StatusCode::OK,
                )
                .await?;
            }
            explained(
                client.get(format!("{base}/about")).bearer_auth(credential),
                StatusCode::TOO_MANY_REQUESTS,
                "configured subject rate budget",
                accept,
                &[],
            )
            .await?;
        }
        state
            .limits
            .xapi
            .clock()
            .set(instant + time::Duration::seconds(59));
        receive(
            client
                .get(format!("{base}/statements"))
                .bearer_auth("statement-writer-a"),
            StatusCode::TOO_MANY_REQUESTS,
        )
        .await?;
        state
            .limits
            .xapi
            .clock()
            .set(instant + time::Duration::seconds(60));
        receive(
            client
                .get(format!("{base}/statements"))
                .bearer_auth("statement-writer-a"),
            StatusCode::OK,
        )
        .await?;
        server.abort();
    }
    state.limits.xapi = XapiLimits::default();
    Ok(())
}

#[test]
fn xapi_limit_configuration_rejects_ambiguous_or_unusable_values() {
    for invalid in [
        r#"{"default":{"statement_body_bytes":0}}"#,
        r#"{"subjects":{"":{}}}"#,
        r#"{"default":{"rate":{"accepted":0,"window_seconds":60}}}"#,
        r#"{"default":{"rate":{"accepted":1,"window_seconds":0}}}"#,
        r#"{"default":{"unknown":1}}"#,
        r#"{"default":{"attachment_bytes":-1}}"#,
    ] {
        assert!(XapiLimits::from_json(invalid).is_err(), "{invalid}");
    }
}

#[tokio::test]
async fn memory_xapi_error_causes_and_internal_read_failure() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_error_causes_and_internal_read_failure() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

async fn receive(
    request: reqwest::RequestBuilder,
    expected: StatusCode,
) -> Result<reqwest::Response, String> {
    let response = evidence::received(
        request
            .header("X-Experience-API-Version", "2.0.0")
            .send()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await?;
    assert_eq!(response.headers()["x-experience-api-version"], "2.0.0");
    if response.url().path().starts_with("/statements") {
        OffsetDateTime::parse(
            response.headers()["x-experience-api-consistent-through"]
                .to_str()
                .map_err(|e| e.to_string())?,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| e.to_string())?;
    }
    assert_eq!(response.status(), expected, "{}", response.url());
    Ok(response)
}

async fn explained(
    request: reqwest::RequestBuilder,
    expected: StatusCode,
    cause: &str,
    accept: &str,
    terms: &[&str],
) -> Result<(), String> {
    let reply = receive(request.header("Accept", accept), expected).await?;
    let media = reply
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let body = reply.text().await.map_err(|e| e.to_string())?;
    // Explanation and requested-media selection are recommendations; retain
    // their observations independently from the mandatory status obligation.
    evidence::observation(
        "error-explanation",
        &json!({"cause":cause,"accept":accept,"media_type":media,"body":body,"explanation_observed":!body.is_empty(),"requested_media_selected":media.split(';').next()==Some(accept)}),
    );
    for term in terms {
        assert!(body.contains(term), "{cause} must explain {term}: {body}");
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
    let router = evidence::router(state);
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let id = Uuid::new_v4();
    let value = statement(id);
    receive(
        client.post(format!("{base}/statements")).json(&value),
        StatusCode::OK,
    )
    .await?;
    let snapshot = state
        .xapi
        .statements
        .settled_snapshot()
        .await
        .map_err(|e| format!("{e:?}"))?;
    for accept in ["text/plain", "text/html", "application/json"] {
        explained(
            client
                .post(format!("{base}/statements"))
                .header("Content-Type", "application/json")
                .body("{\"actor\":"),
            StatusCode::BAD_REQUEST,
            "malformed JSON",
            accept,
            &[],
        )
        .await?;
        explained(
            client
                .get(format!("{base}/statements"))
                .bearer_auth("refused-credential"),
            StatusCode::UNAUTHORIZED,
            "refused token",
            accept,
            &[],
        )
        .await?;
        explained(
            client
                .post(format!("{base}/statements"))
                .bearer_auth("statement-reader")
                .json(&statement(Uuid::new_v4())),
            StatusCode::FORBIDDEN,
            "authenticated reader cannot write",
            accept,
            &[],
        )
        .await?;
        explained(
            client.get(format!("{base}/statements?statementId={}", Uuid::new_v4())),
            StatusCode::NOT_FOUND,
            "fresh absent Statement ID",
            accept,
            &[],
        )
        .await?;
        let mut changed = value.clone();
        changed["verb"]["id"] = json!("https://example.org/different");
        explained(
            client
                .put(format!("{base}/statements?statementId={id}"))
                .json(&changed),
            StatusCode::CONFLICT,
            "existing Statement differs",
            accept,
            &[],
        )
        .await?;
        for family in ["activities/state", "activities/profile", "agents/profile"] {
            let mut url =
                reqwest::Url::parse(&format!("{base}/{family}")).map_err(|e| e.to_string())?;
            let key = Uuid::new_v4().to_string();
            {
                let mut query = url.query_pairs_mut();
                if family != "agents/profile" {
                    query.append_pair("activityId", "https://example.org/error-causes");
                }
                if family != "activities/profile" {
                    query.append_pair("agent", r#"{"mbox":"mailto:errors@example.org"}"#);
                }
                query.append_pair(
                    if family == "activities/state" {
                        "stateId"
                    } else {
                        "profileId"
                    },
                    &key,
                );
            }
            explained(
                client.get(url.clone()),
                StatusCode::NOT_FOUND,
                "fresh absent Document ID",
                accept,
                &[],
            )
            .await?;
            receive(
                client
                    .put(url.clone())
                    .header("Content-Type", "text/plain")
                    .body("baseline"),
                StatusCode::NO_CONTENT,
            )
            .await?;
            let before = receive(client.get(url.clone()), StatusCode::OK).await?;
            let etag = before.headers()["etag"].clone();
            let modified = before.headers()["last-modified"].clone();
            assert_eq!(before.text().await.map_err(|e| e.to_string())?, "baseline");
            explained(
                client
                    .put(url.clone())
                    .header("Content-Type", "text/plain")
                    .body("replace"),
                StatusCode::CONFLICT,
                "existing Document without precondition",
                accept,
                &["Check current state", "If-Match", "current ETag"],
            )
            .await?;
            explained(
                client
                    .put(url.clone())
                    .header("Content-Type", "text/plain")
                    .header("If-Match", "\"stale\"")
                    .body("replace"),
                StatusCode::PRECONDITION_FAILED,
                "stale ETag",
                accept,
                &[],
            )
            .await?;
            let after = receive(client.get(url), StatusCode::OK).await?;
            assert_eq!(after.headers()["etag"], etag);
            assert_eq!(after.headers()["last-modified"], modified);
            assert_eq!(after.text().await.map_err(|e| e.to_string())?, "baseline");
        }

        // Arming receipt is independent of status. Invalid requests and the
        // watermark pre-read must not consume this repository content fault.
        let fault = state.xapi.statements.read_failure();
        let consumed = fault.consumed();
        let armed = fault.arm();
        evidence::observation(
            "internal-read-failure-armed",
            &json!({"id":armed,"boundary":"StatementRepository get/list","consumed_before":consumed}),
        );
        for (query, token, status) in [
            (
                "?statementId=not-a-uuid",
                "statement-writer-a",
                StatusCode::BAD_REQUEST,
            ),
            (
                "?limit=not-a-number",
                "statement-writer-a",
                StatusCode::BAD_REQUEST,
            ),
            (
                "?unrecognized=true",
                "statement-writer-a",
                StatusCode::BAD_REQUEST,
            ),
            ("", "refused-credential", StatusCode::UNAUTHORIZED),
        ] {
            explained(
                client
                    .get(format!("{base}/statements{query}"))
                    .bearer_auth(token),
                status,
                "rejected before storage",
                accept,
                &[],
            )
            .await?;
            assert_eq!(fault.consumed(), consumed);
        }
        explained(
            client.get(format!("{base}/statements?statementId={id}")),
            StatusCode::INTERNAL_SERVER_ERROR,
            "armed repository I/O failure",
            accept,
            &[],
        )
        .await?;
        let mut expected = consumed;
        expected.push(armed);
        assert_eq!(fault.consumed(), expected);
        evidence::observation(
            "internal-read-failure-consumed",
            &json!({"id":armed,"consumed":fault.consumed()}),
        );
        let recovered = receive(
            client.get(format!("{base}/statements?statementId={id}")),
            StatusCode::OK,
        )
        .await?;
        let recovered: Value = recovered.json().await.map_err(|e| e.to_string())?;
        assert_eq!(recovered["id"], value["id"]);
        assert_eq!(
            state
                .xapi
                .statements
                .settled_snapshot()
                .await
                .map_err(|e| format!("{e:?}"))?,
            snapshot
        );
    }
    let armed = state.xapi.statements.read_failure().arm();
    let response = receive(
        client.head(format!("{base}/statements")),
        StatusCode::INTERNAL_SERVER_ERROR,
    )
    .await?;
    assert!(
        response
            .bytes()
            .await
            .map_err(|e| e.to_string())?
            .is_empty()
    );
    assert_eq!(
        state.xapi.statements.read_failure().consumed().last(),
        Some(&armed)
    );
    receive(client.get(format!("{base}/statements")), StatusCode::OK).await?;
    // Unknown Agent/Activity identifiers have mandatory fallback objects.
    let mailbox = format!("mailto:{}@example.org", Uuid::new_v4());
    let url = reqwest::Url::parse_with_params(
        &format!("{base}/agents"),
        [("agent", json!({"mbox":mailbox}).to_string())],
    )
    .map_err(|e| e.to_string())?;
    let person: Value = receive(client.get(url), StatusCode::OK)
        .await?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(person["mbox"], json!([mailbox]));
    let activity = format!("https://example.org/unknown/{}", Uuid::new_v4());
    let url =
        reqwest::Url::parse_with_params(&format!("{base}/activities"), [("activityId", &activity)])
            .map_err(|e| e.to_string())?;
    let fallback: Value = receive(client.get(url), StatusCode::OK)
        .await?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(fallback, json!({"objectType":"Activity","id":activity}));
    parameter_and_credential_controls(&client, &base).await?;
    server.abort();
    Ok(())
}

async fn parameter_and_credential_controls(
    client: &reqwest::Client,
    base: &str,
) -> Result<(), String> {
    evidence::observation(
        "authorization-and-vocabulary",
        &json!({"authentication":"AuthConfig fixture subject/scope decisions before operation","permitted":"statement-writer-a; xapi/read+xapi/write","operation_denied":"statement-reader; xapi/read only","refused":"refused-credential; independently registered refusal","anonymous":"permitted by default fixture","query_vocabulary_source":"bound request allowlists, DocumentKind query validation, About empty allowlist","unknown_parameter":"auditUnknown","recognized_extension_parameters":[]}),
    );
    for path in [
        "statements",
        "about",
        "agents",
        "activities",
        "activities/state",
        "agents/profile",
        "activities/profile",
    ] {
        let mut url = reqwest::Url::parse(&format!("{base}/{path}")).map_err(|e| e.to_string())?;
        {
            let mut query = url.query_pairs_mut();
            if matches!(path, "agents" | "activities/state" | "agents/profile") {
                query.append_pair("agent", r#"{"mbox":"mailto:errors@example.org"}"#);
            }
            if matches!(
                path,
                "activities" | "activities/state" | "activities/profile"
            ) {
                query.append_pair("activityId", "https://example.org/errors");
            }
        }
        receive(client.get(url.clone()), StatusCode::OK).await?;
        url.query_pairs_mut().append_pair("auditUnknown", "true");
        for head in [false, true] {
            let request = if head {
                client.head(url.clone())
            } else {
                client.get(url.clone())
            };
            let reply = receive(request, StatusCode::BAD_REQUEST).await?;
            if head {
                assert!(reply.bytes().await.map_err(|e| e.to_string())?.is_empty());
            }
        }
    }
    let id = Uuid::new_v4();
    let value = statement(id);
    receive(
        client
            .put(format!("{base}/statements?statementId={id}"))
            .bearer_auth("statement-reader")
            .json(&value),
        StatusCode::FORBIDDEN,
    )
    .await?;
    receive(
        client.get(format!("{base}/statements?statementId={id}")),
        StatusCode::NOT_FOUND,
    )
    .await?;
    receive(
        client
            .put(format!("{base}/statements?statementId={id}"))
            .bearer_auth("statement-writer-a")
            .json(&value),
        StatusCode::NO_CONTENT,
    )
    .await?;
    for accept in ["text/plain", "text/html", "application/json"] {
        let reply = evidence::received(
            client
                .get(format!("{base}/statements"))
                .header("X-Experience-API-Version", "2.1.0")
                .header("Accept", accept)
                .send()
                .await
                .map_err(|e| e.to_string())?,
        )
        .await?;
        assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
        assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
        OffsetDateTime::parse(
            reply.headers()["x-experience-api-consistent-through"]
                .to_str()
                .map_err(|e| e.to_string())?,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| e.to_string())?;
        let media = reply.headers()["content-type"]
            .to_str()
            .map_err(|e| e.to_string())?
            .to_owned();
        let body = reply.text().await.map_err(|e| e.to_string())?;
        assert!(
            body.contains("Version") && body.contains("unsupported"),
            "{body}"
        );
        evidence::observation(
            "unsupported-version-explanation",
            &json!({"accept":accept,"media":media,"body":body}),
        );
    }
    Ok(())
}

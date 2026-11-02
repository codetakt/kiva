//! Independent method/existence/condition tables, including selected D-04=A.
#![allow(clippy::wildcard_imports)]
use super::*;
use crate::surfaces::xapi::model::{
    DocumentAddress, DocumentContent, DocumentIdRequirement, DocumentKind, DocumentSelection,
};

#[tokio::test]
async fn memory_xapi_document_precondition_matrix() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_document_precondition_matrix() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

async fn receive(
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
    assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
    assert_eq!(reply.status(), status, "{}", reply.url());
    Ok(reply)
}

fn address(base: &str, kind: DocumentKind) -> Result<reqwest::Url, String> {
    let path = match kind {
        DocumentKind::State => "activities/state",
        DocumentKind::AgentProfile => "agents/profile",
        DocumentKind::ActivityProfile => "activities/profile",
    };
    let mut url = reqwest::Url::parse(&format!("{base}/{path}")).map_err(|e| e.to_string())?;
    {
        let mut q = url.query_pairs_mut();
        if kind != DocumentKind::AgentProfile {
            q.append_pair("activityId", "https://example.org/conditions");
        }
        if kind != DocumentKind::ActivityProfile {
            q.append_pair("agent", r#"{"mbox":"mailto:conditions@example.org"}"#);
        }
        q.append_pair(kind.id_param(), &Uuid::new_v4().to_string());
    }
    Ok(url)
}

async fn snapshot(
    state: &AppState,
    kind: DocumentKind,
    url: &reqwest::Url,
) -> Result<Option<Value>, String> {
    let query = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    let DocumentSelection::Single(address) =
        DocumentAddress::from_query(kind, &query, DocumentIdRequirement::Required)
            .map_err(|e| format!("{e:?}"))?
    else {
        return Err("single document".into());
    };
    state.xapi.documents.get(&address).await.map_err(|e|format!("{e:?}")).map(|record|record.map(|record| {
        let bytes=match record.content {DocumentContent::Json(value)=>value.to_string().into_bytes(),DocumentContent::Bytes{body,..}=>body};
        json!({"bytes":crate::core::response::hex_lower(&bytes),"content_type":record.content_type,"etag":record.etag,"updated":record.updated_at.unix_timestamp_nanos().to_string()})
    }))
}

async fn setup(
    client: &reqwest::Client,
    url: &reqwest::Url,
    exists: bool,
) -> Result<String, String> {
    if exists {
        receive(
            client.put(url.clone()).json(&json!({"original":1})),
            StatusCode::NO_CONTENT,
        )
        .await?;
        Ok(receive(client.get(url.clone()), StatusCode::OK)
            .await?
            .headers()["etag"]
            .to_str()
            .map_err(|e| e.to_string())?
            .to_owned())
    } else {
        receive(client.get(url.clone()), StatusCode::NOT_FOUND).await?;
        Ok("\"absent-current-fixture\"".into())
    }
}

fn different_tag(current: &str) -> String {
    let opaque = current.strip_prefix("W/").unwrap_or(current);
    assert!(opaque.starts_with('"') && opaque.ends_with('"'));
    // Appending inside the captured opaque value proves inequality without
    // assuming that any fixed literal is unused by the target.
    format!("{}!\"", &opaque[..opaque.len() - 1])
}

fn quoted_delimiters(current: &str, escaped_quote: bool) -> String {
    let prefix = &current[..current.len() - 1];
    if escaped_quote {
        format!("{prefix}\\\",*,tag\"")
    } else {
        format!("{prefix},*,tag\"")
    }
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    state.xapi.documents.clock().set(
        OffsetDateTime::parse(
            "2020-01-01T00:00:00.125Z",
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| e.to_string())?,
    );
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
    evidence::observation(
        "document-precondition-policy",
        &json!({"selected":"D-04=A","undefined_pairs":[["If-Match","If-None-Match"],["If-Match","If-Modified-Since"],["If-None-Match","If-Unmodified-Since"],["If-Unmodified-Since","If-Modified-Since"]],"policy_result":400,"ieee_status":"INDETERMINATE for listed undefined combinations only","ordinary_tags":"strong for writes/If-Match; weak permitted only for read If-None-Match","incorporated":"RFC2616 13.3.4,14.24-14.28; If-None-Match plus If-Modified-Since remains defined","known_updated":"2020-01-01T00:00:00.125Z","missing_headers":"existing changed PUT409; POST/DELETE follow ordinary contracts; sender duty separate"}),
    );
    for kind in [
        DocumentKind::State,
        DocumentKind::AgentProfile,
        DocumentKind::ActivityProfile,
    ] {
        for exists in [false, true] {
            for method in [
                reqwest::Method::PUT,
                reqwest::Method::POST,
                reqwest::Method::DELETE,
            ] {
                for variant in 0..18 {
                    let url = address(&base, kind)?;
                    let tag = setup(&client, &url, exists).await?;
                    let before = snapshot(state, kind, &url).await?;
                    let (header, value, passes) = match variant {
                        0 => (None, String::new(), true),
                        1 => (Some("If-Match"), tag.clone(), exists),
                        2 => (Some("If-Match"), different_tag(&tag), false),
                        3 => (Some("If-Match"), "*".into(), exists),
                        4 => (
                            Some("If-Match"),
                            format!("{}, {tag}", different_tag(&tag)),
                            exists,
                        ),
                        5 => (Some("If-Match"), format!("W/{tag}"), false),
                        6 => (Some("If-None-Match"), tag.clone(), !exists),
                        7 => (Some("If-None-Match"), different_tag(&tag), true),
                        8 => (Some("If-None-Match"), "*".into(), !exists),
                        9 => (
                            Some("If-None-Match"),
                            format!("{}, {tag}", different_tag(&tag)),
                            !exists,
                        ),
                        10 => (Some("If-None-Match"), format!("W/{tag}"), true),
                        11 => (Some("If-Match"), quoted_delimiters(&tag, false), false),
                        12 => (Some("If-None-Match"), quoted_delimiters(&tag, false), true),
                        13 => (
                            Some("If-Match"),
                            format!("{}, {tag}", quoted_delimiters(&tag, false)),
                            exists,
                        ),
                        14 => (
                            Some("If-None-Match"),
                            format!("{}, {tag}", quoted_delimiters(&tag, false)),
                            !exists,
                        ),
                        15 => (Some("If-Match"), quoted_delimiters(&tag, true), false),
                        16 => (Some("If-None-Match"), quoted_delimiters(&tag, true), true),
                        _ => (
                            Some("If-Match"),
                            format!("{}, {tag}", quoted_delimiters(&tag, true)),
                            exists,
                        ),
                    };
                    let status = if !passes {
                        StatusCode::PRECONDITION_FAILED
                    } else if variant == 0 && exists && method == reqwest::Method::PUT {
                        StatusCode::CONFLICT
                    } else {
                        StatusCode::NO_CONTENT
                    };
                    let mut request = client.request(method.clone(), url.clone());
                    if method != reqwest::Method::DELETE {
                        request = request.json(&json!({"changed":true}));
                    }
                    if let Some(header) = header {
                        request = request.header(header, value);
                    }
                    let response = receive(request, status).await?;
                    if status == StatusCode::CONFLICT {
                        let reason = response.text().await.map_err(|e| e.to_string())?;
                        for required in ["current state", "If-Match", "current ETag"] {
                            assert!(reason.contains(required), "{reason}");
                        }
                    }
                    if status != StatusCode::NO_CONTENT {
                        assert_eq!(snapshot(state, kind, &url).await?, before);
                    } else if method == reqwest::Method::DELETE {
                        assert!(snapshot(state, kind, &url).await?.is_none());
                    } else {
                        let got: Value = receive(client.get(url.clone()), StatusCode::OK)
                            .await?
                            .json()
                            .await
                            .map_err(|e| e.to_string())?;
                        assert_eq!(
                            got,
                            if method == reqwest::Method::POST && exists {
                                json!({"original":1,"changed":true})
                            } else {
                                json!({"changed":true})
                            }
                        );
                    }
                }
            }
            for method in [
                reqwest::Method::GET,
                reqwest::Method::HEAD,
                reqwest::Method::PUT,
                reqwest::Method::POST,
                reqwest::Method::DELETE,
            ] {
                let url = address(&base, kind)?;
                let tag = setup(&client, &url, exists).await?;
                let before = snapshot(state, kind, &url).await?;
                for names in [
                    vec!["If-Match", "If-None-Match"],
                    vec!["If-Match", "If-Modified-Since"],
                    vec!["If-None-Match", "If-Unmodified-Since"],
                    vec!["If-Unmodified-Since", "If-Modified-Since"],
                    vec!["If-Match", "If-None-Match", "If-Modified-Since"],
                    vec!["If-Match", "If-None-Match", "If-Unmodified-Since"],
                    vec!["If-Match", "If-Unmodified-Since", "If-Modified-Since"],
                    vec!["If-None-Match", "If-Unmodified-Since", "If-Modified-Since"],
                ] {
                    for matching in [true, false] {
                        let different = different_tag(&tag);
                        let mut request = client.request(method.clone(), url.clone());
                        if matches!(method, reqwest::Method::PUT | reqwest::Method::POST) {
                            request = request.json(&json!({"changed":true}));
                        }
                        for name in &names {
                            request = request.header(
                                *name,
                                if name.ends_with("Since") {
                                    if matching {
                                        "Wed, 01 Jan 2020 00:00:00 GMT"
                                    } else {
                                        "Tue, 31 Dec 2019 23:59:59 GMT"
                                    }
                                } else if matching {
                                    &tag
                                } else {
                                    &different
                                },
                            );
                        }
                        let response = receive(request, StatusCode::BAD_REQUEST).await?;
                        assert_eq!(
                            response.headers()["x-kiva-error"],
                            "undefined-precondition-combination"
                        );
                        let body = response.bytes().await.map_err(|e| e.to_string())?;
                        if method == reqwest::Method::HEAD {
                            assert!(body.is_empty());
                        } else {
                            assert!(
                                String::from_utf8_lossy(&body).contains("undefined combination")
                            );
                        }
                        assert_eq!(snapshot(state, kind, &url).await?, before);
                    }
                }
                let reply = receive(
                    client
                        .request(method.clone(), url.clone())
                        .bearer_auth("refused-credential")
                        .header("If-Match", "*")
                        .header("If-None-Match", "*")
                        .json(&json!({"valid":true})),
                    StatusCode::UNAUTHORIZED,
                )
                .await?;
                assert!(!reply.headers().contains_key("x-kiva-error"));
            }
        }
        verify_cache(&client, &base, kind).await?;
        verify_header_octets(state, &client, &base, kind).await?;
        verify_write_dates(state, &client, &base, kind).await?;
        verify_earlier_failures(&client, &base, kind).await?;
    }
    server.abort();
    Ok(())
}

// Build wire values directly; using the production text codec here would hide
// precisely the transport-boundary defect this regression is intended to catch.
fn octet_tag_list(tag: &[u8], current: &str) -> Vec<u8> {
    [tag, b", ", current.as_bytes()].concat()
}

#[allow(clippy::too_many_lines)]
async fn verify_header_octets(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
    kind: DocumentKind,
) -> Result<(), String> {
    let all_high_octets = [vec![b'"'], (0x80..=0xff).collect(), vec![b'"']].concat();
    for stale in [b"\"\xe9\"".as_slice(), all_high_octets.as_slice()] {
        for exists in [false, true] {
            for method in [
                reqwest::Method::PUT,
                reqwest::Method::POST,
                reqwest::Method::DELETE,
            ] {
                for name in ["If-Match", "If-None-Match"] {
                    for variant in 0..4 {
                        let url = address(base, kind)?;
                        let current = setup(client, &url, exists).await?;
                        assert!(current.is_ascii());
                        let before = snapshot(state, kind, &url).await?;
                        let candidates = match variant {
                            0 => vec![stale.to_vec()],
                            1 => vec![octet_tag_list(stale, &current)],
                            2 => vec![stale.to_vec(), current.as_bytes().to_vec()],
                            _ => vec![current.as_bytes().to_vec(), stale.to_vec()],
                        };
                        let matches = exists && variant != 0;
                        let passes = if name == "If-Match" {
                            matches
                        } else {
                            !matches
                        };
                        let status = if passes {
                            StatusCode::NO_CONTENT
                        } else {
                            StatusCode::PRECONDITION_FAILED
                        };
                        let mut request = client.request(method.clone(), url.clone());
                        if method != reqwest::Method::DELETE {
                            request = request.json(&json!({"changed":true}));
                        }
                        for candidate in candidates {
                            request = request.header(name, candidate);
                        }
                        receive(request, status).await?;
                        if !passes {
                            assert_eq!(snapshot(state, kind, &url).await?, before);
                        } else if method == reqwest::Method::DELETE {
                            assert!(snapshot(state, kind, &url).await?.is_none());
                        } else {
                            let got: Value = receive(client.get(url), StatusCode::OK)
                                .await?
                                .json()
                                .await
                                .map_err(|e| e.to_string())?;
                            assert_eq!(
                                got,
                                if method == reqwest::Method::POST && exists {
                                    json!({"original":1,"changed":true})
                                } else {
                                    json!({"changed":true})
                                }
                            );
                        }
                    }
                }
            }
        }
        let url = address(base, kind)?;
        let current = setup(client, &url, true).await?;
        for method in [reqwest::Method::GET, reqwest::Method::HEAD] {
            for name in ["If-Match", "If-None-Match"] {
                for (candidate, matching) in [
                    (stale.to_vec(), false),
                    (octet_tag_list(stale, &current), true),
                ] {
                    let status = match (name, matching) {
                        ("If-Match", false) => StatusCode::PRECONDITION_FAILED,
                        ("If-None-Match", true) => StatusCode::NOT_MODIFIED,
                        _ => StatusCode::OK,
                    };
                    let reply = receive(
                        client
                            .request(method.clone(), url.clone())
                            .header(name, candidate),
                        status,
                    )
                    .await?;
                    if method == reqwest::Method::HEAD || status == StatusCode::NOT_MODIFIED {
                        assert!(reply.bytes().await.map_err(|e| e.to_string())?.is_empty());
                    }
                }
            }
        }
    }
    Ok(())
}

async fn verify_cache(
    client: &reqwest::Client,
    base: &str,
    kind: DocumentKind,
) -> Result<(), String> {
    let url = address(base, kind)?;
    let tag = setup(client, &url, true).await?;
    for method in [reqwest::Method::GET, reqwest::Method::HEAD] {
        for (headers, status) in [
            (vec![("If-Match", tag.clone())], StatusCode::OK),
            (
                vec![("If-Match", quoted_delimiters(&tag, false))],
                StatusCode::PRECONDITION_FAILED,
            ),
            (
                vec![("If-Match", quoted_delimiters(&tag, true))],
                StatusCode::PRECONDITION_FAILED,
            ),
            (
                vec![("If-None-Match", quoted_delimiters(&tag, false))],
                StatusCode::OK,
            ),
            (
                vec![("If-None-Match", quoted_delimiters(&tag, true))],
                StatusCode::OK,
            ),
            (
                vec![(
                    "If-None-Match",
                    format!("W/{}", quoted_delimiters(&tag, false)),
                )],
                StatusCode::OK,
            ),
            (
                vec![(
                    "If-None-Match",
                    format!("{}, W/{tag}", quoted_delimiters(&tag, true)),
                )],
                StatusCode::NOT_MODIFIED,
            ),
            (
                vec![
                    ("If-Match", quoted_delimiters(&tag, true)),
                    ("If-Match", tag.clone()),
                ],
                StatusCode::OK,
            ),
            (
                vec![("If-Match", different_tag(&tag))],
                StatusCode::PRECONDITION_FAILED,
            ),
            (
                vec![("If-Match", format!("W/{tag}"))],
                StatusCode::PRECONDITION_FAILED,
            ),
            (
                vec![("If-Match", different_tag(&tag)), ("If-Match", tag.clone())],
                StatusCode::OK,
            ),
            (
                vec![("If-None-Match", format!("W/{tag}"))],
                StatusCode::NOT_MODIFIED,
            ),
            (
                vec![("If-Modified-Since", "Wed, 01 Jan 2020 00:00:00 GMT".into())],
                StatusCode::NOT_MODIFIED,
            ),
            (
                vec![(
                    "If-Modified-Since",
                    "Wednesday, 01-Jan-20 00:00:00 GMT".into(),
                )],
                StatusCode::NOT_MODIFIED,
            ),
            (
                vec![("If-Modified-Since", "Wed Jan  1 00:00:00 2020".into())],
                StatusCode::NOT_MODIFIED,
            ),
            (
                vec![("If-Modified-Since", "Fri, 01 Jan 2100 00:00:00 GMT".into())],
                StatusCode::OK,
            ),
            (
                vec![("If-Modified-Since", "invalid".into())],
                StatusCode::OK,
            ),
            (
                vec![(
                    "If-Unmodified-Since",
                    "Tue, 31 Dec 2019 23:59:59 GMT".into(),
                )],
                StatusCode::PRECONDITION_FAILED,
            ),
            (
                vec![
                    ("If-None-Match", tag.clone()),
                    ("If-Modified-Since", "Tue, 31 Dec 2019 23:59:59 GMT".into()),
                ],
                StatusCode::OK,
            ),
            (
                vec![
                    ("If-None-Match", tag.clone()),
                    ("If-Modified-Since", "Wed, 01 Jan 2020 00:00:00 GMT".into()),
                ],
                StatusCode::NOT_MODIFIED,
            ),
            (
                vec![
                    ("If-None-Match", different_tag(&tag)),
                    ("If-Modified-Since", "Wed, 01 Jan 2020 00:00:00 GMT".into()),
                ],
                StatusCode::OK,
            ),
        ] {
            let mut request = client.request(method.clone(), url.clone());
            for (name, value) in headers {
                request = request.header(name, value);
            }
            let response = receive(request, status).await?;
            if status == StatusCode::NOT_MODIFIED {
                assert_eq!(response.headers()["etag"], tag);
            }
            if method == reqwest::Method::HEAD || status == StatusCode::NOT_MODIFIED {
                assert!(
                    response
                        .bytes()
                        .await
                        .map_err(|e| e.to_string())?
                        .is_empty()
                );
            }
        }
    }
    Ok(())
}

// Explicitly exercise the defined dual-condition branch on mutations, not
// merely on cache reads. Each request uses an independent resource.
async fn verify_write_dates(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
    kind: DocumentKind,
) -> Result<(), String> {
    for exists in [false, true] {
        for method in [
            reqwest::Method::PUT,
            reqwest::Method::POST,
            reqwest::Method::DELETE,
        ] {
            for (tag_matches, date, unchanged) in [
                (true, "Tue, 31 Dec 2019 23:59:59 GMT", false),
                (true, "Wed, 01 Jan 2020 00:00:00 GMT", true),
                (false, "Wed, 01 Jan 2020 00:00:00 GMT", true),
            ] {
                let url = address(base, kind)?;
                let tag = setup(client, &url, exists).await?;
                let before = snapshot(state, kind, &url).await?;
                let denied = exists && tag_matches && unchanged;
                let mut request = client
                    .request(method.clone(), url.clone())
                    .header(
                        "If-None-Match",
                        if tag_matches {
                            tag
                        } else {
                            different_tag(&tag)
                        },
                    )
                    .header("If-Modified-Since", date);
                if method != reqwest::Method::DELETE {
                    request = request.json(&json!({"changed":true}));
                }
                let response = receive(
                    request,
                    if denied {
                        StatusCode::PRECONDITION_FAILED
                    } else {
                        StatusCode::NO_CONTENT
                    },
                )
                .await?;
                assert!(!response.headers().contains_key("x-kiva-error"));
                if denied {
                    assert_eq!(snapshot(state, kind, &url).await?, before);
                } else if method == reqwest::Method::DELETE {
                    assert!(snapshot(state, kind, &url).await?.is_none());
                } else {
                    let got: Value = receive(client.get(url), StatusCode::OK)
                        .await?
                        .json()
                        .await
                        .map_err(|e| e.to_string())?;
                    assert_eq!(
                        got,
                        if exists && method == reqwest::Method::POST {
                            json!({"original":1,"changed":true})
                        } else {
                            json!({"changed":true})
                        }
                    );
                }
            }
        }
    }
    Ok(())
}

async fn verify_earlier_failures(
    client: &reqwest::Client,
    base: &str,
    kind: DocumentKind,
) -> Result<(), String> {
    let url = address(base, kind)?;
    setup(client, &url, true).await?;
    let mut invalid_query = url.clone();
    invalid_query
        .query_pairs_mut()
        .append_pair("unrecognized", "true");
    for (request, status) in [
        (
            client
                .put(url.clone())
                .bearer_auth("statement-reader")
                .json(&json!({})),
            StatusCode::FORBIDDEN,
        ),
        (client.patch(url.clone()), StatusCode::METHOD_NOT_ALLOWED),
        (
            client
                .post(url.clone())
                .header("content-type", "application/json")
                .body("{"),
            StatusCode::BAD_REQUEST,
        ),
        (client.get(invalid_query), StatusCode::BAD_REQUEST),
    ] {
        let response = receive(
            request.header("If-Match", "*").header("If-None-Match", "*"),
            status,
        )
        .await?;
        assert!(!response.headers().contains_key("x-kiva-error"));
    }
    let got: Value = receive(client.get(url), StatusCode::OK)
        .await?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(got, json!({"original":1}));
    let malformed = address(base, kind)?;
    receive(
        client
            .put(malformed.clone())
            .header("Content-Type", "application/json")
            .body("[1,2]"),
        StatusCode::NO_CONTENT,
    )
    .await?;
    let malformed_tag = receive(client.get(malformed.clone()), StatusCode::OK)
        .await?
        .headers()["etag"]
        .to_str()
        .map_err(|e| e.to_string())?
        .to_owned();
    for mixed in [false, true] {
        let mut request = client
            .post(malformed.clone())
            .json(&json!({"change":true}))
            .header("If-Match", different_tag(&malformed_tag));
        if mixed {
            request = request.header("If-None-Match", "*");
        }
        let response = receive(request, StatusCode::BAD_REQUEST).await?;
        assert!(!response.headers().contains_key("x-kiva-error"));
        assert_eq!(
            receive(client.get(malformed.clone()), StatusCode::OK)
                .await?
                .bytes()
                .await
                .map_err(|e| e.to_string())?,
            "[1,2]"
        );
    }
    Ok(())
}

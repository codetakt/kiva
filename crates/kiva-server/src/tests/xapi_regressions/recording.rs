#![allow(clippy::wildcard_imports)]

use super::*;
use time::format_description::well_known::Rfc3339;

#[tokio::test]
async fn memory_xapi_stored_is_independent_recording_time() -> Result<(), String> {
    verify_stored(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_stored_is_independent_recording_time() -> Result<(), String> {
    verify_stored(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

async fn submit_json(
    client: &reqwest::Client,
    base: &str,
    value: &Value,
    post: bool,
) -> Result<reqwest::Response, String> {
    if !post {
        return put(client, base, value).await;
    }
    let reply = evidence::received(
        client
            .post(format!("{base}/statements"))
            .header("X-Experience-API-Version", "2.0.0")
            .json(value)
            .send()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await?;
    watermark(&reply)?;
    Ok(reply)
}

#[allow(clippy::too_many_lines)]
async fn verify_stored(state: &AppState) -> Result<(), String> {
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
    let clock = state.xapi.statements.clock();
    let mut recording = instant("2020-01-01T00:00:01.250Z")?;
    for post in [false, true] {
        for stored in [
            None,
            Some(json!("1900-01-01T00:00:00Z")),
            Some(json!("not-a-timestamp")),
            Some(json!(42)),
        ] {
            for supplied_timestamp in [false, true] {
                let mut value = statement(Uuid::new_v4());
                if let Some(stored) = &stored {
                    value["stored"] = stored.clone();
                }
                if supplied_timestamp {
                    value["timestamp"] = json!("2010-02-03T12:34:56.123+09:00");
                }
                clock.set(recording);
                let reply = submit_json(&client, &base, &value, post).await?;
                let status = reply.status();
                let invalid = stored.as_ref().is_some_and(|v| v != "1900-01-01T00:00:00Z");
                if invalid {
                    evidence::observation(
                        "stored-validation-advisory",
                        &json!({"input":stored,"post":post,"status":status.as_u16(),"strength":"SHOULD validation; accepted inputs still require mandatory overwrite","independent_recording":recording.format(&Rfc3339).map_err(|e|e.to_string())?}),
                    );
                    if status == StatusCode::BAD_REQUEST {
                        let missing = fetch(
                            &client,
                            reqwest::Method::GET,
                            &format!(
                                "{base}/statements?statementId={}",
                                value["id"].as_str().ok_or("ID")?
                            ),
                        )
                        .await?;
                        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
                        recording += Duration::seconds(1);
                        continue;
                    }
                }
                assert_eq!(
                    status,
                    if post {
                        StatusCode::OK
                    } else {
                        StatusCode::NO_CONTENT
                    }
                );
                if post {
                    assert_eq!(json(reply).await?, json!([value["id"]]));
                }
                let target = format!(
                    "{base}/statements?statementId={}&format=exact",
                    value["id"].as_str().ok_or("ID")?
                );
                let reply = fetch(&client, reqwest::Method::GET, &target).await?;
                assert_eq!(reply.status(), StatusCode::OK);
                assert_eq!(
                    reply.headers()["last-modified"],
                    format!("Wed, 01 Jan 2020 00:00:{:02} GMT", recording.second())
                );
                let original = json(reply).await?;
                assert_eq!(
                    instant(original["stored"].as_str().ok_or("stored")?)?,
                    recording
                );
                if supplied_timestamp {
                    assert_eq!(
                        instant(original["timestamp"].as_str().ok_or("timestamp")?)?,
                        instant("2010-02-03T03:34:56.123Z")?
                    );
                } else {
                    assert_eq!(original["timestamp"], original["stored"]);
                }
                let before = state
                    .xapi
                    .statements
                    .list(None)
                    .await
                    .map_err(|e| format!("{e:?}"))?;
                clock.set(recording + Duration::hours(1));
                let mut changed = value.clone();
                changed["stored"] = json!("2030-01-01T00:00:00Z");
                changed["verb"]["display"] = json!({"en":"attempted replacement"});
                let replay_attempt = submit_json(&client, &base, &changed, post).await?;
                assert!(
                    replay_attempt.status() == StatusCode::CONFLICT
                        || replay_attempt.status()
                            == if post {
                                StatusCode::OK
                            } else {
                                StatusCode::NO_CONTENT
                            }
                );
                assert_eq!(
                    json(fetch(&client, reqwest::Method::GET, &target).await?).await?,
                    original
                );
                assert_eq!(
                    state
                        .xapi
                        .statements
                        .list(None)
                        .await
                        .map_err(|e| format!("{e:?}"))?,
                    before
                );
                recording += Duration::seconds(1);
            }
        }
    }
    server.abort();
    Ok(())
}

#[tokio::test]
async fn memory_xapi_recording_visibility_and_clock_boundaries() -> Result<(), String> {
    verify_recording(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_recording_visibility_and_clock_boundaries() -> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or("PostgreSQL URL must be set")?;
    verify_recording(&state).await
}

#[tokio::test]
async fn memory_xapi_document_recording_boundaries() -> Result<(), String> {
    verify_document_recording(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_document_recording_boundaries() -> Result<(), String> {
    let state = postgres_test_state()
        .await?
        .ok_or("PostgreSQL URL must be set")?;
    verify_document_recording(&state).await
}

async fn document_request(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
    etag: Option<&str>,
    body: &[u8],
) -> Result<reqwest::Response, String> {
    document_typed_request(client, method, url, etag, "text/plain", body).await
}

async fn document_typed_request(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
    etag: Option<&str>,
    content_type: &str,
    body: &[u8],
) -> Result<reqwest::Response, String> {
    let mut request = client
        .request(method, url)
        .header("X-Experience-API-Version", "2.0.0")
        .header("Content-Type", content_type)
        .body(body.to_vec());
    if let Some(etag) = etag {
        request = request.header("If-Match", etag);
    }
    let reply = request.send().await.map_err(|error| error.to_string())?;
    assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
    evidence::received(reply).await
}

async fn document_snapshot(
    client: &reqwest::Client,
    url: &str,
) -> Result<(HeaderMap, Vec<u8>), String> {
    let reply = fetch(client, reqwest::Method::GET, url).await?;
    assert_eq!(reply.status(), StatusCode::OK);
    let mut headers = reply.headers().clone();
    headers.remove("date");
    let bytes = reply.bytes().await.map_err(|error| error.to_string())?;
    Ok((headers, bytes.to_vec()))
}

#[allow(clippy::too_many_lines)]
async fn verify_document_recording(state: &AppState) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| error.to_string())?;
    let base = format!(
        "http://{}",
        listener.local_addr().map_err(|error| error.to_string())?
    );
    let router = evidence::router(state);
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let clock = state.xapi.documents.clock();
    let t0 = instant("2020-01-01T00:00:00.125Z")?;
    let t1 = instant("2020-01-01T00:00:00.875Z")?;
    let bytes = [0_u8, 255, 128, 123, 125];
    let actor = json!({"mbox":"mailto:clock-a@example.org"}).to_string();
    let other_actor = json!({"mbox":"mailto:clock-b@example.org"}).to_string();
    for (path, id_key) in [
        ("/activities/state", "stateId"),
        ("/activities/profile", "profileId"),
        ("/agents/profile", "profileId"),
    ] {
        let mut scope = Vec::new();
        if path != "/agents/profile" {
            scope.push(("activityId", "https://example.org/clock-a"));
        }
        if path != "/activities/profile" {
            scope.push(("agent", actor.as_str()));
        }
        if path == "/activities/state" {
            scope.push(("registration", "3fc682e5-7884-4521-a2d5-c8ae75729f79"));
        }
        let listing = reqwest::Url::parse_with_params(&format!("{base}{path}"), &scope)
            .map_err(|error| error.to_string())?;
        let mut one = listing.clone();
        one.query_pairs_mut().append_pair(id_key, "one");
        let mut two = listing.clone();
        two.query_pairs_mut().append_pair(id_key, "two");
        let mut other_scope = scope.clone();
        if path == "/agents/profile" {
            other_scope[0].1 = &other_actor;
        } else {
            other_scope[0].1 = "https://example.org/clock-b";
        }
        let mut other = reqwest::Url::parse_with_params(&format!("{base}{path}"), &other_scope)
            .map_err(|error| error.to_string())?;
        other.query_pairs_mut().append_pair(id_key, "one");
        clock.set(t0);
        for url in [&one, &two, &other] {
            assert_eq!(
                document_request(&client, reqwest::Method::PUT, url.as_str(), None, &bytes)
                    .await?
                    .status(),
                StatusCode::NO_CONTENT
            );
        }
        let original = document_snapshot(&client, one.as_str()).await?;
        assert_eq!(original.1, bytes);
        assert_eq!(original.0["content-type"], "text/plain");
        assert_eq!(original.0["last-modified"], "Wed, 01 Jan 2020 00:00:00 GMT");
        let other_before = document_snapshot(&client, other.as_str()).await?;
        let mut protected_scopes = Vec::new();
        if path == "/activities/state" {
            // Change each independent scope selector, retaining all others.
            for (name, value) in [
                ("agent", other_actor.as_str()),
                ("registration", "5d4d70cc-3cc5-45e1-84a0-257c949b0cef"),
            ] {
                let mut pairs = scope.clone();
                pairs.retain(|(key, _)| *key != name);
                pairs.push((name, value));
                pairs.push((id_key, "one"));
                let url = reqwest::Url::parse_with_params(&format!("{base}{path}"), &pairs)
                    .map_err(|e| e.to_string())?;
                assert_eq!(
                    document_request(&client, reqwest::Method::PUT, url.as_str(), None, &bytes)
                        .await?
                        .status(),
                    StatusCode::NO_CONTENT
                );
                let snapshot = document_snapshot(&client, url.as_str()).await?;
                protected_scopes.push((url, snapshot));
            }
        }
        clock.set(t1);
        let etag = original.0["etag"]
            .to_str()
            .map_err(|error| error.to_string())?;
        assert_eq!(
            document_request(
                &client,
                reqwest::Method::PUT,
                one.as_str(),
                Some(etag),
                b"changed"
            )
            .await?
            .status(),
            StatusCode::NO_CONTENT
        );
        let updated = document_snapshot(&client, one.as_str()).await?;
        assert_eq!(updated.1, b"changed");
        assert_ne!(updated.0["etag"], original.0["etag"]);
        // Both updates occupy the SAME HTTP-date second: only the independent
        // subsecond clock can distinguish the exclusive since boundary.
        assert_eq!(updated.0["last-modified"], original.0["last-modified"]);
        for (time, expected) in [
            (t0 - Duration::microseconds(1), vec!["one", "two"]),
            (t0, vec!["one"]),
            (t0 + Duration::microseconds(1), vec!["one"]),
            (t1 - Duration::microseconds(1), vec!["one"]),
            (t1, vec![]),
            (t1 + Duration::microseconds(1), vec![]),
        ] {
            let mut url = listing.clone();
            url.query_pairs_mut().append_pair(
                "since",
                &time.format(&Rfc3339).map_err(|error| error.to_string())?,
            );
            let response = fetch(&client, reqwest::Method::GET, url.as_str()).await?;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(json(response).await?, json!(expected));
        }
        let head = fetch(&client, reqwest::Method::HEAD, one.as_str()).await?;
        assert_eq!(head.status(), StatusCode::OK);
        assert_eq!(head.headers()["x-experience-api-version"], "2.0.0");
        assert!(
            head.bytes()
                .await
                .map_err(|error| error.to_string())?
                .is_empty()
        );
        clock.set(t1 + Duration::seconds(3));
        for method in [
            reqwest::Method::PUT,
            reqwest::Method::POST,
            reqwest::Method::DELETE,
        ] {
            assert_eq!(
                document_request(
                    &client,
                    method.clone(),
                    one.as_str(),
                    Some(etag),
                    b"rejected"
                )
                .await?
                .status(),
                // This fixture is text/plain, so POST is already invalid
                // without its conditional header. Valid JSON POST conditions
                // have the separate full method/existence matrix.
                if method == reqwest::Method::POST {
                    StatusCode::BAD_REQUEST
                } else {
                    StatusCode::PRECONDITION_FAILED
                }
            );
            assert_eq!(document_snapshot(&client, one.as_str()).await?, updated);
        }
        let conflict = document_request(
            &client,
            reqwest::Method::PUT,
            one.as_str(),
            None,
            b"rejected",
        )
        .await?;
        assert_eq!(conflict.status(), StatusCode::CONFLICT);
        let explanation = conflict.text().await.map_err(|e| e.to_string())?;
        assert!(
            explanation.contains("current state")
                && explanation.contains("If-Match")
                && explanation.contains("ETag")
        );
        assert_eq!(document_snapshot(&client, one.as_str()).await?, updated);
        assert_eq!(
            document_snapshot(&client, other.as_str()).await?,
            other_before
        );
        // An unsuccessful mutation also leaves the independently tested
        // update instant unchanged, not just its rounded Last-Modified value.
        let mut after = listing.clone();
        after
            .query_pairs_mut()
            .append_pair("since", "2020-01-01T00:00:00.875Z");
        assert_eq!(
            json(fetch(&client, reqwest::Method::GET, after.as_str()).await?).await?,
            json!([])
        );
        let current_tag = updated.0["etag"].to_str().map_err(|e| e.to_string())?;
        assert_eq!(
            document_request(
                &client,
                reqwest::Method::PUT,
                one.as_str(),
                Some(current_tag),
                b"third"
            )
            .await?
            .status(),
            StatusCode::NO_CONTENT
        );
        let third = document_snapshot(&client, one.as_str()).await?;
        assert_eq!(third.0["last-modified"], "Wed, 01 Jan 2020 00:00:03 GMT");
        assert_eq!(third.1, b"third");
        assert_eq!(
            json(fetch(&client, reqwest::Method::GET, after.as_str()).await?).await?,
            json!(["one"])
        );
        if path == "/activities/state" {
            assert_eq!(
                document_request(
                    &client,
                    reqwest::Method::DELETE,
                    listing.as_str(),
                    None,
                    b""
                )
                .await?
                .status(),
                StatusCode::NO_CONTENT
            );
            assert_eq!(
                json(fetch(&client, reqwest::Method::GET, listing.as_str()).await?).await?,
                json!([])
            );
            assert_eq!(
                document_snapshot(&client, other.as_str()).await?,
                other_before
            );
        }
        for (url, snapshot) in protected_scopes {
            assert_eq!(document_snapshot(&client, url.as_str()).await?, snapshot);
        }
        verify_document_merges(&client, &listing, id_key, state).await?;
    }
    server.abort();
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn verify_document_merges(
    client: &reqwest::Client,
    listing: &reqwest::Url,
    id_key: &str,
    state: &AppState,
) -> Result<(), String> {
    let clock = state.xapi.documents.clock();
    let original = instant("2020-01-02T00:00:00.125Z")?;
    let later = instant("2020-01-02T00:00:00.875Z")?;
    let mut one = listing.clone();
    one.query_pairs_mut().append_pair(id_key, "merge");
    clock.set(original);
    let created = document_typed_request(
        client,
        reqwest::Method::POST,
        one.as_str(),
        None,
        "application/json",
        br#"{"keep":1,"nested":{"a":1,"b":2}}"#,
    )
    .await?;
    assert_eq!(created.status(), StatusCode::NO_CONTENT);
    let before = document_snapshot(client, one.as_str()).await?;
    assert_eq!(
        serde_json::from_slice::<Value>(&before.1).map_err(|e| e.to_string())?,
        json!({"keep":1,"nested":{"a":1,"b":2}})
    );
    clock.set(later);
    assert_eq!(
        document_typed_request(
            client,
            reqwest::Method::POST,
            one.as_str(),
            None,
            "application/json",
            br#"{"nested":{"a":3}}"#
        )
        .await?
        .status(),
        StatusCode::NO_CONTENT
    );
    let merged = document_snapshot(client, one.as_str()).await?;
    assert_eq!(
        serde_json::from_slice::<Value>(&merged.1).map_err(|e| e.to_string())?,
        json!({"keep":1,"nested":{"a":3}})
    );
    // The JSON merge is shallow. All invalid incoming partitions must leave
    // bytes, ETag and independently observable update instant unchanged.
    clock.set(later + Duration::seconds(10));
    for (content_type, body) in [
        ("application/json", b"{".as_slice()),
        ("application/json", b"42"),
        ("application/json", b"[]"),
        ("text/plain", b"{}"),
    ] {
        let rejected = document_typed_request(
            client,
            reqwest::Method::POST,
            one.as_str(),
            None,
            content_type,
            body,
        )
        .await?;
        assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
        assert_eq!(document_snapshot(client, one.as_str()).await?, merged);
    }
    let mut after = listing.clone();
    after
        .query_pairs_mut()
        .append_pair("since", "2020-01-02T00:00:00.875Z");
    assert_eq!(
        json(fetch(client, reqwest::Method::GET, after.as_str()).await?).await?,
        json!([])
    );
    // Every existing representation, including malformed declared JSON, is
    // installed through the public byte-preserving PUT interface.
    for (index, (content_type, body)) in [
        ("application/json", b"42".as_slice()),
        ("application/json", b"[]"),
        ("text/plain", b"{}"),
        ("application/json", b"{"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut url = listing.clone();
        let id = format!("existing-{index}");
        url.query_pairs_mut().append_pair(id_key, &id);
        clock.set(original);
        assert_eq!(
            document_typed_request(
                client,
                reqwest::Method::PUT,
                url.as_str(),
                None,
                content_type,
                body
            )
            .await?
            .status(),
            StatusCode::NO_CONTENT
        );
        let snapshot = document_snapshot(client, url.as_str()).await?;
        clock.set(later);
        assert_eq!(
            document_typed_request(
                client,
                reqwest::Method::POST,
                url.as_str(),
                None,
                "application/json",
                b"{}"
            )
            .await?
            .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(document_snapshot(client, url.as_str()).await?, snapshot);
    }
    let mut after = listing.clone();
    after
        .query_pairs_mut()
        .append_pair("since", "2020-01-02T00:00:00.125Z");
    assert_eq!(
        json(fetch(client, reqwest::Method::GET, after.as_str()).await?).await?,
        json!(["merge"])
    );
    // Empty is still a present String ID. In particular, DELETE stateId=
    // must select this document, never the absent-ID bulk-delete operation.
    let mut empty = listing.clone();
    empty.query_pairs_mut().append_pair(id_key, "");
    assert_eq!(
        document_request(
            client,
            reqwest::Method::PUT,
            empty.as_str(),
            None,
            b"empty ID"
        )
        .await?
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        document_snapshot(client, empty.as_str()).await?.1,
        b"empty ID"
    );
    assert!(
        json(fetch(client, reqwest::Method::GET, listing.as_str()).await?)
            .await?
            .as_array()
            .ok_or("IDs")?
            .contains(&json!(""))
    );
    assert_eq!(
        document_request(client, reqwest::Method::DELETE, empty.as_str(), None, b"")
            .await?
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(document_snapshot(client, one.as_str()).await?, merged);
    Ok(())
}

pub(super) fn instant(value: &str) -> Result<OffsetDateTime, String> {
    OffsetDateTime::parse(value, &Rfc3339).map_err(|error| error.to_string())
}

async fn fetch(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
) -> Result<reqwest::Response, String> {
    let reply = client
        .request(method, url)
        .header("X-Experience-API-Version", "2.0.0")
        .send()
        .await
        .map_err(|error| error.to_string())?;
    assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
    if reqwest::Url::parse(url)
        .map_err(|e| e.to_string())?
        .path()
        .starts_with("/statements")
    {
        watermark(&reply)?;
    }
    evidence::received(reply).await
}

async fn put(
    client: &reqwest::Client,
    base: &str,
    value: &Value,
) -> Result<reqwest::Response, String> {
    let reply = client
        .put(format!(
            "{base}/statements?statementId={}",
            value["id"].as_str().ok_or("Statement ID")?
        ))
        .header("X-Experience-API-Version", "2.0.0")
        .json(value)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    watermark(&reply)?;
    evidence::received(reply).await
}

pub(super) fn watermark(reply: &reqwest::Response) -> Result<OffsetDateTime, String> {
    assert_eq!(
        reply
            .headers()
            .get("X-Experience-API-Version")
            .and_then(|v| v.to_str().ok()),
        Some("2.0.0")
    );
    instant(
        reply
            .headers()
            .get("X-Experience-API-Consistent-Through")
            .ok_or("missing consistency watermark")?
            .to_str()
            .map_err(|error| error.to_string())?,
    )
}

pub(super) async fn json(reply: reqwest::Response) -> Result<Value, String> {
    assert_eq!(reply.status(), StatusCode::OK);
    reply.json().await.map_err(|error| error.to_string())
}

async fn assert_last_modified(
    client: &reqwest::Client,
    url: &str,
    expected: &str,
    count: Option<usize>,
) -> Result<(), String> {
    let reply = fetch(client, reqwest::Method::GET, url).await?;
    assert_eq!(reply.status(), StatusCode::OK);
    assert_eq!(reply.headers()["last-modified"], expected);
    let value = json(reply).await?;
    if let Some(count) = count {
        assert_eq!(value["statements"].as_array().ok_or("array")?.len(), count);
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::many_single_char_names)] // A/B/C are the issued recording-clock fixtures.
async fn verify_recording(state: &AppState) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| error.to_string())?;
    let base = format!(
        "http://{}",
        listener.local_addr().map_err(|error| error.to_string())?
    );
    let (entered_tx, mut entered_rx) = tokio::sync::mpsc::channel(2);
    let router = evidence::router(state).layer(axum::middleware::from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
            let sender = entered_tx.clone();
            async move {
                if request.headers().contains_key("x-kiva-clock-inflight") {
                    sender
                        .send(request.method().to_string())
                        .await
                        .expect("recording request receipt");
                }
                next.run(request).await
            }
        },
    ));
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|error| error.to_string())?;
    let clock = state.xapi.statements.clock();
    let t0 = instant("2020-01-01T00:00:00.100Z")?;
    let t1 = instant("2020-01-01T00:00:00.200Z")?;
    let t2 = instant("2020-01-01T00:00:00.300Z")?;
    let t3 = instant("2020-01-01T00:00:03.400Z")?;
    let mut a = statement(Uuid::new_v4());
    let verb = format!("https://example.org/recording/{}", Uuid::new_v4());
    a["verb"]["id"] = json!(verb);
    a["object"] = json!({"objectType":"SubStatement", "actor": a["actor"], "verb": a["verb"], "object":{"id":"https://example.org/nested"}});
    clock.set(t0);
    let admitted = put(&client, &base, &a).await?;
    assert_eq!(admitted.status(), StatusCode::NO_CONTENT);
    let first = json(
        fetch(
            &client,
            reqwest::Method::GET,
            &format!(
                "{base}/statements?format=exact&statementId={}",
                a["id"].as_str().ok_or("ID")?
            ),
        )
        .await?,
    )
    .await?;
    assert_eq!(instant(first["stored"].as_str().ok_or("stored")?)?, t0);
    assert_eq!(first["timestamp"], first["stored"]);
    assert_eq!(first["object"]["timestamp"], first["stored"]);

    let mut b = statement(Uuid::new_v4());
    b["verb"]["id"] = json!(verb);
    b["timestamp"] = json!("2010-01-01T00:00:00Z");
    b["stored"] = json!("1900-01-01T00:00:00Z");
    clock.set(t1);
    let posted = client
        .post(format!("{base}/statements"))
        .header("X-Experience-API-Version", "2.0.0")
        .json(&b)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let posted = evidence::received(posted).await?;
    assert_eq!(posted.status(), StatusCode::OK);
    watermark(&posted)?;
    assert_eq!(json(posted).await?, json!([b["id"]]));
    let second = json(
        fetch(
            &client,
            reqwest::Method::GET,
            &format!(
                "{base}/statements?format=exact&statementId={}",
                b["id"].as_str().ok_or("ID")?
            ),
        )
        .await?,
    )
    .await?;
    assert_eq!(instant(second["stored"].as_str().ok_or("stored")?)?, t1);
    assert_eq!(second["timestamp"], b["timestamp"]);

    let mut c = statement(Uuid::new_v4());
    c["verb"]["id"] = json!(verb);
    clock.set(t2);
    let (assigned, release) = clock.pause_next();
    let pending = {
        let client = client.clone();
        let base = base.clone();
        let c = c.clone();
        tokio::spawn(async move { put(&client, &base, &c).await })
    };
    assert_eq!(assigned.await.map_err(|error| error.to_string())?, t2);
    assert!(!pending.is_finished());
    evidence::observation(
        "independent-recording-intervals",
        &json!({
            "A":["2020-01-01T00:00:00.100Z","2020-01-01T00:00:00.100Z"],
            "B":["2020-01-01T00:00:00.200Z","2020-01-01T00:00:00.200Z"],
            "C":["2020-01-01T00:00:00.300Z","2020-01-01T00:00:00.300Z"],
            "uncertainty":"zero for injected repository clock; C acknowledged by actual admission callback before publication",
            "verb":verb,"ids":[a["id"],b["id"],c["id"]]
        }),
    );
    let mut retries = Vec::new();
    for (post, original) in [(false, a.clone()), (true, b.clone())] {
        let client = client.clone();
        let base = base.clone();
        retries.push(tokio::spawn(async move {
            let request = if post {
                client.post(format!("{base}/statements"))
            } else {
                client.put(format!(
                    "{base}/statements?statementId={}",
                    original["id"].as_str().expect("id")
                ))
            };
            let reply = evidence::received(
                request
                    .header("X-Experience-API-Version", "2.0.0")
                    .header("x-kiva-clock-inflight", "yes")
                    .json(&original)
                    .send()
                    .await
                    .map_err(|e| e.to_string())?,
            )
            .await?;
            watermark(&reply)?;
            assert_eq!(
                reply.status(),
                if post {
                    StatusCode::OK
                } else {
                    StatusCode::NO_CONTENT
                }
            );
            if post {
                assert_eq!(json(reply).await?, json!([original["id"]]));
            }
            Ok::<_, String>(())
        }));
    }
    let mut methods = vec![
        entered_rx.recv().await.ok_or("PUT/POST entry")?,
        entered_rx.recv().await.ok_or("PUT/POST entry")?,
    ];
    methods.sort();
    assert_eq!(methods, ["POST", "PUT"]);
    // Both actual handlers have been entered while C is held. Kiva serializes
    // all admissions, including retries; it has no accepted-but-hidden worker.
    // There is consequently no successful write response during this interval.
    assert!(retries.iter().all(|retry| !retry.is_finished()));
    // The independent admission barrier proves the assigned time of C while C
    // is still absent. Neither C's output nor a guessed sleep is the oracle.
    for (method, suffix, status) in [
        (reqwest::Method::GET, String::new(), StatusCode::OK),
        (reqwest::Method::HEAD, String::new(), StatusCode::OK),
        (
            reqwest::Method::GET,
            format!("?statementId={}", c["id"].as_str().ok_or("ID")?),
            StatusCode::NOT_FOUND,
        ),
        (
            reqwest::Method::GET,
            "?statementId=bad".to_owned(),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let reply = fetch(
            &client,
            method.clone(),
            &format!("{base}/statements{suffix}"),
        )
        .await?;
        assert_eq!(reply.status(), status);
        assert_eq!(watermark(&reply)?, t1);
        if method == reqwest::Method::HEAD {
            assert!(
                reply
                    .bytes()
                    .await
                    .map_err(|error| error.to_string())?
                    .is_empty()
            );
        } else if suffix.is_empty() {
            assert_eq!(
                json(reply).await?["statements"]
                    .as_array()
                    .ok_or("array")?
                    .len(),
                2
            );
        }
    }
    // A later writer must not commit beyond the hidden earlier recording.
    clock.set(t3);
    let mut d = statement(Uuid::new_v4());
    d["verb"]["id"] = json!(verb);
    let later = {
        let client = client.clone();
        let base = base.clone();
        let d = d.clone();
        tokio::spawn(async move { put(&client, &base, &d).await })
    };
    release
        .send(())
        .map_err(|()| "recording release".to_owned())?;
    let admitted_c = pending.await.map_err(|error| error.to_string())??;
    assert_eq!(admitted_c.status(), StatusCode::NO_CONTENT);
    assert!(watermark(&admitted_c)? <= t2);
    for retry in retries {
        retry.await.map_err(|e| e.to_string())??;
    }
    evidence::observation(
        "hidden-writes-response-boundary",
        &json!({
            "PUT_POST_received_while_C_hidden":true,"successful_write_response_while_C_hidden":false,
            "reason":"actual synchronous admission waits for the shared writer lock; replies consumed after release",
            "requirement":"no requirement that acceptance must wait or precede visibility; no fabricated pre-release response",
            "retrieval_readiness":"correlated release and successful handler completion, no guessed sleep or protocol deadline"
        }),
    );
    assert_eq!(
        later.await.map_err(|error| error.to_string())??.status(),
        StatusCode::NO_CONTENT
    );

    let mut url = reqwest::Url::parse_with_params(
        &format!("{base}/statements"),
        [
            ("limit", "2"),
            ("ascending", "true"),
            ("verb", verb.as_str()),
        ],
    )
    .map_err(|e| e.to_string())?
    .to_string();
    let mut seen = Vec::new();
    loop {
        let reply = fetch(&client, reqwest::Method::GET, &url).await?;
        assert_eq!(watermark(&reply)?, t3);
        let body = json(reply).await?;
        seen.extend(body["statements"].as_array().ok_or("array")?.clone());
        let more = body["more"].as_str().ok_or("more")?;
        if more.is_empty() {
            break;
        }
        url = if more.starts_with('/') {
            format!("{base}{more}")
        } else {
            more.to_owned()
        };
    }
    assert_eq!(seen.len(), 4);
    assert_eq!(
        seen.iter().map(|v| v["id"].clone()).collect::<Vec<_>>(),
        vec![
            a["id"].clone(),
            b["id"].clone(),
            c["id"].clone(),
            d["id"].clone()
        ]
    );
    assert_eq!(instant(seen[2]["stored"].as_str().ok_or("stored")?)?, t2);
    assert_eq!(instant(seen[3]["stored"].as_str().ok_or("stored")?)?, t3);
    let since =
        format!("{base}/statements?since=2020-01-01T00:00:00.200Z&until=2020-01-01T00:00:00.300Z");
    let bounded = json(fetch(&client, reqwest::Method::GET, &since).await?).await?;
    assert_eq!(bounded["statements"].as_array().ok_or("array")?.len(), 1);
    assert_eq!(bounded["statements"][0]["id"], c["id"]);

    // Singleton and proper subsets must use their own maximum, not a global
    // repository watermark. D crosses a second; the separate last-modified
    // control also exercises distinct-second maxima and tied instants.
    for (suffix, expected, count) in [
        (
            format!("?statementId={}", a["id"].as_str().ok_or("ID")?),
            "Wed, 01 Jan 2020 00:00:00 GMT",
            None,
        ),
        (
            "?until=2020-01-01T00:00:00.200Z".to_owned(),
            "Wed, 01 Jan 2020 00:00:00 GMT",
            Some(2),
        ),
        (
            "?until=2020-01-01T00:00:00.300Z".to_owned(),
            "Wed, 01 Jan 2020 00:00:00 GMT",
            Some(3),
        ),
        (String::new(), "Wed, 01 Jan 2020 00:00:03 GMT", Some(4)),
    ] {
        assert_last_modified(
            &client,
            &format!("{base}/statements{suffix}"),
            expected,
            count,
        )
        .await?;
    }

    clock.set(t3 + Duration::hours(1));
    let settled_before = state
        .xapi
        .statements
        .settled_snapshot()
        .await
        .map_err(|e| format!("{e:?}"))?;
    let idle = fetch(&client, reqwest::Method::GET, &format!("{base}/statements")).await?;
    let idle_through = watermark(&idle)?;
    assert_eq!(idle_through, t3);
    evidence::observation(
        "consistent-through-recency-advisory",
        &json!({
            "strength":"SHOULD", "ingestion_stopped":true,"controlled_elapsed_seconds":3600,
            "advertised_last_commit":idle_through.format(&Rfc3339).map_err(|e|e.to_string())?,
            "policy":"watermark remains last committed recording instant; no numerical recency bound imposed"
        }),
    );
    let mut changed_a = a.clone();
    changed_a["stored"] = json!("2030-01-01T00:00:00Z");
    changed_a["timestamp"] = json!("2030-01-01T00:00:00Z");
    changed_a["verb"]["display"] = json!({"en": "attempted replacement"});
    let replay_attempt = put(&client, &base, &changed_a).await?;
    assert!(matches!(
        replay_attempt.status(),
        StatusCode::NO_CONTENT | StatusCode::CONFLICT
    ));
    assert_eq!(
        put(&client, &base, &a).await?.status(),
        StatusCode::NO_CONTENT
    );
    let replay = json(
        fetch(
            &client,
            reqwest::Method::GET,
            &format!(
                "{base}/statements?statementId={}",
                a["id"].as_str().ok_or("ID")?
            ),
        )
        .await?,
    )
    .await?;
    assert_eq!(replay, first);
    assert_eq!(
        state
            .xapi
            .statements
            .settled_snapshot()
            .await
            .map_err(|e| format!("{e:?}"))?,
        settled_before
    );
    // A backwards clock must not admit a new row behind an advertised bound.
    clock.set(t0);
    let e = statement(Uuid::new_v4());
    assert_eq!(
        put(&client, &base, &e).await?.status(),
        StatusCode::NO_CONTENT
    );
    let last = json(
        fetch(
            &client,
            reqwest::Method::GET,
            &format!(
                "{base}/statements?statementId={}",
                e["id"].as_str().ok_or("ID")?
            ),
        )
        .await?,
    )
    .await?;
    assert!(instant(last["stored"].as_str().ok_or("stored")?)? > t3);
    server.abort();
    Ok(())
}

#[tokio::test]
async fn memory_xapi_last_modified_empty_and_maximum() -> Result<(), String> {
    verify_last_modified(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_last_modified_empty_and_maximum() -> Result<(), String> {
    verify_last_modified(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

async fn verify_last_modified(state: &AppState) -> Result<(), String> {
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
    let verb = format!("https://example.org/last-modified/{}", Uuid::new_v4());
    let missing = format!("https://example.org/empty/{}", Uuid::new_v4());
    let times = [
        "2026-10-19T23:59:59.125Z",
        "2026-10-20T00:00:00.999Z",
        "2026-10-20T00:00:00.9999Z",
    ];
    evidence::observation(
        "last-modified-policy",
        &json!({"selected":"D-02=A","empty":"omit Last-Modified; no maximum operand","source":"Other LRSs may use a separately known representation modification time not later than origination; syntax alone is insufficient","independent_recording_times":times,"fixture":"Only this writer and fresh unmatched verb; no visibility timeout used to infer emptiness"}),
    );
    let mut ids = Vec::new();
    for time in times {
        state.xapi.statements.clock().set(instant(time)?);
        let mut value = statement(Uuid::new_v4());
        value["verb"] = json!({"id":verb});
        assert_eq!(
            put(&client, &base, &value).await?.status(),
            StatusCode::NO_CONTENT
        );
        let id = value["id"].as_str().ok_or("id")?.to_owned();
        let row = state
            .xapi
            .statements
            .get(Uuid::parse_str(&id).map_err(|e| e.to_string())?)
            .await
            .map_err(|e| format!("{e:?}"))?
            .ok_or("stored record")?;
        assert_eq!(row.stored_at, instant(time)?);
        ids.push(id);
    }
    for method in [reqwest::Method::GET, reqwest::Method::HEAD] {
        let empty = reqwest::Url::parse_with_params(
            &format!("{base}/statements"),
            &[("verb", missing.as_str())],
        )
        .map_err(|e| e.to_string())?;
        let reply = fetch(&client, method.clone(), empty.as_str()).await?;
        assert_eq!(reply.status(), StatusCode::OK);
        assert!(!reply.headers().contains_key("Last-Modified"));
        if method == reqwest::Method::HEAD {
            assert!(reply.bytes().await.map_err(|e| e.to_string())?.is_empty());
        } else {
            assert_eq!(json(reply).await?["statements"], json!([]));
        }
        let single = format!("{base}/statements?statementId={}", ids[1]);
        let reply = fetch(&client, method.clone(), &single).await?;
        assert_eq!(reply.status(), StatusCode::OK);
        assert_eq!(
            reply.headers()["last-modified"],
            "Tue, 20 Oct 2026 00:00:00 GMT"
        );
        if method == reqwest::Method::HEAD {
            assert!(reply.bytes().await.map_err(|e| e.to_string())?.is_empty());
        } else {
            assert_eq!(json(reply).await?["stored"], json!(times[1]));
        }
    }
    for ascending in ["true", "false"] {
        let url = reqwest::Url::parse_with_params(
            &format!("{base}/statements"),
            &[("verb", verb.as_str()), ("ascending", ascending)],
        )
        .map_err(|e| e.to_string())?;
        let reply = fetch(&client, reqwest::Method::GET, url.as_str()).await?;
        assert_eq!(
            reply.headers()["last-modified"],
            "Tue, 20 Oct 2026 00:00:00 GMT"
        );
        let result = json(reply).await?;
        let got = result["statements"]
            .as_array()
            .ok_or("array")?
            .iter()
            .map(|v| v["id"].as_str().ok_or("id").map(str::to_owned))
            .collect::<Result<Vec<_>, _>>()?;
        let mut expected = ids.clone();
        if ascending == "false" {
            expected.reverse();
        }
        assert_eq!(got, expected);
    }
    server.abort();
    Ok(())
}

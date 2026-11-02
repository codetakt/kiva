#![allow(clippy::wildcard_imports)]
use super::*;

#[tokio::test]
async fn memory_xapi_document_selector_and_group_partitions() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_document_selector_and_group_partitions() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

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
    Ok(reply)
}

fn location(
    base: &str,
    path: &str,
    agent: Option<&Value>,
    activity: Option<&str>,
    id: Option<&str>,
    since: Option<&str>,
) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(&format!("{base}{path}")).map_err(|e| e.to_string())?;
    {
        let mut pairs = url.query_pairs_mut();
        if let Some(agent) = agent {
            pairs.append_pair("agent", &agent.to_string());
        }
        if let Some(activity) = activity {
            pairs.append_pair("activityId", activity);
        }
        if let Some(id) = id {
            pairs.append_pair(
                if path == "/activities/state" {
                    "stateId"
                } else {
                    "profileId"
                },
                id,
            );
        }
        if let Some(since) = since {
            pairs.append_pair("since", since);
        }
        if path == "/activities/state" {
            pairs.append_pair("registration", "a02008dd-119d-45ca-9fa7-4d556f911ac7");
        }
    }
    Ok(url)
}

async fn body(client: &reqwest::Client, url: reqwest::Url) -> Result<Value, String> {
    let reply = receive(client.get(url)).await?;
    assert_eq!(reply.status(), StatusCode::OK);
    reply.json::<Value>().await.map_err(|e| e.to_string())
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
    verify_selectors(state, &client, &base).await?;
    verify_groups(&client, &base).await?;
    server.abort();
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn verify_selectors(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
) -> Result<(), String> {
    let agent = json!({"mbox":"mailto:selector@example.org"});
    let clock = state.xapi.documents.clock();
    for path in ["/activities/state", "/activities/profile"] {
        let scoped_agent = if path == "/activities/state" {
            Some(&agent)
        } else {
            None
        };
        for (activity, id, time) in [
            (
                "https://example.org/scope-a",
                "one",
                "2020-01-01T00:00:00.125Z",
            ),
            (
                "https://example.org/scope-a",
                "two",
                "2020-01-01T00:00:00.875Z",
            ),
            (
                "https://example.org/scope-b",
                "other",
                "2020-01-01T00:00:00.875Z",
            ),
        ] {
            clock.set(
                OffsetDateTime::parse(time, &time::format_description::well_known::Rfc3339)
                    .map_err(|e| e.to_string())?,
            );
            let reply = receive(
                client
                    .put(location(
                        base,
                        path,
                        scoped_agent,
                        Some(activity),
                        Some(id),
                        None,
                    )?)
                    .json(&json!({"id":id})),
            )
            .await?;
            assert_eq!(reply.status(), StatusCode::NO_CONTENT);
        }
        // D-05=A is selected before any observation. Only the neither-selector
        // State partition retains the recorded source conflict.
        evidence::observation(
            "document-selector-policy",
            &json!({"path":path,"state_policy":"D-05=A","source_no_id_no_since":"source-indeterminate for State; explicit project policy","clock_uncertainty_ns":0}),
        );
        for (id, since, status, expected) in [
            (None, None, StatusCode::OK, json!(["one", "two"])),
            (
                None,
                Some("2020-01-01T00:00:00.125Z"),
                StatusCode::OK,
                json!(["two"]),
            ),
            (Some("one"), None, StatusCode::OK, json!({"id":"one"})),
            (
                Some("one"),
                Some("2020-01-01T00:00:00.125Z"),
                StatusCode::BAD_REQUEST,
                Value::Null,
            ),
        ] {
            let url = location(
                base,
                path,
                scoped_agent,
                Some("https://example.org/scope-a"),
                id,
                since,
            )?;
            let reply = receive(client.get(url.clone())).await?;
            assert_eq!(reply.status(), status);
            if status == StatusCode::OK {
                assert_eq!(
                    reply.json::<Value>().await.map_err(|e| e.to_string())?,
                    expected
                );
            }
            let head = receive(client.head(url)).await?;
            assert_eq!(head.status(), status);
            assert!(head.bytes().await.map_err(|e| e.to_string())?.is_empty());
            for (id, expected) in [("one", json!({"id":"one"})), ("two", json!({"id":"two"}))] {
                assert_eq!(
                    body(
                        client,
                        location(
                            base,
                            path,
                            scoped_agent,
                            Some("https://example.org/scope-a"),
                            Some(id),
                            None
                        )?
                    )
                    .await?,
                    expected
                );
            }
            assert_eq!(
                body(
                    client,
                    location(
                        base,
                        path,
                        scoped_agent,
                        Some("https://example.org/scope-b"),
                        None,
                        None
                    )?
                )
                .await?,
                json!(["other"])
            );
        }
        if path == "/activities/profile" {
            for (extra_agent, activity, status) in [
                (None, Some("https://example.org/scope-a"), StatusCode::OK),
                (Some(&agent), None, StatusCode::BAD_REQUEST),
                (
                    Some(&agent),
                    Some("https://example.org/scope-a"),
                    StatusCode::BAD_REQUEST,
                ),
                (None, None, StatusCode::BAD_REQUEST),
            ] {
                let reply =
                    receive(client.get(location(base, path, extra_agent, activity, None, None)?))
                        .await?;
                assert_eq!(reply.status(), status);
                if status == StatusCode::OK {
                    assert_eq!(
                        reply.json::<Value>().await.map_err(|e| e.to_string())?,
                        json!(["one", "two"])
                    );
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn verify_groups(client: &reqwest::Client, base: &str) -> Result<(), String> {
    let member_a = json!({"mbox":"mailto:member-a@example.org"});
    let member_b = json!({"mbox":"mailto:member-b@example.org"});
    for identity in [
        json!({"mbox":"mailto:document-group@example.org"}),
        json!({"mbox_sha1sum":"0123456789abcdef0123456789abcdef01234567"}),
        json!({"openid":"https://example.org/group"}),
        json!({"account":{"homePage":"https://example.org/accounts","name":"group"}}),
    ] {
        for identified_group in [false, true] {
            let mut actor = identity.clone();
            actor["objectType"] = json!(if identified_group { "Group" } else { "Agent" });
            if identified_group {
                actor["member"] = json!([member_a]);
            }
            for path in ["/activities/state", "/agents/profile"] {
                let activity = if path == "/activities/state" {
                    Some("https://example.org/group-documents")
                } else {
                    None
                };
                for post in [false, true] {
                    let one = location(
                        base,
                        path,
                        Some(&actor),
                        activity,
                        Some("group-document"),
                        None,
                    )?;
                    let request = if post {
                        client.post(one.clone())
                    } else {
                        client.put(one.clone())
                    };
                    assert_eq!(
                        receive(request.json(&json!({"original":1})))
                            .await?
                            .status(),
                        StatusCode::NO_CONTENT
                    );
                    let mut variants = vec![actor.clone()];
                    if identified_group {
                        let mut absent = actor.clone();
                        absent.as_object_mut().ok_or("Group")?.remove("member");
                        variants.push(absent);
                        let mut changed = actor.clone();
                        changed["member"] = json!([member_b]);
                        variants.push(changed);
                    }
                    for variant in &variants {
                        assert_eq!(
                            body(
                                client,
                                location(
                                    base,
                                    path,
                                    Some(variant),
                                    activity,
                                    Some("group-document"),
                                    None
                                )?
                            )
                            .await?,
                            json!({"original":1})
                        );
                        assert_eq!(
                            body(
                                client,
                                location(base, path, Some(variant), activity, None, None)?
                            )
                            .await?,
                            json!(["group-document"])
                        );
                    }
                    let invalids = [
                        json!({"objectType":"Group","member":[member_a]}),
                        json!({"objectType":"Group","mbox":"mailto:one@example.org","openid":"https://example.org/two"}),
                        json!({"objectType":"Group","mbox":42}),
                        json!({"objectType":"Group","mbox":"mailto:one@example.org","member":null}),
                    ];
                    for invalid in &invalids {
                        for method in [
                            reqwest::Method::PUT,
                            reqwest::Method::POST,
                            reqwest::Method::GET,
                            reqwest::Method::DELETE,
                        ] {
                            let reply = receive(
                                client
                                    .request(
                                        method,
                                        location(
                                            base,
                                            path,
                                            Some(invalid),
                                            activity,
                                            Some("group-document"),
                                            None,
                                        )?,
                                    )
                                    .json(&json!({"changed":2})),
                            )
                            .await?;
                            assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
                            assert_eq!(body(client, one.clone()).await?, json!({"original":1}));
                        }
                        let list = location(base, path, Some(invalid), activity, None, None)?;
                        assert_eq!(
                            receive(client.get(list.clone())).await?.status(),
                            StatusCode::BAD_REQUEST
                        );
                        if path == "/activities/state" {
                            assert_eq!(
                                receive(client.delete(list)).await?.status(),
                                StatusCode::BAD_REQUEST
                            );
                        }
                    }
                    // A changed/omitted member collection still addresses the
                    // same IFI for both a merge and deletion.
                    let last = variants.last().ok_or("actor variant")?;
                    let same = location(
                        base,
                        path,
                        Some(last),
                        activity,
                        Some("group-document"),
                        None,
                    )?;
                    assert_eq!(
                        receive(client.post(same.clone()).json(&json!({"merged":2})))
                            .await?
                            .status(),
                        StatusCode::NO_CONTENT
                    );
                    assert_eq!(
                        body(client, one.clone()).await?,
                        json!({"original":1,"merged":2})
                    );
                    assert_eq!(
                        receive(client.delete(same)).await?.status(),
                        StatusCode::NO_CONTENT
                    );
                    assert_eq!(
                        receive(client.get(one)).await?.status(),
                        StatusCode::NOT_FOUND
                    );
                    assert_eq!(
                        body(
                            client,
                            location(base, path, Some(&actor), activity, None, None)?
                        )
                        .await?,
                        json!([])
                    );
                }
            }
        }
    }
    Ok(())
}

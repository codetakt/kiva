//! Independent maintained-map fixtures for exact/ids/canonical HTTP output.
#![allow(clippy::wildcard_imports)]
use super::*;
use std::sync::Arc;

#[tokio::test]
async fn memory_xapi_metadata_inventory_and_language_projection() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_metadata_inventory_and_language_projection() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
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
    if response.url().path().starts_with("/statements") {
        OffsetDateTime::parse(
            response.headers()["x-experience-api-consistent-through"]
                .to_str()
                .map_err(|e| e.to_string())?,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(response)
}

async fn read(
    client: &reqwest::Client,
    url: reqwest::Url,
    language: Option<&str>,
) -> Result<Value, String> {
    let request = client.get(url);
    let request = if let Some(language) = language {
        request.header("Accept-Language", language)
    } else {
        request
    };
    let response = receive(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.map_err(|e| e.to_string())
}

fn fixture(id: Uuid, component: &str, nested: bool) -> (Value, Vec<String>, Vec<String>) {
    let mut value = statement(id);
    let group = json!({"objectType":"Group","name":"Group name","member":[{"mbox":"mailto:a@example.org","name":"A"},{"mbox":"mailto:b@example.org","name":"B"}]});
    value["actor"] = group.clone();
    value["verb"]["display"] = json!({"ja":"verb-ja","en":"verb-en"});
    value["context"] = json!({"instructor":{"mbox":"mailto:instructor@example.org","name":"Instructor"},"team":group,"contextAgents":[{"objectType":"contextAgent","agent":{"mbox":"mailto:context@example.org","name":"Context Agent"},"relevantTypes":["https://example.org/relevant"]}],"contextGroups":[{"objectType":"contextGroup","group":{"objectType":"Group","mbox":"mailto:identified@example.org","name":"Identified","member":[{"mbox":"mailto:member@example.org","name":"Member"}]},"relevantTypes":["https://example.org/relevant"]}],"extensions":{"https://example.org/opaque":{"en":"opaque-en","ja":"opaque-ja","name":{"en":"owner-en","ja":"owner-ja"}}}});
    let definition = json!({"name":{"ja-JP":"name-ja","en":"name-en"},"description":{"en":"description-en"},"type":"https://example.org/type","moreInfo":"https://unresolvable.invalid/activity-info","interactionType":"other","extensions":{"https://example.org/opaque":{"name":{"en":"owner-en","ja":"owner-ja"}}},component:[{"id":"first","description":{"ja-JP":"first-ja","en":"first-en"}},{"id":"second","description":{"en":"second-en"}}]});
    let mut locations = vec!["/object".to_owned()];
    for role in ["parent", "grouping", "category", "other"] {
        locations.push(format!("/context/contextActivities/{role}/0"));
    }
    value["object"] =
        json!({"id":format!("https://example.org/activity/{id}/object"),"definition":definition});
    for role in ["parent", "grouping", "category", "other"] {
        value["context"]["contextActivities"][role] = json!([{"id":format!("https://example.org/activity/{id}/{role}"),"definition":definition}]);
    }
    let actors = vec![
        "/actor".to_owned(),
        "/context/instructor".to_owned(),
        "/context/team".to_owned(),
        "/context/contextAgents/0/agent".to_owned(),
        "/context/contextGroups/0/group".to_owned(),
    ];
    if nested {
        value.as_object_mut().expect("Statement").remove("id");
        value["objectType"] = json!("SubStatement");
        let mut root = statement(id);
        root["object"] = value;
        value = root;
        (
            value,
            locations
                .into_iter()
                .map(|p| format!("/object{p}"))
                .collect(),
            actors.into_iter().map(|p| format!("/object{p}")).collect(),
        )
    } else {
        (value, locations, actors)
    }
}

async fn verify(state: &AppState) -> Result<(), String> {
    let mut configured = state.clone();
    configured
        .auth
        .register_bearer_subject("metadata-writer-b", "metadata-writer-b");
    configured.auth.register_bearer_scopes(
        "metadata-writer-b",
        [
            kiva_identity::OAuthScope::XapiRead,
            kiva_identity::OAuthScope::XapiWrite,
        ],
    );
    let state = &configured;
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
        "canonical-inventory-policy",
        &json!({"statement_maps":"retained per-Statement definitions; no metadata fetch/correction","activity_resource":"first-known properties; merge missing object fields and language entries; extension-owner values atomic","other_canonical_maps":["Verb display","attachment display/description"],"language_fallback":"first lexical registered key","hosted_metadata":false,"source":"bound projection/language/wire implementation; fixture declared before reads"}),
    );
    for nested in [false, true] {
        for component in ["choices", "scale", "source", "target", "steps"] {
            let id = Uuid::new_v4();
            let (value, locations, actors) = fixture(id, component, nested);
            let response = receive(
                client
                    .put(format!("{base}/statements?statementId={id}"))
                    .json(&value),
            )
            .await?;
            assert_eq!(
                response.status(),
                StatusCode::NO_CONTENT,
                "{component}/{nested}: {}",
                response.text().await.map_err(|e| e.to_string())?
            );
            for format in ["exact", "ids", "canonical"] {
                for (language, japanese) in [
                    (Some("ja, en;q=0.5"), true),
                    (Some("en;q=0.9, ja;q=0.1"), false),
                    (Some("*"), false),
                    (None, false),
                    (Some("JA-jp"), true),
                    (Some("fr"), false),
                ] {
                    let url = reqwest::Url::parse_with_params(
                        &format!("{base}/statements"),
                        [
                            ("statementId", id.to_string()),
                            ("format", format.to_owned()),
                        ],
                    )
                    .map_err(|e| e.to_string())?;
                    let actual = read(&client, url, language).await?;
                    for actor in &actors {
                        let expected = value.pointer(actor).ok_or("actor fixture")?;
                        if format == "ids" {
                            let projected = actual.pointer(actor).ok_or("ids Actor")?;
                            assert!(projected.get("name").is_none());
                            if expected.get("mbox").is_some() {
                                assert_eq!(projected["mbox"], expected["mbox"]);
                                assert!(projected.get("member").is_none());
                            }
                            if expected.get("objectType").and_then(Value::as_str) == Some("Group")
                                && expected.get("mbox").is_none()
                            {
                                let members = projected["member"]
                                    .as_array()
                                    .ok_or("anonymous Group members")?;
                                assert_eq!(members.len(), 2);
                                let returned = members
                                    .iter()
                                    .map(|member| member["mbox"].as_str().expect("member IFI"))
                                    .collect::<std::collections::BTreeSet<_>>();
                                assert_eq!(
                                    returned,
                                    std::collections::BTreeSet::from([
                                        "mailto:a@example.org",
                                        "mailto:b@example.org"
                                    ])
                                );
                                for member in members {
                                    assert!(member.get("name").is_none());
                                }
                            }
                        } else {
                            assert_eq!(actual.pointer(actor), Some(expected));
                        }
                    }
                    for location in &locations {
                        let expected =
                            &value.pointer(location).ok_or("Activity fixture")?["definition"];
                        let definition =
                            &actual.pointer(location).ok_or("Activity result")?["definition"];
                        if format == "ids" {
                            assert!(
                                actual
                                    .pointer(location)
                                    .ok_or("ids Activity")?
                                    .get("definition")
                                    .is_none()
                            );
                            continue;
                        }
                        if format == "exact" {
                            assert_eq!(definition, expected);
                            continue;
                        }
                        assert_eq!(
                            definition["name"],
                            if japanese {
                                json!({"ja-JP":"name-ja"})
                            } else {
                                json!({"en":"name-en"})
                            }
                        );
                        assert_eq!(definition["description"], json!({"en":"description-en"}));
                        assert_eq!(
                            definition[component][0]["description"],
                            if japanese {
                                json!({"ja-JP":"first-ja"})
                            } else {
                                json!({"en":"first-en"})
                            }
                        );
                        assert_eq!(
                            definition[component][1]["description"],
                            json!({"en":"second-en"})
                        );
                        assert_eq!(definition["extensions"], expected["extensions"]);
                    }
                    let prefix = if nested { "/object" } else { "" };
                    if format == "ids" {
                        assert!(actual.pointer(&format!("{prefix}/verb/display")).is_none());
                    } else if format == "canonical" {
                        assert_eq!(
                            actual.pointer(&format!("{prefix}/verb/display")),
                            // This inventory has ja, not ja-JP. Under the
                            // declared basic filtering a ja-JP range cannot
                            // match its parent ja; choose this map's fallback.
                            Some(&if japanese && language != Some("JA-jp") {
                                json!({"ja":"verb-ja"})
                            } else {
                                json!({"en":"verb-en"})
                            })
                        );
                    }
                    assert_eq!(
                        actual.pointer(&format!("{prefix}/context/extensions")),
                        value.pointer(&format!("{prefix}/context/extensions"))
                    );
                }
            }
            for location in &locations {
                let activity = value.pointer(location).ok_or("Activity")?;
                let url = reqwest::Url::parse_with_params(
                    &format!("{base}/activities"),
                    [("activityId", activity["id"].as_str().ok_or("Activity id")?)],
                )
                .map_err(|e| e.to_string())?;
                let actual = read(&client, url, None).await?;
                assert_eq!(actual["id"], activity["id"]);
                assert_eq!(actual["definition"], activity["definition"]);
            }
        }
    }
    let unknown = format!("https://example.org/unknown/{}", Uuid::new_v4());
    let url =
        reqwest::Url::parse_with_params(&format!("{base}/activities"), [("activityId", &unknown)])
            .map_err(|e| e.to_string())?;
    let fallback = read(&client, url, None).await?;
    assert_eq!(fallback["id"], json!(unknown));
    assert_eq!(fallback["objectType"], json!("Activity"));
    assert!(fallback.get("definition").is_none());
    available_information(state, &client, &base).await?;
    hosted_metadata_applicability(&client, &base).await?;
    server.abort();
    Ok(())
}

/// Kiva has no metadata fetcher, metadata hosting route, or asynchronous
/// metadata worker. The source-bound fixture declares that implementation
/// choice before querying; absence of traffic alone is not its proof.
async fn hosted_metadata_applicability(client: &reqwest::Client, base: &str) -> Result<(), String> {
    let calls = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let observed = Arc::clone(&calls);
    let definition = json!({"description":{"en":"hosted description without name"},"type":"https://example.org/hosted-type"});
    let hosted = definition.clone();
    let router = axum::Router::new().fallback(axum::routing::get(
        move |uri: axum::http::Uri, headers: HeaderMap| {
            let observed = Arc::clone(&observed);
            let hosted = hosted.clone();
            async move {
                if headers.get("x-fixture-host-probe").is_none() {
                    observed.lock().expect("host observations").push(uri.to_string());
                }
                if uri.path() == "/html" {
                    ([("content-type", "text/html")], format!("<!doctype html><script type=\"application/json\" id=\"activity-definition\">{hosted}</script>"))
                } else {
                    ([("content-type", "application/json")], hosted.to_string())
                }
            }
        },
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let host = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
    });
    // Independent fixture health controls make an unreachable host distinguishable
    // from an implementation which does not consume hosted metadata.
    for format in ["json", "html"] {
        let activity_id = format!("{host}/{format}");
        let probe = client
            .get(&activity_id)
            .header("x-fixture-host-probe", "1")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        assert_eq!(probe.status(), StatusCode::OK);
        let content = probe.text().await.map_err(|e| e.to_string())?;
        let embedded = if format == "html" {
            content
                .split_once("id=\"activity-definition\">")
                .ok_or("hosted script")?
                .1
                .strip_suffix("</script>")
                .ok_or("hosted script end")?
        } else {
            &content
        };
        assert_eq!(
            serde_json::from_str::<Value>(embedded).map_err(|e| e.to_string())?,
            definition
        );
        let inventory =
            json!({"name":{"en":"known name"},"description":{"en":"submitted description"}});
        evidence::observation(
            "hosted-metadata-applicability",
            &json!({
                "activity_id":activity_id,"host_role":"controlled external fixture, not Kiva",
                "host_representation":format,"independent_definition":definition,
                "implementation":{"consumer":false,"host":false,"deferred_workers":false},
            "source_anchors":["handlers/agents_activities.rs::get_activity","wire.rs::merged_activity_definition","request/projection.rs::formatted_statement"],
                "applicability":{"fetch_accept_and_first_fetch":"not-applicable: no consumer","incorporation_and_missing_field_retention":"not-applicable: no consumer","alternate_document_consideration":"not-applicable: no consumer; MAY, not mandated parsing","hosting_format":"not-applicable: Kiva is not metadata host"},
                "mandatory_inventory":inventory,"qualification":"This source-bound synchronous implementation is tested; a quiet interval alone cannot establish permanent absence of network work."
            }),
        );
        let url = reqwest::Url::parse_with_params(
            &format!("{base}/activities"),
            [("activityId", &activity_id)],
        )
        .map_err(|e| e.to_string())?;
        assert_eq!(
            read(client, url.clone(), None).await?,
            json!({"objectType":"Activity","id":activity_id})
        );
        let id = Uuid::new_v4();
        let mut value = statement(id);
        value["object"] = json!({"id":activity_id,"definition":inventory});
        assert_eq!(
            receive(client.post(format!("{base}/statements")).json(&value))
                .await?
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            read(client, url, None).await?,
            json!({"objectType":"Activity","id":activity_id,"definition":inventory})
        );
        for format in ["exact", "canonical"] {
            let url = reqwest::Url::parse_with_params(
                &format!("{base}/statements"),
                [
                    ("statementId", id.to_string()),
                    ("format", format.to_owned()),
                ],
            )
            .map_err(|e| e.to_string())?;
            assert_eq!(
                read(client, url, Some("en")).await?["object"]["definition"],
                inventory
            );
        }
    }
    let _ = stop.send(());
    server
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    assert!(
        calls.lock().expect("host observations").is_empty(),
        "declared no-fetch policy changed"
    );
    Ok(())
}

async fn available_information(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
) -> Result<(), String> {
    let activity_id = format!("https://example.org/available/{}", Uuid::new_v4());
    // Reverse UUID order deterministically. A UUID-sorted repository used to
    // pick the later English value and owner extension about half the time.
    let mut first = statement(
        Uuid::parse_str("ffffffff-ffff-4fff-bfff-fffffffffff0").map_err(|e| e.to_string())?,
    );
    first["object"] = json!({"id":activity_id,"definition":{"name":{"en":"first-en"},"type":"https://example.org/type","interactionType":"choice","choices":[{"id":"answer-a","description":{"en":"First answer"}},{"id":"answer-b","description":{"en":"Second answer"}}],"correctResponsesPattern":["answer-a"],"extensions":{"https://example.org/owner":{"first":1}}}});
    let mut second = statement(
        Uuid::parse_str("00000000-0000-4000-8000-000000000010").map_err(|e| e.to_string())?,
    );
    second["object"] = json!({"id":activity_id,"definition":{"name":{"en":"first-en-corrected","ja":"added-ja"},"description":{"en":"added-description"},"interactionType":"choice","choices":first["object"]["definition"]["choices"],"correctResponsesPattern":["answer-b"],"moreInfo":"https://unresolvable.invalid/info","extensions":{"https://example.org/owner":{"second":2},"https://example.org/new":3}}});
    for (submitted, token) in [
        (&first, "statement-writer-a"),
        (&second, "metadata-writer-b"),
    ] {
        let reply = receive(
            client
                .post(format!("{base}/statements"))
                .bearer_auth(token)
                .json(submitted),
        )
        .await?;
        assert_eq!(reply.status(), StatusCode::OK);
        for format in [None, Some("exact")] {
            let mut url = reqwest::Url::parse_with_params(
                &format!("{base}/statements"),
                [("statementId", submitted["id"].as_str().ok_or("id")?)],
            )
            .map_err(|e| e.to_string())?;
            if let Some(format) = format {
                url.query_pairs_mut().append_pair("format", format);
            }
            let returned = read(client, url, None).await?;
            assert_eq!(returned["object"], submitted["object"]);
            assert!(returned["verb"].get("display").is_none());
        }
    }
    let expected = json!({"objectType":"Activity","id":activity_id,"definition":{"name":{"en":"first-en","ja":"added-ja"},"description":{"en":"added-description"},"type":"https://example.org/type","interactionType":"choice","choices":first["object"]["definition"]["choices"],"correctResponsesPattern":["answer-a"],"moreInfo":"https://unresolvable.invalid/info","extensions":{"https://example.org/owner":{"first":1},"https://example.org/new":3}}});
    let url = reqwest::Url::parse_with_params(
        &format!("{base}/activities"),
        [("activityId", &activity_id)],
    )
    .map_err(|e| e.to_string())?;
    assert_eq!(read(client, url.clone(), None).await?, expected);
    for (token, status) in [
        ("statement-reader", StatusCode::FORBIDDEN),
        ("refused-credential", StatusCode::UNAUTHORIZED),
    ] {
        let mut denied = second.clone();
        let id = Uuid::new_v4();
        denied["id"] = json!(id);
        denied["object"]["definition"] =
            json!({"name":{"fr":"forbidden"},"extensions":{"https://example.org/forbidden":true}});
        let reply = receive(
            client
                .post(format!("{base}/statements"))
                .bearer_auth(token)
                .json(&denied),
        )
        .await?;
        assert_eq!(reply.status(), status);
        assert!(
            state
                .xapi
                .statements
                .get(id)
                .await
                .map_err(|e| format!("{e:?}"))?
                .is_none()
        );
        assert_eq!(read(client, url.clone(), None).await?, expected);
    }
    Ok(())
}

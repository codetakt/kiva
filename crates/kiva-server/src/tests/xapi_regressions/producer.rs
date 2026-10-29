//! Controlled producer facts and actual LRS preservation, kept as separate roles.
#![allow(clippy::wildcard_imports)]
use super::recording::{instant, json, watermark};
use super::*;

#[tokio::test]
async fn memory_xapi_producer_meaning_and_receiver_boundaries() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_producer_meaning_and_receiver_boundaries() -> Result<(), String> {
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
    watermark(&reply)?;
    Ok(reply)
}

async fn write(
    client: &reqwest::Client,
    base: &str,
    id: Uuid,
    value: &Value,
    post: bool,
) -> Result<reqwest::Response, String> {
    let request = if post {
        client
            .post(format!("{base}/statements"))
            .json(&json!([value]))
    } else {
        client
            .put(format!("{base}/statements?statementId={id}"))
            .json(value)
    };
    receive(request).await
}

async fn retained(client: &reqwest::Client, base: &str, id: Uuid) -> Result<Value, String> {
    let reply =
        receive(client.get(format!("{base}/statements?statementId={id}&format=exact"))).await?;
    assert_eq!(reply.status(), StatusCode::OK);
    json(reply).await
}

fn nest(mut value: Value, id: Uuid, nested: bool) -> Result<Value, String> {
    if nested {
        value.as_object_mut().ok_or("Statement")?.remove("id");
        value["objectType"] = json!("SubStatement");
        let mut root = statement(id);
        root["timestamp"] = json!("2010-01-01T00:00:00Z");
        root["object"] = value;
        Ok(root)
    } else {
        Ok(value)
    }
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../tests/conformance/fixtures/producer-meaning.json"
    ))
    .map_err(|e| e.to_string())?;
    let page = fixture["more_info_html"]
        .as_str()
        .ok_or("fixture HTML")?
        .to_owned();
    let host = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let more_info = format!(
        "http://{}/lesson-17",
        host.local_addr().map_err(|e| e.to_string())?
    );
    let host_router = axum::Router::new().route(
        "/lesson-17",
        axum::routing::get(move || {
            let page = page.clone();
            async move { axum::response::Html(page) }
        }),
    );
    let human_host = tokio::spawn(async move { axum::serve(host, host_router).await });
    let client = reqwest::Client::new();
    let human = client
        .get(&more_info)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(human.status(), StatusCode::OK);
    assert!(
        human.headers()["content-type"]
            .to_str()
            .map_err(|e| e.to_string())?
            .starts_with("text/html")
    );
    let html = human.text().await.map_err(|e| e.to_string())?;
    assert_eq!(html, fixture["more_info_html"]);
    assert!(
        html.contains(
            fixture["human_description"]
                .as_str()
                .ok_or("human description")?
        )
    );
    evidence::observation(
        "independent-producer-and-human-resource",
        &json!({"fixture":fixture,"resolved_more_info":more_info,"actual_html":html,
        "role":"controlled producer and human renderer are test fixtures, not Kiva product roles",
        "normalization":"2 * (17 - 0) / (20 - 0) - 1 = 0.7; not a required universal scoring formula",
        "duration":"independent experience start/end90.25s; event30s is inside; not derived from LRS stored timestamps"}),
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
    state
        .xapi
        .statements
        .clock()
        .set(instant("2026-10-20T00:00:00Z")?);
    // A controlled resolvable identifier and an independently assigned pure
    // name are both legal. Resolution is a producer fixture fact, not inferred
    // from HTTP scheme spelling or requested as a receiver responsibility.
    evidence::observation(
        "identifier-resolution-assignment",
        &json!({
            "resolvable_activity":more_info,"independently_fetched_body":html,
            "name_only_activity":fixture["assignments"]["identifier_log"][2]["id"],
            "ownership":"the local HTTP listener and synthetic assignment log are controlled fixture namespaces"
        }),
    );
    for activity in [more_info.as_str(), "urn:example:lesson:17"] {
        let id = Uuid::new_v4();
        let mut value = statement(id);
        value["object"]["id"] = json!(activity);
        assert_eq!(
            write(&client, &base, id, &value, false).await?.status(),
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            retained(&client, &base, id).await?["object"],
            value["object"]
        );
    }
    for nested in [false, true] {
        for post in [false, true] {
            for known_language in [false, true] {
                for account in [false, true] {
                    for locator in [
                        more_info.as_str(),
                        "https://example.invalid/meaning-unreachable",
                    ] {
                        let id = Uuid::new_v4();
                        let mut value = statement(id);
                        value["actor"] = if account {
                            json!({"account":fixture["assignments"]["account"]})
                        } else {
                            json!({"mbox":fixture["assignments"]["mailbox"]})
                        };
                        value["verb"] = json!({"id":fixture["assignments"]["identifier_log"][0]["id"],"display":{"en":fixture["literal_language_value"]}});
                        value["object"] = json!({"id":fixture["assignments"]["identifier_log"][2]["id"],"definition":{"name":{"ja":"学習17"},"moreInfo":locator}});
                        value["timestamp"] = fixture["experience"]["event"].clone();
                        value["result"] = json!({"score":fixture["experience"]["score"],"duration":fixture["experience"]["duration"]});
                        let group = json!({"objectType":"Group","account":{"homePage":"https://accounts.example.org/","name":"study-team"}});
                        value["context"] = json!({"contextActivities":{"parent":[{"id":"urn:example:course:17"}],"other":[{"id":"urn:example:related:17"}]},
                            "contextAgents":[{"objectType":"contextAgent","agent":value["actor"],"relevantTypes":["urn:example:mentor"]}],
                            "contextGroups":[{"objectType":"contextGroup","group":group,"relevantTypes":["urn:example:cohort"]}],
                            "extensions":{"https://example.org/owner-rendering":fixture["extension_rendering"]}});
                        if known_language {
                            value["context"]["language"] =
                                fixture["experience"]["known_language"].clone();
                        }
                        let value = nest(value, id, nested)?;
                        let reply = write(&client, &base, id, &value, post).await?;
                        assert_eq!(
                            reply.status(),
                            if post {
                                StatusCode::OK
                            } else {
                                StatusCode::NO_CONTENT
                            }
                        );
                        if post {
                            assert_eq!(json(reply).await?, json!([id]));
                        }
                        let stored = retained(&client, &base, id).await?;
                        let expected = if nested { &value["object"] } else { &value };
                        let returned = if nested { &stored["object"] } else { &stored };
                        for key in ["actor", "verb", "object", "result", "context", "timestamp"] {
                            assert_eq!(returned[key], expected[key], "{key}/nested={nested}");
                        }
                        assert!(
                            instant(stored["stored"].as_str().ok_or("stored")?)?
                                > instant(fixture["experience"]["end"].as_str().ok_or("end")?)?
                        );
                    }
                }
            }
        }
    }
    // The assignment log deliberately separates reuse/labels/ownership. These
    // facts never become a receiver-side network truth or vocabulary check.
    for (index, row) in fixture["assignments"]["identifier_log"]
        .as_array()
        .ok_or("assignment log")?
        .iter()
        .enumerate()
    {
        let id = Uuid::new_v4();
        let mut value = statement(id);
        value[if row["entity"] == "lesson-17" {
            "object"
        } else {
            "verb"
        }]["id"] = row["id"].clone();
        value["verb"]["display"] = json!({"en":format!("label independently varied {index}")});
        assert_eq!(
            write(&client, &base, id, &value, false).await?.status(),
            StatusCode::NO_CONTENT
        );
        let stored = retained(&client, &base, id).await?;
        assert_eq!(stored["verb"], value["verb"]);
        assert_eq!(stored["object"], value["object"]);
    }
    receiver_boundaries(state, &client, &base).await?;
    human_host.abort();
    server.abort();
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn receiver_boundaries(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
) -> Result<(), String> {
    evidence::observation(
        "receiver-clock-and-retention-policy",
        &json!({
        "future_clock_margin":"no age/future cut-off within supported timestamp grammar; structural/type/format validation remains",
        "source":"kiva-xapi object_validation timestamp parsing; no comparison to recording clock",
        "retention":"no automatic deletion or retention worker in this fixture; live Statement repository persists until explicit administrative storage action",
        "limits":"default fixture has no per-field limit; HTTP transport still has2MiB body ceiling",
        "role":"future/empty-object/large-field producer recommendations do not become mandatory rejection oracles"}),
    );
    for supplied in [false, true] {
        let id = Uuid::new_v4();
        let mut value = statement(id);
        if !supplied {
            value.as_object_mut().ok_or("Statement")?.remove("id");
        }
        assert_eq!(
            write(client, base, id, &value, false).await?.status(),
            StatusCode::NO_CONTENT
        );
        assert_eq!(retained(client, base, id).await?["id"], json!(id));
    }
    let id = Uuid::new_v4();
    let wrong = statement(Uuid::new_v4());
    let before = state
        .xapi
        .statements
        .settled_snapshot()
        .await
        .map_err(|e| format!("{e:?}"))?;
    let reply = write(client, base, id, &wrong, false).await?;
    evidence::observation(
        "producer-query-body-id-mismatch",
        &json!({"query_id":id,"body_id":wrong["id"],"producer_contract":false,"observed_status":reply.status().as_u16(),"receiver_policy":"Kiva rejects mismatched ids; no new IEEE status inferred from producer-only duty"}),
    );
    assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        state
            .xapi
            .statements
            .settled_snapshot()
            .await
            .map_err(|e| format!("{e:?}"))?,
        before
    );
    for nested in [false, true] {
        for post in [false, true] {
            for partition in [
                "minimal",
                "empty",
                "AgentObject",
                "AgentObjectNoType",
                "missing-actor",
                "missing-verb",
                "missing-object",
                "future",
                "large-name",
                "large-description",
                "large-extension",
            ] {
                let id = Uuid::new_v4();
                let mut value = statement(id);
                match partition {
                    "empty" => {
                        value["result"] = json!({});
                        value["context"] = json!({});
                    }
                    "AgentObject" => {
                        value["object"] =
                            json!({"objectType":"Agent","mbox":"mailto:object@example.org"});
                    }
                    "AgentObjectNoType" => {
                        value["object"] = json!({"mbox":"mailto:object@example.org"});
                    }
                    "future" => {
                        value["timestamp"] = json!("2100-01-01T00:00:00Z");
                    }
                    "large-name" => {
                        value["actor"]["name"] = json!("学".repeat(10000));
                    }
                    "large-description" => {
                        value["object"]["definition"] =
                            json!({"description":{"ja":"学".repeat(10000)}});
                    }
                    "large-extension" => {
                        value["result"] =
                            json!({"extensions":{"https://example.org/large":"学".repeat(10000)}});
                    }
                    _ => {}
                }
                if let Some(key) = partition.strip_prefix("missing-") {
                    value.as_object_mut().ok_or("Statement")?.remove(key);
                }
                let value = nest(value, id, nested)?;
                let rejected =
                    partition.starts_with("missing-") || partition == "AgentObjectNoType";
                let reply = write(client, base, id, &value, post).await?;
                assert_eq!(
                    reply.status(),
                    if rejected {
                        StatusCode::BAD_REQUEST
                    } else if post {
                        StatusCode::OK
                    } else {
                        StatusCode::NO_CONTENT
                    },
                    "{partition}/{nested}/{post}"
                );
                if rejected {
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
                let returned = retained(client, base, id).await?;
                let expected = if nested { &value["object"] } else { &value };
                let stored = if nested {
                    &returned["object"]
                } else {
                    &returned
                };
                for key in ["actor", "verb", "object", "result", "context"] {
                    if let Some(supplied) = expected.get(key) {
                        assert_eq!(stored.get(key), Some(supplied), "{partition}/{key}");
                    }
                }
                if partition == "future" {
                    assert_eq!(stored["timestamp"], expected["timestamp"]);
                }
            }
        }
    }
    let absent = Uuid::new_v4();
    assert!(
        state
            .xapi
            .statements
            .get(absent)
            .await
            .map_err(|e| format!("{e:?}"))?
            .is_none()
    );
    let id = Uuid::new_v4();
    let mut voider = statement(id);
    voider["verb"]["id"] = json!("http://adlnet.gov/expapi/verbs/voided");
    voider["object"] = json!({"objectType":"StatementRef","id":absent});
    assert_eq!(
        write(client, base, id, &voider, false).await?.status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        retained(client, base, id).await?["object"],
        voider["object"]
    );
    Ok(())
}

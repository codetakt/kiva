//! Field identity and JSON object ordering, using actual HTTP and fixed pairs.
#![allow(clippy::wildcard_imports)]
use super::*;
use std::hash::{Hash, Hasher};

#[tokio::test]
async fn memory_xapi_simple_iri_identity() -> Result<(), String> {
    iri_identity(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_simple_iri_identity() -> Result<(), String> {
    iri_identity(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}
#[tokio::test]
async fn memory_xapi_json_member_order_and_array_boundary() -> Result<(), String> {
    member_order(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_json_member_order_and_array_boundary() -> Result<(), String> {
    member_order(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
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

async fn iri_identity(state: &AppState) -> Result<(), String> {
    let (base, server) = serve(state).await?;
    let client = reqwest::Client::new();
    let pairs = [
        ("HTTPS://example.org/path", "https://example.org/path"),
        ("https://EXAMPLE.org/path", "https://example.org/path"),
        ("https://example.org/Path", "https://example.org/path"),
        ("https://example.org/%70ath", "https://example.org/path"),
        ("https://example.org/a/../path", "https://example.org/path"),
        ("https://example.org:443/path", "https://example.org/path"),
        (
            "https://example.org/学習",
            "https://example.org/%E5%AD%A6%E7%BF%92",
        ),
        ("https://example.org/é", "https://example.org/e\u{0301}"),
    ];
    evidence::observation(
        "iri-identity-policy",
        &json!({"method":"RFC3987 5.3.1 simple string comparison","pairs":pairs,
        "roles":["Agent Account homePage with complete name","Verb id","Activity id"],
        "qualification":"URI request transport encoding does not normalize the identity stored in an xAPI field; no reachability/ownership lookup is used"}),
    );
    for (left, right) in pairs {
        for field in ["actor", "verb", "activity"] {
            let scope = format!("https://example.org/identity/{}", Uuid::new_v4());
            let mut ids = Vec::new();
            for (index, iri) in [left, right].into_iter().enumerate() {
                let id = Uuid::new_v4();
                ids.push(id.to_string());
                let mut value = statement(id);
                value["verb"]["id"] = json!(scope);
                value["object"]["id"] = json!(scope);
                match field {
                    "actor" => value["actor"] = json!({"account":{"homePage":iri,"name":scope}}),
                    "verb" => value["verb"]["id"] = json!(iri),
                    _ => value["object"]["id"] = json!(iri),
                }
                let reply = receive(if index == 0 {
                    client
                        .post(format!("{base}/statements"))
                        .json(&json!([value]))
                } else {
                    client
                        .put(format!("{base}/statements?statementId={id}"))
                        .json(&value)
                })
                .await?;
                assert_eq!(
                    reply.status(),
                    if index == 0 {
                        StatusCode::OK
                    } else {
                        StatusCode::NO_CONTENT
                    }
                );
                let reply =
                    receive(client.get(format!("{base}/statements?statementId={id}&format=exact")))
                        .await?;
                assert_eq!(reply.status(), StatusCode::OK);
                let stored = reply.json::<Value>().await.map_err(|e| e.to_string())?;
                for key in ["actor", "verb", "object"] {
                    assert_eq!(stored[key], value[key]);
                }
            }
            for (iri, expected) in [left, right].into_iter().zip(&ids) {
                let agent = json!({"account":{"homePage":iri,"name":scope}}).to_string();
                let query = match field {
                    "actor" => vec![("agent", agent), ("verb", scope.clone())],
                    "verb" => vec![("verb", iri.into()), ("activity", scope.clone())],
                    _ => vec![("activity", iri.into()), ("verb", scope.clone())],
                };
                let url = reqwest::Url::parse_with_params(&format!("{base}/statements"), query)
                    .map_err(|e| e.to_string())?;
                let reply = receive(client.get(url)).await?;
                assert_eq!(reply.status(), StatusCode::OK);
                let result = reply.json::<Value>().await.map_err(|e| e.to_string())?;
                let actual = result["statements"]
                    .as_array()
                    .ok_or("StatementResult")?
                    .iter()
                    .map(|v| v["id"].as_str().expect("id"))
                    .collect::<Vec<_>>();
                assert_eq!(
                    actual,
                    vec![expected.as_str()],
                    "{field}: {left} versus {right}"
                );
                assert_eq!(result["more"], "");
            }
        }
    }
    server.abort();
    Ok(())
}

fn ordered_json(value: &Value, order: usize) -> String {
    match value {
        Value::Object(object) => {
            let mut entries = object.iter().collect::<Vec<_>>();
            entries.sort_by_key(|(key, _)| *key);
            if order == 1 {
                entries.reverse();
            }
            if order == 2 {
                entries.sort_by_cached_key(|(key, _)| {
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    0x9274_2023_u64.hash(&mut hasher);
                    key.hash(&mut hasher);
                    hasher.finish()
                });
            }
            format!(
                "{{{}}}",
                entries
                    .into_iter()
                    .map(|(key, value)| format!(
                        "{}:{}",
                        serde_json::to_string(key).expect("JSON key"),
                        ordered_json(value, order)
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(|v| ordered_json(v, order))
                .collect::<Vec<_>>()
                .join(",")
        ),
        _ => value.to_string(),
    }
}

async fn member_order(state: &AppState) -> Result<(), String> {
    let (base, server) = serve(state).await?;
    let client = reqwest::Client::new();
    let mut original = statement(Uuid::new_v4());
    original.as_object_mut().ok_or("Statement")?.remove("id");
    original["actor"]["name"] = json!("A 日本語");
    original["result"] = json!({"response":"unchanged","score":{"raw":17,"min":0,"max":20},"extensions":{"https://example.org/opaque":{"z":null,"a":["first","second"],"object":{"b":2,"a":1}}}});
    original["context"] = json!({"language":"ja","extensions":{"https://example.org/context":{"id":"opaque","stored":"opaque"}}});
    let encoded = (0..3)
        .map(|order| ordered_json(&original, order))
        .collect::<Vec<_>>();
    assert_eq!(
        encoded
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
    for raw in &encoded {
        assert_eq!(
            serde_json::from_str::<Value>(raw).map_err(|e| e.to_string())?,
            original
        );
    }
    evidence::observation(
        "json-member-orders",
        &json!({"orders":["alphabetical","reverse","seeded hash permutation"],"seed":0x9274_2023_u64,"raw":encoded,"array_policy":"original array sequence remains unchanged; permutation applies only to object members"}),
    );
    for raw in &encoded {
        let id = Uuid::new_v4();
        let url = format!("{base}/statements?statementId={id}");
        assert_eq!(
            receive(
                client
                    .put(&url)
                    .header("Content-Type", "application/json")
                    .body(raw.clone())
            )
            .await?
            .status(),
            StatusCode::NO_CONTENT
        );
        let before = state
            .xapi
            .statements
            .get(id)
            .await
            .map_err(|e| format!("{e:?}"))?
            .ok_or("stored order fixture")?;
        for reordered in &encoded {
            assert_eq!(
                receive(
                    client
                        .put(&url)
                        .header("Content-Type", "application/json")
                        .body(reordered.clone())
                )
                .await?
                .status(),
                StatusCode::NO_CONTENT
            );
            let reply = receive(client.get(&url)).await?;
            assert_eq!(reply.status(), StatusCode::OK);
            let returned = reply.json::<Value>().await.map_err(|e| e.to_string())?;
            for (key, value) in original.as_object().ok_or("Statement")? {
                assert_eq!(returned[key], *value);
            }
            let after = state
                .xapi
                .statements
                .get(id)
                .await
                .map_err(|e| format!("{e:?}"))?
                .ok_or("unchanged stored fixture")?;
            assert_eq!(after.statement, before.statement);
            assert_eq!(after.stored_at, before.stored_at);
            assert_eq!(after.opaque_values, before.opaque_values);
        }
        let mut changed = original.clone();
        changed["result"]["extensions"]["https://example.org/opaque"]["a"] =
            json!(["second", "first"]);
        assert_eq!(
            receive(client.put(&url).json(&changed)).await?.status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            state
                .xapi
                .statements
                .get(id)
                .await
                .map_err(|e| format!("{e:?}"))?
                .ok_or("array unchanged")?
                .statement,
            before.statement
        );
    }
    for spelling in ["actor", "ac\\u0074or"] {
        let raw = format!(
            "{{\"{spelling}\":{},{}",
            original["actor"],
            &encoded[0][1..]
        );
        let id = Uuid::new_v4();
        assert_eq!(
            receive(
                client
                    .put(format!("{base}/statements?statementId={id}"))
                    .header("Content-Type", "application/json")
                    .body(raw)
            )
            .await?
            .status(),
            StatusCode::BAD_REQUEST
        );
        assert!(
            state
                .xapi
                .statements
                .get(id)
                .await
                .map_err(|e| format!("{e:?}"))?
                .is_none()
        );
    }
    server.abort();
    Ok(())
}

#[tokio::test]
async fn memory_xapi_uuid_format_and_variant_recommendation() -> Result<(), String> {
    uuid_roles(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_uuid_format_and_variant_recommendation() -> Result<(), String> {
    uuid_roles(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

async fn uuid_roles(state: &AppState) -> Result<(), String> {
    let (base, server) = serve(state).await?;
    let client = reqwest::Client::new();
    for post in [false, true] {
        for variant in [0x00, 0x80, 0xc0, 0xe0] {
            // All are independent standard-format UUIDs. Variant2 is SHOULD;
            // no external issuance/provenance or resolvability is assumed.
            let id = Uuid::from_fields(
                0,
                0,
                0,
                &[variant, 0, 0, 0, 0, 0, 0, if post { 2 } else { 1 }],
            );
            let value = statement(id);
            let request = if post {
                client
                    .post(format!("{base}/statements"))
                    .json(&json!([value]))
            } else {
                client
                    .put(format!("{base}/statements?statementId={id}"))
                    .json(&value)
            };
            let response = receive(request).await?;
            assert_eq!(
                response.status(),
                if post {
                    StatusCode::OK
                } else {
                    StatusCode::NO_CONTENT
                }
            );
            if post {
                assert_eq!(
                    response.json::<Value>().await.map_err(|e| e.to_string())?,
                    json!([id])
                );
            }
            let response =
                receive(client.get(format!("{base}/statements?statementId={id}&format=exact")))
                    .await?;
            assert_eq!(response.status(), StatusCode::OK);
            let saved: Value = response.json().await.map_err(|e| e.to_string())?;
            assert_eq!(saved["id"], json!(id));
            evidence::observation(
                "uuid-input-recommendation",
                &json!({"id":id,"post":post,
                "variant2":id.get_variant()==uuid::Variant::RFC4122,"strength":"SHOULD, not a format rejection condition"}),
            );
        }
    }
    for count in [1, 2] {
        let mut submitted = Vec::new();
        for _ in 0..count {
            let mut value = statement(Uuid::nil());
            value.as_object_mut().ok_or("Statement")?.remove("id");
            submitted.push(value);
        }
        let body = if count == 1 {
            submitted[0].clone()
        } else {
            json!(submitted)
        };
        let response = receive(client.post(format!("{base}/statements")).json(&body)).await?;
        assert_eq!(response.status(), StatusCode::OK);
        let ids: Vec<String> = response.json().await.map_err(|e| e.to_string())?;
        assert_eq!(ids.len(), count);
        assert_eq!(
            ids.iter().collect::<std::collections::BTreeSet<_>>().len(),
            count
        );
        for id in ids {
            let parsed = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
            assert_eq!(
                id.to_ascii_lowercase(),
                parsed.hyphenated().to_string(),
                "standard UUID string form"
            );
            let response =
                receive(client.get(format!("{base}/statements?statementId={id}"))).await?;
            assert_eq!(response.status(), StatusCode::OK);
            let saved: Value = response.json().await.map_err(|e| e.to_string())?;
            assert_eq!(saved["id"], json!(id));
            assert_eq!(saved["actor"], submitted[0]["actor"]);
            evidence::observation(
                "uuid-generated-recommendation",
                &json!({"id":id,"variant2":parsed.get_variant()==uuid::Variant::RFC4122,
                "strength":"standard string form required; variant2 observed as SHOULD"}),
            );
        }
    }
    server.abort();
    Ok(())
}

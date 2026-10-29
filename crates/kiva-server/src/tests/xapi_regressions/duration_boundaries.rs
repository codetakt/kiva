#![allow(clippy::wildcard_imports)]
use super::*;

#[tokio::test]
async fn memory_xapi_duration_grammar_and_preservation() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_duration_grammar_and_preservation() -> Result<(), String> {
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
    OffsetDateTime::parse(
        reply.headers()["x-experience-api-consistent-through"]
            .to_str()
            .map_err(|e| e.to_string())?,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| e.to_string())?;
    Ok(reply)
}

async fn write(
    client: &reqwest::Client,
    base: &str,
    value: &Value,
    post: bool,
) -> Result<reqwest::Response, String> {
    let request = if post {
        client.post(format!("{base}/statements"))
    } else {
        client.put(format!(
            "{base}/statements?statementId={}",
            value["id"].as_str().ok_or("ID")?
        ))
    };
    receive(request.json(&if post { json!([value]) } else { value.clone() })).await
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
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../tests/conformance/fixtures/preservation-properties.json"
    ))
    .map_err(|e| e.to_string())?;
    for valid in [true, false] {
        let mut vectors = fixture["duration_grammar"][if valid { "valid" } else { "invalid" }]
            .as_array()
            .ok_or("duration vectors")?
            .clone();
        if valid {
            vectors.extend(
                [
                    "PT1.239S",
                    "PT1,239S",
                    "PT1.234S",
                    "PT1.230S",
                    "PT1S",
                    "PT1.23S",
                    "PT0.00001H",
                    "PT9007199254740993.011S",
                ]
                .map(|v| json!(v)),
            );
        } else {
            vectors.push(json!("P0001-02-03T04:05:06"));
        }
        for duration in vectors {
            for nested in [false, true] {
                for post in [false, true] {
                    let id = Uuid::new_v4();
                    let mut value = statement(id);
                    value["result"] = json!({"duration":duration});
                    let pointer = if nested {
                        value.as_object_mut().ok_or("Statement")?.remove("id");
                        value["objectType"] = json!("SubStatement");
                        let mut root = statement(id);
                        root["object"] = value;
                        value = root;
                        "/object/result/duration"
                    } else {
                        "/result/duration"
                    };
                    let reply = write(&client, &base, &value, post).await?;
                    assert_eq!(
                        reply.status(),
                        if valid {
                            if post {
                                StatusCode::OK
                            } else {
                                StatusCode::NO_CONTENT
                            }
                        } else {
                            StatusCode::BAD_REQUEST
                        },
                        "{duration}/{pointer}/POST={post}"
                    );
                    if valid && post {
                        assert_eq!(
                            reply.json::<Value>().await.map_err(|e| e.to_string())?,
                            json!([id])
                        );
                    }
                    let url = format!("{base}/statements?statementId={id}&format=exact");
                    let reply = receive(client.get(&url)).await?;
                    assert_eq!(
                        reply.status(),
                        if valid {
                            StatusCode::OK
                        } else {
                            StatusCode::NOT_FOUND
                        }
                    );
                    if !valid {
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
                    let original = reply.json::<Value>().await.map_err(|e| e.to_string())?;
                    // Kiva selects exact preservation, a stronger permitted
                    // outcome than centisecond truncation. No fixed calendar
                    // conversion is used by this input/output assertion.
                    assert_eq!(original.pointer(pointer), Some(&duration));
                    let mut changed = value.clone();
                    *changed.pointer_mut(pointer).ok_or("duration")? = json!("PT9S");
                    let replay_attempt = write(&client, &base, &changed, post).await?;
                    assert!(
                        replay_attempt.status() == StatusCode::CONFLICT
                            || replay_attempt.status()
                                == if post {
                                    StatusCode::OK
                                } else {
                                    StatusCode::NO_CONTENT
                                }
                    );
                    let after = receive(client.get(&url)).await?;
                    assert_eq!(after.status(), StatusCode::OK);
                    assert_eq!(
                        after.json::<Value>().await.map_err(|e| e.to_string())?,
                        original
                    );
                }
            }
        }
    }
    server.abort();
    Ok(())
}

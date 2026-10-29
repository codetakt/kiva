#![allow(clippy::wildcard_imports)]
use super::*;

#[tokio::test]
async fn memory_xapi_exact_numeric_field_boundaries() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_exact_numeric_field_boundaries() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
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
    let mut cases = [
        (r#"{"scaled":1.0000000000000001}"#, false),
        (r#"{"scaled":-1.0000000000000001}"#, false),
        (r#"{"scaled":0.999999999999999999999999999}"#, true),
        (r#"{"scaled":1e-99999999999999999}"#, true),
        (r#"{"scaled":-0.000}"#, true),
        (r#"{"scaled":true}"#, false),
        (r#"{"raw":"1"}"#, false),
        (
            r#"{"min":9007199254740992,"max":9007199254740993,"raw":9007199254740993}"#,
            true,
        ),
        (
            r#"{"min":9007199254740992,"max":9007199254740993,"raw":9007199254740994}"#,
            false,
        ),
        (
            r#"{"min":-9007199254740993,"max":-9007199254740992,"raw":-9007199254740994}"#,
            false,
        ),
        (
            r#"{"min":1,"max":1.0000000000000001,"raw":1.00000000000000001}"#,
            true,
        ),
        (
            r#"{"min":1,"max":1.0000000000000001,"raw":1.00000000000000011}"#,
            false,
        ),
        (r#"{"min":1e10,"max":10000000000.0}"#, false),
        (
            r#"{"min":-1e9999999999999,"max":1e9999999999999,"raw":2e9999999999998}"#,
            true,
        ),
        (
            r#"{"min":-1e9999999999999,"max":1e9999999999999,"raw":2e9999999999999}"#,
            false,
        ),
        (
            r#"{"min":0,"max":1e-9999999999999,"raw":2e-9999999999999}"#,
            false,
        ),
        (r#"{"raw":18446744073709551616}"#, true),
        (r#"{"raw":-9223372036854775809}"#, true),
    ]
    .into_iter()
    .map(|(raw, valid)| (raw.to_owned(), valid))
    .collect::<Vec<_>>();
    // Exact original decimal tokens, with both binary32 edges and values beyond
    // that range. Kiva retains greater precision/range instead of rounding.
    for (token, scaled) in [
        ("0", true),
        ("-0", true),
        ("1", true),
        ("-1", true),
        ("16777217", false),
        ("0.10000000149011612", true),
        ("0.33333334", true),
        ("-0.33333334", true),
        ("1.401298464324817e-45", true),
        ("-1.401298464324817e-45", true),
        ("1.1754943508222875e-38", true),
        ("-1.1754943508222875e-38", true),
        ("3.4028234663852886e38", false),
        ("-3.4028234663852886e38", false),
        ("3.4028236e38", false),
        ("1e400", false),
        ("-1e400", false),
        ("9223372036854775807", false),
        ("9223372036854775808", false),
        ("18446744073709551615", false),
        ("18446744073709551616", false),
    ] {
        for field in ["scaled", "raw", "min", "max"] {
            cases.push((
                format!("{{\"{field}\":{token}}}"),
                field != "scaled" || scaled,
            ));
        }
    }
    for field in ["scaled", "raw", "min", "max"] {
        for invalid in ["true", "false", "null", "\"1\"", "NaN", "Infinity"] {
            cases.push((format!("{{\"{field}\":{invalid}}}"), false));
        }
    }
    evidence::observation(
        "numeric-input-contract",
        &json!({"value_model":"original JSON decimal tokens, exact retained value; no binary64 input conversion","rounding":"no rounding on storage; greater range/precision than binary32","placements":"root/SubStatement scaled/raw/min/max; scaled range remains independent","vectors":cases}),
    );
    for nested in [false, true] {
        for post in [false, true] {
            for (score, accepted) in &cases {
                let id = Uuid::new_v4();
                let mut value = statement(id);
                if nested {
                    let mut child = statement(Uuid::nil());
                    child.as_object_mut().ok_or("child")?.remove("id");
                    child["objectType"] = json!("SubStatement");
                    child["result"] = json!({"score":"SCORE-SENTINEL"});
                    value["object"] = child;
                } else {
                    value["result"] = json!({"score":"SCORE-SENTINEL"});
                }
                let raw = (if post {
                    json!([value]).to_string()
                } else {
                    value.to_string()
                })
                .replace("\"SCORE-SENTINEL\"", score);
                let request = if post {
                    client.post(format!("{base}/statements"))
                } else {
                    client.put(format!("{base}/statements?statementId={id}"))
                };
                let reply = evidence::received(
                    request
                        .header("X-Experience-API-Version", "2.0.0")
                        .header("Content-Type", "application/json")
                        .body(raw)
                        .send()
                        .await
                        .map_err(|e| e.to_string())?,
                )
                .await?;
                assert_eq!(
                    reply.status(),
                    if *accepted {
                        if post {
                            StatusCode::OK
                        } else {
                            StatusCode::NO_CONTENT
                        }
                    } else {
                        StatusCode::BAD_REQUEST
                    },
                    "{score}/{nested}/{post}: {}",
                    reply.text().await.map_err(|e| e.to_string())?
                );
                let saved = state
                    .xapi
                    .statements
                    .get(id)
                    .await
                    .map_err(|e| format!("{e:?}"))?;
                assert_eq!(saved.is_some(), *accepted);
                if let Some(saved) = saved {
                    let expected: Value = serde_json::from_str(score).map_err(|e| e.to_string())?;
                    let path = if nested {
                        "/object/result/score"
                    } else {
                        "/result/score"
                    };
                    assert_eq!(saved.statement.pointer(path), Some(&expected));
                    let reply = evidence::received(
                        client
                            .get(format!("{base}/statements?statementId={id}"))
                            .header("X-Experience-API-Version", "2.0.0")
                            .send()
                            .await
                            .map_err(|e| e.to_string())?,
                    )
                    .await?;
                    assert_eq!(reply.status(), StatusCode::OK);
                    let returned: Value =
                        serde_json::from_str(&reply.text().await.map_err(|e| e.to_string())?)
                            .map_err(|e| e.to_string())?;
                    assert_eq!(
                        returned.pointer(path),
                        Some(&expected),
                        "numeric tokens must remain exact"
                    );
                }
            }
        }
    }
    server.abort();
    Ok(())
}

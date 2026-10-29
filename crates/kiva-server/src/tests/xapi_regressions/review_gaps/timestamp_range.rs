//! UTC conversion/formatting failures cannot abort admission or partly store a batch.
#![allow(clippy::wildcard_imports)]
use super::*;

async fn send(
    http: &HttpProbe,
    payload: &Value,
    post: bool,
    mime: bool,
) -> Result<reqwest::Response, String> {
    let request = if post {
        http.client.post(format!("{}/statements", http.base))
    } else {
        http.client.put(format!(
            "{}/statements?statementId={}",
            http.base,
            payload["id"].as_str().ok_or("id")?
        ))
    };
    HttpProbe::receive(if mime {
        request
            .header("Content-Type", "multipart/mixed; boundary=review-gap")
            .body(multipart(payload, &[]))
    } else {
        request.json(payload)
    })
    .await
}

async fn read(http: &HttpProbe, value: &Value) -> Result<Value, String> {
    let reply = HttpProbe::receive(http.client.get(format!(
        "{}/statements?statementId={}",
        http.base,
        value["id"].as_str().ok_or("id")?
    )))
    .await?;
    assert_eq!(reply.status(), StatusCode::OK);
    reply.json().await.map_err(|e| e.to_string())
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    for nested in [false, true] {
        for mode in 0..3 {
            for mime in [false, true] {
                for (input, expected) in [
                    (
                        "9999-12-31T22:30:00.123456789012-01:00",
                        "9999-12-31T23:30:00.123456789012Z",
                    ),
                    ("9999-12-31T23:30:00+01:00", "9999-12-31T22:30:00Z"),
                    (
                        "0000-01-01T01:30:00.000000000001+01:00",
                        "0000-01-01T00:30:00.000000000001Z",
                    ),
                    ("0000-01-01T00:30:00-01:00", "0000-01-01T01:30:00Z"),
                ] {
                    let (mut value, path) = root_and_owner(nested);
                    value.pointer_mut(path).ok_or("owner")?["timestamp"] = json!(input);
                    let payload = if mode == 2 {
                        json!([value])
                    } else {
                        value.clone()
                    };
                    let reply = send(&http, &payload, mode != 0, mime).await?;
                    assert_eq!(
                        reply.status(),
                        if mode == 0 {
                            StatusCode::NO_CONTENT
                        } else {
                            StatusCode::OK
                        }
                    );
                    let saved = read(&http, &value).await?;
                    assert_eq!(
                        saved.pointer(&format!("{path}/timestamp")),
                        Some(&json!(expected))
                    );
                    // The accepted UTC spelling is also safe to replay.
                    *value
                        .pointer_mut(&format!("{path}/timestamp"))
                        .ok_or("timestamp")? = json!(expected);
                    let payload = if mode == 2 {
                        json!([value])
                    } else {
                        value.clone()
                    };
                    assert!(
                        send(&http, &payload, mode != 0, mime)
                            .await?
                            .status()
                            .is_success()
                    );
                    assert_eq!(read(&http, &value).await?, saved);
                }
                for input in [
                    "9999-12-31T23:30:00-01:00",
                    "9999-12-31T23:59:59.999999999999-00:01",
                    "0000-01-01T00:30:00+01:00",
                    "0000-01-01T00:00:00.000000000001+00:01",
                ] {
                    let before = state
                        .xapi
                        .statements
                        .settled_snapshot()
                        .await
                        .map_err(|e| format!("{e:?}"))?;
                    let (mut invalid, path) = root_and_owner(nested);
                    invalid.pointer_mut(path).ok_or("owner")?["timestamp"] = json!(input);
                    let payload = if mode == 2 {
                        // A valid prefix and suffix must not leak from a rejected batch.
                        json!([
                            statement(Uuid::new_v4()),
                            invalid,
                            statement(Uuid::new_v4())
                        ])
                    } else {
                        invalid.clone()
                    };
                    let reply = send(&http, &payload, mode != 0, mime).await?;
                    assert_eq!(reply.status(), StatusCode::BAD_REQUEST, "{input}");
                    assert!(
                        reply
                            .text()
                            .await
                            .map_err(|e| e.to_string())?
                            .contains("UTC RFC3339")
                    );
                    assert_eq!(
                        state
                            .xapi
                            .statements
                            .settled_snapshot()
                            .await
                            .map_err(|e| format!("{e:?}"))?,
                        before
                    );
                    let values = payload
                        .as_array()
                        .map_or_else(|| vec![&payload], |values| values.iter().collect());
                    for value in values {
                        let reply = HttpProbe::receive(http.client.get(format!(
                            "{}/statements?statementId={}",
                            http.base,
                            value["id"].as_str().ok_or("id")?
                        )))
                        .await?;
                        assert_eq!(reply.status(), StatusCode::NOT_FOUND);
                    }
                    let reply =
                        HttpProbe::receive(http.client.get(format!("{}/about", http.base))).await?;
                    assert_eq!(reply.status(), StatusCode::OK);
                }
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn memory_xapi_utc_range_admission_and_atomicity() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_utc_range_admission_and_atomicity() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

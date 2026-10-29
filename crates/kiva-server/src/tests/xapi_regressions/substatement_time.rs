#![allow(clippy::wildcard_imports)]
use super::*;
use crate::surfaces::xapi::storage::take_replay_observations;

#[tokio::test]
async fn memory_xapi_substatement_time_default_and_replay() -> Result<(), String> {
    for post in [false, true] {
        for explicit in [false, true] {
            verify(
                &memory_test_state_with_auth(AuthConfig::default()),
                post,
                explicit,
            )
            .await?;
        }
    }
    Ok(())
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_substatement_time_default_and_replay() -> Result<(), String> {
    for post in [false, true] {
        for explicit in [false, true] {
            verify(
                &postgres_test_state()
                    .await?
                    .ok_or("PostgreSQL URL must be set")?,
                post,
                explicit,
            )
            .await?;
        }
    }
    Ok(())
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
async fn send(
    client: &reqwest::Client,
    base: &str,
    value: &Value,
    post: bool,
) -> Result<reqwest::Response, String> {
    receive(if post {
        client
            .post(format!("{base}/statements"))
            .json(&json!([value]))
    } else {
        client
            .put(format!(
                "{base}/statements?statementId={}",
                value["id"].as_str().ok_or("id")?
            ))
            .json(value)
    })
    .await
}

async fn verify(state: &AppState, post: bool, explicit: bool) -> Result<(), String> {
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
    let recorded = OffsetDateTime::parse(
        "2026-10-20T00:00:00.125Z",
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| e.to_string())?;
    state.xapi.statements.clock().set(recorded);
    let id = Uuid::new_v4();
    let mut value = statement(id);
    value["timestamp"] = json!("2000-01-01T00:00:00Z");
    let mut child = statement(Uuid::nil());
    child.as_object_mut().ok_or("child")?.remove("id");
    child["objectType"] = json!("SubStatement");
    if explicit {
        child["timestamp"] = json!("2020-01-01T01:00:00.123+01:00");
    }
    value["object"] = child;
    assert!(
        send(&client, &base, &value, post)
            .await?
            .status()
            .is_success()
    );
    let reply = receive(client.get(format!("{base}/statements?statementId={id}"))).await?;
    assert_eq!(reply.status(), StatusCode::OK);
    let saved: Value = reply.json().await.map_err(|e| e.to_string())?;
    let actual_recorded = OffsetDateTime::parse(
        saved["stored"].as_str().ok_or("stored")?,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| e.to_string())?;
    assert_eq!(actual_recorded, recorded);
    assert_eq!(saved["timestamp"], value["timestamp"]);
    assert!(saved["object"].get("stored").is_none());
    assert_eq!(
        saved["object"]["timestamp"],
        json!(if explicit {
            "2020-01-01T00:00:00.123Z"
        } else {
            "2026-10-20T00:00:00.125Z"
        })
    );
    let before = state
        .xapi
        .statements
        .list(None)
        .await
        .map_err(|e| format!("{e:?}"))?;
    // Keep R fixed for later fixtures; the clock's rollback clamp is
    // exercised elsewhere. A later sample must not replace saved U/R.
    for (pointer, changed, equivalent) in [
        ("/timestamp", "2001-01-01T00:00:00Z", true),
        ("/object/timestamp", "2021-01-01T00:00:00Z", false),
    ] {
        let mut retransmission = value.clone();
        if pointer == "/object/timestamp" {
            retransmission["object"]["timestamp"] = json!(changed);
        } else {
            retransmission["timestamp"] = json!(changed);
        }
        take_replay_observations();
        let reply = send(&client, &base, &retransmission, post).await?;
        let observations = take_replay_observations();
        evidence::observation(
            "nested-time-replay",
            &json!({"post":post,"explicit":explicit,"pointer":pointer,"status":reply.status().as_u16(),"calls":observations}),
        );
        assert!(
            observations
                .iter()
                .any(|o| o["existing_id"] == json!(id) && o["equivalent"] == json!(equivalent))
        );
        assert!(matches!(
            reply.status(),
            StatusCode::OK | StatusCode::NO_CONTENT | StatusCode::CONFLICT
        ));
        assert_eq!(
            state
                .xapi
                .statements
                .list(None)
                .await
                .map_err(|e| format!("{e:?}"))?,
            before
        );
        let reply = receive(client.get(format!("{base}/statements?statementId={id}"))).await?;
        assert_eq!(
            reply.json::<Value>().await.map_err(|e| e.to_string())?,
            saved
        );
    }
    server.abort();
    Ok(())
}

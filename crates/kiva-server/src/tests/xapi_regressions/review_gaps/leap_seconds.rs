//! Literal UTC expectations avoid repeating the runtime's lossy leap parser.
#![allow(clippy::wildcard_imports)]
use super::*;
use crate::surfaces::xapi::storage::take_replay_observations;

async fn send(http: &HttpProbe, value: &Value, mode: u8) -> Result<reqwest::Response, String> {
    HttpProbe::receive(match mode {
        0 => http
            .client
            .put(format!(
                "{}/statements?statementId={}",
                http.base,
                value["id"].as_str().ok_or("id")?
            ))
            .json(value),
        1 => http
            .client
            .post(format!("{}/statements", http.base))
            .json(value),
        _ => http
            .client
            .post(format!("{}/statements", http.base))
            .json(&json!([value])),
    })
    .await
}

async fn read(http: &HttpProbe, id: &str, format: &str) -> Result<Value, String> {
    let reply = HttpProbe::receive(http.client.get(format!(
        "{}/statements?statementId={id}&format={format}",
        http.base
    )))
    .await?;
    assert_eq!(reply.status(), StatusCode::OK);
    reply.json().await.map_err(|e| e.to_string())
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    for mode in 0..3 {
        for nested in [false, true] {
            for (input, expected) in [
                ("2016-12-31T23:59:60Z", "2016-12-31T23:59:60Z"),
                ("2016-12-31T23:59:60.250Z", "2016-12-31T23:59:60.25Z"),
                ("2016-12-31T23:59:60.750Z", "2016-12-31T23:59:60.75Z"),
                ("2017-01-01T08:59:60.250+09:00", "2016-12-31T23:59:60.25Z"),
                ("2016-12-31T18:29:60.750-05:30", "2016-12-31T23:59:60.75Z"),
                ("2015-06-30t23:59:60.000z", "2015-06-30T23:59:60Z"),
                (
                    "2016-12-31T23:59:60.123456789012Z",
                    "2016-12-31T23:59:60.123456789012Z",
                ),
                (
                    "2016-12-31T23:59:59.999999999Z",
                    "2016-12-31T23:59:59.999999999Z",
                ),
                (
                    "1970-01-01T00:30:00.000000000001+01:00",
                    "1969-12-31T23:30:00.000000000001Z",
                ),
            ] {
                let (mut value, owner) = root_and_owner(nested);
                let pointer = format!("{owner}/timestamp");
                value.pointer_mut(owner).ok_or("owner")?["timestamp"] = json!(input);
                let reply = send(&http, &value, mode).await?;
                assert_eq!(
                    reply.status(),
                    if mode == 0 {
                        StatusCode::NO_CONTENT
                    } else {
                        StatusCode::OK
                    },
                    "{input}: {}",
                    reply.text().await.map_err(|e| e.to_string())?
                );
                let id = value["id"].as_str().ok_or("id")?;
                for format in ["exact", "canonical", "ids"] {
                    let saved = read(&http, id, format).await?;
                    assert_eq!(
                        saved.pointer(&pointer),
                        Some(&json!(expected)),
                        "{input}, {pointer}, {format}"
                    );
                }
                if input != "2016-12-31T23:59:60.250Z" {
                    continue;
                }
                let before = read(&http, id, "exact").await?;
                for (changed, same_instant) in [
                    ("2017-01-01T08:59:60.2500+09:00", true),
                    ("2016-12-31T23:59:60.750Z", false),
                    ("2016-12-31T23:59:59.999999999Z", false),
                ] {
                    let mut retransmission = value.clone();
                    *retransmission.pointer_mut(&pointer).ok_or("timestamp")? = json!(changed);
                    take_replay_observations();
                    let reply = send(&http, &retransmission, mode).await?;
                    let calls = take_replay_observations();
                    // The selected replay policy excludes the root timestamp,
                    // but an explicitly supplied SubStatement time is content.
                    let equivalent = !nested || same_instant;
                    evidence::observation(
                        "leap-second-replay",
                        &json!({"mode":mode,"pointer":pointer,"changed":changed,"calls":calls}),
                    );
                    assert!(calls.iter().any(|call| call["existing_id"] == value["id"]
                        && call["equivalent"] == json!(equivalent)));
                    if equivalent {
                        assert!(reply.status().is_success());
                    } else {
                        assert_eq!(reply.status(), StatusCode::CONFLICT);
                    }
                    assert_eq!(read(&http, id, "exact").await?, before);
                }
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn memory_xapi_leap_second_preservation_and_replay() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_leap_second_preservation_and_replay() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

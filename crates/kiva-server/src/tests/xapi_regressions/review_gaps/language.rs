#![allow(clippy::wildcard_imports)]
use super::*;

async fn language_preferences(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    let maps = json!({"en-US":"English","ja":"Japanese"});
    let (mut value, owner) = root_and_owner(true);
    let mut metadata = attachment(b"", &format!("{:x}", Sha256::digest(b"")));
    metadata["fileUrl"] = json!("https://example.invalid/file");
    metadata["display"] = maps.clone();
    metadata["description"] = maps.clone();
    let activity = json!({"id":"https://example.org/language", "definition":{
        "name":maps, "description":maps, "interactionType":"choice", "choices":[{"id":"a","description":maps}]}});
    for prefix in ["", owner] {
        let target = value.pointer_mut(prefix).ok_or("owner")?;
        target["verb"]["display"] = maps.clone();
        target["attachments"] = json!([metadata]);
        target["context"] = json!({"contextActivities":{"parent":[activity]}});
    }
    value["object"]["object"] = activity;
    http.submit(&value, false, None, true).await?;
    let pointers = [
        "/verb/display",
        "/attachments/0/display",
        "/attachments/0/description",
        "/context/contextActivities/parent/0/definition/name",
        "/context/contextActivities/parent/0/definition/description",
        "/context/contextActivities/parent/0/definition/choices/0/description",
        "/object/verb/display",
        "/object/attachments/0/display",
        "/object/attachments/0/description",
        "/object/object/definition/name",
        "/object/object/definition/description",
        "/object/object/definition/choices/0/description",
        "/object/context/contextActivities/parent/0/definition/name",
    ];
    for (fields, expected) in [
        (vec!["en;q=0", "*;q=0.5"], "ja"),
        (vec!["en;q=1", "en-US;q=0.1", "ja;q=0.5"], "ja"),
        (vec!["*;q=1", "en;q=0.1", "ja;q=0.5"], "ja"),
        (vec!["en;q=0", "EN-us;q=1", "ja;q=0.5"], "en-US"),
        (vec!["en;q=0.1", "ja;q=1"], "ja"),
        (vec!["en;q =0", "ja;q =1"], "ja"),
        (vec!["en;q= 0", "ja;q= 1"], "ja"),
        (vec!["en; q = 0", "ja; Q = 1"], "ja"),
        (vec!["en;q\t=\t0", "ja;q\t=\t1"], "ja"),
        (vec!["*;q = 0", "ja;q = 1"], "ja"),
        (vec!["ja;q=0.5", "en;q=0.5"], "ja"),
        (vec!["en;q=0.5", "ja;q=0.5"], "en-US"),
        (vec!["de"], "en-US"),
    ] {
        for collection in [false, true] {
            for split in [false, true] {
                let selector = if collection {
                    String::new()
                } else {
                    format!("statementId={}&", value["id"].as_str().ok_or("id")?)
                };
                let url = format!("{}/statements?{selector}format=canonical", http.base);
                let mut request = http.client.get(url);
                if split {
                    for field in &fields {
                        request = request.header(
                            if field.starts_with("en;") {
                                "aCcEpT-lAnGuAgE"
                            } else {
                                "Accept-Language"
                            },
                            *field,
                        );
                    }
                } else {
                    request = request.header("Accept-Language", fields.join(","));
                }
                let reply = HttpProbe::receive(request).await?;
                assert_eq!(reply.status(), StatusCode::OK);
                let body: Value = reply.json().await.map_err(|e| e.to_string())?;
                let result = if collection {
                    &body["statements"][0]
                } else {
                    &body
                };
                for pointer in pointers {
                    assert_eq!(
                        result.pointer(pointer),
                        Some(&json!({expected:maps[expected]})),
                        "{fields:?}/{split}/{collection}/{pointer}"
                    );
                }
            }
        }
    }
    let reply = HttpProbe::receive(http.client.get(format!(
        "{}/statements?statementId={}&format=exact",
        http.base,
        value["id"].as_str().ok_or("id")?
    )))
    .await?;
    let exact: Value = reply.json().await.map_err(|e| e.to_string())?;
    for pointer in pointers {
        assert_eq!(
            exact.pointer(pointer),
            Some(&maps),
            "canonical retrieval must not mutate stored language maps"
        );
    }
    Ok(())
}

#[tokio::test]
async fn memory_xapi_language_preferences() -> Result<(), String> {
    language_preferences(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_language_preferences() -> Result<(), String> {
    language_preferences(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

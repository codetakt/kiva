#![allow(clippy::wildcard_imports)]
use super::*;

async fn substatement_properties(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    for post in [false, true] {
        for route in ["json", "multipart", "signed-payload"] {
            for extra in [
                None,
                Some(Value::Null),
                Some(json!(1)),
                Some(json!(true)),
                Some(json!("x")),
                Some(json!([])),
                Some(json!({})),
                Some(json!({"relative":42})),
            ] {
                let (mut value, _) = root_and_owner(true);
                // Legal extension owners must continue to retain arbitrary JSON.
                let extensions =
                    json!({"https://example.org/owner":{"extensions":null, "array":[1,null,"x"]}});
                value["object"]["result"] = json!({"extensions":extensions});
                value["object"]["context"] = json!({"extensions":extensions});
                value["object"]["object"]["definition"] = json!({"extensions":extensions});
                let mut payload = value.clone();
                if let Some(extra) = &extra {
                    payload["object"]["extensions"] = extra.clone();
                }
                let signature = if route == "signed-payload" {
                    let jws = format!(
                        "{}.{}.AA",
                        URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256"}"#),
                        URL_SAFE_NO_PAD.encode(payload.to_string())
                    );
                    let hash = format!("{:x}", Sha256::digest(jws.as_bytes()));
                    let mut metadata = attachment(jws.as_bytes(), &hash);
                    metadata["usageType"] = json!("http://adlnet.gov/expapi/attachments/signature");
                    value["attachments"] = json!([metadata]);
                    Some((hash, jws))
                } else {
                    value = payload;
                    None
                };
                let id = value["id"].clone();
                let value = if post {
                    json!([statement(Uuid::new_v4()), value])
                } else {
                    value
                };
                let body = match route {
                    "json" => None,
                    "signed-payload" => {
                        let (hash, jws) = signature.as_ref().ok_or("signature")?;
                        Some(multipart(
                            &value,
                            &[(hash, Some("application/octet-stream"), jws.as_bytes())],
                        ))
                    }
                    _ => Some(multipart(&value, &[])),
                };
                http.submit(&value, post, body, extra.is_none()).await?;
                if extra.is_none() {
                    let reply = HttpProbe::receive(http.client.get(format!(
                        "{}/statements?statementId={}",
                        http.base,
                        id.as_str().ok_or("id")?
                    )))
                    .await?;
                    let saved: Value = reply.json().await.map_err(|e| e.to_string())?;
                    for pointer in [
                        "/object/result/extensions",
                        "/object/context/extensions",
                        "/object/object/definition/extensions",
                    ] {
                        assert_eq!(saved.pointer(pointer), Some(&extensions));
                    }
                }
            }
        }
    }
    Ok(())
}

async fn locator_syntax(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    for nested in [false, true] {
        for post in [false, true] {
            for role in ["moreInfo", "context-moreInfo", "fileUrl", "homePage"] {
                for (locator, accepted) in [
                    ("https:relative", false),
                    ("https:///path", false),
                    ("https://", false),
                    ("HTTP:/path", false),
                    ("https://?query", false),
                    ("https://example.invalid/学習", true),
                    ("http://example.invalid/path", true),
                    ("urn:example:location", true),
                ] {
                    let (mut value, path) = root_and_owner(nested);
                    let owner = value.pointer_mut(path).ok_or("owner")?;
                    match role {
                        "moreInfo" => owner["object"]["definition"] = json!({"moreInfo":locator}),
                        "context-moreInfo" => {
                            owner["context"] = json!({"contextActivities":{"parent":[{"id":"urn:example:activity","definition":{"moreInfo":locator}}]}});
                        }
                        "fileUrl" => {
                            let mut metadata =
                                attachment(b"", &format!("{:x}", Sha256::digest(b"")));
                            metadata["fileUrl"] = json!(locator);
                            owner["attachments"] = json!([metadata]);
                        }
                        _ => {
                            owner["actor"] =
                                json!({"account":{"homePage":locator,"name":"learner"}});
                        }
                    }
                    let value = if post {
                        json!([statement(Uuid::new_v4()), value])
                    } else {
                        value
                    };
                    http.submit(&value, post, None, accepted).await?;
                }
            }
            // General identifiers are not accidentally subject to locator rules.
            let (mut value, path) = root_and_owner(nested);
            value.pointer_mut(path).ok_or("owner")?["verb"]["id"] = json!("https:identifier");
            http.submit(&value, post, None, true).await?;
        }
    }
    Ok(())
}

#[tokio::test]
async fn memory_xapi_substatement_properties() -> Result<(), String> {
    substatement_properties(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_substatement_properties() -> Result<(), String> {
    substatement_properties(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

#[tokio::test]
async fn memory_xapi_locator_syntax() -> Result<(), String> {
    locator_syntax(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_locator_syntax() -> Result<(), String> {
    locator_syntax(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

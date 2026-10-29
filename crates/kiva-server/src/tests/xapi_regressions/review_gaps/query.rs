#![allow(clippy::wildcard_imports)]
use super::*;

fn location(base: &str, path: &str, agent: &str, id: Option<&str>) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(&format!("{base}{path}")).map_err(|e| e.to_string())?;
    let mut pairs = url.query_pairs_mut();
    pairs.append_pair("agent", agent);
    if path == "/activities/state" {
        pairs.append_pair("activityId", "https://example.org/query-gap");
    }
    if let Some(id) = id {
        pairs.append_pair(
            if path == "/activities/state" {
                "stateId"
            } else {
                "profileId"
            },
            id,
        );
    }
    drop(pairs);
    Ok(url)
}

async fn query_duplicates(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    for (duplicate, valid) in [
        (
            r#"{"mbox":"mailto:first@example.org","mbox":"mailto:last@example.org"}"#,
            r#"{"mbox":"mailto:last@example.org"}"#,
        ),
        (
            r#"{"mbox":null,"mbox":"mailto:last@example.org"}"#,
            r#"{"mbox":"mailto:last@example.org"}"#,
        ),
        (
            r#"{"mbox":"mailto:first@example.org","mbo\u0078":"mailto:last@example.org"}"#,
            r#"{"mbox":"mailto:last@example.org"}"#,
        ),
        (
            r#"{"account":{"homePage":"https://example.org/account","name":"first","name":"last"}}"#,
            r#"{"account":{"homePage":"https://example.org/account","name":"last"}}"#,
        ),
        (
            r#"{"objectType":"Group","mbox":"mailto:group@example.org","member":[{"mbox":"mailto:first@example.org","mbox":"mailto:last@example.org"}]}"#,
            r#"{"objectType":"Group","mbox":"mailto:group@example.org","member":[{"mbox":"mailto:last@example.org"}]}"#,
        ),
    ] {
        for path in ["/statements", "/agents"] {
            if path == "/agents" && valid.contains("Group") {
                continue;
            }
            for method in [reqwest::Method::GET, reqwest::Method::HEAD] {
                let reply = HttpProbe::receive(
                    http.client
                        .request(method.clone(), location(&http.base, path, valid, None)?),
                )
                .await?;
                assert_eq!(reply.status(), StatusCode::OK);
                let reply = HttpProbe::receive(
                    http.client
                        .request(method, location(&http.base, path, duplicate, None)?),
                )
                .await?;
                assert_eq!(
                    reply.status(),
                    StatusCode::BAD_REQUEST,
                    "{path} {duplicate}"
                );
            }
        }
        // Same duplicate rule at the Statement-body boundary.
        let mut value = statement(Uuid::new_v4());
        value["actor"] = json!("RAW-ACTOR");
        let reply = HttpProbe::receive(
            http.client
                .post(format!("{}/statements", http.base))
                .header("Content-Type", "application/json")
                .body(value.to_string().replace("\"RAW-ACTOR\"", duplicate)),
        )
        .await?;
        assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
        let reply = HttpProbe::receive(http.client.get(format!(
            "{}/statements?statementId={}",
            http.base,
            value["id"].as_str().ok_or("id")?
        )))
        .await?;
        assert_eq!(reply.status(), StatusCode::NOT_FOUND);
        for path in ["/agents/profile", "/activities/state"] {
            let id = Uuid::new_v4().to_string();
            let good = location(&http.base, path, valid, Some(&id))?;
            let bad = location(&http.base, path, duplicate, Some(&id))?;
            let original = "{\"original\":true}";
            let reply = HttpProbe::receive(
                http.client
                    .put(good.clone())
                    .header("Content-Type", "application/json")
                    .body(original),
            )
            .await?;
            assert_eq!(reply.status(), StatusCode::NO_CONTENT);
            let reply = HttpProbe::receive(http.client.get(good.clone())).await?;
            assert_eq!(reply.status(), StatusCode::OK);
            let etag = reply.headers()["etag"].clone();
            for method in [
                reqwest::Method::GET,
                reqwest::Method::HEAD,
                reqwest::Method::PUT,
                reqwest::Method::POST,
                reqwest::Method::DELETE,
            ] {
                let reply = HttpProbe::receive(
                    http.client
                        .request(method, bad.clone())
                        .header("If-Match", etag.clone())
                        .json(&json!({"changed":true})),
                )
                .await?;
                assert_eq!(
                    reply.status(),
                    StatusCode::BAD_REQUEST,
                    "{path}: {duplicate}"
                );
                let reply = HttpProbe::receive(http.client.get(good.clone())).await?;
                assert_eq!(reply.status(), StatusCode::OK);
                assert_eq!(reply.headers()["etag"], etag);
                assert_eq!(reply.text().await.map_err(|e| e.to_string())?, original);
            }
            for method in [reqwest::Method::GET, reqwest::Method::HEAD] {
                let reply = HttpProbe::receive(
                    http.client
                        .request(method, location(&http.base, path, duplicate, None)?),
                )
                .await?;
                assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
            }
            if path == "/activities/state" {
                let reply = HttpProbe::receive(
                    http.client
                        .delete(location(&http.base, path, duplicate, None)?),
                )
                .await?;
                assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
                let reply = HttpProbe::receive(http.client.get(good)).await?;
                assert_eq!(reply.status(), StatusCode::OK);
                assert_eq!(reply.headers()["etag"], etag);
                assert_eq!(reply.text().await.map_err(|e| e.to_string())?, original);
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn memory_xapi_query_duplicates() -> Result<(), String> {
    query_duplicates(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_query_duplicates() -> Result<(), String> {
    query_duplicates(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

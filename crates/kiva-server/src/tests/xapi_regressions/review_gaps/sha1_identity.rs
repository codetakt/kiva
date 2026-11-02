//! Real HTTP reads and conditional mutations share a digest identity.
#![allow(clippy::wildcard_imports)]
use super::*;

#[tokio::test]
async fn memory_xapi_sha1_identity_scopes() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_sha1_identity_scopes() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

fn url(http: &HttpProbe, path: &str, hash: &str, id: Option<&str>) -> Result<reqwest::Url, String> {
    let mut url =
        reqwest::Url::parse(&format!("{}{path}", http.base)).map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair("agent", &json!({"mbox_sha1sum":hash}).to_string());
    if path == "/activities/state" {
        url.query_pairs_mut()
            .append_pair("activityId", "https://example.org/sha-identity");
    }
    if let Some(id) = id {
        url.query_pairs_mut().append_pair(
            if path == "/activities/state" {
                "stateId"
            } else {
                "profileId"
            },
            id,
        );
    }
    Ok(url)
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    let lower = "abcdef0123456789abcdef0123456789abcdef01";
    let upper = lower.to_ascii_uppercase();
    let mixed = "aBcDeF0123456789aBcDeF0123456789aBcDeF01";
    let other = "abcdef0123456789abcdef0123456789abcdef04";
    let mut value = statement(Uuid::new_v4());
    value["actor"] = json!({"mbox_sha1sum":upper});
    http.submit(&value, false, None, true).await?;
    for hash in [lower, upper.as_str(), mixed, other] {
        let reply =
            HttpProbe::receive(http.client.get(url(&http, "/statements", hash, None)?)).await?;
        assert_eq!(reply.status(), StatusCode::OK);
        let body: Value = reply.json().await.map_err(|e| e.to_string())?;
        let values = body["statements"].as_array().ok_or("statements")?;
        assert_eq!(values.len(), usize::from(hash != other));
        if hash != other {
            assert_eq!(values[0]["id"], value["id"]);
            assert_eq!(values[0]["actor"], value["actor"]);
        }
    }
    for path in ["/activities/state", "/agents/profile"] {
        let id = "identity\0|agent:53:mbox_sha1sum:ABCDEF0123456789ABCDEF0123456789ABCDEF01|";
        let body = json!({"before":true});
        let reply = HttpProbe::receive(
            http.client
                .put(url(&http, path, &upper, Some(id))?)
                .json(&body),
        )
        .await?;
        assert_eq!(reply.status(), StatusCode::NO_CONTENT);
        let mut tag = String::new();
        for hash in [lower, upper.as_str(), mixed, other] {
            let reply =
                HttpProbe::receive(http.client.get(url(&http, path, hash, Some(id))?)).await?;
            if hash == other {
                assert_eq!(reply.status(), StatusCode::NOT_FOUND);
            } else {
                assert_eq!(reply.status(), StatusCode::OK);
                let current = reply.headers()["etag"]
                    .to_str()
                    .map_err(|e| e.to_string())?
                    .to_owned();
                if tag.is_empty() {
                    tag = current;
                } else {
                    assert_eq!(tag, current);
                }
                assert_eq!(
                    reply.json::<Value>().await.map_err(|e| e.to_string())?,
                    body
                );
            }
            let reply = HttpProbe::receive(http.client.get(url(&http, path, hash, None)?)).await?;
            assert_eq!(reply.status(), StatusCode::OK);
            assert_eq!(
                reply.json::<Value>().await.map_err(|e| e.to_string())?,
                if hash == other {
                    json!([])
                } else {
                    json!([id])
                }
            );
        }
        let selected = url(&http, path, mixed, Some(id))?;
        let reply = HttpProbe::receive(
            http.client
                .post(selected.clone())
                .header("If-Match", "\"stale\"")
                .json(&json!({"bad":true})),
        )
        .await?;
        assert_eq!(reply.status(), StatusCode::PRECONDITION_FAILED);
        let reply = HttpProbe::receive(
            http.client
                .post(selected.clone())
                .header("If-Match", &tag)
                .json(&json!({"after":true})),
        )
        .await?;
        assert_eq!(reply.status(), StatusCode::NO_CONTENT);
        let reply = HttpProbe::receive(http.client.get(url(&http, path, lower, Some(id))?)).await?;
        let updated_tag = reply.headers()["etag"]
            .to_str()
            .map_err(|e| e.to_string())?
            .to_owned();
        assert_ne!(updated_tag, tag);
        assert_eq!(
            reply.json::<Value>().await.map_err(|e| e.to_string())?,
            json!({"before":true,"after":true})
        );
        let reply =
            HttpProbe::receive(http.client.delete(selected.clone()).header("If-Match", tag))
                .await?;
        assert_eq!(reply.status(), StatusCode::PRECONDITION_FAILED);
        let reply =
            HttpProbe::receive(http.client.delete(selected).header("If-Match", updated_tag))
                .await?;
        assert_eq!(reply.status(), StatusCode::NO_CONTENT);
        let reply =
            HttpProbe::receive(http.client.get(url(&http, path, &upper, Some(id))?)).await?;
        assert_eq!(reply.status(), StatusCode::NOT_FOUND);
    }
    Ok(())
}

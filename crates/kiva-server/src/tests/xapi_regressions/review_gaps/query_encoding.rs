//! Invalid query octets must never alias an existing valid Unicode identifier.
#![allow(clippy::wildcard_imports)]
use super::*;

fn location(http: &HttpProbe, path: &str, id: &str) -> Result<reqwest::Url, String> {
    let mut url =
        reqwest::Url::parse(&format!("{}{path}", http.base)).map_err(|e| e.to_string())?;
    let mut query = url.query_pairs_mut();
    if path != "/activities/profile" {
        query.append_pair("agent", r#"{"mbox":"mailto:query-encoding@example.org"}"#);
    }
    if path != "/agents/profile" {
        query.append_pair("activityId", "https://example.org/query-encoding");
    }
    query.append_pair(
        if path == "/activities/state" {
            "stateId"
        } else {
            "profileId"
        },
        id,
    );
    drop(query);
    Ok(url)
}

async fn verify(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    for path in [
        "/activities/state",
        "/agents/profile",
        "/activities/profile",
    ] {
        for (token, replacement) in [
            ("%FF", "�"),
            ("%C0%AF", "��"),
            ("%ED%A0%80", "���"),
            ("%F4%90%80%80", "����"),
            ("%E2%82", "�"),
        ] {
            // Separate suffixes ensure each valid id is new on this backend.
            let id = format!("{}-{replacement}-{token}", Uuid::new_v4());
            let good = location(&http, path, &id)?;
            let bad =
                good.as_str()
                    .replacen(&"%EF%BF%BD".repeat(replacement.chars().count()), token, 1);
            let body = r#"{"sentinel":"unchanged"}"#;
            let reply = HttpProbe::receive(
                http.client
                    .put(good.clone())
                    .header("Content-Type", "application/json")
                    .body(body),
            )
            .await?;
            assert_eq!(reply.status(), StatusCode::NO_CONTENT);
            let reply = HttpProbe::receive(http.client.get(good.clone())).await?;
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
                        .request(method.clone(), &bad)
                        .header("If-Match", etag.clone())
                        .json(&json!({"changed":true})),
                )
                .await?;
                assert_eq!(
                    reply.status(),
                    StatusCode::BAD_REQUEST,
                    "{path} {method} {token}"
                );
                let reply = HttpProbe::receive(http.client.get(good.clone())).await?;
                assert_eq!(reply.status(), StatusCode::OK);
                assert_eq!(reply.headers()["etag"], etag);
                assert_eq!(reply.text().await.map_err(|e| e.to_string())?, body);
            }
        }
        // Literal escaped percent, '+', space, replacement character and
        // multibyte Unicode remain separate valid identities after one pass.
        let suffix = Uuid::new_v4();
        for (i, id) in ["%FF", "+", " ", "�", "学習😀"].iter().enumerate() {
            let url = location(&http, path, &format!("{suffix}-{id}"))?;
            let body = format!("document-{i}");
            let reply = HttpProbe::receive(http.client.put(url.clone()).body(body.clone())).await?;
            assert_eq!(reply.status(), StatusCode::NO_CONTENT);
            let reply = HttpProbe::receive(http.client.get(url)).await?;
            assert_eq!(reply.text().await.map_err(|e| e.to_string())?, body);
        }
    }
    for path in ["/agents", "/statements"] {
        let mut good =
            reqwest::Url::parse(&format!("{}{path}", http.base)).map_err(|e| e.to_string())?;
        good.query_pairs_mut().append_pair(
            "agent",
            &json!({"account":{"homePage":"https://example.org/encoding","name":"�"}}).to_string(),
        );
        for method in [reqwest::Method::GET, reqwest::Method::HEAD] {
            let reply =
                HttpProbe::receive(http.client.request(method.clone(), good.clone())).await?;
            assert_eq!(reply.status(), StatusCode::OK);
            for token in ["%FF", "%C0%AF", "%ED%A0%80", "%F4%90%80%80", "%E2%82"] {
                let bad = good.as_str().replace("%EF%BF%BD", token);
                let reply = HttpProbe::receive(http.client.request(method.clone(), bad)).await?;
                assert_eq!(reply.status(), StatusCode::BAD_REQUEST);
                if path == "/statements" {
                    assert!(
                        reply
                            .headers()
                            .contains_key("x-experience-api-consistent-through")
                    );
                }
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn memory_xapi_query_utf8_preserves_document_identity() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_query_utf8_preserves_document_identity() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

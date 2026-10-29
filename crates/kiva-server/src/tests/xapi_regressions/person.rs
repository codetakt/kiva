#![allow(clippy::wildcard_imports)]
use super::*;

#[tokio::test]
async fn memory_xapi_person_mailboxes_and_fallback_inventory() -> Result<(), String> {
    verify_person(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_person_mailboxes_and_fallback_inventory() -> Result<(), String> {
    verify_person(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

async fn person_request(
    client: &reqwest::Client,
    base: &str,
    agent: &Value,
) -> Result<reqwest::Response, String> {
    let url =
        reqwest::Url::parse_with_params(&format!("{base}/agents"), &[("agent", agent.to_string())])
            .map_err(|e| e.to_string())?;
    let reply = client
        .get(url)
        .header("X-Experience-API-Version", "2.0.0")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
    evidence::received(reply).await
}

#[allow(clippy::too_many_lines)]
async fn verify_person(state: &AppState) -> Result<(), String> {
    // Both factories create a fresh repository/unique PostgreSQL tenant. The
    // inventory is established BEFORE the Person request, not inferred from it.
    assert!(
        state
            .xapi
            .statements
            .list(None)
            .await
            .map_err(|e| format!("{e:?}"))?
            .is_empty()
    );
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
    evidence::observation(
        "person-inventory-authority",
        &json!({
            "revision":"isolated-native-person-v2","directory_enrichment":false,
            "source":"handlers/agents_activities.rs::get_agent_person -> request/person.rs::person_from_agent; no directory service or association worker",
            "empty_repository_before_setup":true,"isolation":"fresh memory store or unique PostgreSQL tenant",
            "fallback":"query identity and optional name only; no extra associations are maintained",
            "enriched_branch":"not applicable to this target capability; separate independent HTTP peers exercise multiple associated arrays without imposing singleton restrictions"
        }),
    );
    let unrelated = json!({"mbox":"mailto:unrelated-person-b@example.org", "name":"Only B"});
    let mut stored = statement(Uuid::new_v4());
    stored["actor"] = unrelated;
    let reply = client
        .post(format!("{base}/statements"))
        .header("X-Experience-API-Version", "2.0.0")
        .json(&stored)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let reply = evidence::received(reply).await?;
    assert_eq!(reply.status(), StatusCode::OK);
    assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
    assert!(
        reply
            .headers()
            .contains_key("x-experience-api-consistent-through")
    );
    let _ = reply.bytes().await.map_err(|e| e.to_string())?;
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../../tests/conformance/fixtures/mailto-contract.json"
    ))
    .map_err(|e| e.to_string())?;
    for row in vectors["vectors"].as_array().ok_or("mailbox vectors")? {
        let agent = json!({"mbox":row["iri"],"name":"Only A\u{0000}日本語"});
        evidence::observation(
            "person-fallback-inventory",
            &json!({"revision":"isolated-native-person-v2","queried":agent,"associated":{"mbox":[row["iri"]],"name":[agent["name"]]},"unrelated":{"mbox":["mailto:unrelated-person-b@example.org"],"name":["Only B"]},"no_additional_information":true,"query_valid":row["valid"]}),
        );
        let reply = person_request(&client, &base, &agent).await?;
        if row["valid"] == true {
            assert_eq!(reply.status(), StatusCode::OK, "{}", row["id"]);
            let person: Value = reply.json().await.map_err(|e| e.to_string())?;
            assert_eq!(
                person,
                json!({"objectType":"Person","mbox":[row["iri"]],"name":[agent["name"]]}),
                "{}",
                row["id"]
            );
        } else {
            // This records Kiva's chosen optional input-format validation.
            // No invalid input is echoed as an invalid returned Person value.
            assert_eq!(reply.status(), StatusCode::BAD_REQUEST, "{}", row["id"]);
        }
    }
    for (field, value) in [
        (
            "mbox_sha1sum",
            json!("0123456789abcdef0123456789ABCDEF01234567"),
        ),
        ("openid", json!("scheme:")),
        ("openid", json!("urn:example:learner")),
        (
            "account",
            json!({"homePage":"https://example.org/accounts","name":""}),
        ),
        (
            "account",
            json!({"homePage":"https://example.org/accounts","name":"a:b"}),
        ),
        (
            "account",
            json!({"homePage":"https://example.org/accounts:a","name":"b"}),
        ),
        (
            "account",
            json!({"homePage":"https://例え.example/経路","name":"learner"}),
        ),
    ] {
        for name in [None, Some("Only A")] {
            let mut agent = json!({field:value});
            let mut expected = json!({"objectType":"Person",field:[value]});
            if let Some(name) = name {
                agent["name"] = json!(name);
                expected["name"] = json!([name]);
            }
            evidence::observation(
                "person-fallback-inventory",
                &json!({"revision":"isolated-native-person-v2","queried":agent,"associated":expected,"unrelated":{"mbox":["mailto:unrelated-person-b@example.org"],"name":["Only B"]},"no_additional_information":true}),
            );
            let reply = person_request(&client, &base, &agent).await?;
            assert_eq!(reply.status(), StatusCode::OK, "{agent}");
            assert_eq!(
                reply.json::<Value>().await.map_err(|e| e.to_string())?,
                expected
            );
        }
    }
    for agent in [
        json!({"openid":"https://example.org/%ZZ"}),
        json!({"openid":"https://例え.example/"}),
        json!({"account":{"homePage":"http:///path","name":"a"}}),
    ] {
        assert_eq!(
            person_request(&client, &base, &agent).await?.status(),
            StatusCode::BAD_REQUEST
        );
    }
    server.abort();
    Ok(())
}

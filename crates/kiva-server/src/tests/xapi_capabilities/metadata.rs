#![allow(clippy::wildcard_imports)]
use super::*;
use reqwest::Method;

#[path = "canonical_boundaries.rs"]
mod canonical_boundaries;
#[path = "voiding_boundaries.rs"]
mod voiding_boundaries;

#[tokio::test]
async fn memory_xapi_product_canonical_and_authority() -> Result<(), String> {
    canonical(memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_product_canonical_and_authority() -> Result<(), String> {
    canonical(
        postgres_test_state()
            .await?
            .ok_or("PostgreSQL fixture required")?,
    )
    .await
}

async fn canonical(state: AppState) -> Result<(), String> {
    voiding_boundaries::verify(state.clone()).await?;
    let server = Server::start(state).await?;
    canonical_boundaries::verify(&server).await?;
    let fallback = Server::options(
        server.state.clone(),
        json!({"canonical_inventory":false,"activity_hosting":false}),
    )
    .await?;
    canonical_boundaries::verify(&fallback).await?;
    let activity = "https://example.org/capability/activity";
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let mut first = statement(a);
    first["object"] = json!({"id":activity,"definition":{"name":{"en":"Originnal","ja":"名前"},"interactionType":"true-false","correctResponsesPattern":["true"],"extensions":{"https://example.org/owner":{"n":"\u{0000}"}}}});
    first["authority"] = json!({"mbox":"mailto:supplied-a@example.org"});
    first["verb"]["display"] = json!({"en":"Did"});
    server
        .receive(
            server.request(Method::POST, "/statements").json(&first),
            StatusCode::OK,
        )
        .await?;
    let saved = server
        .json(
            server
                .request(Method::GET, "/statements")
                .query(&[("statementId", a.to_string())]),
        )
        .await?;
    assert_eq!(saved["authority"], first["authority"]);
    let mut unauthorized = first.clone();
    unauthorized["id"] = json!(b);
    unauthorized["object"]["definition"]["name"]["en"] = json!("Untrusted correction");
    server
        .receive(
            server
                .request_as(Method::POST, "/statements", "writer-b")
                .json(&unauthorized),
            StatusCode::OK,
        )
        .await?;
    let other = server
        .json(
            server
                .request(Method::GET, "/statements")
                .query(&[("statementId", b.to_string())]),
        )
        .await?;
    assert_ne!(other["authority"], first["authority"]);
    let current = server
        .json(
            server
                .request(Method::GET, "/activities")
                .query(&[("activityId", activity)]),
        )
        .await?;
    assert_eq!(current["definition"]["name"]["en"], "Originnal");
    let mut correction = first.clone();
    correction["id"] = json!(c);
    correction["object"]["definition"]["name"] = json!({"en":"Original","fr":"Nom"});
    correction["object"]["definition"]["correctResponsesPattern"] = json!(["false"]);
    correction["verb"]["display"] = json!({"en":"Completed"});
    server
        .receive(
            server
                .request(Method::POST, "/statements")
                .json(&correction),
            StatusCode::OK,
        )
        .await?;
    let current = server
        .json(
            server
                .request(Method::GET, "/activities")
                .query(&[("activityId", activity)]),
        )
        .await?;
    assert_eq!(
        current["definition"]["name"],
        json!({"en":"Original","ja":"名前","fr":"Nom"})
    );
    assert_eq!(
        current["definition"]["correctResponsesPattern"],
        json!(["true"])
    );
    let canonical = server
        .json(
            server
                .request(Method::GET, "/statements")
                .query(&[
                    ("statementId", a.to_string()),
                    ("format", "canonical".into()),
                ])
                .header("Accept-Language", "fr,en;q=0.5"),
        )
        .await?;
    assert_eq!(
        canonical["object"]["definition"]["name"],
        json!({"fr":"Nom"})
    );
    assert_eq!(canonical["verb"]["display"], json!({"en":"Completed"}));
    assert_eq!(
        canonical["object"]["definition"]["extensions"],
        first["object"]["definition"]["extensions"]
    );
    assert_eq!(
        server
            .json(
                server
                    .request(Method::GET, "/statements")
                    .query(&[("statementId", a.to_string())])
            )
            .await?,
        saved
    );
    let void_id = Uuid::new_v4();
    let mut void = statement(void_id);
    void["verb"] = json!({"id":"http://adlnet.gov/expapi/verbs/voided"});
    void["object"] = json!({"objectType":"StatementRef","id":c});
    server
        .receive(
            server.request(Method::POST, "/statements").json(&void),
            StatusCode::OK,
        )
        .await?;
    let after = server
        .json(
            server
                .request(Method::GET, "/activities")
                .query(&[("activityId", activity)]),
        )
        .await?;
    assert_eq!(
        after["definition"]["name"],
        json!({"en":"Originnal","ja":"名前"})
    );
    let original = server
        .json(
            server
                .request(Method::GET, "/statements")
                .query(&[("voidedStatementId", c.to_string())]),
        )
        .await?;
    assert_eq!(original["object"], correction["object"]);
    let sources = server
        .state
        .xapi
        .statements
        .canonical_sources()
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert!(
        sources
            .iter()
            .find(|s| s.statement["id"] == a.to_string())
            .unwrap()
            .may_update
    );
    assert!(
        !sources
            .iter()
            .find(|s| s.statement["id"] == b.to_string())
            .unwrap()
            .may_update
    );
    evidence::observation(
        "product-capability",
        &json!({"capabilities":["canonical-inventory","trusted-authority","void-metadata-rollback"],"enabled":true,"authorized_corrections":true,"immutable_exact":true}),
    );
    Ok(())
}

#[tokio::test]
async fn memory_xapi_product_host_and_person_directory() -> Result<(), String> {
    directory(memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_product_host_and_person_directory() -> Result<(), String> {
    directory(
        postgres_test_state()
            .await?
            .ok_or("PostgreSQL fixture required")?,
    )
    .await
}

async fn directory(state: AppState) -> Result<(), String> {
    let server = Server::start(state).await?;
    canonical_boundaries::hosted(&server).await?;
    let path = format!("/extensions/activities/{}", Uuid::new_v4());
    let id = format!("{}{path}", server.base);
    let definition = json!({"name":{"en":"Published"},"description":{"en":"Known"}});
    server
        .receive(
            server
                .request_as(Method::PUT, &path, "writer-b")
                .json(&definition),
            StatusCode::FORBIDDEN,
        )
        .await?;
    let reply = server
        .receive(
            server
                .request(Method::PUT, &path)
                .header("If-None-Match", "*")
                .json(&definition),
            StatusCode::NO_CONTENT,
        )
        .await?;
    let etag = reply.headers()["etag"].to_str().unwrap().to_owned();
    assert_eq!(
        server.json(server.request(Method::GET, &path)).await?,
        definition
    );
    server
        .receive(
            server
                .request(Method::PUT, &path)
                .header("If-Match", "\"stale\"")
                .json(&json!({"name":{"en":"Bad"}})),
            StatusCode::PRECONDITION_FAILED,
        )
        .await?;
    server
        .receive(
            server
                .request(Method::PUT, &path)
                .header("If-Match", &etag)
                .json(&json!({"name":{"en":"Corrected","ja":"公開"}})),
            StatusCode::NO_CONTENT,
        )
        .await?;
    let hosted = server.json(server.request(Method::GET, &path)).await?;
    assert_eq!(hosted["description"], definition["description"]);
    assert_eq!(
        server
            .json(
                server
                    .request(Method::GET, "/activities")
                    .query(&[("activityId", &id)])
            )
            .await?["definition"],
        hosted
    );
    let person_path = format!("/extensions/people/{}", Uuid::new_v4());
    let person = json!({"objectType":"Person","name":["Directory name"],"mbox":["mailto:person-a@example.org","mailto:person-alias@example.org"],"mbox_sha1sum":["0123456789012345678901234567890123456789"],"openid":["https://example.org/person/openid"],"account":[{"homePage":"https://example.org/account","name":"person\u{0000}a"}]});
    server
        .receive(
            server.request(Method::PUT, &person_path).json(&person),
            StatusCode::NO_CONTENT,
        )
        .await?;
    for (key, values) in person
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, _)| !["objectType", "name"].contains(&key.as_str()))
    {
        for value in values.as_array().unwrap() {
            let mut agent = json!({});
            agent[key] = value.clone();
            assert_eq!(
                server
                    .json(
                        server
                            .request(Method::GET, "/agents")
                            .query(&[("agent", agent.to_string())])
                    )
                    .await?,
                person
            );
        }
    }
    server
        .receive(
            server
                .request(
                    Method::PUT,
                    &format!("/extensions/people/{}", Uuid::new_v4()),
                )
                .json(&person),
            StatusCode::CONFLICT,
        )
        .await?;
    let sid = Uuid::new_v4();
    let mut contributed = statement(sid);
    contributed["actor"] = json!({"mbox":"mailto:person-a@example.org","name":"Statement name"});
    server
        .receive(
            server
                .request(Method::POST, "/statements")
                .json(&contributed),
            StatusCode::OK,
        )
        .await?;
    let agent = json!({"mbox":"mailto:person-alias@example.org"}).to_string();
    let enriched = server
        .json(
            server
                .request(Method::GET, "/agents")
                .query(&[("agent", &agent)]),
        )
        .await?;
    assert_eq!(
        enriched["name"],
        json!(["Directory name", "Statement name"])
    );
    let mut void = statement(Uuid::new_v4());
    void["verb"] = json!({"id":"http://adlnet.gov/expapi/verbs/voided"});
    void["object"] = json!({"objectType":"StatementRef","id":sid});
    server
        .receive(
            server.request(Method::POST, "/statements").json(&void),
            StatusCode::OK,
        )
        .await?;
    assert_eq!(
        server
            .json(
                server
                    .request(Method::GET, "/agents")
                    .query(&[("agent", &agent)])
            )
            .await?,
        person
    );
    let reply = server
        .receive(server.request(Method::GET, &person_path), StatusCode::OK)
        .await?;
    let person_etag = reply.headers()["etag"].to_str().unwrap().to_owned();
    server
        .receive(
            server
                .request(Method::DELETE, &person_path)
                .header("If-Match", person_etag),
            StatusCode::NO_CONTENT,
        )
        .await?;
    let fallback = server
        .json(
            server
                .request(Method::GET, "/agents")
                .query(&[("agent", &agent)]),
        )
        .await?;
    assert_eq!(
        fallback,
        json!({"objectType":"Person","mbox":["mailto:person-alias@example.org"]})
    );
    let disabled = Server::options(
        server.state.clone(),
        json!({"activity_hosting":false,"canonical_inventory":false}),
    )
    .await?;
    for method in [Method::GET, Method::PUT, Method::DELETE] {
        disabled
            .receive(
                disabled.request(method, &path).json(&definition),
                StatusCode::SERVICE_UNAVAILABLE,
            )
            .await?;
    }
    let about = disabled
        .json(disabled.request(Method::GET, "/about"))
        .await?;
    assert_eq!(
        about["extensions"]["https://kiva.dev/xapi/capabilities"]["activityHosting"],
        false
    );
    evidence::observation(
        "product-capability",
        &json!({"capabilities":["metadata-host","person-enrichment","void-metadata-rollback"],"enabled":true,"conditional_writes":true,"all_ifi_kinds":true}),
    );
    Ok(())
}

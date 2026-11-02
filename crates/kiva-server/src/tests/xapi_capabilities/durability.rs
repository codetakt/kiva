//! Cross-instance conditional updates, tenant separation and cursor lifetime.
#![allow(clippy::wildcard_imports)]
use super::*;
use reqwest::Method;

#[tokio::test]
async fn memory_xapi_product_metadata_concurrency() -> Result<(), String> {
    verify(memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_product_metadata_concurrency() -> Result<(), String> {
    verify(
        postgres_test_state()
            .await?
            .ok_or("PostgreSQL fixture required")?,
    )
    .await
}

pub(super) async fn reopen(state: &AppState) -> Result<AppState, String> {
    use crate::surfaces::xapi::capabilities::CapabilityStore;
    if let CapabilityStore::Postgres { tenant, .. } = &state.xapi.capabilities.store {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(4)
            .connect(&std::env::var("KIVA_POSTGRES_TEST_DATABASE_URL").map_err(|e| e.to_string())?)
            .await
            .map_err(|e| e.to_string())?;
        Ok(AppState::from_repositories(
            StatementRepository::postgres(pool.clone(), tenant.clone())
                .map_err(|e| e.to_string())?,
            DocumentRepository::postgres(pool.clone(), tenant.clone())
                .map_err(|e| e.to_string())?,
            PdsRepository::postgres(pool.clone(), tenant.clone()).map_err(|e| e.to_string())?,
            ConfidentialRepository::postgres(pool, tenant.clone()).map_err(|e| e.to_string())?,
            RuntimeLimits::default(),
            AuthConfig::default(),
        ))
    } else {
        Ok(state.clone())
    }
}

async fn verify(state: AppState) -> Result<(), String> {
    let server = Server::options(
        state,
        json!({"pagination":"snapshot","cursor_ttl_seconds":2}),
    )
    .await?;
    let id = Uuid::new_v4();
    let path = format!("/extensions/activities/{id}");
    let first = server
        .receive(
            server
                .request(Method::PUT, &path)
                .header("If-None-Match", "*")
                .json(&json!({"name":{"en":"first"},"type":"https://example.org/type"})),
            StatusCode::NO_CONTENT,
        )
        .await?;
    let etag = first.headers()["etag"].to_str().unwrap().to_owned();
    let person_id = Uuid::new_v4();
    let person_path = format!("/extensions/people/{person_id}");
    let person = json!({"objectType":"Person","mbox":["mailto:associated@example.org","mailto:alias@example.org"]});
    server
        .receive(
            server
                .request(Method::PUT, &person_path)
                .header("If-None-Match", "*")
                .json(&person),
            StatusCode::NO_CONTENT,
        )
        .await?;
    let record = statement(Uuid::new_v4());
    server
        .receive(
            server.request(Method::POST, "/statements").json(&record),
            StatusCode::OK,
        )
        .await?;
    server
        .receive(
            server
                .request(Method::POST, "/statements")
                .json(&statement(Uuid::new_v4())),
            StatusCode::OK,
        )
        .await?;
    let page = server
        .json(server.request(Method::GET, "/statements?limit=1"))
        .await?;
    let more = page["more"].as_str().unwrap();
    assert!(!more.is_empty());
    let second = Server::options(
        reopen(&server.state).await?,
        json!({"public_base_url":server.base,"pagination":"snapshot"}),
    )
    .await?;
    assert_eq!(
        second
            .json(second.request(Method::GET, &person_path))
            .await?,
        person
    );
    let persisted = second
        .json(
            second
                .request(Method::GET, "/statements")
                .query(&[("statementId", record["id"].as_str().unwrap())]),
        )
        .await?;
    assert_eq!(persisted["object"], record["object"]);
    assert_eq!(
        second.json(second.request(Method::GET, more)).await?["statements"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let public = evidence::received(
        second
            .client
            .get(format!("{}{path}", second.base))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await?;
    assert_eq!(public.status(), StatusCode::OK);
    assert_eq!(public.headers()["etag"], etag);
    assert_eq!(
        public.json::<Value>().await.map_err(|e| e.to_string())?["name"]["en"],
        "first"
    );
    let left = server
        .request(Method::PUT, &path)
        .header("If-Match", &etag)
        .json(&json!({"name":{"en":"left"}}));
    let right = second
        .request(Method::PUT, &path)
        .header("If-Match", &etag)
        .json(&json!({"name":{"en":"right"}}));
    let (left, right) = tokio::join!(left.send(), right.send());
    let left = evidence::received(left.map_err(|e| e.to_string())?).await?;
    let right = evidence::received(right.map_err(|e| e.to_string())?).await?;
    let mut statuses = [left.status().as_u16(), right.status().as_u16()];
    statuses.sort_unstable();
    assert_eq!(statuses, [204, 412]);
    let current = second.json(second.request(Method::GET, &path)).await?;
    assert!(current["name"]["en"] == "left" || current["name"]["en"] == "right");
    assert_eq!(current["type"], "https://example.org/type");
    // Two processes attempt to assign the same previously unknown IFI.
    let a = format!("/extensions/people/{}", Uuid::new_v4());
    let b = format!("/extensions/people/{}", Uuid::new_v4());
    let association = json!({"objectType":"Person","openid":["https://example.org/same-identity"]});
    let (a, b) = tokio::join!(
        server
            .request(Method::PUT, &a)
            .header("If-None-Match", "*")
            .json(&association)
            .send(),
        second
            .request(Method::PUT, &b)
            .header("If-None-Match", "*")
            .json(&association)
            .send()
    );
    let a = evidence::received(a.map_err(|e| e.to_string())?).await?;
    let b = evidence::received(b.map_err(|e| e.to_string())?).await?;
    let mut statuses = [a.status().as_u16(), b.status().as_u16()];
    statuses.sort_unstable();
    assert_eq!(statuses, [204, 409]);
    let foreign = if matches!(
        server.state.xapi.capabilities.store,
        crate::surfaces::xapi::capabilities::CapabilityStore::Postgres { .. }
    ) {
        postgres_test_state()
            .await?
            .ok_or("PostgreSQL fixture required")?
    } else {
        memory_test_state_with_auth(AuthConfig::default())
    };
    let foreign = Server::start(foreign).await?;
    for url in [&path, &person_path, more] {
        foreign
            .receive(foreign.request(Method::GET, url), StatusCode::NOT_FOUND)
            .await?;
    }
    tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
    server
        .receive(server.request(Method::GET, more), StatusCode::NOT_FOUND)
        .await?;
    evidence::observation(
        "product-capability",
        &json!({"capabilities":["metadata-host","person-enrichment","pagination-modes"],"enabled":true,"pagination_mode":"snapshot","new_database_connection":matches!(server.state.xapi.capabilities.store,crate::surfaces::xapi::capabilities::CapabilityStore::Postgres{..}),"tenant_isolation":true,"conditional_concurrent_updates":true,"cursor_expiry":true}),
    );
    Ok(())
}

//! Real mailbox alias queries, bounded live/snapshot continuations and extension routes.
#![allow(clippy::wildcard_imports)]
use super::*;
use reqwest::Method;
use std::collections::BTreeSet;

#[tokio::test]
async fn memory_xapi_product_queries() -> Result<(), String> {
    verify(memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_product_queries() -> Result<(), String> {
    verify(
        postgres_test_state()
            .await?
            .ok_or("PostgreSQL fixture required")?,
    )
    .await
}

async fn verify(state: AppState) -> Result<(), String> {
    alias_shapes(state.clone()).await?;
    inline_cursor(state.clone()).await?;
    for mode in ["live", "snapshot"] {
        let server = Server::options(
            state.clone(),
            json!({"pagination":mode,"legacy_extension_aliases":false}),
        )
        .await?;
        let mailbox = format!("mailto:{}@example.org", Uuid::new_v4());
        let hash =
            ring::digest::digest(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY, mailbox.as_bytes());
        let hash = crate::core::response::hex_lower(hash.as_ref());
        let agent = json!({"mbox":mailbox,"name":"長".repeat(2100)}).to_string();
        let verb = format!("https://example.org/{}", Uuid::new_v4());
        let ids = (0..4).map(|_| Uuid::new_v4()).collect::<Vec<_>>();
        for (index, id) in ids.iter().enumerate() {
            let mut item = statement(*id);
            item["verb"]["id"] = json!(verb);
            item["actor"] = if index % 2 == 0 {
                json!({"mbox":mailbox})
            } else {
                json!({"mbox_sha1sum":hash})
            };
            server
                .receive(
                    server.request(Method::POST, "/statements").json(&item),
                    StatusCode::OK,
                )
                .await?;
        }
        let first = server
            .json(server.request(Method::GET, "/statements").query(&[
                ("agent", agent.as_str()),
                ("verb", verb.as_str()),
                ("ascending", "true"),
                ("limit", "1"),
            ]))
            .await?;
        assert_eq!(first["statements"][0]["id"], json!(ids[0]));
        let more = first["more"].as_str().unwrap();
        assert!(more.len() < 100, "bounded URL independent of filter length");
        server
            .receive(
                server
                    .client
                    .get(format!("{}{more}", server.base))
                    .header("X-Experience-API-Version", "2.0.0"),
                StatusCode::UNAUTHORIZED,
            )
            .await?;
        server
            .receive(
                server.request(Method::GET, &format!("{more}&verb=urn:tampered")),
                StatusCode::NOT_FOUND,
            )
            .await?;
        server
            .receive(
                server.request(
                    Method::GET,
                    &format!("/statements/more?cursor={}", Uuid::new_v4()),
                ),
                StatusCode::NOT_FOUND,
            )
            .await?;
        let mut added = statement(Uuid::new_v4());
        added["verb"]["id"] = json!(verb);
        added["actor"] = json!({"mbox_sha1sum":hash});
        server
            .receive(
                server.request(Method::POST, "/statements").json(&added),
                StatusCode::OK,
            )
            .await?;
        let mut void = statement(Uuid::new_v4());
        void["verb"]["id"] = json!("http://adlnet.gov/expapi/verbs/voided");
        void["object"] = json!({"objectType":"StatementRef","id":ids[1]});
        server
            .receive(
                server.request(Method::POST, "/statements").json(&void),
                StatusCode::OK,
            )
            .await?;
        let mut next = more.to_owned();
        let mut seen = BTreeSet::new();
        while !next.is_empty() {
            let page = server.json(server.request(Method::GET, &next)).await?;
            for item in page["statements"].as_array().unwrap() {
                assert!(seen.insert(item["id"].as_str().unwrap().to_owned()));
            }
            next = page["more"].as_str().unwrap().to_owned();
            assert!(seen.len() < 8);
        }
        let mut expected = BTreeSet::from([ids[2].to_string(), ids[3].to_string()]);
        if mode == "live" {
            expected.insert(added["id"].as_str().unwrap().to_owned());
            expected.insert(void["id"].as_str().unwrap().to_owned());
        }
        assert_eq!(seen, expected);
        // A rebuilt router resolves persistent tokens using current permissions.
        let mut restarted = server.state.clone();
        restarted.auth.statement_visibility =
            crate::core::statement_visibility::StatementVisibility::from_json(r#"{"default":[]}"#)
                .map_err(|e| e.to_string())?;
        let restarted = Server::options(restarted, json!({"pagination":mode})).await?;
        let page = restarted.json(restarted.request(Method::GET, more)).await?;
        assert_eq!(page["statements"], json!([]));
        let about = server.json(server.request(Method::GET, "/about")).await?;
        let profile = &about["extensions"]["https://kiva.dev/xapi/capabilities"];
        assert_eq!(profile["pagination"]["selected"], mode);
        assert_eq!(profile["legacyExtensionAliases"], false);
        let path = "/extensions/__kiva/conformance/rate-limit";
        let reply = evidence::received(
            server
                .request(Method::GET, path)
                .send()
                .await
                .map_err(|e| e.to_string())?,
        )
        .await?;
        assert_eq!(reply.status(), StatusCode::OK);
        let reply = evidence::received(
            server
                .request(Method::GET, "/__kiva/conformance/rate-limit")
                .send()
                .await
                .map_err(|e| e.to_string())?,
        )
        .await?;
        assert_eq!(reply.status(), StatusCode::NOT_FOUND);
        evidence::observation(
            "product-capability",
            &json!({"capabilities":["mailbox-hash-query","pagination-modes","extension-namespace"],"enabled":true,"pagination_mode":mode,"maximum_more_octets":more.len(),"permission_recheck":true,"snapshot_void_suppression":true}),
        );
    }
    Ok(())
}

async fn inline_cursor(state: AppState) -> Result<(), String> {
    let server = Server::options(state, json!({"self_contained_live_cursors":true})).await?;
    let verb = format!("https://example.org/{}", Uuid::new_v4());
    let mut expected = BTreeSet::new();
    for _ in 0..3 {
        let mut item = statement(Uuid::new_v4());
        item["verb"]["id"] = json!(verb);
        expected.insert(item["id"].as_str().unwrap().to_owned());
        server
            .receive(
                server.request(Method::POST, "/statements").json(&item),
                StatusCode::OK,
            )
            .await?;
    }
    let first = server
        .json(
            server
                .request(Method::GET, "/statements")
                .query(&[("verb", verb.as_str()), ("limit", "1")]),
        )
        .await?;
    let mut next = first["more"].as_str().unwrap().to_owned();
    assert!(next.contains("cursorStored="));
    let mut found = BTreeSet::from([first["statements"][0]["id"].as_str().unwrap().to_owned()]);
    while !next.is_empty() {
        let page = server.json(server.request(Method::GET, &next)).await?;
        assert!(found.insert(page["statements"][0]["id"].as_str().unwrap().to_owned()));
        next = page["more"].as_str().unwrap().to_owned();
        assert!(found.len() <= 3);
    }
    assert_eq!(found, expected);
    let agent = json!({"mbox":"mailto:capability@example.org","name":"x".repeat(5000)}).to_string();
    let long = server
        .json(server.request(Method::GET, "/statements").query(&[
            ("verb", verb.as_str()),
            ("limit", "1"),
            ("agent", agent.as_str()),
        ]))
        .await?;
    let next = long["more"].as_str().unwrap();
    assert!(next.contains("?cursor="));
    assert!(next.len() < 100);
    assert_eq!(
        server.json(server.request(Method::GET, next)).await?["statements"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    evidence::observation(
        "product-capability",
        &json!({"capabilities":["pagination-modes"],"enabled":true,"pagination_mode":"live","self_contained_short_and_opaque_long":true}),
    );
    Ok(())
}

async fn alias_shapes(state: AppState) -> Result<(), String> {
    let server = Server::start(state).await?;
    let mailbox = format!("mailto:{}@example.org", Uuid::new_v4());
    let digest = ring::digest::digest(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY, mailbox.as_bytes());
    let hashed = json!({"objectType":"Agent","mbox_sha1sum":crate::core::response::hex_lower(digest.as_ref()).to_ascii_uppercase()});
    let mut expected = BTreeSet::new();
    let mut related = BTreeSet::new();
    for location in ["actor", "object", "member", "instructor", "substatement"] {
        let mut item = statement(Uuid::new_v4());
        match location {
            "actor" => item["actor"] = hashed.clone(),
            "object" => item["object"] = hashed.clone(),
            "member" => item["actor"] = json!({"objectType":"Group","member":[hashed]}),
            "instructor" => item["context"] = json!({"instructor":hashed}),
            "substatement" => {
                item["object"] = json!({"objectType":"SubStatement","actor":hashed,"verb":{"id":"https://example.org/sub"},"object":{"id":"https://example.org/activity"}});
            }
            _ => unreachable!(),
        }
        let id = item["id"].as_str().unwrap().to_owned();
        related.insert(id.clone());
        if !["instructor", "substatement"].contains(&location) {
            expected.insert(id.clone());
        }
        server
            .receive(
                server.request(Method::POST, "/statements").json(&item),
                StatusCode::OK,
            )
            .await?;
        if location == "actor" {
            let mut reference = statement(Uuid::new_v4());
            reference["object"] = json!({"objectType":"StatementRef","id":id});
            server
                .receive(
                    server.request(Method::POST, "/statements").json(&reference),
                    StatusCode::OK,
                )
                .await?;
            expected.insert(reference["id"].as_str().unwrap().to_owned());
            related.insert(reference["id"].as_str().unwrap().to_owned());
        }
    }
    server
        .receive(
            server
                .request(Method::POST, "/statements")
                .json(&statement(Uuid::new_v4())),
            StatusCode::OK,
        )
        .await?;
    for (related_flag, expected) in [("false", expected), ("true", related)] {
        let agent = json!({"mbox":mailbox}).to_string();
        let body = server
            .json(
                server
                    .request(Method::GET, "/statements")
                    .query(&[("agent", agent.as_str()), ("related_agents", related_flag)]),
            )
            .await?;
        let found = body["statements"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            found, expected,
            "mailbox alias at {related_flag} related-agent setting"
        );
    }
    Ok(())
}

//! Metadata rollback applies independently of the canonical inventory switch.
#![allow(clippy::wildcard_imports)]
use super::*;

pub(super) async fn verify(state: AppState) -> Result<(), String> {
    for canonical in [false, true] {
        for rollback in [false, true] {
            let server = Server::options(
                state.clone(),
                json!({"canonical_inventory":canonical,"activity_hosting":false,"person_enrichment":false,
                    "rollback_voided_metadata":rollback}),
            )
            .await?;
            let about = server.json(server.request(Method::GET, "/about")).await?;
            assert_eq!(
                about["extensions"]["https://kiva.dev/xapi/capabilities"]["voidMetadataRollback"],
                rollback,
                "active rollback must be advertised independently of inventory mode"
            );
            for role in ["object", "substatement", "context"] {
                contributions(&server, rollback, role).await?;
            }
        }
    }
    Ok(())
}

async fn contributions(server: &Server, rollback: bool, role: &str) -> Result<(), String> {
    let activity = format!("https://example.org/rollback/{}", Uuid::new_v4());
    let owner = "https://example.org/opaque";
    let mut originals = Vec::new();
    for language in ["en", "ja"] {
        let mut item = statement(Uuid::new_v4());
        let object = json!({"id":activity,"definition":{
            "name":{language:language},"extensions":{owner:{"language":language,"nul":"\u{0000}"}}
        }});
        match role {
            "object" => item["object"] = object,
            "substatement" => {
                item["object"] = json!({"objectType":"SubStatement",
                    "actor":item["actor"],"verb":item["verb"],"object":object});
            }
            _ => item["context"] = json!({"contextActivities":{"parent":[object]}}),
        }
        // Keep the first contribution authoritative in both inventory modes.
        server
            .receive(
                server
                    .request_as(Method::POST, "/statements", "writer-b")
                    .json(&item),
                StatusCode::OK,
            )
            .await?;
        let saved = server
            .json(
                server
                    .request(Method::GET, "/statements")
                    .query(&[("statementId", item["id"].as_str().unwrap())]),
            )
            .await?;
        originals.push(saved);
    }
    let before = server
        .json(
            server
                .request(Method::GET, "/activities")
                .query(&[("activityId", &activity)]),
        )
        .await?;
    assert_eq!(before["definition"]["name"], json!({"en":"en","ja":"ja"}));
    assert_eq!(before["definition"]["extensions"][owner]["language"], "en");
    for (index, original) in originals.iter().enumerate() {
        let mut void = statement(Uuid::new_v4());
        void["verb"] = json!({"id":"http://adlnet.gov/expapi/verbs/voided"});
        void["object"] = json!({"objectType":"StatementRef","id":original["id"]});
        server
            .receive(
                server.request(Method::POST, "/statements").json(&void),
                StatusCode::OK,
            )
            .await?;
        let current = server
            .json(
                server
                    .request(Method::GET, "/activities")
                    .query(&[("activityId", &activity)]),
            )
            .await?;
        if !rollback {
            assert_eq!(current, before, "rollback disabled: {role}");
        } else if index == 0 {
            assert_eq!(
                current["definition"],
                json!({"name":{"ja":"ja"},
                "extensions":{owner:{"language":"ja","nul":"\u{0000}"}}}),
                "surviving contribution: {role}"
            );
        } else {
            assert!(
                current.get("definition").is_none(),
                "no contributors: {role}"
            );
        }
        let exact = server
            .json(
                server
                    .request(Method::GET, "/statements")
                    .query(&[("voidedStatementId", original["id"].as_str().unwrap())]),
            )
            .await?;
        assert_eq!(&exact, original, "voiding preserves exact data: {role}");
    }
    evidence::observation(
        "metadata-rollback-modes",
        &json!({
            "canonical_inventory":server.state.xapi.capabilities.config.canonical_inventory,
            "rollback":rollback,"role":role,"surviving_sources":true,"exact_unchanged":true
        }),
    );
    Ok(())
}

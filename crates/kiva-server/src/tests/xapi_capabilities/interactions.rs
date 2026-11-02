//! Positive and negative optional interaction-definition controls through HTTP.
#![allow(clippy::wildcard_imports)]
use super::*;
use reqwest::Method;

#[tokio::test]
async fn memory_xapi_product_interactions() -> Result<(), String> {
    verify(memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_product_interactions() -> Result<(), String> {
    verify(
        postgres_test_state()
            .await?
            .ok_or("PostgreSQL fixture required")?,
    )
    .await
}

async fn verify(state: AppState) -> Result<(), String> {
    let server = Server::options(state, json!({"detailed_interactions":true})).await?;
    let definitions = vec![
        json!({"interactionType":"choice","choices":[{"id":""},{"id":"a b"}],"correctResponsesPattern":["","a b"]}),
        json!({"interactionType":"matching","source":[{"id":""}],"target":[{"id":""}],"correctResponsesPattern":["[.]"]}),
        json!({"interactionType":"performance","steps":[{"id":""}],"correctResponsesPattern":["[.]response","{broken"]}),
        json!({"interactionType":"true-false","correctResponsesPattern":["true","false"]}),
        json!({"interactionType":"choice","choices":[{"id":"a"},{"id":"b"}],"correctResponsesPattern":["a[,]b"]}),
        json!({"interactionType":"sequencing","choices":[{"id":"a"},{"id":"b"}],"correctResponsesPattern":["b[,]a"]}),
        json!({"interactionType":"likert","scale":[{"id":"a"}],"correctResponsesPattern":["a"]}),
        json!({"interactionType":"matching","source":[{"id":"s"}],"target":[{"id":"t"}],"correctResponsesPattern":["s[.]t"]}),
        json!({"interactionType":"performance","steps":[{"id":"p"}],"correctResponsesPattern":["{order_matters=INVALID}p[.]{lang=INVALID}text","p[.]1[:]9"]}),
        json!({"interactionType":"numeric","correctResponsesPattern":[".1","-1.0e9999[:]1e10000","9007199254740992[:]9007199254740993","[:]4","-1[:]","-0[:]+0"]}),
        json!({"interactionType":"fill-in","correctResponsesPattern":["{lang=INVALID}  ","{unknown=arbitrary}answer", "{broken", ""]}),
        json!({"interactionType":"long-fill-in","correctResponsesPattern":["{case_matters=INVALID}some long text"]}),
        json!({"interactionType":"other","correctResponsesPattern":["anything[,]including[.]delimiters"]}),
    ];
    for definition in definitions {
        for nested in [false, true] {
            let mut item = statement(Uuid::new_v4());
            item["object"]["definition"] = definition.clone();
            // Learner responses are captured faithfully, not rejected for being
            // an incorrect answer to an otherwise valid interaction.
            item["result"] = json!({"response":"wrong answer"});
            if nested {
                let mut child = item.clone();
                child.as_object_mut().unwrap().remove("id");
                child["objectType"] = json!("SubStatement");
                child["timestamp"] = json!("2026-01-01T00:00:00Z");
                item["object"] = child;
            }
            server
                .receive(
                    server.request(Method::POST, "/statements").json(&item),
                    StatusCode::OK,
                )
                .await?;
            let received = server
                .json(
                    server
                        .request(Method::GET, "/statements")
                        .query(&[("statementId", item["id"].as_str().unwrap())]),
                )
                .await?;
            assert_eq!(received["object"], item["object"]);
        }
    }
    for definition in [
        json!({"interactionType":"true-false","correctResponsesPattern":["TRUE"]}),
        json!({"interactionType":"numeric","correctResponsesPattern":["9007199254740993[:]9007199254740992"]}),
        json!({"interactionType":"numeric","correctResponsesPattern":["1e-10001[:]1e-10002"]}),
        json!({"interactionType":"numeric","correctResponsesPattern":["NaN"]}),
        json!({"interactionType":"numeric","correctResponsesPattern":["[:]"]}),
        json!({"interactionType":"choice","choices":[{"id":"a"}],"correctResponsesPattern":["b"]}),
        json!({"interactionType":"sequencing","choices":[{"id":"a"},{"id":"a"}]}),
        json!({"interactionType":"likert","correctResponsesPattern":["a[,]b"]}),
        json!({"interactionType":"matching","correctResponsesPattern":["s[.]t[.]extra"]}),
        json!({"interactionType":"performance","steps":[{"id":"p"}],"correctResponsesPattern":["q[.]response"]}),
        json!({"interactionType":"fill-in","choices":[{"id":"invalid-list"}]}),
    ] {
        for nested in [false, true] {
            let mut invalid = statement(Uuid::new_v4());
            invalid["object"]["definition"] = definition.clone();
            if nested {
                let mut child = invalid.clone();
                child.as_object_mut().unwrap().remove("id");
                child["objectType"] = json!("SubStatement");
                invalid["object"] = child;
            }
            let good = statement(Uuid::new_v4());
            server
                .receive(
                    server
                        .request(Method::POST, "/statements")
                        .json(&json!([good, invalid])),
                    StatusCode::BAD_REQUEST,
                )
                .await?;
            for id in [
                good["id"].as_str().unwrap(),
                invalid["id"].as_str().unwrap(),
            ] {
                server
                    .receive(
                        server
                            .request(Method::GET, "/statements")
                            .query(&[("statementId", id)]),
                        StatusCode::NOT_FOUND,
                    )
                    .await?;
            }
        }
    }
    evidence::observation(
        "product-capability",
        &json!({"capabilities":["interaction-validation"],"enabled":true,"all_interaction_types":10,"root_and_substatement":true,"batch_atomicity":true,"characterstring_parameters":"not validated"}),
    );
    Ok(())
}

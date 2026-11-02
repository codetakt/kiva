//! Canonical metadata must not combine incompatible interaction definitions.
#![allow(clippy::wildcard_imports)]
use super::*;

pub(super) async fn verify(server: &Server) -> Result<(), String> {
    component_languages(server).await?;
    for (original, incoming, expected) in [
        (
            json!({"interactionType":"true-false"}),
            json!({"interactionType":"choice","choices":[{"id":"b"}],"correctResponsesPattern":["b"]}),
            json!({"interactionType":"true-false"}),
        ),
        (
            json!({"interactionType":"choice","choices":[{"id":"a"}]}),
            json!({"interactionType":"choice","choices":[{"id":"b"}],"correctResponsesPattern":["b"]}),
            json!({"interactionType":"choice","choices":[{"id":"a"}]}),
        ),
        (
            json!({"interactionType":"choice","choices":[{"id":"a"}]}),
            json!({"interactionType":"choice","correctResponsesPattern":["b"]}),
            json!({"interactionType":"choice","choices":[{"id":"a"}]}),
        ),
        (
            json!({"interactionType":"choice","correctResponsesPattern":["a"]}),
            json!({"interactionType":"choice","choices":[{"id":"b"}]}),
            json!({"interactionType":"choice","correctResponsesPattern":["a"]}),
        ),
        (
            json!({"interactionType":"choice","choices":[{"id":"a"}]}),
            json!({"interactionType":"choice","correctResponsesPattern":["a"]}),
            json!({"interactionType":"choice","choices":[{"id":"a"}],"correctResponsesPattern":["a"]}),
        ),
    ] {
        let activity = format!("https://example.org/merge/{}", Uuid::new_v4());
        let mut first = statement(Uuid::new_v4());
        first["object"] = json!({"id":activity,"definition":original});
        let mut next = statement(Uuid::new_v4());
        next["object"] = json!({"id":activity,"definition":incoming});
        next["object"]["definition"]["name"] = json!({"en":"An independent label"});
        for item in [&first, &next] {
            server
                .receive(
                    server.request(Method::POST, "/statements").json(item),
                    StatusCode::OK,
                )
                .await?;
        }
        let mut expected = expected;
        expected["name"] = json!({"en":"An independent label"});
        let current = server
            .json(
                server
                    .request(Method::GET, "/activities")
                    .query(&[("activityId", &activity)]),
            )
            .await?;
        assert_eq!(current["definition"], expected, "{original} + {incoming}");
        for item in [&first, &next] {
            let id = item["id"].as_str().unwrap();
            let exact = server
                .json(
                    server
                        .request(Method::GET, "/statements")
                        .query(&[("statementId", id)]),
                )
                .await?;
            assert_eq!(exact["object"], item["object"]);
            let canonical = server
                .json(
                    server
                        .request(Method::GET, "/statements")
                        .query(&[("statementId", id), ("format", "canonical")]),
                )
                .await?;
            let projected = if server.state.xapi.capabilities.config.canonical_inventory {
                &expected
            } else {
                &item["object"]["definition"]
            };
            assert_eq!(&canonical["object"]["definition"], projected);
        }
    }
    Ok(())
}

async fn component_languages(server: &Server) -> Result<(), String> {
    for (kind, field) in [
        ("choice", "choices"),
        ("sequencing", "choices"),
        ("likert", "scale"),
        ("matching", "source"),
        ("matching", "target"),
        ("performance", "steps"),
    ] {
        let activity = format!("https://example.org/component-languages/{}", Uuid::new_v4());
        let mut first = statement(Uuid::new_v4());
        first["object"] = json!({"id":activity,"definition":{"interactionType":kind}});
        first["object"]["definition"][field] = json!([
            {"id":"a","description":{"en":"Original"}}, {"id":"b"}
        ]);
        let mut next = first.clone();
        next["id"] = json!(Uuid::new_v4());
        // Match components by ID, keep established order, and fill an absent
        // description as well as an additional language in an existing map.
        next["object"]["definition"][field] = json!([
            {"id":"b","description":{"ja":"二番目"}},
            {"id":"a","description":{"en":"Untrusted change","ja":"一番目"}}
        ]);
        for item in [&first, &next] {
            server
                .receive(
                    server
                        .request_as(Method::POST, "/statements", "writer-b")
                        .json(item),
                    StatusCode::OK,
                )
                .await?;
        }
        let current = server
            .json(
                server
                    .request(Method::GET, "/activities")
                    .query(&[("activityId", &activity)]),
            )
            .await?;
        assert_eq!(
            current["definition"][field],
            json!([
                {"id":"a","description":{"en":"Original","ja":"一番目"}},
                {"id":"b","description":{"ja":"二番目"}}
            ]),
            "{kind}/{field}"
        );
        for item in [&first, &next] {
            let exact = server
                .json(
                    server
                        .request(Method::GET, "/statements")
                        .query(&[("statementId", item["id"].as_str().unwrap())]),
                )
                .await?;
            assert_eq!(exact["object"], item["object"]);
            if server.state.xapi.capabilities.config.canonical_inventory {
                let canonical = server
                    .json(
                        server
                            .request(Method::GET, "/statements")
                            .query(&[
                                ("statementId", item["id"].as_str().unwrap()),
                                ("format", "canonical"),
                            ])
                            .header("Accept-Language", "ja"),
                    )
                    .await?;
                assert_eq!(
                    canonical["object"]["definition"][field],
                    json!([
                        {"id":"a","description":{"ja":"一番目"}},
                        {"id":"b","description":{"ja":"二番目"}}
                    ])
                );
            }
        }
    }
    Ok(())
}

pub(super) async fn hosted(server: &Server) -> Result<(), String> {
    let path = format!("/extensions/activities/{}", Uuid::new_v4());
    let activity = format!("{}{path}", server.base);
    let mut received = statement(Uuid::new_v4());
    received["object"] = json!({"id":activity,"definition":{"interactionType":"true-false","correctResponsesPattern":["true"],"name":{"en":"Preserved label"}}});
    server
        .receive(
            server.request(Method::POST, "/statements").json(&received),
            StatusCode::OK,
        )
        .await?;
    let definition = json!({"interactionType":"choice","choices":[{"id":"a"}]});
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
    let mut expected = definition.clone();
    expected["name"] = received["object"]["definition"]["name"].clone();
    let current = server
        .json(
            server
                .request(Method::GET, "/activities")
                .query(&[("activityId", &activity)]),
        )
        .await?;
    assert_eq!(current["definition"], expected);
    // Each incoming document is valid by itself; its combination with the
    // established choices is not. Reject without changing body or ETag.
    server
        .receive(
            server
                .request(Method::PUT, &path)
                .header("If-Match", &etag)
                .json(&json!({"interactionType":"choice","correctResponsesPattern":["b"]})),
            StatusCode::CONFLICT,
        )
        .await?;
    let unchanged = server
        .receive(server.request(Method::GET, &path), StatusCode::OK)
        .await?;
    assert_eq!(unchanged.headers()["etag"], etag);
    assert_eq!(
        unchanged.json::<Value>().await.map_err(|e| e.to_string())?,
        definition
    );
    let valid = server
        .receive(
            server
                .request(Method::PUT, &path)
                .header("If-Match", &etag)
                .json(&json!({"interactionType":"choice","correctResponsesPattern":["a"]})),
            StatusCode::NO_CONTENT,
        )
        .await?;
    assert_ne!(valid.headers()["etag"], etag);
    let mut expected = definition;
    expected["correctResponsesPattern"] = json!(["a"]);
    assert_eq!(
        server.json(server.request(Method::GET, &path)).await?,
        expected
    );
    Ok(())
}

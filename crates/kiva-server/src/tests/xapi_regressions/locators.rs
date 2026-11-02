//! Known unreachable/Unicode locators and general nonresolvable identifiers.
#![allow(clippy::wildcard_imports)]
use super::*;

#[tokio::test]
async fn memory_xapi_locator_roles_and_unreachable_identifiers() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_locator_roles_and_unreachable_identifiers() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}
async fn receive(request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let response = evidence::received(
        request
            .header("X-Experience-API-Version", "2.0.0")
            .send()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await?;
    assert_eq!(response.headers()["x-experience-api-version"], "2.0.0");
    if response.url().path().starts_with("/statements") {
        OffsetDateTime::parse(
            response.headers()["x-experience-api-consistent-through"]
                .to_str()
                .map_err(|e| e.to_string())?,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(response)
}
fn value(locator: &str, id: Uuid, object_kind: &str) -> Value {
    let agent =
        json!({"objectType":"Agent","account":{"homePage":locator,"name":"assigned-learner"}});
    let group = json!({"objectType":"Group","account":{"homePage":locator,"name":"assigned-group"},"member":[agent]});
    let activity =
        json!({"id":format!("urn:example:activity:{id}"),"definition":{"moreInfo":locator}});
    let mut value = statement(id);
    value["actor"] = group.clone();
    value["authority"] = group.clone();
    value["verb"] = json!({"id":"urn:example:verb"});
    value["object"] = match object_kind {
        "Agent" => agent.clone(),
        "Group" => group.clone(),
        _ => activity.clone(),
    };
    value["context"] = json!({"instructor":group,"team":group,
        "contextAgents":[{"objectType":"contextAgent","agent":agent,"relevantTypes":["urn:example:relationship"]}],
        "contextGroups":[{"objectType":"contextGroup","group":group,"relevantTypes":["urn:example:relationship"]}],
        "contextActivities":{"parent":[activity],"grouping":[activity],"category":[activity],"other":[activity]}});
    value["attachments"] = json!([{"usageType":"urn:example:evidence","display":{"en":"file"},"contentType":"text/plain","length":0,
        "sha2":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","fileUrl":locator}]);
    value
}
#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
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
    identifier_queries(&client, &base).await?;
    evidence::observation(
        "locator-role-declaration",
        &json!({
        "locators":["https://example.invalid/account","https://example.invalid/学習"],
        "mapping":"https://example.invalid/%E5%AD%A6%E7%BF%92",
        "access":"HTTP(S) resource access semantics; success does not require a DNS or HTTP lookup",
        "identifier_roles":["Verb.id","Activity.id","context relationship relevantTypes","Attachment.usageType"],
        "input_policy":"No new D06 rejection policy; pure-name and unknown classifier contrasts are separate independent oracle controls",
        "authority":"Submitted authority is overwritten. Its resulting URL is checked against the declared authenticated-authority mapping in the authority fixture."}),
    );
    for locator in [
        "https://example.invalid/account",
        "https://example.invalid/学習",
    ] {
        for object_kind in ["Activity", "Agent", "Group"] {
            for nested in [false, true] {
                for post in [false, true] {
                    let id = Uuid::new_v4();
                    let mut input = value(locator, id, object_kind);
                    if nested {
                        input.as_object_mut().ok_or("Statement")?.remove("id");
                        input
                            .as_object_mut()
                            .ok_or("Statement")?
                            .remove("authority");
                        input["objectType"] = json!("SubStatement");
                        let mut root = value(locator, id, "Activity");
                        root["object"] = input;
                        input = root;
                    }
                    let request = if post {
                        client
                            .post(format!("{base}/statements"))
                            .json(&json!([input]))
                    } else {
                        client
                            .put(format!("{base}/statements?statementId={id}"))
                            .json(&input)
                    };
                    let reply = receive(request).await?;
                    assert_eq!(
                        reply.status(),
                        if post {
                            StatusCode::OK
                        } else {
                            StatusCode::NO_CONTENT
                        },
                        "{locator}/{object_kind}/{nested}/{post}"
                    );
                    for format in ["", "&format=exact"] {
                        let reply = receive(
                            client.get(format!("{base}/statements?statementId={id}{format}")),
                        )
                        .await?;
                        assert_eq!(reply.status(), StatusCode::OK);
                        let output: Value = reply.json().await.map_err(|e| e.to_string())?;
                        for prefix in if nested {
                            vec!["", "/object"]
                        } else {
                            vec![""]
                        } {
                            for key in ["actor", "verb", "context", "attachments"] {
                                let pointer = format!("{prefix}/{key}");
                                assert_eq!(
                                    output.pointer(&pointer),
                                    input.pointer(&pointer),
                                    "{pointer}"
                                );
                            }
                        }
                        // A nested Statement's timestamp default is separate
                        // from preserving its Object's locator/identifier.
                        let pointer = if nested { "/object/object" } else { "/object" };
                        assert_eq!(output.pointer(pointer), input.pointer(pointer));
                    }
                }
            }
        }
        for name in ["assigned-learner", "another-learner"] {
            let agent = json!({"account":{"homePage":locator,"name":name}});
            let reply = receive(
                client
                    .get(format!("{base}/agents"))
                    .query(&[("agent", agent.to_string())]),
            )
            .await?;
            assert_eq!(reply.status(), StatusCode::OK);
            let person: Value = reply.json().await.map_err(|e| e.to_string())?;
            assert_eq!(person["objectType"], "Person");
            assert_eq!(person["account"], json!([agent["account"]]));
        }
    }
    server.abort();
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn identifier_queries(client: &reqwest::Client, base: &str) -> Result<(), String> {
    for method in [reqwest::Method::GET, reqwest::Method::HEAD] {
        for selector in ["verb", "activity"] {
            for text in [
                "",
                "no-scheme",
                "https://invalid host/value",
                "https://example.org/%ZZ",
            ] {
                let reply = receive(
                    client
                        .request(method.clone(), format!("{base}/statements"))
                        .query(&[(selector, text)]),
                )
                .await?;
                assert_eq!(
                    reply.status(),
                    StatusCode::BAD_REQUEST,
                    "{method}/{selector}/{text}"
                );
            }
            for text in [
                "https://example.invalid/unreachable",
                "urn:example:unassigned",
            ] {
                let reply = receive(
                    client
                        .request(method.clone(), format!("{base}/statements"))
                        .query(&[(selector, text)]),
                )
                .await?;
                assert_eq!(reply.status(), StatusCode::OK);
                if method == reqwest::Method::HEAD {
                    assert!(reply.bytes().await.map_err(|e| e.to_string())?.is_empty());
                } else {
                    assert_eq!(
                        reply.json::<Value>().await.map_err(|e| e.to_string())?["statements"],
                        json!([])
                    );
                }
            }
        }
        let unassigned = Uuid::new_v4();
        let forms = [
            String::new(),
            "not-a-uuid".into(),
            unassigned.simple().to_string(),
            unassigned.urn().to_string(),
            unassigned.braced().to_string(),
        ];
        for selector in ["statementId", "voidedStatementId", "registration"] {
            for text in &forms {
                let reply = receive(
                    client
                        .request(method.clone(), format!("{base}/statements"))
                        .query(&[(selector, text)]),
                )
                .await?;
                assert_eq!(
                    reply.status(),
                    StatusCode::BAD_REQUEST,
                    "{method}/{selector}/{text}"
                );
            }
            for valid in [
                unassigned.to_string(),
                unassigned.to_string().to_uppercase(),
            ] {
                let reply = receive(
                    client
                        .request(method.clone(), format!("{base}/statements"))
                        .query(&[(selector, valid)]),
                )
                .await?;
                assert_eq!(
                    reply.status(),
                    if selector == "registration" {
                        StatusCode::OK
                    } else {
                        StatusCode::NOT_FOUND
                    }
                );
            }
        }
    }
    // State registration has the same UUID grammar for every method, including
    // list and bulk-delete dispatch. Invalid input must not mutate the valid scope.
    let agent = json!({"mbox":"mailto:uuid-query@example.org"}).to_string();
    let registration = Uuid::new_v4();
    let scope = BTreeMap::from([
        ("activityId", "urn:example:uuid-query".to_owned()),
        ("agent", agent),
        ("registration", registration.to_string()),
        ("stateId", "probe".to_owned()),
    ]);
    assert_eq!(
        receive(
            client
                .put(format!("{base}/activities/state"))
                .query(&scope)
                .json(&json!({"preserved":true}))
        )
        .await?
        .status(),
        StatusCode::NO_CONTENT
    );
    for method in [
        reqwest::Method::GET,
        reqwest::Method::HEAD,
        reqwest::Method::PUT,
        reqwest::Method::POST,
        reqwest::Method::DELETE,
    ] {
        for single in [true, false] {
            if !single && (method == reqwest::Method::PUT || method == reqwest::Method::POST) {
                continue;
            }
            for form in [
                String::new(),
                "not-a-uuid".into(),
                registration.simple().to_string(),
                registration.urn().to_string(),
                registration.braced().to_string(),
            ] {
                let mut q = scope.clone();
                if !single {
                    q.remove("stateId");
                }
                q.insert("registration", form);
                let mut request = client
                    .request(method.clone(), format!("{base}/activities/state"))
                    .query(&q);
                if method == reqwest::Method::PUT || method == reqwest::Method::POST {
                    request = request
                        .header("If-Match", "*")
                        .json(&json!({"changed":true}));
                }
                assert_eq!(
                    receive(request).await?.status(),
                    StatusCode::BAD_REQUEST,
                    "State/{method}/{q:?}"
                );
                let unchanged =
                    receive(client.get(format!("{base}/activities/state")).query(&scope)).await?;
                assert_eq!(unchanged.status(), StatusCode::OK);
                assert_eq!(
                    unchanged.json::<Value>().await.map_err(|e| e.to_string())?,
                    json!({"preserved":true})
                );
            }
        }
    }
    let mut uppercase = scope.clone();
    uppercase.insert("registration", registration.to_string().to_uppercase());
    let same = receive(
        client
            .get(format!("{base}/activities/state"))
            .query(&uppercase),
    )
    .await?;
    assert_eq!(same.status(), StatusCode::OK);
    assert_eq!(
        same.json::<Value>().await.map_err(|e| e.to_string())?,
        json!({"preserved":true})
    );
    let id = Uuid::new_v4();
    for form in [
        id.simple().to_string(),
        id.urn().to_string(),
        id.braced().to_string(),
    ] {
        let mut input = statement(id);
        input.as_object_mut().ok_or("Statement")?.remove("id");
        assert_eq!(
            receive(
                client
                    .put(format!("{base}/statements"))
                    .query(&[("statementId", form)])
                    .json(&input)
            )
            .await?
            .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            receive(client.get(format!("{base}/statements?statementId={id}")))
                .await?
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    Ok(())
}

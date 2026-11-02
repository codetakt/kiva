//! Actual receiver behavior; grading belongs to the separately controlled interpreter.
#![allow(clippy::wildcard_imports)]
use super::*;

#[tokio::test]
async fn memory_xapi_interaction_structure_and_opaque_characterstrings() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_interaction_structure_and_opaque_characterstrings() -> Result<(), String> {
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
    OffsetDateTime::parse(
        response.headers()["x-experience-api-consistent-through"]
            .to_str()
            .map_err(|e| e.to_string())?,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| e.to_string())?;
    Ok(response)
}

fn definition(kind: &str, pattern: &str) -> Value {
    let mut value = json!({"interactionType":kind,"correctResponsesPattern":[pattern]});
    for (name, ids) in match kind {
        "choice" | "sequencing" => vec![("choices", vec!["a", "b"])],
        "likert" => vec![("scale", vec!["a", "b"])],
        "matching" => vec![("source", vec!["s1", "s2"]), ("target", vec!["t1", "t2"])],
        "performance" => vec![("steps", vec!["p1", "p2"])],
        _ => vec![],
    } {
        value[name] = json!(
            ids.iter()
                .map(|id| json!({"id":id,"description":{"en":format!("component {id}")}}))
                .collect::<Vec<_>>()
        );
    }
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
    evidence::observation(
        "interaction-receiver-policy",
        &json!({
        "structural_validation":"interactionType vocabulary/presence, JSON types, component id strings, property names and Language Maps",
        "not_interpreted":"pattern grammar, grading, characterstring parameters (including invalid/unknown values)",
        "producer_advice":"whitespace-free component ids, not a mandatory receiver restriction",
        "source":"IEEE 4.2.4.2 Interaction Types, Characterstring Parameters and Interaction Components tables",
        "implementation":"kiva-xapi::object_validation::validate_activity_definition/validate_interaction_arrays"}),
    );
    let mut vectors = Vec::new();
    for (kind, good, bad) in [
        ("true-false", "true", "TRUE"),
        ("choice", "a[,]b", "a[,]"),
        ("sequencing", "b[,]a", "b[.]a"),
        ("likert", "a", "a[,]b"),
        ("matching", "s1[.]t1[,]s1[.]t2", "s1[.]"),
        (
            "performance",
            "p1[.]text[,]p2[.]1[:]2",
            "p1text[,]p2[.]1[:]2[:]3",
        ),
        ("numeric", "1", "1[:]2[:]3"),
        ("numeric", "1[:]2", "[:]"),
        ("numeric", "[:]2", "not-a-number"),
        ("numeric", "1[:]", "1,2"),
        ("fill-in", "alpha[,]beta", "alpha[,]"),
        ("long-fill-in", "long alpha[,]long beta", "alpha[,]"),
        // Other has no forbidden String grammar. A non-String is tested below.
        ("other", "any string", "{unknown=invalid}[,][.]"),
    ] {
        for (label, pattern) in [("valid", good), ("uninterpreted", bad)] {
            vectors.push((
                format!("{kind}/{label}/{pattern}"),
                definition(kind, pattern),
                json!(pattern),
                true,
            ));
        }
    }
    for kind in ["fill-in", "long-fill-in", "performance"] {
        let items = if kind == "performance" {
            "p1[.]{lang=ja}学習[,]p2[.]{lang=en}Study"
        } else {
            "{lang=ja}学習[,]{lang=en}Study"
        };
        for parameter in [
            "",
            "{order_matters=true}",
            "{order_matters=false}",
            "{order_matters=invalid}",
            "{unknown=invalid}",
        ] {
            let pattern = format!("{parameter}{items}");
            vectors.push((
                format!("{kind}/{parameter}"),
                definition(kind, &pattern),
                json!(pattern),
                true,
            ));
        }
        if kind != "performance" {
            for parameter in [
                "{case_matters=false}",
                "{case_matters=true}",
                "{case_matters=invalid}",
                "{case_matters=true}{order_matters=false}",
            ] {
                let pattern = format!("{parameter}{items}");
                vectors.push((
                    format!("{kind}/{parameter}"),
                    definition(kind, &pattern),
                    json!(pattern),
                    true,
                ));
            }
        }
        for pattern in [
            "{lang=not_a_tag}value",
            "first[,]{case_matters=true}Second",
            "{lang=en}1[:]2",
        ] {
            vectors.push((
                format!("{kind}/uninterpreted-position/{pattern}"),
                definition(kind, pattern),
                json!(pattern),
                true,
            ));
        }
    }
    let long = "学".repeat(251);
    vectors.push((
        "fill-in/251-characters".into(),
        definition("fill-in", &long),
        json!(long),
        true,
    ));
    let mut spaced = definition("choice", "a b[,]b");
    spaced["choices"][0]["id"] = json!("a b");
    vectors.push((
        "producer-advice/whitespace-id".into(),
        spaced,
        json!("a b[,]b"),
        true,
    ));
    for (label, mut def) in [
        ("missing-type", definition("choice", "a")),
        ("invalid-type", definition("invalid", "a")),
        ("null-type", definition("choice", "a")),
        ("key-case", definition("choice", "a")),
        ("patterns-not-array", definition("choice", "a")),
        ("pattern-not-string", definition("other", "a")),
        ("component-null", definition("choice", "a")),
        ("component-missing-id", definition("choice", "a")),
        ("component-id-number", definition("choice", "a")),
        ("component-key-case", definition("choice", "a")),
        ("table-label-is-not-property", definition("choice", "a")),
    ] {
        match label {
            "missing-type" => {
                def.as_object_mut()
                    .ok_or("object")?
                    .remove("interactionType");
            }
            "null-type" => def["interactionType"] = Value::Null,
            "key-case" => {
                def.as_object_mut()
                    .ok_or("object")?
                    .remove("interactionType");
                def["InteractionType"] = json!("choice");
            }
            "patterns-not-array" => def["correctResponsesPattern"] = json!("a"),
            "pattern-not-string" => def["correctResponsesPattern"] = json!([17]),
            "component-null" => def["choices"] = Value::Null,
            "component-missing-id" => def["choices"] = json!([{}]),
            "component-id-number" => def["choices"][0]["id"] = json!(17),
            "component-key-case" => def["choices"][0] = json!({"Id":"a"}),
            "table-label-is-not-property" => def["Interaction Activities"] = json!({}),
            _ => {}
        }
        vectors.push((label.into(), def, json!("a"), false));
    }
    for nested in [false, true] {
        for post in [false, true] {
            for (label, def, answer, accepted) in &vectors {
                let id = Uuid::new_v4();
                let mut value = statement(id);
                value["object"] =
                    json!({"id":format!("https://example.org/interaction/{id}"),"definition":def});
                value["result"] = json!({"response":answer});
                if nested {
                    value.as_object_mut().ok_or("Statement")?.remove("id");
                    value["objectType"] = json!("SubStatement");
                    let mut root = statement(id);
                    root["object"] = value;
                    value = root;
                }
                let request = if post {
                    client
                        .post(format!("{base}/statements"))
                        .json(&json!([value]))
                } else {
                    client
                        .put(format!("{base}/statements?statementId={id}"))
                        .json(&value)
                };
                let response = receive(request).await?;
                assert_eq!(
                    response.status(),
                    if *accepted {
                        if post {
                            StatusCode::OK
                        } else {
                            StatusCode::NO_CONTENT
                        }
                    } else {
                        StatusCode::BAD_REQUEST
                    },
                    "{label}/nested={nested}/post={post}"
                );
                if *accepted {
                    for format in ["exact", "default"] {
                        let url = format!(
                            "{base}/statements?statementId={id}{}",
                            if format == "exact" {
                                "&format=exact"
                            } else {
                                ""
                            }
                        );
                        let read = receive(client.get(url)).await?;
                        assert_eq!(read.status(), StatusCode::OK);
                        let saved: Value = read.json().await.map_err(|e| e.to_string())?;
                        for pointer in if nested {
                            ["/object/object/definition", "/object/result/response"]
                        } else {
                            ["/object/definition", "/result/response"]
                        } {
                            assert_eq!(
                                saved.pointer(pointer),
                                value.pointer(pointer),
                                "{label}/{pointer}/{format}"
                            );
                        }
                    }
                } else {
                    assert!(
                        state
                            .xapi
                            .statements
                            .get(id)
                            .await
                            .map_err(|e| format!("{e:?}"))?
                            .is_none()
                    );
                }
            }
            let id = Uuid::new_v4();
            let mut value = statement(id);
            value["object"] = json!({"id":"https://example.org/duplicate-interaction","definition":definition("choice","a")});
            if nested {
                value.as_object_mut().ok_or("Statement")?.remove("id");
                value["objectType"] = json!("SubStatement");
                let mut root = statement(id);
                root["object"] = value;
                value = root;
            }
            let raw = (if post { json!([value]) } else { value })
                .to_string()
                .replace(
                    "\"interactionType\":\"choice\"",
                    "\"interactionType\":\"choice\",\"interactionType\":\"choice\"",
                );
            let request = if post {
                client.post(format!("{base}/statements"))
            } else {
                client.put(format!("{base}/statements?statementId={id}"))
            };
            assert_eq!(
                receive(request.header("Content-Type", "application/json").body(raw))
                    .await?
                    .status(),
                StatusCode::BAD_REQUEST
            );
            assert!(
                state
                    .xapi
                    .statements
                    .get(id)
                    .await
                    .map_err(|e| format!("{e:?}"))?
                    .is_none()
            );
        }
    }
    server.abort();
    Ok(())
}

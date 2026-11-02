#![allow(clippy::wildcard_imports)]
use super::*;

#[tokio::test]
async fn memory_xapi_array_presence_and_element_partitions() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_array_presence_and_element_partitions() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

struct ArrayField {
    statement: Value,
    pointer: String,
    member: Value,
    required: bool,
    nonempty: bool,
}

fn field(
    statement: Value,
    pointer: &str,
    member: Value,
    required: bool,
    nonempty: bool,
) -> ArrayField {
    ArrayField {
        statement,
        pointer: pointer.to_owned(),
        member,
        required,
        nonempty,
    }
}

// These partitions come from the individual tables: no shared minimum is
// inferred from relevantTypes. Context singleton objects are a separate rule.
#[allow(clippy::too_many_lines)]
fn fields() -> Vec<ArrayField> {
    let base = statement(Uuid::nil());
    let agent = json!({"objectType":"Agent","mbox":"mailto:member@example.org"});
    let group = json!({"objectType":"Group","mbox":"mailto:group@example.org"});
    let activity = json!({"objectType":"Activity","id":"https://example.org/context-activity"});
    let mut result = Vec::new();
    for identified in [false, true] {
        for pointer in [
            "/actor",
            "/object",
            "/context/instructor",
            "/context/team",
            "/context/contextGroups/0/group",
        ] {
            let mut value = base.clone();
            value["context"] = json!({"instructor":agent,"team":group,"contextGroups":[{"objectType":"contextGroup","group":group}]});
            let mut actor = group.clone();
            if !identified {
                actor.as_object_mut().expect("group").remove("mbox");
            }
            actor["member"] = json!([]);
            *value.pointer_mut(pointer).expect("group location") = actor;
            result.push(field(
                value,
                &format!("{pointer}/member"),
                agent.clone(),
                !identified,
                false,
            ));
        }
    }
    for key in ["parent", "grouping", "category", "other"] {
        let mut value = base.clone();
        value["context"] = json!({"contextActivities":{key:[]}});
        result.push(field(
            value,
            &format!("/context/contextActivities/{key}"),
            activity.clone(),
            false,
            false,
        ));
    }
    for (key, kind, actor_key, actor) in [
        ("contextAgents", "contextAgent", "agent", agent),
        ("contextGroups", "contextGroup", "group", group),
    ] {
        let relation = json!({"objectType":kind,actor_key:actor});
        let mut value = base.clone();
        value["context"] = json!({key:[]});
        result.push(field(
            value,
            &format!("/context/{key}"),
            relation.clone(),
            false,
            false,
        ));
        let mut value = base.clone();
        value["context"] = json!({key:[relation]});
        value["context"][key][0]["relevantTypes"] = json!([]);
        result.push(field(
            value,
            &format!("/context/{key}/0/relevantTypes"),
            json!("https://example.org/relationship"),
            false,
            true,
        ));
    }
    for (key, interaction) in [
        ("choices", "choice"),
        ("scale", "likert"),
        ("source", "matching"),
        ("target", "matching"),
        ("steps", "performance"),
        ("correctResponsesPattern", "other"),
    ] {
        let mut value = base.clone();
        value["object"]["definition"] = json!({"type":"http://adlnet.gov/expapi/activities/cmi.interaction","interactionType":interaction,key:[]});
        let member = if key == "correctResponsesPattern" {
            json!("")
        } else {
            json!({"id":"same-id","description":{"en":"component"}})
        };
        result.push(field(
            value,
            &format!("/object/definition/{key}"),
            member,
            false,
            false,
        ));
    }
    let mut value = base;
    value["attachments"] = json!([]);
    result.push(field(value,"/attachments",json!({"usageType":"https://example.org/file","display":{"en":"file"},"contentType":"text/plain","length":0,"sha2":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","fileUrl":"https://example.invalid/not-fetched"}),false,false));
    result
}

async fn receive(request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let reply = evidence::received(
        request
            .header("X-Experience-API-Version", "2.0.0")
            .send()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await?;
    assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
    OffsetDateTime::parse(
        reply.headers()["x-experience-api-consistent-through"]
            .to_str()
            .map_err(|e| e.to_string())?,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|e| e.to_string())?;
    Ok(reply)
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
    for case in fields() {
        for nested in [false, true] {
            for post in [false, true] {
                for (partition, replacement, valid) in [
                    ("absent", None, !case.required),
                    ("null", Some(Value::Null), false),
                    ("empty", Some(json!([])), !case.nonempty),
                    ("one", Some(json!([case.member])), true),
                    (
                        "two-distinct",
                        Some(json!([
                            case.member,
                            if case.nonempty {
                                json!("https://example.org/another-relationship")
                            } else {
                                case.member.clone()
                            }
                        ])),
                        true,
                    ),
                    (
                        "two-including-duplicate",
                        Some(json!([case.member, case.member])),
                        true,
                    ),
                    ("invalid-first", Some(json!([null])), false),
                    ("invalid-later", Some(json!([case.member, null])), false),
                ] {
                    let mut value = case.statement.clone();
                    if let Some(replacement) = replacement {
                        *value.pointer_mut(&case.pointer).ok_or("field")? = replacement;
                    } else {
                        let (parent, key) = case.pointer.rsplit_once('/').ok_or("field parent")?;
                        value
                            .pointer_mut(parent)
                            .and_then(Value::as_object_mut)
                            .ok_or("field object")?
                            .remove(key);
                    }
                    let id = Uuid::new_v4();
                    let pointer = if nested {
                        value.as_object_mut().ok_or("Statement")?.remove("id");
                        value["objectType"] = json!("SubStatement");
                        let mut root = statement(id);
                        root["object"] = value;
                        value = root;
                        format!("/object{}", case.pointer)
                    } else {
                        value["id"] = json!(id);
                        case.pointer.clone()
                    };
                    let context = format!("{pointer}/{partition}/POST={post}");
                    let request = if post {
                        client.post(format!("{base}/statements"))
                    } else {
                        client.put(format!("{base}/statements?statementId={id}"))
                    };
                    let reply =
                        receive(request.json(&if post { json!([value]) } else { value.clone() }))
                            .await?;
                    assert_eq!(
                        reply.status(),
                        if valid {
                            if post {
                                StatusCode::OK
                            } else {
                                StatusCode::NO_CONTENT
                            }
                        } else {
                            StatusCode::BAD_REQUEST
                        },
                        "{context}: {}",
                        reply.text().await.map_err(|e| e.to_string())?
                    );
                    let reply = receive(
                        client.get(format!("{base}/statements?statementId={id}&format=exact")),
                    )
                    .await?;
                    assert_eq!(
                        reply.status(),
                        if valid {
                            StatusCode::OK
                        } else {
                            StatusCode::NOT_FOUND
                        },
                        "{context}"
                    );
                    if valid {
                        let got = reply.json::<Value>().await.map_err(|e| e.to_string())?;
                        assert_eq!(got.pointer(&pointer), value.pointer(&pointer), "{context}");
                    } else {
                        assert!(
                            state
                                .xapi
                                .statements
                                .get(id)
                                .await
                                .map_err(|e| format!("{e:?}"))?
                                .is_none(),
                            "{context}"
                        );
                    }
                }
            }
        }
    }
    server.abort();
    Ok(())
}

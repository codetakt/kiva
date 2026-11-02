#![allow(clippy::wildcard_imports)]
use super::*;

#[tokio::test]
async fn memory_xapi_preserves_duplicate_extension_members() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_preserves_duplicate_extension_members() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

const RAW: &str = r#"{"a":1,"\u0061":2,"nested":{"x":3,"x":4},"array":[null,true,{"duration":"opaque","duration":"kept"}]}"#;
const KEY: &str = "https://example.org/opaque/~value";
const ORDINARY: &str = r#"{"ordinary":null,"numbers":[0,-0,1,-1,16777217,0.10000000149011612,0.33333334,-0.33333334,1.401298464324817e-45,-1.401298464324817e-45,1.1754943508222875e-38,-1.1754943508222875e-38,3.4028234663852886e38,-3.4028234663852886e38,3.4028236e38,1e400,-1e400],"array":[1,false,{"result":{"score":{"scaled":999}},"objectType":"SubStatement"}],"$serde_json::private::Number":"123"}"#;

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
    if reply.url().path().starts_with("/statements") {
        OffsetDateTime::parse(
            reply.headers()["x-experience-api-consistent-through"]
                .to_str()
                .map_err(|e| e.to_string())?,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(reply)
}

async fn submit_raw(
    client: &reqwest::Client,
    base: &str,
    id: Uuid,
    raw: &str,
    post: bool,
    multipart: bool,
) -> Result<reqwest::Response, String> {
    let request = if post {
        client.post(format!("{base}/statements"))
    } else {
        client.put(format!("{base}/statements?statementId={id}"))
    };
    let (media, body) = if multipart {
        (
            "multipart/mixed; boundary=opaque-control",
            format!(
                "--opaque-control\r\nContent-Type: application/json\r\n\r\n{raw}\r\n--opaque-control--\r\n"
            ),
        )
    } else {
        ("application/json", raw.to_owned())
    };
    receive(request.header("Content-Type", media).body(body)).await
}

fn fixtures() -> Vec<(Value, String)> {
    let mut fixtures = Vec::new();
    for scope in ["result", "context"] {
        let mut value = statement(Uuid::nil());
        value[scope] = json!({"extensions":{KEY:"RAW-SENTINEL"}});
        fixtures.push((
            value,
            format!(
                "/{scope}/extensions/{}",
                KEY.replace('~', "~0").replace('/', "~1")
            ),
        ));
    }
    let mut value = statement(Uuid::nil());
    value["object"]["definition"] = json!({"extensions":{KEY:"RAW-SENTINEL"}});
    fixtures.push((
        value,
        format!(
            "/object/definition/extensions/{}",
            KEY.replace('~', "~0").replace('/', "~1")
        ),
    ));
    for key in ["parent", "grouping", "category", "other"] {
        for array in [false, true] {
            let mut value = statement(Uuid::nil());
            let activity = json!({"id":"https://example.org/opaque-activity","definition":{"extensions":{KEY:"RAW-SENTINEL"}}});
            value["context"] =
                json!({"contextActivities":{key:if array {json!([activity])} else {activity}}});
            fixtures.push((
                value,
                format!(
                    "/context/contextActivities/{key}{}/definition/extensions/{}",
                    if array { "/0" } else { "" },
                    KEY.replace('~', "~0").replace('/', "~1")
                ),
            ));
        }
    }
    fixtures
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
    for (template, path) in fixtures() {
        for nested in [false, true] {
            for post in [false, true] {
                for multipart in [false, true] {
                    let id = Uuid::new_v4();
                    let mut value = template.clone();
                    let path = if nested {
                        value.as_object_mut().ok_or("Statement")?.remove("id");
                        value["objectType"] = json!("SubStatement");
                        let mut root = statement(id);
                        root["object"] = value;
                        value = root;
                        format!("/object{path}")
                    } else {
                        value["id"] = json!(id);
                        path.clone()
                    };
                    let verb = format!("https://example.org/opaque-verb/{id}");
                    value["verb"]["id"] = json!(verb);
                    let activity_id = format!("https://example.org/opaque-activity/{id}");
                    if let Some((activity_path, _)) = path.split_once("/definition/extensions/") {
                        value
                            .pointer_mut(activity_path)
                            .ok_or("Activity location")?["id"] = json!(activity_id);
                    }
                    let raw = if post {
                        json!([value]).to_string()
                    } else {
                        value.to_string()
                    }
                    .replace("\"RAW-SENTINEL\"", RAW);
                    let reply = submit_raw(&client, &base, id, &raw, post, multipart).await?;
                    assert_eq!(
                        reply.status(),
                        if post {
                            StatusCode::OK
                        } else {
                            StatusCode::NO_CONTENT
                        },
                        "{path}/{post}/{multipart}: {}",
                        reply.text().await.map_err(|e| e.to_string())?
                    );
                    let record = state
                        .xapi
                        .statements
                        .get(id)
                        .await
                        .map_err(|e| format!("{e:?}"))?
                        .ok_or("stored Statement")?;
                    assert_eq!(
                        record.opaque_values.get(&path).map(String::as_str),
                        Some(RAW)
                    );
                    let ordinary_id = Uuid::new_v4();
                    let ordinary = raw
                        .replace(&id.to_string(), &ordinary_id.to_string())
                        .replace(RAW, ORDINARY);
                    assert!(
                        submit_raw(&client, &base, ordinary_id, &ordinary, post, multipart)
                            .await?
                            .status()
                            .is_success()
                    );
                    let ordinary_reply =
                        receive(client.get(format!("{base}/statements?statementId={ordinary_id}")))
                            .await?;
                    assert_eq!(ordinary_reply.status(), StatusCode::OK);
                    assert!(
                        ordinary_reply
                            .text()
                            .await
                            .map_err(|e| e.to_string())?
                            .contains(ORDINARY)
                    );
                    if path.contains("/definition/extensions/") {
                        let url = reqwest::Url::parse_with_params(
                            &format!("{base}/activities"),
                            [("activityId", &activity_id)],
                        )
                        .map_err(|e| e.to_string())?;
                        let reply = receive(client.get(url)).await?;
                        assert_eq!(reply.status(), StatusCode::OK);
                        assert!(
                            reply.text().await.map_err(|e| e.to_string())?.contains(RAW),
                            "Activity owner value lost: {path}"
                        );
                    }
                    for format in ["exact", "canonical", "ids"] {
                        for attachments in [false, true] {
                            let reply=receive(client.get(format!("{base}/statements?statementId={id}&format={format}&attachments={attachments}"))).await?;
                            assert_eq!(reply.status(), StatusCode::OK);
                            let text = reply.text().await.map_err(|e| e.to_string())?;
                            let projected = if format == "ids" {
                                kiva_xapi::statement_ids_format(record.statement.clone())
                            } else {
                                record.statement.clone()
                            };
                            if projected.pointer(&path).is_some() {
                                assert!(
                                    text.contains(RAW),
                                    "lost member sequence: {path}/{format}/{attachments}: {text}"
                                );
                            } else {
                                assert!(!text.contains(RAW));
                            }
                        }
                    }
                    let url = reqwest::Url::parse_with_params(
                        &format!("{base}/statements"),
                        [("verb", verb.as_str()), ("format", "canonical")],
                    )
                    .map_err(|e| e.to_string())?;
                    let reply = receive(client.get(url)).await?;
                    assert_eq!(reply.status(), StatusCode::OK);
                    assert!(reply.text().await.map_err(|e| e.to_string())?.contains(RAW));
                    let changed = raw.replace(RAW, r#"{"a":2}"#);
                    let conflict =
                        submit_raw(&client, &base, id, &changed, post, multipart).await?;
                    assert!(matches!(
                        conflict.status(),
                        StatusCode::CONFLICT | StatusCode::OK | StatusCode::NO_CONTENT
                    ));
                    let after = state
                        .xapi
                        .statements
                        .get(id)
                        .await
                        .map_err(|e| format!("{e:?}"))?
                        .ok_or("unchanged Statement")?;
                    assert_eq!(after.opaque_values, record.opaque_values);
                    assert_eq!(after.statement, record.statement);
                }
            }
        }
    }
    for post in [false, true] {
        for multipart in [false, true] {
            for suffix in [
                r#", "verb":{"id":"https://example.org/duplicate"}"#,
                r#", "context":{"extensions":{"https://example.org/key":1,"https://example.org/key":2}}"#,
                r#", "context":{"extensions":{"https://example.org/key":{"x":1,}}}"#,
            ] {
                let id = Uuid::new_v4();
                let mut raw = statement(id).to_string();
                raw.pop();
                raw.push_str(suffix);
                raw.push('}');
                if post {
                    raw = format!("[{raw}]");
                }
                assert_eq!(
                    submit_raw(&client, &base, id, &raw, post, multipart)
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
    }
    replay_relations(state, &client, &base).await?;
    owner_values(&client, &base).await?;
    extension_key_boundaries(state, &client, &base).await?;
    batch_and_export(&client, &base).await?;
    document_private_names(&client, &base).await?;
    server.abort();
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn extension_key_boundaries(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
) -> Result<(), String> {
    for (template, path) in fixtures() {
        let (map_path, _) = path.rsplit_once('/').ok_or("extension map path")?;
        for nested in [false, true] {
            for post in [false, true] {
                for (key, accepted) in [
                    ("", false),
                    ("no-scheme", false),
                    ("https://invalid host/value", false),
                    ("https://example.org/%ZZ", false),
                    ("https://example.invalid/unreachable", true),
                    ("urn:example:unassigned", true),
                ] {
                    let id = Uuid::new_v4();
                    let mut value = template.clone();
                    let owner = json!({"timestamp":"not a timestamp","duration":null,"id":false});
                    *value.pointer_mut(map_path).ok_or("extension map")? = json!({key:owner});
                    let mut pointer = if nested {
                        value.as_object_mut().ok_or("Statement")?.remove("id");
                        value["objectType"] = json!("SubStatement");
                        let mut root = statement(id);
                        root["object"] = value;
                        value = root;
                        format!("/object{map_path}")
                    } else {
                        value["id"] = json!(id);
                        map_path.to_owned()
                    };
                    for category in ["parent", "grouping", "category", "other"] {
                        pointer = pointer.replace(
                            &format!("/contextActivities/{category}/definition/"),
                            &format!("/contextActivities/{category}/0/definition/"),
                        );
                    }
                    let sibling_ids = [Uuid::new_v4(), Uuid::new_v4()];
                    let payload = if post {
                        json!([statement(sibling_ids[0]), value, statement(sibling_ids[1])])
                    } else {
                        value
                    };
                    let baseline = if accepted {
                        None
                    } else {
                        Some(
                            state
                                .xapi
                                .statements
                                .settled_snapshot()
                                .await
                                .map_err(|e| format!("{e:?}"))?,
                        )
                    };
                    let reply =
                        submit_raw(client, base, id, &payload.to_string(), post, false).await?;
                    assert_eq!(
                        reply.status(),
                        if !accepted {
                            StatusCode::BAD_REQUEST
                        } else if post {
                            StatusCode::OK
                        } else {
                            StatusCode::NO_CONTENT
                        },
                        "{key}/{pointer}/{post}"
                    );
                    if accepted && post {
                        assert_eq!(
                            reply.json::<Value>().await.map_err(|e| e.to_string())?,
                            json!([sibling_ids[0], id, sibling_ids[1]])
                        );
                    }
                    if let Some(baseline) = baseline {
                        assert_eq!(
                            state
                                .xapi
                                .statements
                                .settled_snapshot()
                                .await
                                .map_err(|e| format!("{e:?}"))?,
                            baseline,
                            "inline handler completion plus repository writer barrier; rejected key cannot commit any batch member"
                        );
                    }
                    let reply = receive(
                        client.get(format!("{base}/statements?statementId={id}&format=exact")),
                    )
                    .await?;
                    assert_eq!(
                        reply.status(),
                        if accepted {
                            StatusCode::OK
                        } else {
                            StatusCode::NOT_FOUND
                        }
                    );
                    if accepted {
                        let stored = reply.json::<Value>().await.map_err(|e| e.to_string())?;
                        assert_eq!(
                            stored.pointer(&pointer),
                            Some(&json!({key:owner})),
                            "only the extension key has an IRI contract"
                        );
                    } else if post {
                        for sibling in sibling_ids {
                            assert_eq!(
                                receive(
                                    client.get(format!("{base}/statements?statementId={sibling}"))
                                )
                                .await?
                                .status(),
                                StatusCode::NOT_FOUND
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

async fn owner_values(client: &reqwest::Client, base: &str) -> Result<(), String> {
    // Independent JSON values at every owner position, not just objects that
    // happen to contain scalar properties. Core lookalikes stay opaque.
    for (template, path) in fixtures() {
        for nested in [false, true] {
            for post in [false, true] {
                for owner in [
                    json!(null),
                    json!(false),
                    json!(3),
                    json!("text"),
                    json!([]),
                    json!([null, false, 3, "text", {}, []]),
                    json!({}),
                    json!({"timestamp":null,"duration":42,"member":"arbitrary","unknown":true}),
                ] {
                    let id = Uuid::new_v4();
                    let mut value = template.clone();
                    *value.pointer_mut(&path).ok_or("owner pointer")? = owner.clone();
                    let pointer = if nested {
                        value.as_object_mut().ok_or("Statement")?.remove("id");
                        value["objectType"] = json!("SubStatement");
                        let mut root = statement(id);
                        root["object"] = value;
                        value = root;
                        format!("/object{path}")
                    } else {
                        value["id"] = json!(id);
                        path.clone()
                    };
                    // Context Activities input may be a singleton; its output
                    // is always an array under the adopted table contract.
                    let mut pointer = pointer;
                    for key in ["parent", "grouping", "category", "other"] {
                        pointer = pointer.replace(
                            &format!("/contextActivities/{key}/definition/"),
                            &format!("/contextActivities/{key}/0/definition/"),
                        );
                    }
                    let raw = if post { json!([value]) } else { value }.to_string();
                    assert_eq!(
                        submit_raw(client, base, id, &raw, post, false)
                            .await?
                            .status(),
                        if post {
                            StatusCode::OK
                        } else {
                            StatusCode::NO_CONTENT
                        }
                    );
                    for format in ["exact", "default"] {
                        let suffix = if format == "exact" {
                            "&format=exact"
                        } else {
                            ""
                        };
                        let reply = receive(
                            client.get(format!("{base}/statements?statementId={id}{suffix}")),
                        )
                        .await?;
                        assert_eq!(reply.status(), StatusCode::OK);
                        let returned = reply.json::<Value>().await.map_err(|e| e.to_string())?;
                        assert_eq!(
                            returned.pointer(&pointer),
                            Some(&owner),
                            "{pointer}/{format}"
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

async fn document_private_names(client: &reqwest::Client, base: &str) -> Result<(), String> {
    for path in [
        "/activities/state",
        "/agents/profile",
        "/activities/profile",
    ] {
        let mut pairs = vec![(
            if path.ends_with("state") {
                "stateId"
            } else {
                "profileId"
            },
            "opaque-document",
        )];
        if path != "/agents/profile" {
            pairs.push(("activityId", "https://example.org/opaque-document"));
        }
        if path != "/activities/profile" {
            pairs.push(("agent", r#"{"mbox":"mailto:opaque-doc@example.org"}"#));
        }
        let url = reqwest::Url::parse_with_params(&format!("{base}{path}"), pairs)
            .map_err(|e| e.to_string())?;
        let reply = receive(
            client
                .put(url.clone())
                .header("Content-Type", "application/json")
                .body(ORDINARY),
        )
        .await?;
        assert_eq!(reply.status(), StatusCode::NO_CONTENT);
        let read = receive(client.get(url.clone())).await?;
        assert_eq!(read.status(), StatusCode::OK);
        let etag = read.headers()["etag"]
            .to_str()
            .map_err(|e| e.to_string())?
            .to_owned();
        let raw = read.text().await.map_err(|e| e.to_string())?;
        let members: BTreeMap<String, Box<serde_json::value::RawValue>> =
            serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        assert_eq!(members["$serde_json::private::Number"].get(), r#""123""#);
        assert_eq!(members["ordinary"].get(), "null");
        assert_eq!(members.len(), 4);
        let reply = receive(
            client
                .post(url.clone())
                .header("Content-Type", "application/json")
                .header("If-Match", etag)
                .body(r#"{"$serde_json::private::RawValue":"user value","new":9007199254740993}"#),
        )
        .await?;
        assert_eq!(reply.status(), StatusCode::NO_CONTENT);
        let read = receive(client.get(url)).await?;
        assert_eq!(read.status(), StatusCode::OK);
        let raw = read.text().await.map_err(|e| e.to_string())?;
        let members: BTreeMap<String, Box<serde_json::value::RawValue>> =
            serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        assert_eq!(members["$serde_json::private::Number"].get(), r#""123""#);
        assert_eq!(
            members["$serde_json::private::RawValue"].get(),
            r#""user value""#
        );
        assert_eq!(members["new"].get(), "9007199254740993");
        assert_eq!(members.len(), 6);
    }
    Ok(())
}

async fn batch_and_export(client: &reqwest::Client, base: &str) -> Result<(), String> {
    let verb = format!("https://example.org/opaque-batch/{}", Uuid::new_v4());
    let raw_values = [
        RAW,
        r#"{"duplicate":null,"duplicate":[0,1]}"#,
        r#"{"opaque_values":"ordinary data","x":"\u0000","x":"\\u0000"}"#,
    ];
    let ids = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    let mut batch = Vec::new();
    for (id, raw) in ids.iter().zip(raw_values) {
        let mut value = statement(*id);
        value["actor"] = crate::surfaces::pds::wire::subject_agent("learner-a");
        value["verb"]["id"] = json!(verb);
        value["result"] = json!({"extensions":{KEY:"RAW-SENTINEL"}});
        batch.push(value.to_string().replace("\"RAW-SENTINEL\"", raw));
    }
    let body = format!("[{}]", batch.join(","));
    assert_eq!(
        submit_raw(client, base, ids[0], &body, true, true)
            .await?
            .status(),
        StatusCode::OK
    );
    for portability in [false, true] {
        let endpoint = if portability {
            "/pds/data-portability-exports"
        } else {
            "/statements"
        };
        let mut url = reqwest::Url::parse_with_params(
            &format!("{base}{endpoint}"),
            if portability {
                vec![("pageSize", "1")]
            } else {
                vec![("limit", "1"), ("verb", &verb), ("ascending", "true")]
            },
        )
        .map_err(|e| e.to_string())?;
        let mut seen = std::collections::BTreeSet::new();
        loop {
            let request = client.get(url.clone());
            let reply = receive(if portability {
                request.bearer_auth("learner-a")
            } else {
                request
            })
            .await?;
            assert_eq!(reply.status(), StatusCode::OK, "{url}");
            let raw = reply.text().await.map_err(|e| e.to_string())?;
            let value: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            let path = if portability {
                "/statements/0/statement/id"
            } else {
                "/statements/0/id"
            };
            let id = Uuid::parse_str(
                value
                    .pointer(path)
                    .and_then(Value::as_str)
                    .ok_or("page id")?,
            )
            .map_err(|e| e.to_string())?;
            let at = ids
                .iter()
                .position(|expected| *expected == id)
                .ok_or("unexpected subject/verb record")?;
            assert!(seen.insert(id), "continuation repeated a record");
            assert!(
                raw.contains(raw_values[at]),
                "continuation/export lost duplicates: {raw}"
            );
            let next = if portability {
                value["nextCursor"].as_str()
            } else {
                value["more"].as_str().filter(|s| !s.is_empty())
            };
            let Some(next) = next else {
                break;
            };
            url = if portability {
                reqwest::Url::parse_with_params(
                    &format!("{base}{endpoint}"),
                    [("pageSize", "1"), ("cursor", next)],
                )
                .map_err(|e| e.to_string())?
            } else {
                url.join(next).map_err(|e| e.to_string())?
            };
        }
        assert_eq!(seen.len(), ids.len());
    }
    Ok(())
}

async fn replay_relations(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
) -> Result<(), String> {
    use crate::surfaces::xapi::storage::take_replay_observations;
    let relations = [
        (r#"{"a":1,"a":2}"#, r#"{"a":2,"\u0061":1}"#, true),
        (r#"{"a":1,"a":2}"#, r#"{"a":2}"#, false),
        (r#"{"x":[1,2]}"#, r#"{"x":[2,1]}"#, false),
        (r#"{"x":{"a":1,"a":2}}"#, r#"{"x":{"a":2,"a":1}}"#, true),
        (
            r#"{"a":1.000e99999999999999999999}"#,
            r#"{"a":10e99999999999999999998}"#,
            true,
        ),
        (
            r#"{"a":9007199254740992}"#,
            r#"{"a":9007199254740993}"#,
            false,
        ),
        (r#"{"a":-0.0}"#, r#"{"a":0}"#, true),
        (r#"{"x":1,"x":1}"#, r#"{"x":1}"#, false),
        (r#"{"x":"e\u0301"}"#, r#"{"x":"\u00e9"}"#, false),
    ];
    for post in [false, true] {
        for multipart in [false, true] {
            for (original, changed, equivalent) in relations {
                let id = Uuid::new_v4();
                let mut value = statement(id);
                value["context"] = json!({"extensions":{KEY:"RAW-SENTINEL"}});
                let template = if post {
                    json!([value]).to_string()
                } else {
                    value.to_string()
                };
                let original_body = template.replace("\"RAW-SENTINEL\"", original);
                let changed_body = template.replace("\"RAW-SENTINEL\"", changed);
                let reply = submit_raw(client, base, id, &original_body, post, multipart).await?;
                assert!(
                    reply.status().is_success(),
                    "{original}: {}",
                    reply.text().await.map_err(|e| e.to_string())?
                );
                let before = state
                    .xapi
                    .statements
                    .get(id)
                    .await
                    .map_err(|e| format!("{e:?}"))?
                    .ok_or("stored")?;
                take_replay_observations();
                let reply = submit_raw(client, base, id, &changed_body, post, multipart).await?;
                let observations = take_replay_observations();
                assert!(
                    observations
                        .iter()
                        .any(|v| v["existing_id"] == json!(id)
                            && v["equivalent"] == json!(equivalent)),
                    "{original} -> {changed}: {observations:?}"
                );
                if equivalent {
                    assert!(reply.status().is_success());
                } else {
                    assert!(matches!(
                        reply.status(),
                        StatusCode::CONFLICT | StatusCode::OK | StatusCode::NO_CONTENT
                    ));
                }
                let after = state
                    .xapi
                    .statements
                    .get(id)
                    .await
                    .map_err(|e| format!("{e:?}"))?
                    .ok_or("preserved")?;
                assert_eq!(before.statement, after.statement);
                assert_eq!(before.opaque_values, after.opaque_values);
                assert_eq!(before.stored_at, after.stored_at);
            }
        }
    }
    Ok(())
}

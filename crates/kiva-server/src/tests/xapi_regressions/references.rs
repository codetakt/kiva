//! Independently timed reference graph and complete actual HTTP page traversal.
#![allow(clippy::wildcard_imports)]
use super::recording::{instant, json, watermark};
use super::*;
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

#[tokio::test]
async fn memory_xapi_reference_graph_targeter_time_and_related_scopes() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_reference_graph_targeter_time_and_related_scopes() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

async fn receive(request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let reply = evidence::received(
        request
            .bearer_auth("authority-a")
            .header("X-Experience-API-Version", "2.0.0")
            .send()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await?;
    watermark(&reply)?;
    Ok(reply)
}

fn record(id: Uuid) -> Value {
    json!({"id":id,"actor":{"mbox":format!("mailto:{id}@example.org")},
        "verb":{"id":format!("https://example.org/verb/{id}")},
        "object":{"id":format!("https://example.org/activity/{id}")},
        "context":{"registration":Uuid::new_v4()}})
}

async fn put(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
    value: &Value,
    at: OffsetDateTime,
) -> Result<Value, String> {
    state.xapi.statements.clock().set(at);
    let id = value["id"].as_str().ok_or("id")?;
    let reply = receive(
        client
            .put(format!("{base}/statements?statementId={id}"))
            .json(value),
    )
    .await?;
    assert_eq!(reply.status(), StatusCode::NO_CONTENT);
    let reply =
        receive(client.get(format!("{base}/statements?statementId={id}&format=exact"))).await?;
    assert_eq!(reply.status(), StatusCode::OK);
    let stored = json(reply).await?;
    assert_eq!(instant(stored["stored"].as_str().ok_or("stored")?)?, at);
    for key in ["id", "actor", "verb", "object", "context"] {
        assert_eq!(stored[key], value[key]);
    }
    Ok(stored)
}

async fn page(client: &reqwest::Client, url: reqwest::Url) -> Result<Value, String> {
    let reply = receive(client.get(url)).await?;
    assert_eq!(reply.status(), StatusCode::OK);
    assert!(
        reply.headers()["content-type"]
            .to_str()
            .map_err(|e| e.to_string())?
            .starts_with("application/json")
    );
    json(reply).await
}

fn query(base: &str, parameters: &BTreeMap<String, String>) -> Result<reqwest::Url, String> {
    reqwest::Url::parse_with_params(&format!("{base}/statements"), parameters)
        .map_err(|e| e.to_string())
}

fn page_ids(value: &Value) -> Result<Vec<Uuid>, String> {
    value["statements"]
        .as_array()
        .ok_or("StatementResult")?
        .iter()
        .map(|v| {
            Uuid::parse_str(v["id"].as_str().ok_or("Statement id")?).map_err(|e| e.to_string())
        })
        .collect()
}

async fn chain(
    client: &reqwest::Client,
    mut url: reqwest::Url,
    expected: &[Uuid],
    maximum: usize,
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    let mut ids = Vec::new();
    loop {
        assert!(seen.insert(url.to_string()), "repeated continuation");
        assert!(
            seen.len() <= expected.len() + 1,
            "bounded traversal exceeded independently known size"
        );
        let current = page(client, url.clone()).await?;
        let current_ids = page_ids(&current)?;
        assert!(
            current_ids.len() <= maximum,
            "configured maximum applies on every page"
        );
        ids.extend(current_ids);
        assert!(
            expected.starts_with(&ids),
            "observed forbidden, duplicated or misordered id: {ids:?} != {expected:?}"
        );
        let more = current["more"]
            .as_str()
            .ok_or("D-03=A terminal representation")?;
        evidence::observation(
            "reference-continuation",
            &json!({"url":url.as_str(),"more":more,"url_octets":more.len(),"state":"live stored/id cursor; no cache or clock expiry","advisory_url_length":true}),
        );
        if more.is_empty() {
            break;
        }
        assert!(
            ids.len() < expected.len(),
            "continuation after exhausted inventory"
        );
        url = url.join(more).map_err(|e| e.to_string())?;
    }
    assert_eq!(ids, expected);
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut configured = state.clone();
    configured.auth = AuthConfig::with_token_verifier(
        true,
        Arc::new(authority::Verifier {
            calls: Arc::clone(&calls),
        }),
    );
    configured.limits.max_statement_page_size = std::num::NonZeroUsize::new(2);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let base = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let router = evidence::router(&configured);
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let epoch = instant("2020-01-01T00:00:00Z")?;
    for (run, ascending) in [true, false].into_iter().enumerate() {
        let epoch = epoch + Duration::seconds(i64::try_from(run).map_err(|e| e.to_string())? * 100);
        let ids: [Uuid; 7] = std::array::from_fn(|_| Uuid::new_v4());
        let mut values = ids.map(record);
        values[1]["object"] = json!({"objectType":"StatementRef","id":ids[0]});
        values[2]["object"] = json!({"objectType":"StatementRef","id":ids[1]});
        values[3]["context"]["statement"] = json!({"objectType":"StatementRef","id":ids[0]});
        let absent = Uuid::new_v4();
        assert!(
            state
                .xapi
                .statements
                .get(absent)
                .await
                .map_err(|e| format!("{e:?}"))?
                .is_none()
        );
        values[4]["object"] = json!({"objectType":"StatementRef","id":absent});
        values[5]["object"] = json!({"objectType":"StatementRef","id":ids[6]});
        values[6]["object"] = json!({"objectType":"StatementRef","id":ids[5]});
        evidence::observation(
            "independent-reference-graph",
            &json!({"ids":ids,"times_seconds":[1,2,3,4,5,6,7],"edges":{"B":"A","C":"B","D_context":"A","E":"absent","U":"V","V":"U"},"missing_id":absent,"page_maximum":2,"authenticated_subject":"subject-a","values":values}),
        );
        for (index, value) in values.iter().enumerate() {
            put(
                state,
                &client,
                &base,
                value,
                epoch + Duration::seconds(i64::try_from(index + 1).map_err(|e| e.to_string())?),
            )
            .await?;
        }
        let filters = [
            ("agent", values[0]["actor"].to_string()),
            (
                "verb",
                values[0]["verb"]["id"].as_str().ok_or("verb")?.to_owned(),
            ),
            (
                "activity",
                values[0]["object"]["id"]
                    .as_str()
                    .ok_or("activity")?
                    .to_owned(),
            ),
            (
                "registration",
                values[0]["context"]["registration"]
                    .as_str()
                    .ok_or("registration")?
                    .to_owned(),
            ),
        ];
        let mut expected = ids[..3].to_vec();
        if !ascending {
            expected.reverse();
        }
        for (field, value) in &filters {
            for limit in [None, Some("0"), Some("1"), Some("2"), Some("3")] {
                let mut q = BTreeMap::from([
                    ((*field).to_string(), value.clone()),
                    ("ascending".into(), ascending.to_string()),
                ]);
                if let Some(limit) = limit {
                    q.insert("limit".into(), limit.into());
                }
                let url = query(&base, &q)?;
                let first = page(&client, url.clone()).await?;
                assert_eq!(
                    page_ids(&first)?.len(),
                    if limit == Some("1") { 1 } else { 2 }
                );
                chain(
                    &client,
                    url,
                    &expected,
                    if limit == Some("1") { 1 } else { 2 },
                )
                .await?;
                q.insert(
                    "since".into(),
                    (epoch + Duration::seconds(2))
                        .format(&time::format_description::well_known::Rfc3339)
                        .map_err(|e| e.to_string())?,
                );
                q.insert(
                    "until".into(),
                    (epoch + Duration::seconds(3))
                        .format(&time::format_description::well_known::Rfc3339)
                        .map_err(|e| e.to_string())?,
                );
                chain(&client, query(&base, &q)?, &[ids[2]], 1).await?;
            }
        }
        // Direct ID lookup never expands the Object graph.
        for format in ["exact", "ids", "canonical"] {
            let reply = receive(client.get(format!(
                "{base}/statements?statementId={}&format={format}&attachments=false",
                ids[1]
            )))
            .await?;
            assert_eq!(reply.status(), StatusCode::OK);
            let returned = json(reply).await?;
            assert_eq!(returned["id"], json!(ids[1]));
            assert_eq!(returned["object"], values[1]["object"]);
            assert!(returned.get("statements").is_none());
        }
        // Void a target that has NOT yet appeared, after the first page. A
        // continuation must suppress it even if an earlier inventory included it.
        let q = BTreeMap::from([
            ("verb".into(), filters[1].1.clone()),
            ("limit".into(), "1".into()),
            ("ascending".into(), ascending.to_string()),
        ]);
        let first_url = query(&base, &q)?;
        let first = page(&client, first_url.clone()).await?;
        assert_eq!(page_ids(&first)?, vec![expected[0]]);
        let next = first_url
            .join(first["more"].as_str().ok_or("more")?)
            .map_err(|e| e.to_string())?;
        let mut added = record(Uuid::new_v4());
        added["verb"] = values[0]["verb"].clone();
        put(state, &client, &base, &added, epoch + Duration::seconds(8)).await?;
        let target = if ascending { ids[2] } else { ids[0] };
        let mut voider = record(Uuid::new_v4());
        voider["verb"]["id"] = json!("http://adlnet.gov/expapi/verbs/voided");
        voider["object"] = json!({"objectType":"StatementRef","id":target});
        put(state, &client, &base, &voider, epoch + Duration::seconds(9)).await?;
        let remaining = if ascending {
            vec![
                ids[1],
                Uuid::parse_str(added["id"].as_str().ok_or("F")?).map_err(|e| e.to_string())?,
                Uuid::parse_str(voider["id"].as_str().ok_or("voider")?)
                    .map_err(|e| e.to_string())?,
            ]
        } else {
            vec![ids[1]]
        };
        chain(&client, next, &remaining, 1).await?;
        let normal = receive(client.get(format!("{base}/statements?statementId={target}"))).await?;
        assert_eq!(normal.status(), StatusCode::NOT_FOUND);
        let archived = receive(client.get(format!(
            "{base}/statements?voidedStatementId={target}&format=exact"
        )))
        .await?;
        assert_eq!(archived.status(), StatusCode::OK);
        assert_eq!(json(archived).await?["id"], json!(target));
    }
    related(state, &client, &base, epoch + Duration::seconds(1000)).await?;
    voiding_sequence(state, &client, &base, epoch + Duration::seconds(2000)).await?;
    evidence::observation(
        "reference-verifier-calls",
        &json!(*calls.lock().map_err(|e| e.to_string())?),
    );
    server.abort();
    Ok(())
}

async fn voiding_sequence(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
    epoch: OffsetDateTime,
) -> Result<(), String> {
    let ids: [Uuid; 3] = std::array::from_fn(|_| Uuid::new_v4());
    let mut values = ids.map(record);
    let actor = values[0]["actor"].clone();
    for index in 1..3 {
        values[index]["actor"] = actor.clone();
        values[index]["verb"]["id"] = json!("http://adlnet.gov/expapi/verbs/voided");
        values[index]["object"] = json!({"objectType":"StatementRef","id":ids[index-1]});
    }
    let mut accepted_c = false;
    for stage in 0_usize..3 {
        let at = epoch + Duration::seconds(i64::try_from(stage).map_err(|e| e.to_string())?);
        if stage < 2 {
            put(state, client, base, &values[stage], at).await?;
        } else {
            state.xapi.statements.clock().set(at);
            let before = state
                .xapi
                .statements
                .settled_snapshot()
                .await
                .map_err(|e| format!("{e:?}"))?;
            let reply = receive(
                client
                    .put(format!("{base}/statements?statementId={}", ids[stage]))
                    .json(&values[stage]),
            )
            .await?;
            assert!([204, 400, 403, 409].contains(&reply.status().as_u16()));
            accepted_c = reply.status() == StatusCode::NO_CONTENT;
            if !accepted_c {
                assert_eq!(
                    state
                        .xapi
                        .statements
                        .settled_snapshot()
                        .await
                        .map_err(|e| format!("{e:?}"))?,
                    before
                );
            }
            evidence::observation(
                "voiding-voider-admission",
                &json!({"id":ids[2],"actual_status":reply.status().as_u16(),"accepted":accepted_c,"rule":"either admitted or refused C cannot make B voided"}),
            );
        }
        for (index, id) in ids.iter().enumerate() {
            let exists = index <= stage && (index < 2 || accepted_c);
            let ordinary = exists && (index != 0 || stage == 0);
            for selector in ["statementId", "voidedStatementId"] {
                let reply =
                    receive(client.get(format!("{base}/statements?{selector}={id}&format=exact")))
                        .await?;
                if selector == "statementId" {
                    assert_eq!(
                        reply.status(),
                        if ordinary {
                            StatusCode::OK
                        } else {
                            StatusCode::NOT_FOUND
                        }
                    );
                } else if !exists {
                    assert_eq!(reply.status(), StatusCode::NOT_FOUND);
                } else if index == 0 && stage > 0 {
                    assert_eq!(reply.status(), StatusCode::OK);
                } else {
                    assert!(
                        [StatusCode::OK, StatusCode::NOT_FOUND].contains(&reply.status()),
                        "nonvoided selector is not required to fabricate voiding"
                    );
                }
                if reply.status() == StatusCode::OK {
                    let returned = json(reply).await?;
                    assert_eq!(returned["id"], json!(id));
                    assert_eq!(returned["object"], values[index]["object"]);
                }
            }
        }
        let expected = if stage == 0 {
            vec![ids[0]]
        } else if accepted_c {
            vec![ids[1], ids[2]]
        } else {
            vec![ids[1]]
        };
        let q = BTreeMap::from([
            ("agent".into(), actor.to_string()),
            ("limit".into(), "1".into()),
            ("ascending".into(), "true".into()),
        ]);
        chain(client, query(base, &q)?, &expected, 1).await?;
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn related(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
    epoch: OffsetDateTime,
) -> Result<(), String> {
    let ids: [Uuid; 6] = std::array::from_fn(|_| Uuid::new_v4());
    let mut values = ids.map(record);
    let activity = format!("urn:example:related:{}", Uuid::new_v4());
    let actor = json!({"mbox":format!("mailto:{}@example.org",Uuid::new_v4())});
    values[0]["context"]["contextActivities"] = json!({"parent":[{"id":activity}]});
    values[1]["context"]["contextAgents"] =
        json!([{"objectType":"contextAgent","agent":actor,"relevantTypes":["urn:example:mentor"]}]);
    values[2]["object"] = json!({"id":activity});
    values[2]["actor"] = actor.clone();
    let sub = json!({"objectType":"SubStatement","actor":actor,"verb":{"id":"urn:example:sub-verb"},"object":{"id":activity}});
    values[3]["object"] = sub;
    values[4]["context"]["statement"] = json!({"objectType":"StatementRef","id":ids[2]});
    values[5]["object"] = json!({"objectType":"StatementRef","id":ids[0]});
    for (index, value) in values.iter().enumerate() {
        // The missing nested timestamp is independently checked after defaulting
        // elsewhere; supply one here so unrelated preservation cannot mask scope.
        let mut value = value.clone();
        if index == 3 {
            value["object"]["timestamp"] = json!("2010-01-01T00:00:00Z");
        }
        put(
            state,
            client,
            base,
            &value,
            epoch + Duration::seconds(i64::try_from(index).map_err(|e| e.to_string())?),
        )
        .await?;
    }
    for related_activities in [false, true] {
        for related_agents in [false, true] {
            for selector in ["activity", "agent"] {
                let q = BTreeMap::from([
                    (
                        selector.into(),
                        if selector == "activity" {
                            activity.clone()
                        } else {
                            actor.to_string()
                        },
                    ),
                    ("related_activities".into(), related_activities.to_string()),
                    ("related_agents".into(), related_agents.to_string()),
                    ("ascending".into(), "true".into()),
                    ("limit".into(), "1".into()),
                ]);
                let expected = match (selector, related_activities, related_agents) {
                    ("activity", true, _) => vec![ids[0], ids[2], ids[3], ids[5]],
                    ("agent", _, true) => vec![ids[1], ids[2], ids[3]],
                    _ => vec![ids[2]],
                };
                chain(client, query(base, &q)?, &expected, 1).await?;
            }
        }
    }
    Ok(())
}

#![allow(clippy::wildcard_imports)]
use super::*;
use crate::core::response::hex_lower;
use crate::surfaces::xapi::model::{
    DocumentAddress, DocumentIdRequirement, DocumentKind, DocumentSelection,
};
use crate::surfaces::xapi::storage::take_replay_observations;
use sha2::{Digest, Sha256};

#[tokio::test(flavor = "current_thread")]
async fn memory_xapi_replay_preserves_all_affected_resources() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_replay_preserves_all_affected_resources() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

const FIRST: &[u8] = b"first binary\x00\xff\x80\r\n--not-the-boundary";
const SECOND: &[u8] = b"second distinct binary\x00\xfe";

fn multipart(value: &Value, post: bool) -> Vec<u8> {
    let value = if post { json!([value]) } else { value.clone() };
    let mut body =
        format!("--replay-boundary\r\nContent-Type: application/json\r\n\r\n{value}\r\n")
            .into_bytes();
    for data in [FIRST, SECOND] {
        body.extend_from_slice(format!("--replay-boundary\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: binary\r\nX-Experience-API-Hash: {}\r\n\r\n",hex_lower(&Sha256::digest(data))).as_bytes());
        body.extend_from_slice(data);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(b"--replay-boundary--\r\n");
    body
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

async fn submit(
    client: &reqwest::Client,
    base: &str,
    value: &Value,
    post: bool,
) -> Result<reqwest::Response, String> {
    let request = if post {
        client.post(format!("{base}/statements"))
    } else {
        client.put(format!(
            "{base}/statements?statementId={}",
            value["id"].as_str().ok_or("ID")?
        ))
    };
    receive(
        request
            .header("Content-Type", "multipart/mixed; boundary=replay-boundary")
            .body(multipart(value, post)),
    )
    .await
}

fn url(base: &str, path: &str, pairs: &[(&str, String)]) -> Result<reqwest::Url, String> {
    reqwest::Url::parse_with_params(&format!("{base}{path}"), pairs).map_err(|e| e.to_string())
}

async fn snapshot(
    state: &AppState,
    client: &reqwest::Client,
    json_urls: &[reqwest::Url],
    docs: &[reqwest::Url],
    attachment_url: &reqwest::Url,
) -> Result<Value, String> {
    let mut observations = Vec::new();
    for url in json_urls {
        let reply = receive(client.get(url.clone())).await?;
        assert_eq!(reply.status(), StatusCode::OK);
        observations.push(reply.json::<Value>().await.map_err(|e| e.to_string())?);
    }
    for url in docs {
        let reply = receive(client.get(url.clone())).await?;
        assert_eq!(reply.status(), StatusCode::OK);
        let headers = reply.headers();
        let metadata = ["etag", "last-modified", "content-type"]
            .map(|name| {
                headers[name]
                    .to_str()
                    .map(str::to_owned)
                    .map_err(|e| e.to_string())
            })
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let body = reply.bytes().await.map_err(|e| e.to_string())?;
        observations.push(json!({"metadata":metadata,"bytes":hex_lower(&body)}));
        let kind = match url.path() {
            "/activities/state" => DocumentKind::State,
            "/activities/profile" => DocumentKind::ActivityProfile,
            _ => DocumentKind::AgentProfile,
        };
        let pairs = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        let DocumentSelection::Single(address) =
            DocumentAddress::from_query(kind, &pairs, DocumentIdRequirement::Required)
                .map_err(|e| format!("{e:?}"))?
        else {
            return Err("single Document".into());
        };
        let record = state
            .xapi
            .documents
            .get(&address)
            .await
            .map_err(|e| format!("{e:?}"))?
            .ok_or("document snapshot")?;
        observations
            .push(json!({"document_updated":record.updated_at.unix_timestamp_nanos().to_string()}));
    }
    let reply = receive(client.get(attachment_url.clone())).await?;
    assert_eq!(reply.status(), StatusCode::OK);
    assert!(
        reply.headers()["content-type"]
            .to_str()
            .map_err(|e| e.to_string())?
            .starts_with("multipart/mixed")
    );
    let body = reply.bytes().await.map_err(|e| e.to_string())?;
    for expected in [FIRST, SECOND] {
        assert!(body.windows(expected.len()).any(|part| part == expected));
    }
    // Independent repository snapshots cover binary attachments and precise
    // recording time without treating a new MIME boundary as a state mutation.
    let mut records = state
        .xapi
        .statements
        .list(None)
        .await
        .map_err(|e| format!("{e:?}"))?;
    records.sort_by_key(|v| v["id"].to_string());
    for value in records {
        let id = Uuid::parse_str(value["id"].as_str().ok_or("ID")?).map_err(|e| e.to_string())?;
        let record = state
            .xapi
            .statements
            .get(id)
            .await
            .map_err(|e| format!("{e:?}"))?
            .ok_or("snapshot record")?;
        let attachments=record.attachments.iter().map(|(hash,attachment)|json!({"hash":hash,"contentType":attachment.content_type,"bytes":hex_lower(&attachment.body)})).collect::<Vec<_>>();
        observations.push(json!({"statement":record.statement,"recorded":record.stored_at.unix_timestamp_nanos().to_string(),"attachments":attachments}));
    }
    Ok(json!(observations))
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    assert!(
        state
            .xapi
            .statements
            .list(None)
            .await
            .map_err(|e| format!("{e:?}"))?
            .is_empty()
    );
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
    let mut original = statement(Uuid::new_v4());
    original["actor"] = json!({"objectType":"Group","mbox":"mailto:group@example.org","name":"original group","member":[{"mbox":"mailto:a@example.org","name":"member A"},{"mbox":"mailto:b@example.org","name":"member B"}]});
    original["verb"] = json!({"id":"https://example.org/replay-verb","display":{"en":"original"}});
    original["object"] = json!({"id":"https://example.org/replay-activity","definition":{"name":{"en":"original"},"description":{"en":"retained"}}});
    original["result"] = json!({"duration":"PT1.011S","extensions":{"https://example.org/result":{"duration":"PT1.011S","array":[1,2]}}});
    original["context"] = json!({"language":"en","extensions":{"https://example.org/context":{"timestamp":"opaque"}}});
    original["timestamp"] = json!("2020-01-01T00:00:00Z");
    original["version"] = json!("2.0.0");
    original["attachments"]=json!([FIRST,SECOND].into_iter().enumerate().map(|(i,data)|json!({"usageType":"https://example.org/evidence","display":{"en":format!("file {i}")},"description":{"en":"original"},"contentType":"application/octet-stream","length":data.len(),"sha2":hex_lower(&Sha256::digest(data))})).collect::<Vec<_>>());
    state.xapi.statements.clock().set(fixed_time());
    assert_eq!(
        submit(&client, &base, &original, false).await?.status(),
        StatusCode::NO_CONTENT
    );
    let id = original["id"].as_str().ok_or("ID")?.to_owned();
    let member = original["actor"]["member"][0].clone();
    let mut docs = Vec::new();
    for path in [
        "/activities/state",
        "/activities/profile",
        "/agents/profile",
    ] {
        let mut pairs = Vec::new();
        if path != "/agents/profile" {
            pairs.push((
                "activityId",
                "https://example.org/replay-activity".to_owned(),
            ));
        }
        if path != "/activities/profile" {
            pairs.push(("agent", member.to_string()));
        }
        pairs.push((
            if path == "/activities/state" {
                "stateId"
            } else {
                "profileId"
            },
            "protected".to_owned(),
        ));
        let target = url(&base, path, &pairs)?;
        assert_eq!(
            receive(
                client
                    .put(target.clone())
                    .header("Content-Type", "text/plain")
                    .body("protected bytes")
            )
            .await?
            .status(),
            StatusCode::NO_CONTENT
        );
        docs.push(target);
    }
    let mut json_urls = vec![
        url(&base, "/statements", &[("statementId", id.clone())])?,
        url(&base, "/statements", &[("format", "canonical".to_owned())])?,
    ];
    for activity in [
        "https://example.org/replay-activity",
        "https://example.org/changed-activity",
    ] {
        json_urls.push(url(
            &base,
            "/activities",
            &[("activityId", activity.to_owned())],
        )?);
    }
    for actor in [
        member,
        original["actor"]["member"][1].clone(),
        json!({"mbox":"mailto:changed@example.org"}),
    ] {
        json_urls.push(url(&base, "/agents", &[("agent", actor.to_string())])?);
    }
    let attachment_url = url(
        &base,
        "/statements",
        &[
            ("statementId", id.clone()),
            ("attachments", "true".to_owned()),
        ],
    )?;
    let before = snapshot(state, &client, &json_urls, &docs, &attachment_url).await?;
    let mutations = vec![
        (
            "/result/extensions/https:~1~1example.org~1result/duration",
            json!("PT1.019S"),
        ),
        ("/actor/mbox", json!("mailto:changed@example.org")),
        ("/actor/name", json!("changed group")),
        (
            "/actor/member",
            json!([{"mbox":"mailto:b@example.org","name":"member B"},{"mbox":"mailto:a@example.org","name":"member A"}]),
        ),
        ("/verb/id", json!("https://example.org/changed-verb")),
        ("/object/id", json!("https://example.org/changed-activity")),
        ("/result/duration", json!("PT1.02S")),
        ("/verb/display", json!({"en":"changed"})),
        ("/object/definition", json!({"name":{"en":"changed"}})),
        ("/timestamp", json!("2021-01-01T00:00:00Z")),
        ("/version", json!("1.0.3")),
        ("/context", json!({"language":"ja"})),
        ("/attachments/0/description", json!({"en":"changed"})),
    ];
    let mut variants = vec![original.clone()];
    for (pointer, value) in mutations {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).ok_or("mutation pointer")? = value;
        variants.push(changed);
    }
    let mut changed = original.clone();
    changed["authority"] = json!({"mbox":"mailto:other-authority@example.org"});
    variants.push(changed);
    let mut changed = original.clone();
    changed["stored"] = json!("2021-01-01T00:00:00Z");
    variants.push(changed);
    state
        .xapi
        .statements
        .clock()
        .set(fixed_time() + Duration::days(1));
    for post in [false, true] {
        for changed in &variants {
            take_replay_observations();
            let reply = submit(&client, &base, changed, post).await?;
            let status = reply.status();
            let comparisons = take_replay_observations();
            if changed.pointer("/result/extensions/https:~1~1example.org~1result/duration")
                != original.pointer("/result/extensions/https:~1~1example.org~1result/duration")
            {
                assert!(
                    !comparisons.is_empty(),
                    "actual strict replay comparator must run"
                );
                assert!(
                    comparisons
                        .iter()
                        .all(|call| call["equivalent"] == json!(false)),
                    "extension-owned duration is opaque, never centisecond-normalized"
                );
            }
            evidence::observation(
                "replay-comparison-advisory",
                &json!({"post":post,"status":status.as_u16(),"compared":!comparisons.is_empty(),"calls":comparisons,"strength":"SHOULD observation; status is not a Boolean comparator oracle"}),
            );
            assert!(
                matches!(status, StatusCode::NO_CONTENT | StatusCode::CONFLICT)
                    || post && status == StatusCode::OK,
                "{status}: {changed}"
            );
            if status == StatusCode::OK {
                assert_eq!(
                    reply.json::<Value>().await.map_err(|e| e.to_string())?,
                    json!([id])
                );
            }
            assert_eq!(
                snapshot(state, &client, &json_urls, &docs, &attachment_url).await?,
                before,
                "POST={post}: {changed}"
            );
        }
    }
    server.abort();
    Ok(())
}

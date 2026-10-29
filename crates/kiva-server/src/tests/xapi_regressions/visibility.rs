//! Independently configured disjoint read sets, across actual HTTP requests.
#![allow(clippy::wildcard_imports)]
use super::recording::{instant, json, watermark};
use super::*;
use crate::core::statement_visibility::StatementVisibility;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

#[tokio::test]
async fn memory_xapi_subject_visibility_and_operation_permissions() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_subject_visibility_and_operation_permissions() -> Result<(), String> {
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
            .header("X-Experience-API-Version", "2.0.0")
            .send()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await?;
    assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
    if reply.url().path().starts_with("/statements") {
        watermark(&reply)?;
    }
    Ok(reply)
}

async fn page(
    reply: reqwest::Response,
    attachments: bool,
) -> Result<(Value, Vec<Vec<u8>>), String> {
    assert_eq!(reply.status(), StatusCode::OK);
    if attachments {
        let mut parts = attachment_boundaries::decoded(reply).await?.into_iter();
        let (_, first) = parts.next().ok_or("JSON part")?;
        Ok((
            serde_json::from_slice(&first).map_err(|e| e.to_string())?,
            parts.map(|(_, bytes)| bytes).collect(),
        ))
    } else {
        Ok((json(reply).await?, Vec::new()))
    }
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    let ids: Vec<Uuid> = (0..6).map(|_| Uuid::new_v4()).collect();
    let allowed = [vec![ids[0], ids[2], ids[4]], vec![ids[1], ids[3], ids[5]]];
    let policy =
        json!({"subjects":{"subject-a":allowed[0], "subject-b":allowed[1], "subject-admin":"all"}});
    let mut configured = state.clone();
    let calls = Arc::new(Mutex::new(Vec::new()));
    configured.auth = AuthConfig::with_token_verifier(
        true,
        Arc::new(authority::Verifier {
            calls: Arc::clone(&calls),
        }),
    );
    configured.auth.statement_visibility =
        StatementVisibility::from_json(&policy.to_string()).map_err(|e| e.to_string())?;
    for invalid in [
        r#"{"default":"al"}"#,
        r#"{"subjects":{"a":["not-a-uuid"]}}"#,
        r#"{"subject":{"a":"all"}}"#,
    ] {
        assert!(StatementVisibility::from_json(invalid).is_err());
    }
    evidence::observation(
        "independent-statement-permissions",
        &json!({
            "policy":policy, "default":"deny all Statement ids when configured without a default",
            "authority":"deployment fixture, assigned before any write or read; not inferred from target output",
            "credential_alias":"authority-a-alias maps to subject-a; raw token text never selects permissions",
            "single_denied":"404, indistinguishable from absent; operation denied remains403",
            "cursor":"position only, current authenticated subject re-evaluated on every continuation",
            "write":"all authenticated writers may submit; unauthorized whole-batch rejection recommendation observed separately"
        }),
    );
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
    let verb = format!("https://example.org/visibility/{}", Uuid::new_v4());
    let mut payloads = BTreeMap::new();
    for (index, id) in ids.iter().enumerate() {
        state.xapi.statements.clock().set(
            instant("2020-01-01T00:00:00Z")?
                + Duration::seconds(i64::try_from(index).map_err(|e| e.to_string())?),
        );
        let raw = format!("private attachment for {id}\0").into_bytes();
        let hash = format!("{:x}", Sha256::digest(&raw));
        let mut value = statement(*id);
        value["verb"]["id"] = json!(verb);
        value["attachments"] = json!([{"usageType":"https://example.org/private","display":{"en":"private"},"contentType":"application/octet-stream","length":raw.len(),"sha2":hash}]);
        let reply = receive(
            client
                .put(format!("{base}/statements?statementId={id}"))
                .bearer_auth("authority-admin")
                .header("Content-Type", "multipart/mixed; boundary=sha2-control")
                .body(attachment_boundaries::encoded(&value, &raw, &hash, false)),
        )
        .await?;
        assert_eq!(reply.status(), StatusCode::NO_CONTENT);
        payloads.insert(id.to_string(), raw);
    }
    for (token, subject_index) in [
        ("authority-a", 0),
        ("authority-a-alias", 0),
        ("authority-b", 1),
    ] {
        let expected: BTreeSet<String> = allowed[subject_index]
            .iter()
            .map(ToString::to_string)
            .collect();
        for attachments in [false, true] {
            for ascending in [false, true] {
                let mut url = reqwest::Url::parse(&format!("{base}/statements"))
                    .map_err(|e| e.to_string())?;
                url.query_pairs_mut()
                    .append_pair("verb", &verb)
                    .append_pair("limit", "1")
                    .append_pair("ascending", if ascending { "true" } else { "false" })
                    .append_pair("attachments", if attachments { "true" } else { "false" });
                let mut seen = BTreeSet::new();
                loop {
                    let (body, raw) = page(
                        receive(client.get(url.clone()).bearer_auth(token)).await?,
                        attachments,
                    )
                    .await?;
                    let values = body["statements"].as_array().ok_or("StatementResult")?;
                    assert_eq!(values.len(), 1, "filter before page limits");
                    let id = values[0]["id"].as_str().ok_or("Statement ID")?;
                    assert!(expected.contains(id));
                    assert!(seen.insert(id.to_owned()));
                    if attachments {
                        assert_eq!(raw, vec![payloads[id].clone()]);
                    }
                    let more = body["more"].as_str().ok_or("more")?;
                    if more.is_empty() {
                        break;
                    }
                    url = reqwest::Url::parse(&base)
                        .map_err(|e| e.to_string())?
                        .join(more)
                        .map_err(|e| e.to_string())?;
                    assert_eq!(
                        receive(client.get(url.clone())).await?.status(),
                        StatusCode::UNAUTHORIZED
                    );
                    assert_eq!(
                        receive(client.get(url.clone()).bearer_auth("refused"))
                            .await?
                            .status(),
                        StatusCode::UNAUTHORIZED
                    );
                    // Reusing A's position with B's valid credential cannot return an A row.
                    let other = if subject_index == 0 {
                        "authority-b"
                    } else {
                        "authority-a"
                    };
                    let other_expected: BTreeSet<String> = allowed[1 - subject_index]
                        .iter()
                        .map(ToString::to_string)
                        .collect();
                    let (cross, cross_raw) = page(
                        receive(client.get(url.clone()).bearer_auth(other)).await?,
                        attachments,
                    )
                    .await?;
                    let cross_values = cross["statements"]
                        .as_array()
                        .ok_or("cross StatementResult")?;
                    for value in cross_values {
                        assert!(other_expected.contains(value["id"].as_str().ok_or("id")?));
                    }
                    if attachments {
                        assert_eq!(
                            cross_raw,
                            cross_values
                                .iter()
                                .map(|v| payloads[v["id"].as_str().expect("id")].clone())
                                .collect::<Vec<_>>()
                        );
                    }
                }
                assert_eq!(seen, expected);
            }
        }
        for id in &ids {
            for selector in ["statementId", "voidedStatementId"] {
                for method in [reqwest::Method::GET, reqwest::Method::HEAD] {
                    let reply = receive(
                        client
                            .request(
                                method.clone(),
                                format!("{base}/statements?{selector}={id}&attachments=true"),
                            )
                            .bearer_auth(token),
                    )
                    .await?;
                    assert_eq!(
                        reply.status(),
                        if expected.contains(&id.to_string()) {
                            StatusCode::OK
                        } else {
                            StatusCode::NOT_FOUND
                        }
                    );
                    if method == reqwest::Method::HEAD {
                        assert!(reply.bytes().await.map_err(|e| e.to_string())?.is_empty());
                    } else if reply.status() == StatusCode::OK {
                        let (_, raw) = page(reply, true).await?;
                        assert_eq!(raw, vec![payloads[&id.to_string()].clone()]);
                    } else {
                        assert!(
                            !reply
                                .text()
                                .await
                                .map_err(|e| e.to_string())?
                                .contains(&id.to_string())
                        );
                    }
                }
            }
        }
    }
    // A verified subject without an explicit grant sees an empty set; this is
    // distinct from lacking the read operation, or invalid authentication.
    let denied = json(
        receive(
            client
                .get(format!("{base}/statements"))
                .bearer_auth("authority-reader"),
        )
        .await?,
    )
    .await?;
    assert_eq!(denied["statements"], json!([]));
    assert_eq!(denied["more"], "");
    operation_matrix(&client, &base).await?;
    let mut voider = statement(Uuid::new_v4());
    voider["verb"]["id"] = json!("http://adlnet.gov/expapi/verbs/voided");
    voider["object"] = json!({"objectType":"StatementRef","id":ids[0]});
    assert_eq!(
        receive(
            client
                .post(format!("{base}/statements"))
                .bearer_auth("authority-admin")
                .json(&voider)
        )
        .await?
        .status(),
        StatusCode::OK
    );
    for token in ["authority-a", "authority-b"] {
        assert_eq!(
            receive(
                client
                    .get(format!("{base}/statements?statementId={}", ids[0]))
                    .bearer_auth(token)
            )
            .await?
            .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            receive(
                client
                    .get(format!("{base}/statements?voidedStatementId={}", ids[0]))
                    .bearer_auth(token)
            )
            .await?
            .status(),
            if token == "authority-a" {
                StatusCode::OK
            } else {
                StatusCode::NOT_FOUND
            }
        );
        let value =
            json(receive(client.get(format!("{base}/statements")).bearer_auth(token)).await?)
                .await?;
        assert!(
            !value["statements"]
                .as_array()
                .ok_or("array")?
                .iter()
                .any(|v| v["id"] == voider["id"] || v["id"] == json!(ids[0]))
        );
    }
    evidence::observation(
        "actual-visibility-authentication-delegations",
        &json!(calls.lock().map_err(|e| e.to_string())?.clone()),
    );
    server.abort();
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn operation_matrix(client: &reqwest::Client, base: &str) -> Result<(), String> {
    let agent = json!({"mbox":"mailto:permission@example.org"}).to_string();
    for resource in [
        "statements",
        "activities",
        "agents",
        "activities/state",
        "activities/profile",
        "agents/profile",
    ] {
        let mut methods = vec![reqwest::Method::GET, reqwest::Method::HEAD];
        if resource == "statements" || resource.contains('/') {
            methods.extend([reqwest::Method::PUT, reqwest::Method::POST]);
        }
        if resource.contains('/') {
            methods.push(reqwest::Method::DELETE);
        }
        for method in methods {
            let write = method != reqwest::Method::GET && method != reqwest::Method::HEAD;
            for token in [
                None,
                Some("refused"),
                Some("authority-reader"),
                Some("authority-admin"),
            ] {
                let id = Uuid::new_v4();
                let mut url = reqwest::Url::parse(&format!("{base}/{resource}"))
                    .map_err(|e| e.to_string())?;
                match resource {
                    "statements" if method != reqwest::Method::POST => {
                        url.query_pairs_mut()
                            .append_pair("statementId", &id.to_string());
                    }
                    "activities" => {
                        url.query_pairs_mut()
                            .append_pair("activityId", "https://example.org/permission");
                    }
                    "agents" => {
                        url.query_pairs_mut().append_pair("agent", &agent);
                    }
                    "activities/state" => {
                        url.query_pairs_mut()
                            .append_pair("activityId", "https://example.org/permission")
                            .append_pair("agent", &agent)
                            .append_pair("stateId", &id.to_string());
                    }
                    "activities/profile" => {
                        url.query_pairs_mut()
                            .append_pair("activityId", "https://example.org/permission")
                            .append_pair("profileId", &id.to_string());
                    }
                    "agents/profile" => {
                        url.query_pairs_mut()
                            .append_pair("agent", &agent)
                            .append_pair("profileId", &id.to_string());
                    }
                    _ => {}
                }
                let mut request = client.request(method.clone(), url);
                if let Some(token) = token {
                    request = request.bearer_auth(token);
                }
                if method == reqwest::Method::PUT || method == reqwest::Method::POST {
                    request = request.json(&if resource == "statements" {
                        statement(id)
                    } else {
                        json!({"permission":"test"})
                    });
                }
                let reply = receive(request).await?;
                let expected = if token.is_none() || token == Some("refused") {
                    StatusCode::UNAUTHORIZED
                } else if write && token == Some("authority-reader") {
                    StatusCode::FORBIDDEN
                } else if !write {
                    if resource == "activities" || resource == "agents" {
                        StatusCode::OK
                    } else {
                        StatusCode::NOT_FOUND
                    }
                } else if resource == "statements" && method == reqwest::Method::POST {
                    StatusCode::OK
                } else {
                    StatusCode::NO_CONTENT
                };
                assert_eq!(reply.status(), expected, "{resource} {method} {token:?}");
                if method == reqwest::Method::HEAD {
                    assert!(reply.bytes().await.map_err(|e| e.to_string())?.is_empty());
                }
            }
        }
    }
    let batch = [statement(Uuid::new_v4()), statement(Uuid::new_v4())];
    let reply = receive(
        client
            .post(format!("{base}/statements"))
            .bearer_auth("authority-reader")
            .json(&batch),
    )
    .await?;
    assert_eq!(reply.status(), StatusCode::FORBIDDEN);
    for value in &batch {
        assert_eq!(
            receive(
                client
                    .get(format!(
                        "{base}/statements?statementId={}",
                        value["id"].as_str().ok_or("id")?
                    ))
                    .bearer_auth("authority-admin")
            )
            .await?
            .status(),
            StatusCode::NOT_FOUND
        );
    }
    evidence::observation(
        "unauthorized-batch-recommendation",
        &json!({"strength":"SHOULD whole-batch rejection","status":403,"no_stored_members":true,"structural_atomicity":"separate mandatory controls"}),
    );
    Ok(())
}

//! State scope mutations must condition the selected list, inside the mutation lock.
#![allow(clippy::wildcard_imports)]
use super::*;

#[tokio::test]
async fn memory_xapi_state_scope_preconditions() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_state_scope_preconditions() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

async fn receive(
    request: reqwest::RequestBuilder,
    expected: StatusCode,
) -> Result<reqwest::Response, String> {
    let response = HttpProbe::receive(request).await?;
    assert_eq!(response.status(), expected, "{}", response.url());
    Ok(response)
}

fn scope(http: &HttpProbe, index: usize) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(&format!("{}/activities/state", http.base))
        .map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair(
            "activityId",
            &format!("https://example.org/scope-condition/{index}"),
        )
        .append_pair("agent", r#"{"mbox":"mailto:scope-condition@example.org"}"#);
    Ok(url)
}

async fn snapshot(
    http: &HttpProbe,
    url: &reqwest::Url,
    expected: &Value,
) -> Result<String, String> {
    let response = receive(http.client.get(url.clone()), StatusCode::OK).await?;
    let tag = response.headers()["etag"]
        .to_str()
        .map_err(|e| e.to_string())?
        .to_owned();
    assert_eq!(
        response.json::<Value>().await.map_err(|e| e.to_string())?,
        *expected
    );
    Ok(tag)
}

#[allow(clippy::too_many_lines)]
async fn verify(app: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(app).await?;
    evidence::observation(
        "state-scope-preconditions",
        &json!({
            "fixture":"fresh scopes; only this test writes; stale tag derived from captured list tag",
            "oracle":"If-Match/If-None-Match select the GET representation of the same URI, not each member",
            "scope":"empty and populated; registration-specific and all registrations; repeated fields; rejected deletion preserves every document body/tag and neighboring scope",
            "policy":"lists have no known modification date; strong comparison on DELETE, weak tags do not match"
        }),
    );
    for populated in [true, false] {
        for registration in [false, true] {
            for case in 0..16 {
                let index = case + usize::from(registration) * 16 + usize::from(populated) * 32;
                let mut selected = scope(&http, index)?;
                if registration {
                    selected
                        .query_pairs_mut()
                        .append_pair("registration", "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa");
                }
                let mut documents = Vec::new();
                if populated {
                    // Repeated IDs across registrations must select one list ID,
                    // while scope deletion still removes every selected record.
                    for (id, extra_registration) in
                        [("日本語\"\\", false), ("a", false), ("a", true)]
                    {
                        let mut url = selected.clone();
                        url.query_pairs_mut().append_pair("stateId", id);
                        if extra_registration {
                            if registration {
                                continue;
                            }
                            url.query_pairs_mut().append_pair(
                                "registration",
                                "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
                            );
                        }
                        receive(
                            http.client.put(url.clone()).json(&json!({"id":id})),
                            StatusCode::NO_CONTENT,
                        )
                        .await?;
                        let body = json!({"id":id});
                        let tag = snapshot(&http, &url, &body).await?;
                        documents.push((url, body, tag));
                    }
                }
                let mut neighbor = if registration {
                    scope(&http, index)?
                } else {
                    scope(&http, index + 1000)?
                };
                neighbor
                    .query_pairs_mut()
                    .append_pair("stateId", "neighbor");
                receive(
                    http.client
                        .put(neighbor.clone())
                        .json(&json!({"keep":true})),
                    StatusCode::NO_CONTENT,
                )
                .await?;
                let neighbor_tag = snapshot(&http, &neighbor, &json!({"keep":true})).await?;
                let ids = if populated {
                    json!(["a", "日本語\"\\"])
                } else {
                    json!([])
                };
                let current = snapshot(&http, &selected, &ids).await?;
                let stale = format!("{}!\"", &current[..current.len() - 1]);
                let weak = format!("W/{current}");
                let (headers, expected) = match case {
                    // This was incorrectly accepted and deleted all member records.
                    0 => (
                        vec![("If-None-Match", current.clone())],
                        StatusCode::PRECONDITION_FAILED,
                    ),
                    1 => (vec![("If-Match", current.clone())], StatusCode::NO_CONTENT),
                    2 => (
                        vec![("If-Match", stale.clone())],
                        StatusCode::PRECONDITION_FAILED,
                    ),
                    3 => (
                        vec![("If-Match", weak.clone())],
                        StatusCode::PRECONDITION_FAILED,
                    ),
                    4 => (vec![("If-Match", "*".into())], StatusCode::NO_CONTENT),
                    5 => (
                        vec![("If-Match", format!("{stale}, {current}"))],
                        StatusCode::NO_CONTENT,
                    ),
                    6 => (
                        vec![("If-Match", stale.clone()), ("iF-mAtCh", current.clone())],
                        StatusCode::NO_CONTENT,
                    ),
                    7 => (vec![("If-None-Match", stale)], StatusCode::NO_CONTENT),
                    8 => (vec![("If-None-Match", weak)], StatusCode::NO_CONTENT),
                    9 => (
                        vec![("If-None-Match", "*".into())],
                        StatusCode::PRECONDITION_FAILED,
                    ),
                    10 => (
                        vec![("If-None-Match", format!("\"unused\", {current}"))],
                        StatusCode::PRECONDITION_FAILED,
                    ),
                    11 => (
                        vec![
                            ("If-None-Match", "\"unused\"".into()),
                            ("IF-NONE-MATCH", current.clone()),
                        ],
                        StatusCode::PRECONDITION_FAILED,
                    ),
                    12 => (
                        vec![(
                            "If-Unmodified-Since",
                            "Thu, 01 Jan 1970 00:00:00 GMT".into(),
                        )],
                        StatusCode::NO_CONTENT,
                    ),
                    13 => (
                        vec![
                            ("If-Modified-Since", "Thu, 01 Jan 1970 00:00:00 GMT".into()),
                            ("If-None-Match", current.clone()),
                        ],
                        StatusCode::PRECONDITION_FAILED,
                    ),
                    14 => (
                        vec![("If-Match", current.clone()), ("If-None-Match", "*".into())],
                        StatusCode::BAD_REQUEST,
                    ),
                    _ => (vec![], StatusCode::NO_CONTENT),
                };
                let mut request = http.client.delete(selected.clone());
                for (name, value) in headers {
                    request = request.header(name, value);
                }
                receive(request, expected).await?;
                if expected == StatusCode::NO_CONTENT {
                    snapshot(&http, &selected, &json!([])).await?;
                    for (url, _, _) in documents {
                        receive(http.client.get(url), StatusCode::NOT_FOUND).await?;
                    }
                } else {
                    assert_eq!(snapshot(&http, &selected, &ids).await?, current);
                    for (url, body, tag) in documents {
                        assert_eq!(snapshot(&http, &url, &body).await?, tag);
                    }
                }
                assert_eq!(
                    snapshot(&http, &neighbor, &json!({"keep":true})).await?,
                    neighbor_tag
                );
            }
        }
    }
    Ok(())
}

//! Conditional reads select the ID-list representation, including an empty list.
#![allow(clippy::wildcard_imports)]
use super::*;

#[tokio::test]
async fn memory_xapi_document_list_preconditions() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_document_list_preconditions() -> Result<(), String> {
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
    let reply = HttpProbe::receive(request).await?;
    assert_eq!(reply.status(), expected, "{}", reply.url());
    Ok(reply)
}

async fn list(http: &HttpProbe, url: &reqwest::Url, expected: &Value) -> Result<String, String> {
    let reply = receive(http.client.get(url.clone()), StatusCode::OK).await?;
    let etag = reply.headers()["etag"]
        .to_str()
        .map_err(|e| e.to_string())?
        .to_owned();
    assert!(etag.starts_with('"') && etag.ends_with('"'));
    // The list has no recorded modification history (deletions included).
    // Do not invent a date from the maximum timestamp of surviving documents.
    assert!(!reply.headers().contains_key("last-modified"));
    assert_eq!(
        reply.json::<Value>().await.map_err(|e| e.to_string())?,
        *expected
    );
    Ok(etag)
}

#[allow(clippy::too_many_lines)]
async fn matrix(http: &HttpProbe, url: &reqwest::Url, expected: &Value) -> Result<String, String> {
    let current = list(http, url, expected).await?;
    let stale = format!("{}!\"", &current[..current.len() - 1]);
    let weak = format!("W/{current}");
    // Both one-field lists and repeated field lines must retain every tag.
    let cases = [
        (vec![], StatusCode::OK),
        (vec![("If-Match", current.clone())], StatusCode::OK),
        (
            vec![("If-Match", stale.clone())],
            StatusCode::PRECONDITION_FAILED,
        ),
        (
            vec![("If-Match", weak.clone())],
            StatusCode::PRECONDITION_FAILED,
        ),
        (vec![("If-Match", "*".into())], StatusCode::OK),
        (
            vec![("If-Match", format!("{stale}, {current}"))],
            StatusCode::OK,
        ),
        (
            vec![("If-Match", stale.clone()), ("iF-mAtCh", current.clone())],
            StatusCode::OK,
        ),
        (
            vec![("If-None-Match", current.clone())],
            StatusCode::NOT_MODIFIED,
        ),
        (vec![("If-None-Match", weak)], StatusCode::NOT_MODIFIED),
        (
            vec![("If-None-Match", "*".into())],
            StatusCode::NOT_MODIFIED,
        ),
        (vec![("If-None-Match", stale.clone())], StatusCode::OK),
        (
            vec![("If-None-Match", format!("{stale}, {current}"))],
            StatusCode::NOT_MODIFIED,
        ),
        (
            vec![
                ("If-None-Match", stale.clone()),
                ("if-none-match", current.clone()),
            ],
            StatusCode::NOT_MODIFIED,
        ),
        // Date conditions have no known list modification time to compare.
        (
            vec![("If-Modified-Since", "Thu, 01 Jan 1970 00:00:00 GMT".into())],
            StatusCode::OK,
        ),
        (
            vec![(
                "If-Unmodified-Since",
                "Thu, 01 Jan 1970 00:00:00 GMT".into(),
            )],
            StatusCode::OK,
        ),
        (
            vec![
                ("If-None-Match", current.clone()),
                ("If-Modified-Since", "Thu, 01 Jan 1970 00:00:00 GMT".into()),
            ],
            StatusCode::NOT_MODIFIED,
        ),
        (
            vec![
                ("If-None-Match", stale),
                ("If-Modified-Since", "Thu, 01 Jan 2099 00:00:00 GMT".into()),
            ],
            StatusCode::OK,
        ),
        // D-04 still rejects the explicitly undefined combinations.
        (
            vec![("If-Match", current.clone()), ("If-None-Match", "*".into())],
            StatusCode::BAD_REQUEST,
        ),
        (
            vec![
                ("If-Match", current.clone()),
                ("If-Modified-Since", "Thu, 01 Jan 1970 00:00:00 GMT".into()),
            ],
            StatusCode::BAD_REQUEST,
        ),
    ];
    for method in [reqwest::Method::GET, reqwest::Method::HEAD] {
        for (headers, status) in &cases {
            let mut request = http.client.request(method.clone(), url.clone());
            for (name, value) in headers {
                request = request.header(*name, value);
            }
            let reply = receive(request, *status).await?;
            if matches!(*status, StatusCode::OK | StatusCode::NOT_MODIFIED) {
                if method == reqwest::Method::HEAD {
                    evidence::observation(
                        "document-list-head-metadata-recommendation",
                        &json!({
                            "url":url.as_str(), "status":status.as_u16(),
                            "level":"advisory", "expected_get_etag":current,
                            "observed_etag":reply.headers().get("etag").and_then(|v|v.to_str().ok()),
                            "matches":reply.headers().get("etag").is_some_and(|v|v==current.as_str())
                        }),
                    );
                } else {
                    assert_eq!(reply.headers()["etag"], current);
                }
            }
            let body = reply.bytes().await.map_err(|e| e.to_string())?;
            if method == reqwest::Method::HEAD || *status == StatusCode::NOT_MODIFIED {
                assert!(body.is_empty());
            } else if *status == StatusCode::OK {
                assert_eq!(
                    serde_json::from_slice::<Value>(&body).map_err(|e| e.to_string())?,
                    *expected
                );
            }
        }
    }
    assert_eq!(list(http, url, expected).await?, current);
    Ok(current)
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    evidence::observation(
        "document-list-conditional-policy",
        &json!({
            "representation":"exact UTF-8 JSON ID list after selectors/since; empty list exists",
            "etag":"strong, stable for unchanged bytes, changes with returned ID list",
            "dates":"no list modification history; do not infer it from surviving members",
            "status":"failed If-Match requires412; Kiva selects recommended304 for matching read If-None-Match",
            "fixture":"fresh tenant and scopes; no concurrent writer; stale tag derived from captured list tag",
            "scope":"three families, GET/HEAD, empty/populated/filtered lists, member add/update/delete, mixed policy, syntax error precedence"
        }),
    );
    for (path, key) in [
        ("/activities/state", "stateId"),
        ("/agents/profile", "profileId"),
        ("/activities/profile", "profileId"),
    ] {
        let mut scope =
            reqwest::Url::parse(&format!("{}{path}", http.base)).map_err(|e| e.to_string())?;
        if path != "/agents/profile" {
            scope
                .query_pairs_mut()
                .append_pair("activityId", "https://example.org/list-conditions");
        }
        if path != "/activities/profile" {
            scope
                .query_pairs_mut()
                .append_pair("agent", r#"{"mbox":"mailto:list-conditions@example.org"}"#);
        }
        // A weak tag cannot satisfy strong If-Match even without knowing the
        // target's tag. This reproduces the original bypass before any new tag assertion.
        receive(
            http.client
                .get(scope.clone())
                .header("If-Match", "W/\"never-strong\""),
            StatusCode::PRECONDITION_FAILED,
        )
        .await?;
        let empty = matrix(&http, &scope, &json!([])).await?;
        let mut document = scope.clone();
        document.query_pairs_mut().append_pair(key, "日本語\"\\");
        receive(
            http.client
                .put(document.clone())
                .json(&json!({"original":1})),
            StatusCode::NO_CONTENT,
        )
        .await?;
        let before = receive(http.client.get(document.clone()), StatusCode::OK).await?;
        let document_tag = before.headers()["etag"].clone();
        assert_eq!(
            before.json::<Value>().await.map_err(|e| e.to_string())?,
            json!({"original":1})
        );
        let populated = matrix(&http, &scope, &json!(["日本語\"\\"])).await?;
        assert_ne!(empty, populated);
        receive(
            http.client.get(scope.clone()).header("If-Match", &empty),
            StatusCode::PRECONDITION_FAILED,
        )
        .await?;
        receive(
            http.client
                .get(scope.clone())
                .header("If-None-Match", &empty),
            StatusCode::OK,
        )
        .await?;

        // Filtering selects a different representation of the same scope.
        let mut filtered = scope.clone();
        filtered
            .query_pairs_mut()
            .append_pair("since", "9999-12-31T23:59:59Z");
        assert_eq!(matrix(&http, &filtered, &json!([])).await?, empty);
        receive(
            http.client.get(filtered).header("If-Match", &populated),
            StatusCode::PRECONDITION_FAILED,
        )
        .await?;

        // Updating document bytes must not change the list of IDs. Verify the
        // failed reads have not altered the stored document or its own ETag.
        let original = receive(http.client.get(document.clone()), StatusCode::OK).await?;
        assert_eq!(original.headers()["etag"], document_tag);
        assert_eq!(
            original.json::<Value>().await.map_err(|e| e.to_string())?,
            json!({"original":1})
        );
        receive(
            http.client
                .put(document.clone())
                .header("If-Match", document_tag)
                .json(&json!({"updated":2})),
            StatusCode::NO_CONTENT,
        )
        .await?;
        assert_eq!(
            list(&http, &scope, &json!(["日本語\"\\"])).await?,
            populated
        );
        let updated = receive(http.client.get(document.clone()), StatusCode::OK).await?;
        assert_eq!(
            updated.json::<Value>().await.map_err(|e| e.to_string())?,
            json!({"updated":2})
        );

        let mut invalid = scope.clone();
        invalid
            .query_pairs_mut()
            .append_pair("unknownListParameter", "1");
        for method in [reqwest::Method::GET, reqwest::Method::HEAD] {
            receive(
                http.client
                    .request(method, invalid.clone())
                    .header("If-None-Match", "*"),
                StatusCode::BAD_REQUEST,
            )
            .await?;
        }
        receive(http.client.delete(document), StatusCode::NO_CONTENT).await?;
        assert_eq!(list(&http, &scope, &json!([])).await?, empty);
        receive(
            http.client
                .get(scope.clone())
                .header("If-Match", &populated),
            StatusCode::PRECONDITION_FAILED,
        )
        .await?;
        receive(
            http.client.get(scope).header("If-None-Match", empty),
            StatusCode::NOT_MODIFIED,
        )
        .await?;
    }
    Ok(())
}

//! A legal query count is not bounded by the server's integer storage width.
#![allow(clippy::wildcard_imports)]
use super::*;

#[tokio::test]
async fn memory_xapi_query_limit_range() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_query_limit_range() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    let verb = format!("https://example.org/large-limit/{}", Uuid::new_v4());
    let mut ids = Vec::new();
    for index in 0..3 {
        let id = Uuid::new_v4().to_string();
        let value = json!({"id":id,"actor":{"mbox":"mailto:limit@example.org"},
            "verb":{"id":verb},"object":{"id":format!("https://example.org/limit/{index}")}});
        let response = HttpProbe::receive(
            http.client
                .put(format!("{}/statements", http.base))
                .query(&[("statementId", &id)])
                .json(&value),
        )
        .await?;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        ids.push(id);
    }
    ids.sort();
    evidence::observation(
        "query-limit-range",
        &json!({
            "requirement":"IP-get-param-limit: nonnegative integer, maximum count; zero uses server maximum",
            "fixture":"three admitted Statements under unique verb; stable fixture during traversal",
            "boundary":"large legal integers may exceed machine word size; no size-proportional allocation",
            "scope":"GET/HEAD, no cap and two-row cap, complete continuation traversal, independent invalid syntax and single-Statement conflict"
        }),
    );
    let values = [
        "18446744073709551616".to_owned(),
        "18446744073709551615".to_owned(),
        "4294967296".to_owned(),
        "9".repeat(300),
        format!("{}1", "0".repeat(300)),
        "0".repeat(300),
        "+3".into(),
    ];
    for cap in [None, std::num::NonZeroUsize::new(2)] {
        let mut configured = state.clone();
        configured.limits.max_statement_page_size = cap;
        let http = HttpProbe::new(&configured).await?;
        for value in &values {
            let mut url = reqwest::Url::parse(&format!("{}/statements", http.base))
                .map_err(|e| e.to_string())?;
            url.query_pairs_mut()
                .append_pair("verb", &verb)
                .append_pair("limit", value);
            let head = HttpProbe::receive(http.client.head(url.clone())).await?;
            assert_eq!(head.status(), StatusCode::OK, "HEAD limit={value}");
            assert!(head.bytes().await.map_err(|e| e.to_string())?.is_empty());
            let page_max = if value.ends_with('1') {
                1
            } else {
                cap.map_or(3, std::num::NonZeroUsize::get)
            };
            let mut found = Vec::new();
            let mut pages = 0;
            loop {
                pages += 1;
                assert!(pages <= 3, "continuation failed to terminate");
                let response = HttpProbe::receive(http.client.get(url)).await?;
                assert_eq!(response.status(), StatusCode::OK, "GET limit={value}");
                let body = response.json::<Value>().await.map_err(|e| e.to_string())?;
                let statements = body["statements"]
                    .as_array()
                    .ok_or("StatementResult array missing")?;
                assert!(!statements.is_empty() && statements.len() <= page_max);
                found.extend(
                    statements
                        .iter()
                        .map(|v| v["id"].as_str().map(str::to_owned))
                        .collect::<Option<Vec<_>>>()
                        .ok_or("missing ID")?,
                );
                let more = body["more"].as_str().ok_or("more missing")?;
                if more.is_empty() {
                    break;
                }
                url = reqwest::Url::parse(&http.base)
                    .map_err(|e| e.to_string())?
                    .join(more)
                    .map_err(|e| e.to_string())?;
            }
            found.sort();
            assert_eq!(found, ids, "complete membership for limit={value}");
        }
        for value in [
            "-1".into(),
            "1.5".into(),
            String::new(),
            "١".into(),
            format!("{}x", "9".repeat(300)),
        ] {
            let response = HttpProbe::receive(
                http.client
                    .get(format!("{}/statements", http.base))
                    .query(&[("limit", value.as_str())]),
            )
            .await?;
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "invalid limit={value}"
            );
        }
        let response = HttpProbe::receive(
            http.client
                .get(format!("{}/statements", http.base))
                .query(&[
                    ("statementId", ids[0].as_str()),
                    ("limit", values[0].as_str()),
                ]),
        )
        .await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    Ok(())
}

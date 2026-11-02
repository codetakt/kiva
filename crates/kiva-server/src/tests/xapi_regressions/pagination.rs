#![allow(clippy::wildcard_imports)]

use super::*;

#[tokio::test]
async fn memory_xapi_http_continuation_policy() -> Result<(), String> {
    verify_http(&memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_http_continuation_policy() -> Result<(), String> {
    verify_http(
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

async fn page(client: &reqwest::Client, url: reqwest::Url) -> Result<Value, String> {
    let reply = receive(client.get(url)).await?;
    assert_eq!(reply.status(), StatusCode::OK);
    reply.json().await.map_err(|e| e.to_string())
}

async fn verify_http(state: &AppState) -> Result<(), String> {
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
    for ascending in [false, true] {
        let verb = format!("https://example.org/pagination/{}", Uuid::new_v4());
        let mut ids = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        ids.sort();
        let values = ids.map(|id| {
            let mut value = statement(id);
            value["verb"]["id"] = json!(verb);
            value
        });
        let reply = receive(client.post(format!("{base}/statements")).json(&values)).await?;
        assert_eq!(reply.status(), StatusCode::OK);
        assert_eq!(
            reply.json::<Value>().await.map_err(|e| e.to_string())?,
            json!(ids)
        );
        if !ascending {
            ids.reverse();
        }
        let first_url = reqwest::Url::parse_with_params(
            &format!("{base}/statements"),
            [
                ("verb", verb.as_str()),
                ("limit", "1"),
                ("ascending", if ascending { "true" } else { "false" }),
            ],
        )
        .map_err(|e| e.to_string())?;
        let first = page(&client, first_url.clone()).await?;
        assert_eq!(first["statements"].as_array().ok_or("first page")?.len(), 1);
        assert_eq!(first["statements"][0]["id"], json!(ids[0]));
        let more = first["more"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("usable continuation")?;
        let next = first_url.join(more).map_err(|e| e.to_string())?;
        let second = page(&client, next.clone()).await?;
        assert_eq!(second["statements"][0]["id"], json!(ids[1]));
        let mut added = statement(Uuid::new_v4());
        added["verb"]["id"] = json!(verb);
        assert_eq!(
            receive(client.post(format!("{base}/statements")).json(&added))
                .await?
                .status(),
            StatusCode::OK
        );
        let mut voiding_ids = Vec::new();
        for id in [ids[0], ids[1]] {
            let mut voiding = statement(Uuid::new_v4());
            voiding_ids.push(voiding["id"].clone());
            voiding["verb"]["id"] = json!("http://adlnet.gov/expapi/verbs/voided");
            voiding["object"] = json!({"objectType":"StatementRef","id":id});
            assert_eq!(
                receive(client.post(format!("{base}/statements")).json(&voiding))
                    .await?
                    .status(),
                StatusCode::OK
            );
        }
        let mut cursor = first_url
            .join(
                second["more"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .ok_or("third page link")?,
            )
            .map_err(|e| e.to_string())?;
        let mut remaining = Vec::new();
        loop {
            let current = page(&client, cursor.clone()).await?;
            let records = current["statements"].as_array().ok_or("page statements")?;
            assert_eq!(records.len(), 1);
            remaining.push(records[0]["id"].clone());
            assert!(
                remaining.len() <= 4,
                "continuation repeated/exceeded known membership"
            );
            let more = current["more"]
                .as_str()
                .ok_or("D-03=A requires more property")?;
            if more.is_empty() {
                break;
            }
            cursor = cursor.join(more).map_err(|e| e.to_string())?;
        }
        let expected = if ascending {
            // StatementRef filtering also returns the new voiding Statements
            // whose referenced targets match the verb. The targets themselves
            // remain excluded. These are distinct, required membership rules.
            let mut expected = vec![json!(ids[2]), added["id"].clone()];
            expected.extend(voiding_ids);
            expected
        } else {
            vec![json!(ids[2])]
        };
        assert_eq!(remaining, expected);
        // Kiva's live cursor has no clock expiry or cache lifetime. Advancing
        // the independent clock observes that declared choice, not an IEEE
        // 24-hour minimum. A retained link re-runs and excludes now-voided rows.
        state
            .xapi
            .statements
            .clock()
            .set(fixed_time() + Duration::days(40));
        let delayed = page(&client, next).await?;
        assert_eq!(delayed["statements"][0]["id"], json!(ids[2]));
        evidence::observation(
            "continuation-policy",
            &json!({"strategy":"live requery with stored/id cursor","expiry":"none","terminal":"D-03=A empty string","ascending":ascending,"observed_remaining":remaining,"new_arrival":added["id"],"voided_excluded":[ids[0],ids[1]],"other_lrs_terminal_absence":"allowed by IEEE; Kiva policy mismatch only"}),
        );
    }
    let empty = reqwest::Url::parse_with_params(
        &format!("{base}/statements"),
        [(
            "verb",
            format!("https://example.org/unknown/{}", Uuid::new_v4()),
        )],
    )
    .map_err(|e| e.to_string())?;
    assert_eq!(
        page(&client, empty.clone()).await?,
        json!({"statements":[],"more":""})
    );
    let head = receive(client.head(empty)).await?;
    assert_eq!(head.status(), StatusCode::OK);
    assert!(head.bytes().await.map_err(|e| e.to_string())?.is_empty());
    server.abort();
    Ok(())
}

pub(super) async fn assert_continuations(state: &AppState) -> Result<(), String> {
    for ascending in [false, true] {
        let verb = format!("https://example.org/{}", Uuid::new_v4());
        let mut ids = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        ids.sort();
        let values = ids.map(|id| {
            let mut value = statement(id);
            value["verb"]["id"] = json!(verb);
            value
        });
        // A batch gives all three Statements the same stored time, exercising ID ties.
        let batch = json!(values);
        expect_status(
            post_statements(
                State(state.xapi_http_context()),
                confidential_headers_for("statement-writer-a"),
                empty_query(),
                json_body(&batch, "batch")?,
            )
            .await,
            StatusCode::OK,
            "batch admission",
        )?;
        if !ascending {
            ids.reverse();
        }
        let query = BTreeMap::from([
            ("verb".to_owned(), verb.clone()),
            ("limit".to_owned(), "1".to_owned()),
            ("ascending".to_owned(), ascending.to_string()),
        ]);
        let first = get_statements(
            State(state.xapi_http_context()),
            HeaderMap::new(),
            Query(query),
        )
        .await;
        let first = response_json(expect_status(first, StatusCode::OK, "first page")?).await?;
        assert_eq!(first["statements"][0]["id"], json!(ids[0]));

        let mut inserted = statement(Uuid::new_v4());
        inserted["verb"]["id"] = json!(verb);
        expect_status(
            submit(state, &inserted, false).await?,
            StatusCode::NO_CONTENT,
            "concurrent insertion",
        )?;
        let second = next_page(state, &first).await?;
        assert_eq!(second["statements"][0]["id"], json!(ids[1]));

        // Remove both preceding positions from the visible collection, including the cursor.
        for id in [ids[0], ids[1]] {
            let mut voiding = statement(Uuid::new_v4());
            voiding["verb"]["id"] = json!("http://adlnet.gov/expapi/verbs/voided");
            voiding["object"] = json!({"objectType": "StatementRef", "id": id});
            expect_status(
                submit(state, &voiding, false).await?,
                StatusCode::NO_CONTENT,
                "concurrent voiding",
            )?;
        }
        let third = next_page(state, &second).await?;
        assert_eq!(third["statements"][0]["id"], json!(ids[2]));
        if ascending {
            let fourth = next_page(state, &third).await?;
            assert_eq!(fourth["statements"][0]["id"], inserted["id"]);
        } else {
            assert_eq!(third["more"], json!(""));
        }
    }
    Ok(())
}

async fn next_page(state: &AppState, page: &Value) -> Result<Value, String> {
    let more = page["more"].as_str().ok_or("missing more URL")?;
    let (_, query) = more.split_once('?').ok_or("missing continuation query")?;
    let query = url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();
    let response = get_more_statements(
        State(state.xapi_http_context()),
        HeaderMap::new(),
        Query(query),
    )
    .await;
    response_json(expect_status(
        response,
        StatusCode::OK,
        "continuation page",
    )?)
    .await
}

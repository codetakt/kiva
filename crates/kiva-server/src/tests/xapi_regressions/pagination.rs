#![allow(clippy::wildcard_imports)]

use super::*;

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

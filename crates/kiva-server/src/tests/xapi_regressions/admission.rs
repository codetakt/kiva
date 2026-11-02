#![allow(clippy::wildcard_imports)]

use super::*;

pub(super) async fn assert_retries(state: &AppState) -> Result<(), String> {
    for post in [false, true] {
        for supplied_timestamp in [false, true] {
            let id = Uuid::new_v4();
            let mut value = statement(id);
            let mut nested = statement(Uuid::new_v4());
            nested
                .as_object_mut()
                .ok_or("Statement object")?
                .remove("id");
            nested["objectType"] = json!("SubStatement");
            if supplied_timestamp {
                nested["timestamp"] = json!("2024-01-01T00:00:00Z");
            }
            value["object"] = nested;
            let expected = if post {
                StatusCode::OK
            } else {
                StatusCode::NO_CONTENT
            };
            expect_status(
                submit(state, &value, post).await?,
                expected,
                "first admission",
            )?;
            let original = state
                .xapi
                .statements
                .get(id)
                .await
                .map_err(|e| format!("{e:?}"))?
                .ok_or("stored Statement")?;
            // Make the second normalization observably later on both backends.
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            expect_status(
                submit(state, &value, post).await?,
                expected,
                "identical retry",
            )?;
            let unchanged = state
                .xapi
                .statements
                .get(id)
                .await
                .map_err(|e| format!("{e:?}"))?
                .ok_or("stored retry")?;
            assert_eq!(unchanged.statement, original.statement);
            assert_eq!(unchanged.stored_at, original.stored_at);

            let mut explicit = value.clone();
            explicit["object"]["timestamp"] = original.statement["object"]["timestamp"].clone();
            expect_status(
                submit(state, &explicit, post).await?,
                expected,
                "same explicit timestamp",
            )?;
            explicit["object"]["timestamp"] = json!("2020-01-01T00:00:00Z");
            expect_status(
                submit(state, &explicit, post).await?,
                StatusCode::CONFLICT,
                "changed explicit timestamp",
            )?;

            value["object"]["verb"]["id"] = json!("https://example.org/changed");
            expect_status(
                submit(state, &value, post).await?,
                StatusCode::CONFLICT,
                "changed SubStatement",
            )?;
        }
    }
    for timestamp in [
        None,
        Some("2024-01-01T09:00:00+09:00"),
        Some("2023-12-31T19:00:00-05:00"),
        Some("2024-01-01T00:00:00.000Z"),
    ] {
        assert_legacy_retries(state, timestamp).await?;
    }
    Ok(())
}

async fn assert_legacy_retries(state: &AppState, timestamp: Option<&str>) -> Result<(), String> {
    let id = Uuid::new_v4();
    let mut value = statement(id);
    let mut nested = statement(Uuid::new_v4());
    nested.as_object_mut().ok_or("SubStatement")?.remove("id");
    nested["objectType"] = json!("SubStatement");
    if let Some(timestamp) = timestamp {
        nested["timestamp"] = json!(timestamp);
    }
    value["object"] = nested;
    let mut legacy = normalized_statement(
        value.clone(),
        id,
        fixed_time(),
        &authority_agent("statement-writer-a"),
    )
    .map_err(|error| error.message)?;
    // Older admission preserved nested timestamp spelling, including absence.
    legacy["object"] = value["object"].clone();
    let stored = state
        .xapi
        .statements
        .put(
            id,
            StoredStatement::new(legacy.clone(), fixed_time(), BTreeMap::new()),
        )
        .await
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(stored, StoreOutcome::Stored);
    // Admission now assigns recording time inside the repository. Capture the
    // seeded legacy row BEFORE retries, then require exact preservation of it.
    let seeded = state
        .xapi
        .statements
        .get(id)
        .await
        .map_err(|error| format!("{error:?}"))?
        .ok_or("seeded legacy record")?;
    legacy = seeded.statement;
    let recorded_at = seeded.stored_at;
    expect_status(
        submit(state, &value, false).await?,
        StatusCode::NO_CONTENT,
        "legacy PUT retry",
    )?;

    let new_id = Uuid::new_v4();
    let batch = json!([value.clone(), statement(new_id)]);
    expect_status(
        submit(state, &batch, true).await?,
        StatusCode::OK,
        "legacy retry in POST batch",
    )?;
    assert!(
        state
            .xapi
            .statements
            .get(new_id)
            .await
            .map_err(|error| format!("{error:?}"))?
            .is_some()
    );
    if timestamp.is_some() {
        let mut equivalent = value.clone();
        equivalent["object"]["timestamp"] = json!("2024-01-01T00:00:00Z");
        expect_status(
            submit(state, &equivalent, false).await?,
            StatusCode::NO_CONTENT,
            "equivalent legacy PUT timestamp",
        )?;
        expect_status(
            submit(state, &equivalent, true).await?,
            StatusCode::OK,
            "equivalent legacy POST timestamp",
        )?;
    }
    let original = state
        .xapi
        .statements
        .get(id)
        .await
        .map_err(|error| format!("{error:?}"))?
        .ok_or("legacy record")?;
    assert_eq!(original.statement, legacy);
    assert_eq!(original.stored_at, recorded_at);

    // Adding a missing timestamp or changing its instant remains a conflict.
    value["object"]["timestamp"] = json!("2024-01-01T00:00:00.000000001Z");
    expect_status(
        submit(state, &value, false).await?,
        StatusCode::CONFLICT,
        "explicit legacy timestamp change",
    )?;
    let rejected_id = Uuid::new_v4();
    let batch = json!([statement(rejected_id), value]);
    expect_status(
        submit(state, &batch, true).await?,
        StatusCode::CONFLICT,
        "conflicting legacy POST batch",
    )?;
    assert!(
        state
            .xapi
            .statements
            .get(rejected_id)
            .await
            .map_err(|error| format!("{error:?}"))?
            .is_none()
    );
    Ok(())
}

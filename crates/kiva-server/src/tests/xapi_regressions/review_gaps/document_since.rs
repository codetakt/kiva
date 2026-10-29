//! Untrusted since bounds must survive UTC range conversion and retain precision.
#![allow(clippy::wildcard_imports)]
use super::*;

async fn listing(http: &HttpProbe, url: reqwest::Url, expected: &Value) -> Result<(), String> {
    let reply = HttpProbe::receive(http.client.get(url.clone())).await?;
    assert_eq!(reply.status(), StatusCode::OK, "{url}");
    assert_eq!(
        reply.json::<Value>().await.map_err(|e| e.to_string())?,
        *expected,
        "{url}"
    );
    let head = HttpProbe::receive(http.client.head(url)).await?;
    assert_eq!(head.status(), StatusCode::OK);
    assert!(head.bytes().await.map_err(|e| e.to_string())?.is_empty());
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    for (path, id_key) in [
        ("/activities/state", "stateId"),
        ("/activities/profile", "profileId"),
        ("/agents/profile", "profileId"),
    ] {
        // Exercise fractions on both sides of the Unix and PostgreSQL epochs.
        for day in ["1969-12-31", "1970-01-01", "1999-12-31", "2000-01-01"] {
            let mut scope =
                reqwest::Url::parse(&format!("{}{path}", http.base)).map_err(|e| e.to_string())?;
            // Independent scopes avoid overwriting a previous clock fixture.
            let agent = json!({"mbox":format!("mailto:{day}@example.org")}).to_string();
            if path != "/activities/profile" {
                scope.query_pairs_mut().append_pair("agent", &agent);
            }
            if path != "/agents/profile" {
                scope.query_pairs_mut().append_pair(
                    "activityId",
                    &format!("https://example.org/since-range/{day}"),
                );
            }
            let recorded = format!("{day}T12:00:00.123456Z");
            state.xapi.documents.clock().set(
                time::OffsetDateTime::parse(
                    &recorded,
                    &time::format_description::well_known::Rfc3339,
                )
                .map_err(|e| e.to_string())?,
            );
            let mut document = scope.clone();
            document.query_pairs_mut().append_pair(id_key, "saved");
            let reply =
                HttpProbe::receive(http.client.put(document.clone()).json(&json!({"ok":true})))
                    .await?;
            assert_eq!(reply.status(), StatusCode::NO_CONTENT);
            for (bound, include) in [
                (format!("{day}T12:00:00.123455999999Z"), true),
                (recorded, false),
                (format!("{day}T12:00:00.123456000001Z"), false),
                (format!("{day}T12:00:00.123457Z"), false),
                (format!("{day}T13:00:00.123455999999+01:00"), true),
                (format!("{day}T11:00:00.123456000001-01:00"), false),
                ("9999-12-31T23:30:00-01:00".to_owned(), false),
                ("9999-12-31T23:59:59.999999999999-23:59".to_owned(), false),
                ("0000-01-01T00:30:00+01:00".to_owned(), true),
                ("0000-01-01T00:00:00.000000000001+23:59".to_owned(), true),
            ] {
                let mut query = scope.clone();
                query.query_pairs_mut().append_pair("since", &bound);
                listing(
                    &http,
                    query,
                    &if include { json!(["saved"]) } else { json!([]) },
                )
                .await?;
            }
            // Unbounded queries and the stored bytes remain unchanged.
            listing(&http, scope, &json!(["saved"])).await?;
            let reply = HttpProbe::receive(http.client.get(document)).await?;
            assert_eq!(reply.status(), StatusCode::OK);
            assert_eq!(
                reply.json::<Value>().await.map_err(|e| e.to_string())?,
                json!({"ok":true})
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn memory_xapi_document_since_range_and_precision() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_document_since_range_and_precision() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

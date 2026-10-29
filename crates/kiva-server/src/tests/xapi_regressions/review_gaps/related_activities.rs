//! Context Activity selectors stop at the declared Activity boundary.
#![allow(clippy::wildcard_imports)]
use super::*;
use std::collections::BTreeSet;

async fn selected_ids(
    http: &HttpProbe,
    activity: &str,
    related: bool,
    ascending: bool,
) -> Result<BTreeSet<String>, String> {
    let mut url =
        reqwest::Url::parse(&format!("{}/statements", http.base)).map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair("activity", activity)
        .append_pair("related_activities", if related { "true" } else { "false" })
        .append_pair("ascending", if ascending { "true" } else { "false" })
        .append_pair("limit", "1");
    let mut ids = BTreeSet::new();
    let mut pages = BTreeSet::new();
    loop {
        assert!(pages.insert(url.to_string()) && pages.len() <= 8);
        let reply = HttpProbe::receive(http.client.get(url)).await?;
        assert_eq!(reply.status(), StatusCode::OK);
        let body: Value = reply.json().await.map_err(|e| e.to_string())?;
        let values = body["statements"].as_array().ok_or("StatementResult")?;
        assert!(values.len() <= 1);
        for value in values {
            assert!(ids.insert(value["id"].as_str().ok_or("id")?.to_owned()));
        }
        let more = body["more"].as_str().ok_or("more")?;
        if more.is_empty() {
            return Ok(ids);
        }
        url = reqwest::Url::parse(&http.base)
            .and_then(|base| base.join(more))
            .map_err(|e| e.to_string())?;
    }
}

async fn context_activity_shapes(state: &AppState) -> Result<(), String> {
    let http = HttpProbe::new(state).await?;
    for nested in [false, true] {
        for role in ["parent", "grouping", "category", "other"] {
            let activity = format!("https://example.org/context/{}", Uuid::new_v4());
            let phantom = format!("https://example.org/opaque/{}", Uuid::new_v4());
            let mut related_ids = BTreeSet::new();
            for array in [false, true] {
                let (mut value, owner) = root_and_owner(nested);
                let context = json!({"id":activity,"definition":{"extensions":{
                    "https://example.org/owner":[{"id":phantom}]
                }}});
                value.pointer_mut(owner).ok_or("owner")?["context"] =
                    json!({"contextActivities":{role:if array {json!([context])} else {context}}});
                http.submit(&value, array, None, true).await?;
                related_ids.insert(value["id"].as_str().ok_or("id")?.to_owned());
                let mut reference = statement(Uuid::new_v4());
                reference["object"] = json!({"objectType":"StatementRef","id":value["id"]});
                http.submit(&reference, false, None, true).await?;
                related_ids.insert(reference["id"].as_str().ok_or("id")?.to_owned());

                let reply = HttpProbe::receive(http.client.get(format!(
                    "{}/statements?statementId={}",
                    http.base,
                    value["id"].as_str().ok_or("id")?
                )))
                .await?;
                let saved: Value = reply.json().await.map_err(|e| e.to_string())?;
                assert_eq!(
                    saved.pointer(&format!("{owner}/context/contextActivities/{role}/0/id")),
                    Some(&json!(activity))
                );
                assert_eq!(
                    saved.pointer(&format!(
                        "{owner}/context/contextActivities/{role}/0/definition/extensions"
                    )),
                    Some(&json!({"https://example.org/owner":[{"id":phantom}]}))
                );
            }
            let mut ordinary = statement(Uuid::new_v4());
            ordinary["object"] = json!({"id":activity});
            http.submit(&ordinary, false, None, true).await?;
            let ordinary_ids = BTreeSet::from([ordinary["id"].as_str().ok_or("id")?.to_owned()]);
            related_ids.extend(ordinary_ids.iter().cloned());
            for ascending in [false, true] {
                assert_eq!(
                    selected_ids(&http, &activity, false, ascending).await?,
                    ordinary_ids
                );
                assert_eq!(
                    selected_ids(&http, &activity, true, ascending).await?,
                    related_ids,
                    "nested={nested}, role={role}, ascending={ascending}"
                );
                assert!(
                    selected_ids(&http, &phantom, true, ascending)
                        .await?
                        .is_empty(),
                    "opaque owner values must not become related Activities"
                );
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn memory_xapi_context_activity_shapes() -> Result<(), String> {
    context_activity_shapes(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_context_activity_shapes() -> Result<(), String> {
    context_activity_shapes(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

//! SP-REFERENCE-CONJUNCTION: inherit each non-time predicate, then conjoin.
#![allow(clippy::wildcard_imports)]
use super::*;

#[tokio::test]
async fn memory_xapi_reference_conjunction_interpretation() -> Result<(), String> {
    verify(&memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_reference_conjunction_interpretation() -> Result<(), String> {
    verify(
        &postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

#[allow(clippy::too_many_lines)]
async fn verify(state: &AppState) -> Result<(), String> {
    let mut configured = state.clone();
    configured.auth = AuthConfig::with_token_verifier(
        true,
        Arc::new(authority::Verifier {
            calls: Arc::new(Mutex::new(Vec::new())),
        }),
    );
    configured.limits.max_statement_page_size = std::num::NonZeroUsize::new(2);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let base = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let server =
        tokio::spawn(async move { axum::serve(listener, evidence::router(&configured)).await });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let epoch = instant("2020-01-01T00:00:00Z")?;
    let ids: [Uuid; 8] = std::array::from_fn(|_| Uuid::new_v4());
    let mut values = ids.map(record);
    // A <- B <- C <- D. Four selectors will have four different witnesses:
    // A's Activity, B's Agent, C's Verb and D's registration.
    for i in 1..4 {
        values[i]["object"] = json!({"objectType":"StatementRef","id":ids[i-1]});
    }
    let agent = values[1]["actor"].to_string();
    let verb = values[2]["verb"]["id"].as_str().ok_or("verb")?.to_owned();
    let activity = values[0]["object"]["id"]
        .as_str()
        .ok_or("activity")?
        .to_owned();
    let registration = values[3]["context"]["registration"]
        .as_str()
        .ok_or("registration")?
        .to_owned();
    // F has three direct matches, but only Context.statement points to A.
    values[4]["actor"] = values[1]["actor"].clone();
    values[4]["verb"] = values[2]["verb"].clone();
    values[4]["context"]["registration"] = values[3]["context"]["registration"].clone();
    values[4]["context"]["statement"] = json!({"objectType":"StatementRef","id":ids[0]});
    // M has the same three matches but its Object target does not exist.
    values[5]["actor"] = values[1]["actor"].clone();
    values[5]["verb"] = values[2]["verb"].clone();
    values[5]["context"]["registration"] = values[3]["context"]["registration"].clone();
    values[5]["object"] = json!({"objectType":"StatementRef","id":Uuid::new_v4()});
    // U <-> V contains separate Agent/Verb/registration seeds but no Activity
    // seed: a cycle neither fabricates a missing predicate nor loses real ones.
    values[6]["actor"] = values[1]["actor"].clone();
    values[6]["object"] = json!({"objectType":"StatementRef","id":ids[7]});
    values[7]["verb"] = values[2]["verb"].clone();
    values[7]["context"]["registration"] = values[3]["context"]["registration"].clone();
    values[7]["object"] = json!({"objectType":"StatementRef","id":ids[6]});
    evidence::observation(
        "reference-conjunction-fixture",
        &json!({
            "interpretation":"SP-REFERENCE-CONJUNCTION", "branch":"REF-FILTER-01",
            "values":values,"ids":ids,"stored_seconds":[1,2,3,4,5,6,7,8],
            "claim":"Explicit project source interpretation; not issuer adjudication",
            "four_predicate_witnesses":{"activity":"A","agent":"B","verb":"C","registration":"D"}
        }),
    );
    for (i, value) in values.iter().enumerate() {
        put(
            state,
            &client,
            &base,
            value,
            epoch + Duration::seconds(i64::try_from(i + 1).map_err(|e| e.to_string())?),
        )
        .await?;
    }
    let missing = values[5]["object"]["id"].as_str().ok_or("missing target")?;
    assert_eq!(
        receive(client.get(format!("{base}/statements?statementId={missing}")))
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    let four = BTreeMap::from([
        ("agent".into(), agent.clone()),
        ("verb".into(), verb.clone()),
        ("activity".into(), activity.clone()),
        ("registration".into(), registration.clone()),
    ]);
    let split = BTreeMap::from([
        ("agent".into(), agent),
        (
            "verb".into(),
            values[0]["verb"]["id"].as_str().ok_or("verb A")?.to_owned(),
        ),
    ]);
    let same = BTreeMap::from([
        ("agent".into(), values[0]["actor"].to_string()),
        (
            "verb".into(),
            values[0]["verb"]["id"].as_str().ok_or("verb A")?.to_owned(),
        ),
    ]);
    let mut three = four.clone();
    three.remove("activity");
    let mut cases = vec![
        ("split-two", split.clone(), ids[1..4].to_vec()),
        ("same-node", same, ids[..4].to_vec()),
        ("split-four", four.clone(), vec![ids[3]]),
        (
            "cycle-with-three-seeds",
            three,
            vec![ids[3], ids[4], ids[5], ids[6], ids[7]],
        ),
    ];
    for (field, absent) in [
        (
            "agent",
            json!({"mbox":format!("mailto:{}@example.org",Uuid::new_v4())}).to_string(),
        ),
        (
            "verb",
            format!("https://example.org/absent/{}", Uuid::new_v4()),
        ),
        (
            "activity",
            format!("https://example.org/absent/{}", Uuid::new_v4()),
        ),
        ("registration", Uuid::new_v4().to_string()),
    ] {
        let mut negative = four.clone();
        negative.insert(field.into(), absent);
        cases.push((field, negative, vec![]));
    }
    let mut bounded = split.clone();
    bounded.insert("since".into(), "2020-01-01T00:00:02Z".into());
    bounded.insert("until".into(), "2020-01-01T00:00:03Z".into());
    cases.push(("targeter-time", bounded, vec![ids[2]]));
    let mut early = split;
    early.insert("until".into(), "2020-01-01T00:00:02Z".into());
    cases.push(("inclusive-until", early, vec![ids[1]]));
    for ascending in [true, false] {
        for limit in [1, 2] {
            for (name, filters, ascending_ids) in &cases {
                let mut parameters = filters.clone();
                parameters.insert("ascending".into(), ascending.to_string());
                parameters.insert("limit".into(), limit.to_string());
                let mut expected = ascending_ids.clone();
                if !ascending {
                    expected.reverse();
                }
                evidence::observation(
                    "reference-conjunction-query",
                    &json!({
                        "interpretation":"SP-REFERENCE-CONJUNCTION","case":name,
                        "query":parameters,"expected_ids":expected,
                        "oracle":"All predicates independently inherited through Object refs, conjoined; targeter time and sequence apply"
                    }),
                );
                chain(&client, query(&base, &parameters)?, &expected, limit).await?;
            }
        }
    }
    // The new interpretation does not change single-ID selection or the
    // forbidden combination of a single-ID selector with collection filters.
    for id in &ids[..4] {
        let reply =
            receive(client.get(format!("{base}/statements?statementId={id}&format=exact"))).await?;
        assert_eq!(reply.status(), StatusCode::OK);
        assert_eq!(json(reply).await?["id"], json!(id));
    }
    let mut forbidden = four;
    forbidden.insert("statementId".into(), ids[3].to_string());
    assert_eq!(
        receive(client.get(query(&base, &forbidden)?))
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    server.abort();
    Ok(())
}

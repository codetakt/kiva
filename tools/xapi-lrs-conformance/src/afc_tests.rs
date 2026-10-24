//! Counterexamples for allowed timestamp ties and exclusive/inclusive filters.
use super::*;

#[test]
fn query_pagination_witness_and_more_syntax_are_independent_of_page_limits() {
    for timestamp in ["2026-01-01T00:00:00Z", "2016-12-31T23:59:60Z"] {
        let captures = BTreeMap::from([
            ("a".into(), timestamp.into()),
            ("b".into(), timestamp.into()),
        ]);
        for limit in [0, 1, 2] {
            let check: Check = serde_json::from_value(serde_json::json!({"kind":"statement-query",
                "stored_captures":{"a":"a","b":"b"},"limit":limit}))
            .unwrap();
            for count in [0, 1, 2] {
                for more in [
                    None,
                    Some(serde_json::json!("")),
                    Some(serde_json::json!("?cursor=2")),
                    Some(serde_json::json!("//[Vf.a:b]/next")),
                    Some(serde_json::json!(42)),
                    Some(serde_json::json!(null)),
                    Some(serde_json::json!("\n")),
                    Some(serde_json::json!("/next%xx")),
                    Some(serde_json::json!("http://[broken")),
                ] {
                    let items: Vec<_> = ["a", "b"]
                        .iter()
                        .take(count)
                        .map(|id| serde_json::json!({"id":id,"stored":timestamp}))
                        .collect();
                    let mut body = serde_json::json!({"statements":items});
                    if let Some(value) = &more {
                        body["more"] = value.clone();
                    }
                    let has_link = more.as_ref().is_some_and(|value| {
                        matches!(value.as_str(), Some("?cursor=2" | "//[Vf.a:b]/next"))
                    });
                    let valid_syntax = more.as_ref().is_none_or(|value| {
                        matches!(value.as_str(), Some("" | "?cursor=2" | "//[Vf.a:b]/next"))
                    });
                    let defect =
                        !valid_syntax || count < 2 && !has_link || limit != 0 && count > limit;
                    let errors = check.evaluate(
                        200,
                        &HeaderMap::new(),
                        body.to_string().as_bytes(),
                        &captures,
                    );
                    assert_eq!(
                        errors.iter().any(|e| !e.starts_with("ORACLE-UNAVAILABLE:")),
                        defect,
                        "{timestamp}/{limit}/{count}/{more:?}: {errors:?}"
                    );
                    if !defect {
                        assert_eq!(errors.is_empty(), timestamp.starts_with("2026"));
                    }
                }
            }
        }
    }
}

#[test]
fn query_ties_boundaries_order_and_page_limits() {
    for equal in [false, true] {
        let first = "2026-01-01T00:00:00Z";
        let second = if equal { first } else { "2026-01-01T00:00:01Z" };
        let captures = BTreeMap::from([("a".into(), first.into()), ("b".into(), second.into())]);
        let item =
            |id: &str| serde_json::json!({"id":id,"stored":if id=="a" {first} else {second}});
        for (descending, ids, success) in [
            (true, vec!["b", "a"], true),
            (true, vec!["a", "b"], equal),
            (false, vec!["a", "b"], true),
            (false, vec!["b", "a"], equal),
            (true, vec!["a", "a"], false),
        ] {
            let check: Check = serde_json::from_value(serde_json::json!({"kind":"statement-query","stored_captures":{"a":"a","b":"b"},"descending":descending})).unwrap();
            let bytes = serde_json::to_vec(
                &serde_json::json!({"statements":ids.iter().map(|id|item(id)).collect::<Vec<_>>()}),
            )
            .unwrap();
            assert_eq!(
                check
                    .evaluate(200, &HeaderMap::new(), &bytes, &captures)
                    .is_empty(),
                success
            );
        }
        for (bound, ids, success) in [
            ("since_capture", vec![], equal),
            ("since_capture", vec!["b"], !equal),
            ("since_capture", vec!["a"], false),
            ("until_capture", vec!["a"], !equal),
            ("until_capture", vec!["b", "a"], equal),
        ] {
            let mut spec = serde_json::json!({"kind":"statement-query","stored_captures":{"a":"a","b":"b"},"descending":true});
            spec[bound] = serde_json::json!("a");
            let check: Check = serde_json::from_value(spec).unwrap();
            let bytes = serde_json::to_vec(
                &serde_json::json!({"statements":ids.iter().map(|id|item(id)).collect::<Vec<_>>()}),
            )
            .unwrap();
            assert_eq!(
                check
                    .evaluate(200, &HeaderMap::new(), &bytes, &captures)
                    .is_empty(),
                success,
                "{equal} {bound} {ids:?}"
            );
        }
        let check:Check=serde_json::from_value(serde_json::json!({"kind":"statement-query","stored_captures":{"a":"a","b":"b"},"descending":true,"limit":1})).unwrap();
        for (ids, success) in [
            (vec!["b"], true),
            (vec!["a"], equal),
            (vec!["b", "a"], false),
        ] {
            let bytes = serde_json::to_vec(
                &serde_json::json!({"statements":ids.iter().map(|id|item(id)).collect::<Vec<_>>(),"more":"?cursor=2"}),
            )
            .unwrap();
            assert_eq!(
                check
                    .evaluate(200, &HeaderMap::new(), &bytes, &captures)
                    .is_empty(),
                success
            );
        }
    }
}

#[tokio::test]
async fn destructive_permission_guard_is_unexecuted() {
    let case: TestCase = serde_json::from_value(serde_json::json!({
        "id":"guard-control", "requirements":["fixture"],
        "safety":{"destructive":true,"requires_isolated_lrs":true},
        "request":{"method":"PUT","path":"/statements"},"expect":{"status":204}
    }))
    .unwrap();
    let report = execute_case(
        &Client::new(),
        &Url::parse("http://127.0.0.1:9").unwrap(),
        &BTreeMap::new(),
        &BTreeMap::new(),
        None,
        false,
        case,
    )
    .await;
    assert_eq!(report.outcome, CaseOutcome::NotRun);
    assert!(report.failures.is_empty());
    assert!(report.not_run_reasons[0].contains("no HTTP request"));
}

#[test]
fn named_admission_guard_requires_prior_unconditional_allowed_status() {
    let text = r"
suite: {id: named-branch, standard: IEEE/ISO/IEC 39274-1-1-2025}
cases:
- id: branch
  requirements: [xapi-lrs-4.2.5-voided-state]
  safety: {destructive: true, requires_isolated_lrs: true}
  steps:
  - id: write
    request: {method: PUT, path: /statements}
    expect: {status: 204, status_in: [400]}
  - id: intervening
    request: {method: GET, path: /statements}
    expect: {status: 200}
  - id: accepted
    when_step_status: {step: write, status: 204}
    request: {method: GET, path: /statements}
    expect: {status: 200}
  - id: rejected
    when_step_status: {step: write, status: 400}
    request: {method: GET, path: /statements}
    expect: {status: 404}
";
    for (input, valid) in [
        (text.to_owned(), true),
        (text.replace("step: write", "step: absent"), false),
        (
            text.replace("step: write, status: 400", "step: write, status: 403"),
            false,
        ),
        (
            text.replace("step: write, status: 204", "step: rejected, status: 204"),
            false,
        ),
        (
            text.replace("step: write, status: 400", "step: accepted, status: 200"),
            false,
        ),
        (
            text.replace(
                "  - id: rejected",
                "  - id: rejected\n    when_previous_status: 200",
            ),
            false,
        ),
    ] {
        let document: super::SuiteDocument = serde_yaml::from_str(&input).expect("fixture YAML");
        assert_eq!(
            super::validate_case_steps("branch", &document.cases[0]).is_ok(),
            valid
        );
    }
}

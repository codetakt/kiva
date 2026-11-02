//! Fixed DC-TIME ordinary/unknown/defect witnesses through the actual checks.
use super::Check;
use reqwest::header::HeaderMap;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const UNKNOWN: &str = "2016-12-31T23:59:60Z";
const EARLY: &str = "2025-01-01T00:00:00Z";
const LATE: &str = "2025-01-02T00:00:00Z";
const A: &str = "00000000-0000-4000-8000-000000000001";
const B: &str = "00000000-0000-4000-8000-000000000002";
const C: &str = "00000000-0000-4000-8000-000000000003";

#[expect(
    clippy::needless_pass_by_value,
    reason = "Paper-vector calls own their JSON literals for both evaluation and failure diagnostics"
)]
fn observe(
    check: Value,
    body: Value,
    header: Option<&str>,
    captures: &BTreeMap<String, String>,
    expected: &str,
) {
    let check: Check = serde_json::from_value(check).expect("fixed definition");
    check.validate().expect("valid assertion");
    let mut headers = HeaderMap::new();
    if let Some(header) = header {
        headers.insert("Last-Modified", header.parse().expect("header value"));
    }
    let results = check.evaluate(200, &headers, body.to_string().as_bytes(), captures);
    let outcome = if results
        .iter()
        .any(|v| !v.starts_with("ORACLE-UNAVAILABLE:"))
    {
        "failed"
    } else if results.is_empty() {
        "passed"
    } else {
        "not-run"
    };
    assert_eq!(outcome, expected, "{check:?}\nbody={body}\n{results:?}");
}
fn headers() -> Value {
    json!({"kind":"header-time","header":"Last-Modified","max_pointer":"/statements","optional_if_empty":true})
}
fn query(descending: bool, limit: usize, since: bool) -> Value {
    let mut q = json!({"kind":"statement-query","stored_captures":{A:"a",B:"b"},"descending":descending,"limit":limit});
    if since {
        q["since_capture"] = json!("since");
    }
    q
}
fn captures(a: &str, b: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("a".into(), a.into()),
        ("b".into(), b.into()),
        ("since".into(), "2024-01-01T00:00:00Z".into()),
    ])
}
fn records(items: &[(&str, &str)], more: &str) -> Value {
    json!({"statements":items.iter().map(|(id,time)|json!({"id":id,"stored":time})).collect::<Vec<_>>(),"more":more})
}

#[test]
fn required_headers_and_later_defects_precede_unknown_time() {
    for stored in [EARLY, UNKNOWN] {
        observe(
            headers(),
            records(&[(A, stored)], ""),
            None,
            &BTreeMap::new(),
            "failed",
        );
        observe(
            headers(),
            records(&[(A, stored)], ""),
            Some("invalid"),
            &BTreeMap::new(),
            "failed",
        );
    }
    for missing in [json!({}), json!({"stored":17})] {
        let body = json!({"statements":[{"stored":UNKNOWN},missing]});
        observe(
            headers(),
            body.clone(),
            Some("Thu, 02 Jan 2025 00:00:00 GMT"),
            &BTreeMap::new(),
            "failed",
        );
        observe(
            json!({"kind":"json-order","pointer":"/statements","by":"stored"}),
            body,
            None,
            &BTreeMap::new(),
            "failed",
        );
    }
    observe(
        headers(),
        records(&[(A, UNKNOWN), (B, LATE)], ""),
        Some("Wed, 01 Jan 2025 00:00:00 GMT"),
        &BTreeMap::new(),
        "failed",
    );
    observe(
        headers(),
        records(&[(A, UNKNOWN)], ""),
        Some("Thu, 02 Jan 2025 00:00:00 GMT"),
        &BTreeMap::new(),
        "not-run",
    );
    observe(
        headers(),
        records(&[(A, EARLY), (B, LATE)], ""),
        Some("Thu, 02 Jan 2025 00:00:00 GMT"),
        &BTreeMap::new(),
        "passed",
    );
}

#[test]
fn empty_collection_needs_meaning_evidence_for_a_supplied_date() {
    for (header, expected) in [
        (None, "passed"),
        (Some("invalid"), "failed"),
        (Some("Wed, 01 Jan 2025 00:00:00 GMT"), "not-run"),
    ] {
        observe(
            headers(),
            records(&[], ""),
            header,
            &BTreeMap::new(),
            expected,
        );
    }
}

#[test]
fn known_subsequences_survive_unknown_order_operands() {
    for descending in [false, true] {
        let check = json!({"kind":"json-order","pointer":"/statements","by":"stored","descending":descending});
        let (left, right) = if descending {
            (EARLY, LATE)
        } else {
            (LATE, EARLY)
        };
        observe(
            check.clone(),
            records(&[(A, left), (B, UNKNOWN), (C, right)], ""),
            None,
            &BTreeMap::new(),
            "failed",
        );
        observe(
            check,
            records(&[(A, right), (B, UNKNOWN), (C, left)], ""),
            None,
            &BTreeMap::new(),
            "not-run",
        );
    }
}

#[test]
fn query_unknowns_preserve_structure_membership_and_cardinality() {
    let caps = captures(EARLY, UNKNOWN);
    // Keep the duplicate-ID cause independent from the page-limit cause.
    observe(
        query(false, 0, true),
        records(&[(A, EARLY), (A, EARLY)], "/next"),
        None,
        &caps,
        "failed",
    );
    for body in [
        json!({"statements":null}),
        json!({"statements":{}}),
        records(&[(A, EARLY), (B, UNKNOWN)], ""),
        records(&[(C, EARLY)], "/next"),
        records(&[(A, EARLY), (A, EARLY)], "/next"),
    ] {
        observe(query(false, 1, true), body, None, &caps, "failed");
    }
    for bad in [json!({"id":B}), json!({"id":B,"stored":17})] {
        observe(
            query(false, 0, true),
            json!({"statements":[{"id":A,"stored":EARLY},bad]}),
            None,
            &caps,
            "failed",
        );
    }
    observe(
        query(false, 0, true),
        records(&[], ""),
        None,
        &caps,
        "failed",
    );
    observe(
        query(false, 0, true),
        records(&[(A, EARLY)], ""),
        None,
        &caps,
        "not-run",
    );
    let before = captures("2023-01-01T00:00:00Z", UNKNOWN);
    observe(
        query(false, 0, true),
        records(&[(A, "2023-01-01T00:00:00Z")], ""),
        None,
        &before,
        "failed",
    );
}

#[test]
fn returned_boundary_prefix_and_cross_known_order_remain_decidable() {
    let caps = captures(UNKNOWN, LATE);
    let mut q = query(false, 1, true);
    observe(
        q.clone(),
        records(&[(A, "2023-01-01T00:00:00Z")], "/next"),
        None,
        &caps,
        "failed",
    );
    q["until_capture"] = json!("a");
    q.as_object_mut().expect("object").remove("since_capture");
    let caps = BTreeMap::from([("a".into(), EARLY.into()), ("b".into(), UNKNOWN.into())]);
    observe(q, records(&[(B, LATE)], "/next"), None, &caps, "failed");
    for descending in [false, true] {
        let (first, second) = if descending {
            (EARLY, LATE)
        } else {
            (LATE, EARLY)
        };
        let caps = captures(first, UNKNOWN);
        observe(
            query(descending, 0, false),
            records(&[(A, UNKNOWN), (B, second)], ""),
            None,
            &caps,
            "failed",
        );
        let caps = captures(second, UNKNOWN);
        observe(
            query(descending, 0, false),
            records(&[(A, UNKNOWN), (B, first)], ""),
            None,
            &caps,
            "not-run",
        );
    }
    let caps = captures("2017-01-02T00:00:00Z", UNKNOWN);
    observe(
        query(true, 1, false),
        records(&[(B, "2017-01-01T00:00:00Z")], "/next"),
        None,
        &caps,
        "failed",
    );
    let caps = captures(EARLY, UNKNOWN);
    observe(
        query(false, 1, false),
        records(&[(B, LATE)], "/next"),
        None,
        &caps,
        "failed",
    );
}

#[test]
fn json_time_unknowns_never_hide_utc_fields_or_later_comparisons() {
    let q = json!({"kind":"json-time","pointer":"/timestamp","expected":UNKNOWN,"utc":true});
    for value in ["2025-01-01T09:00:00+09:00", "2016-12-31T23:59:60+09:00"] {
        observe(
            q.clone(),
            json!({"timestamp":value}),
            None,
            &BTreeMap::new(),
            "failed",
        );
    }
    let q =
        json!({"kind":"json-time","pointer":"/timestamp","equals_pointer":"/stored","utc":true});
    for body in [
        json!({"timestamp":UNKNOWN}),
        json!({"timestamp":UNKNOWN,"stored":17}),
    ] {
        observe(q.clone(), body, None, &BTreeMap::new(), "failed");
    }
    for field in ["equals_pointer", "equals_capture"] {
        let mut q = json!({"kind":"json-time","pointer":"/timestamp","expected":UNKNOWN});
        q[field] = json!(if field == "equals_pointer" {
            "/stored"
        } else {
            "other"
        });
        observe(
            q,
            json!({"timestamp":EARLY,"stored":LATE}),
            None,
            &BTreeMap::from([("other".into(), LATE.into())]),
            "failed",
        );
    }
    observe(
        json!({"kind":"json-time","pointer":"/timestamp","expected":UNKNOWN,"utc":true}),
        json!({"timestamp":UNKNOWN}),
        None,
        &BTreeMap::new(),
        "not-run",
    );
}

#[test]
fn subnanosecond_query_and_order_use_exact_decimal_instants() {
    for (before, after) in [
        (
            "2025-01-01T00:00:00.0000000001Z",
            "2025-01-01T00:00:00.0000000002Z",
        ),
        (
            "1969-12-31T23:59:59.9999999998Z",
            "1969-12-31T23:59:59.9999999999Z",
        ),
        (
            "2024-12-31T23:59:59.9999999999Z",
            "2025-01-01T09:00:00.0000000001+09:00",
        ),
    ] {
        let caps = BTreeMap::from([
            ("a".into(), before.into()),
            ("b".into(), after.into()),
            ("since".into(), before.into()),
        ]);
        observe(
            query(false, 0, true),
            records(&[(B, after)], ""),
            None,
            &caps,
            "passed",
        );
        let mut inclusive = query(false, 0, false);
        inclusive["until_capture"] = json!("b");
        observe(
            inclusive,
            records(&[(A, before), (B, after)], ""),
            None,
            &caps,
            "passed",
        );
        for descending in [false, true] {
            let check = json!({"kind":"json-order","pointer":"/statements","by":"stored","descending":descending});
            let (first, last) = if descending {
                (after, before)
            } else {
                (before, after)
            };
            observe(
                check.clone(),
                records(&[(A, first), (B, last)], ""),
                None,
                &caps,
                "passed",
            );
            observe(
                check,
                records(&[(B, last), (A, first)], ""),
                None,
                &caps,
                "failed",
            );
        }
    }
}

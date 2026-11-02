//! Independently transcribed DC-MIME output controls. These qualify the oracle;
//! they are not Kiva executions or evidence of target acceptance.

use std::collections::BTreeMap;

use reqwest::header::HeaderMap;
use serde_json::{Value, json};

use crate::Check;

const HASH: &str = "13fc365da2ef41f756d160b40d4cdbfffd582ee569cf33fec523b00c366cbda5";
const RAW: &[u8] = b"portable\0binary\xffbytes";

struct Response {
    outer: String,
    first: String,
    metadata: Value,
    parts: Vec<(String, Vec<u8>)>,
    invalid_json: bool,
}

impl Response {
    fn valid() -> Self {
        Self {
            outer: "multipart/mixed; boundary=b".into(),
            first: "application/json".into(),
            metadata: json!({"attachments":[{"sha2":HASH,"length":21,"contentType":"application/octet-stream"}]}),
            parts: vec![(
                format!(
                    "Content-Type: application/octet-stream\r\nContent-Transfer-Encoding: binary\r\nX-Experience-API-Hash: {HASH}\r\nContent-Length: 21"
                ),
                RAW.to_vec(),
            )],
            invalid_json: false,
        }
    }

    fn evaluate(&self, extra_data: bool) -> Vec<String> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Content-Type",
            self.outer.parse().expect("fixed HTTP header"),
        );
        let json = if self.invalid_json {
            "{".into()
        } else {
            self.metadata.to_string()
        };
        let mut body =
            format!("--b\r\nContent-Type: {}\r\n\r\n{json}\r\n", self.first).into_bytes();
        for (fields, raw) in &self.parts {
            body.extend_from_slice(format!("--b\r\n{fields}\r\n\r\n").as_bytes());
            body.extend_from_slice(raw);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(b"--b--\r\n");
        let definition = if extra_data {
            json!({"kind":"multipart","expected_data_hex":["616263"]})
        } else {
            json!({"kind":"multipart"})
        };
        let check: Check = serde_json::from_value(definition).expect("fixed assertion");
        check.evaluate(200, &headers, &body, &BTreeMap::new())
    }
}

fn outcome(errors: &[String]) -> &'static str {
    if errors.is_empty() {
        "PASS"
    } else if errors.iter().all(|e| e.starts_with("ORACLE-UNAVAILABLE:")) {
        "NOT-RUN"
    } else {
        "FAIL"
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn duplicate_parameter_unknowns_never_mask_independent_mime_failures() {
    for fp in 1..=18 {
        let mut response = Response::valid();
        if [2, 3, 4, 10, 11].contains(&fp) {
            response.parts[0].0 = response.parts[0].0.replace(
                "application/octet-stream",
                "application/octet-stream; x=1; x=1",
            );
        }
        if [5, 6, 9, 12, 13].contains(&fp) {
            response.first = "application/json; charset=utf-8; charset=utf-8".into();
        }
        if [7, 8].contains(&fp) {
            response.metadata["attachments"][0]["contentType"] =
                json!("application/octet-stream; x=1; x=1");
        }
        if [3, 8].contains(&fp) {
            let mut later = response.parts[0].clone();
            later.0 = later.0.replace(HASH, &format!("0{}", &HASH[1..]));
            response.parts.push(later);
        }
        match fp {
            4 => {
                response.parts[0].0 = response.parts[0]
                    .0
                    .replace("Content-Length: 21", "Content-Length: 20");
            }
            6 => {
                response.parts[0].0 = response.parts[0]
                    .0
                    .replace(&format!("X-Experience-API-Hash: {HASH}\r\n"), "");
            }
            9 => response.invalid_json = true,
            10 => {
                let mut missing = response.metadata["attachments"][0].clone();
                missing["sha2"] = json!("0".repeat(64));
                response.metadata["attachments"]
                    .as_array_mut()
                    .expect("array")
                    .push(missing);
            }
            12 | 18 => {
                response.parts[0].0 = response.parts[0]
                    .0
                    .replace("application/octet-stream", "text/plain");
            }
            13 => {
                response.metadata["attachments"][0]["length"] =
                    serde_json::from_str("21.0").expect("exact decimal");
            }
            14 => response.outer = "multipart/mixed; boundary=b; boundary=c".into(),
            15 => response.first = "text/plain".into(),
            16 => response.first = "application".into(),
            17 => {
                response.parts[0].0 = response.parts[0]
                    .0
                    .replace("application/octet-stream", "application");
            }
            _ => {}
        }
        let errors = response.evaluate(fp == 11);
        let expected = if fp == 1 {
            "PASS"
        } else if [2, 5, 7, 13, 14].contains(&fp) {
            "NOT-RUN"
        } else {
            "FAIL"
        };
        assert_eq!(outcome(&errors), expected, "FP-{fp:02}: {errors:?}");
    }
    for (outer, first, wrong_hash, expected) in [
        (
            "multipart/mixed; boundary=b",
            "application/json",
            false,
            "PASS",
        ),
        ("text/plain; x=1; x=2", "application/json", false, "FAIL"),
        (
            "multipart/mixed; x=1; x=2",
            "application/json",
            false,
            "FAIL",
        ),
        (
            "multipart/mixed; boundary=b; boundary=c",
            "application/json",
            false,
            "NOT-RUN",
        ),
        (
            "multipart/mixed; boundary=b",
            "text/plain; x=1; x=2",
            false,
            "FAIL",
        ),
        (
            "multipart/mixed; boundary=b; x=1; x=2",
            "application/json",
            true,
            "FAIL",
        ),
        (
            "multipart/mixed; boundary=b; x=1; x=2",
            "application/json",
            false,
            "NOT-RUN",
        ),
        (
            "multipart/mixed; boundary=b; x=1; x=2; broken",
            "application/json",
            false,
            "FAIL",
        ),
    ] {
        let mut response = Response::valid();
        response.outer = outer.into();
        response.first = first.into();
        if wrong_hash {
            response.parts[0].1[0] ^= 1;
        }
        let errors = response.evaluate(false);
        assert_eq!(
            outcome(&errors),
            expected,
            "FFCR-01 {outer}/{first}: {errors:?}"
        );
    }
}

#[test]
fn typed_parameter_relations_and_output_header_roles_keep_their_limits() {
    for (metadata, part, expected) in [
        (
            "message/external-body; access-type=LOCAL-FILE",
            "message/external-body; access-type=local-file",
            "PASS",
        ),
        (
            "text/plain; charset=UTF-8",
            "text/plain; charset=utf-8",
            "PASS",
        ),
        ("text/plain; charset=US-ASCII", "text/plain", "PASS"),
        (
            "multipart/mixed; boundary=AA",
            "multipart/mixed; boundary=aa",
            "FAIL",
        ),
        ("text/plain; x=One", "text/plain; x=one", "NOT-RUN"),
        (
            "text/plain; charset=UTF-8; x=One",
            "text/plain; charset=US-ASCII; x=one",
            "FAIL",
        ),
        (
            "text/plain; x=One",
            "application/octet-stream; x=one",
            "FAIL",
        ),
        ("text/plain; charset=UTF-8", "text/plain; x=1; x=1", "FAIL"),
        ("text/plain; x=1; x=1", "text/plain", "NOT-RUN"),
        ("text/x-unbound; charset=UTF-8", "text/x-unbound", "NOT-RUN"),
        (
            "text/plain; charset=UTF-8; x=1; x=1",
            "text/plain; charset=US-ASCII",
            "FAIL",
        ),
    ] {
        let mut response = Response::valid();
        response.metadata["attachments"][0]["contentType"] = json!(metadata);
        response.parts[0].0 = response.parts[0]
            .0
            .replace("application/octet-stream", part);
        let errors = response.evaluate(false);
        assert_eq!(
            outcome(&errors),
            expected,
            "{metadata} / {part}: {errors:?}"
        );
    }
    for (encoding, expected) in [
        (Some("BiNaRy (nested (note))"), "PASS"),
        (Some("binary (escaped \\) note)"), "PASS"),
        (Some("\r\n\tbinary"), "PASS"),
        (None, "FAIL"),
        (Some("base64"), "FAIL"),
        (Some("\"binary\""), "FAIL"),
        (Some("binary extra"), "FAIL"),
        (Some("binary (open"), "FAIL"),
        (Some("binary (日本語)"), "FAIL"),
        (Some("binary (a\rb)"), "FAIL"),
        (Some("binary\n (note)"), "FAIL"),
    ] {
        let mut response = Response::valid();
        response.parts[0].0 = response.parts[0].0.replace(
            "Content-Transfer-Encoding: binary\r\n",
            &encoding.map_or_else(String::new, |v| {
                format!("Content-Transfer-Encoding: {v}\r\n")
            }),
        );
        let errors = response.evaluate(false);
        assert_eq!(
            outcome(&errors),
            expected,
            "output CTE {encoding:?}: {errors:?}"
        );
    }
    let mut optional = Response::valid();
    optional.parts[0].0 = optional.parts[0]
        .0
        .replace("Content-Type: application/octet-stream\r\n", "");
    assert!(
        optional.evaluate(false).is_empty(),
        "additional part type is optional"
    );
}

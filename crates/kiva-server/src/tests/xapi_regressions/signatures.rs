#![allow(clippy::wildcard_imports)]

use super::*;
use crate::surfaces::xapi::request::take_logical_observations;
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use jsonwebtoken::{Algorithm, EncodingKey};
use sha2::{Digest, Sha256};
use std::fmt::Write;

use crate::core::{
    config::SIGNATURE_ATTACHMENT_USAGE_TYPE, response::hex_lower, signature_policy::SignaturePolicy,
};

#[tokio::test]
async fn memory_xapi_signed_admission_checks_trust_and_preserves_batch_atomicity()
-> Result<(), String> {
    verify_signatures(memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_signed_admission_checks_trust_and_preserves_batch_atomicity()
-> Result<(), String> {
    verify_signatures(
        postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

fn signed_statement(algorithm: Algorithm, cause: &str) -> Result<(Value, String), String> {
    let mut received = statement(Uuid::new_v4());
    let mut payload = received.clone();
    // All variants are valid Statements; equivalence is tested independently of
    // schema admission. Excluded fields are deliberately different.
    payload["id"] = json!(Uuid::new_v4());
    payload["context"] = json!({"language":"en"});
    if cause.starts_with("opaque-") {
        received["context"] = json!({"extensions":{"https://example.org/owner":"OPAQUE-RECEIVED"}});
        payload["context"] = json!({"extensions":{"https://example.org/owner":"OPAQUE-SIGNED"}});
    }
    if cause == "core-mismatch" {
        payload["verb"]["id"] = json!("https://example.org/different");
    }
    let certificate = STANDARD.encode(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-certificate.der"
    )));
    let mut header = json!({"alg":algorithm,"x5c":[certificate]});
    if cause.starts_with("no-certificate") {
        header.as_object_mut().ok_or("header object")?.remove("x5c");
        header["kid"] = json!(
            if cause.ends_with("unknown") || cause.ends_with("malformed") {
                "Fixture"
            } else {
                "fixture"
            }
        );
    }
    if cause == "critical" {
        header["crit"] = json!(["extension"]);
        header["extension"] = json!(true);
    }
    if cause == "noncritical" {
        header["extension"] = json!(true);
        header["$serde_json::private::Number"] = json!("ordinary noncritical header");
        header["$serde_json::private::RawValue"] = json!("ordinary noncritical header");
    }
    let message = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header.to_string()),
        URL_SAFE_NO_PAD.encode(payload.to_string().replace(
            "\"OPAQUE-SIGNED\"",
            if cause == "opaque-multiplicity" {
                r#"{"a":2}"#
            } else {
                r#"{"a":2,"a":1}"#
            }
        ))
    );
    let ordinary_key = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-key.pem"
    ));
    let other_key = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-rollover-key.pem"
    ));
    let key = EncodingKey::from_rsa_pem(
        if cause.ends_with("key-mismatch") || cause.ends_with("unknown") {
            &other_key[..]
        } else {
            &ordinary_key[..]
        },
    )
    .map_err(|error| error.to_string())?;
    let mut signature = jsonwebtoken::crypto::sign(message.as_bytes(), &key, algorithm)
        .map_err(|error| error.to_string())?;
    if cause.ends_with("signature") {
        signature.replace_range(..1, if signature.starts_with('A') { "B" } else { "A" });
    }
    if cause.ends_with("malformed") {
        signature = "%%%".into();
    }
    Ok((received, format!("{message}.{signature}")))
}

fn multipart(items: &[(Value, String)], batch: bool, orphan: bool) -> (Value, Vec<u8>) {
    let values = items.iter().map(|(value, jws)| {
        let mut value = value.clone();
        value["attachments"] = json!([{"usageType":SIGNATURE_ATTACHMENT_USAGE_TYPE,"display":{"en":"signature"},"contentType":"application/octet-stream","length":jws.len(),"sha2":hex_lower(&Sha256::digest(jws.as_bytes()))}]);
        value
    }).collect::<Vec<_>>();
    let submitted = if batch {
        json!(values)
    } else {
        values[0].clone()
    };
    let mut body = format!(
        "--kiva-signature-boundary\r\nContent-Type: application/json\r\n\r\n{submitted}\r\n"
    );
    for (_, jws) in items {
        write!(&mut body, "--kiva-signature-boundary\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: binary\r\nX-Experience-API-Hash: {}\r\n\r\n{jws}\r\n", hex_lower(&Sha256::digest(jws.as_bytes()))).expect("String formatting");
    }
    if orphan {
        write!(&mut body, "--kiva-signature-boundary\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: binary\r\nX-Experience-API-Hash: {}\r\n\r\norphan\r\n", hex_lower(&Sha256::digest(b"orphan"))).expect("String formatting");
    }
    body.push_str("--kiva-signature-boundary--\r\n");
    (
        submitted,
        body.replace("\"OPAQUE-RECEIVED\"", r#"{"a":1,"a":2}"#)
            .into_bytes(),
    )
}

pub(super) async fn send(
    client: &reqwest::Client,
    base: &str,
    items: &[(Value, String)],
    batch: bool,
    orphan: bool,
) -> Result<reqwest::Response, String> {
    let (_, body) = multipart(items, batch, orphan);
    let request = if batch {
        client.post(format!("{base}/statements"))
    } else {
        client.put(format!(
            "{base}/statements?statementId={}",
            items[0].0["id"].as_str().ok_or("id")?
        ))
    };
    let reply = request
        .header("X-Experience-API-Version", "2.0.0")
        .header(
            "Content-Type",
            "multipart/mixed; boundary=kiva-signature-boundary",
        )
        .body(body)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    evidence::received(reply).await
}

#[allow(clippy::too_many_lines)]
async fn verify_signatures(mut state: AppState) -> Result<(), String> {
    let baseline: Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/controls/kiva_certificate_policy.json"
    )))
    .map_err(|error| error.to_string())?;
    for mode in [
        "good",
        "unconfigured",
        "unknown",
        "revoked",
        "stale",
        "outside-validity",
    ] {
        let mut policy = baseline.clone();
        policy["validation_time"] = json!("2026-10-27T00:00:00Z");
        match mode {
            "unknown" | "revoked" => policy["out_of_band_status"][0]["status"] = json!(mode),
            "stale" => {
                policy["out_of_band_status"][0]["valid_until"] = json!("2026-01-01T00:00:00Z");
            }
            "outside-validity" => policy["validation_time"] = json!("2035-01-01T00:00:00.001Z"),
            _ => {}
        }
        state.xapi.signature_policy = if mode == "unconfigured" {
            SignaturePolicy::default()
        } else {
            SignaturePolicy::from_json(policy.to_string().as_bytes())
                .map_err(|error| error.message)?
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|error| error.to_string())?;
        let base = format!(
            "http://{}",
            listener.local_addr().map_err(|error| error.to_string())?
        );
        let router = evidence::router(&state);
        let server = tokio::spawn(async move { axum::serve(listener, router).await });
        let client = reqwest::Client::new();
        for algorithm in [Algorithm::RS256, Algorithm::RS384, Algorithm::RS512] {
            for batch in [false, true] {
                let items = vec![signed_statement(algorithm, "valid")?];
                let response = send(&client, &base, &items, batch, false).await?;
                let expected = if mode == "good" {
                    if batch {
                        StatusCode::OK
                    } else {
                        StatusCode::NO_CONTENT
                    }
                } else {
                    StatusCode::BAD_REQUEST
                };
                assert_eq!(response.status(), expected, "{mode}/{algorithm:?}/{batch}");
                assert_eq!(response.headers()["X-Experience-API-Version"], "2.0.0");
                assert!(
                    response
                        .headers()
                        .contains_key("X-Experience-API-Consistent-Through")
                );
                let id = Uuid::parse_str(items[0].0["id"].as_str().ok_or("id")?)
                    .map_err(|error| error.to_string())?;
                let saved = state
                    .xapi
                    .statements
                    .get(id)
                    .await
                    .map_err(|error| format!("{error:?}"))?;
                assert_eq!(saved.is_some(), mode == "good");
                if let Some(saved) = saved {
                    assert_eq!(saved.attachments.len(), 1);
                    assert_eq!(
                        saved.attachments.values().next().ok_or("attachment")?.body,
                        items[0].1.as_bytes()
                    );
                    let read = client
                        .get(format!(
                            "{base}/statements?statementId={id}&attachments=true"
                        ))
                        .header("X-Experience-API-Version", "2.0.0")
                        .send()
                        .await
                        .map_err(|error| error.to_string())?;
                    let read = evidence::received(read).await?;
                    assert_eq!(read.status(), StatusCode::OK);
                    let bytes = read.bytes().await.map_err(|error| error.to_string())?;
                    assert!(
                        bytes
                            .windows(items[0].1.len())
                            .any(|part| part == items[0].1.as_bytes())
                    );
                } else {
                    assert!(
                        !response
                            .bytes()
                            .await
                            .map_err(|error| error.to_string())?
                            .is_empty()
                    );
                }
            }
        }
        if mode == "good" {
            verify_equivalence(&state, &client, &base).await?;
            for cause in [
                "valid",
                "noncritical",
                "opaque-order",
                "opaque-multiplicity",
                "core-mismatch",
                "signature",
                "critical",
                "orphan",
            ] {
                let items = vec![
                    signed_statement(Algorithm::RS256, "valid")?,
                    signed_statement(Algorithm::RS512, cause)?,
                ];
                let response = send(&client, &base, &items, true, cause == "orphan").await?;
                // Context is excluded from the issued signature invariant;
                // both object-order and content differences here are allowed.
                let accepted = matches!(
                    cause,
                    "valid" | "noncritical" | "opaque-order" | "opaque-multiplicity"
                );
                assert_eq!(
                    response.status(),
                    if accepted {
                        StatusCode::OK
                    } else {
                        StatusCode::BAD_REQUEST
                    },
                    "{cause}"
                );
                for (value, _) in &items {
                    let id = Uuid::parse_str(value["id"].as_str().ok_or("id")?)
                        .map_err(|error| error.to_string())?;
                    assert_eq!(
                        state
                            .xapi
                            .statements
                            .get(id)
                            .await
                            .map_err(|error| format!("{error:?}"))?
                            .is_some(),
                        accepted,
                        "batch must be atomic: {cause}"
                    );
                }
            }
        }
        server.abort();
    }
    Ok(())
}

fn sign_payload(payload: &Value) -> Result<String, String> {
    let certificate = STANDARD.encode(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-certificate.der"
    )));
    let header = json!({"alg":"RS256","x5c":[certificate]});
    let input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header.to_string()),
        URL_SAFE_NO_PAD.encode(payload.to_string())
    );
    let key = EncodingKey::from_rsa_pem(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-key.pem"
    )))
    .map_err(|e| e.to_string())?;
    let signature = jsonwebtoken::crypto::sign(input.as_bytes(), &key, Algorithm::RS256)
        .map_err(|e| e.to_string())?;
    Ok(format!("{input}.{signature}"))
}

async fn send_comparison(
    client: &reqwest::Client,
    base: &str,
    received: Value,
    payload: &Value,
    batch: bool,
    expected: bool,
) -> Result<reqwest::Response, String> {
    take_logical_observations();
    let response = send(
        client,
        base,
        &[(received, sign_payload(payload)?)],
        batch,
        false,
    )
    .await?;
    let calls = take_logical_observations();
    assert_eq!(
        calls.len(),
        1,
        "one actual comparator call after cryptographic verification"
    );
    assert_eq!(calls[0]["right"], *payload);
    assert_eq!(calls[0]["equivalent"], json!(expected));
    evidence::observation(
        "actual-logical-comparator",
        &json!({"batch":batch,"expected":expected,"calls":calls,
        "relation":"general 4.2 invariant core, applied to signatures by selected SP-01 policy; strict storage replay is separate"}),
    );
    Ok(response)
}

async fn verify_equivalence(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
) -> Result<(), String> {
    let mut template = statement(Uuid::nil());
    template["actor"] = json!({"objectType":"Group","mbox":"mailto:Group@example.org","member":[{"mbox":"mailto:a@example.org"},{"mbox":"mailto:b@example.org"}]});
    template["verb"] = json!({"id":"https://example.org/Verb","display":{"en":"original"}});
    template["object"] = json!({"id":"https://example.org/Activity","definition":{"name":{"en":"original"},"extensions":{"https://example.org/e":{"id":"opaque","timestamp":"arbitrary"}}}});
    template["result"] = json!({"duration":"PT1.011S","score":{"raw":1},"response":"old"});
    template["context"] = json!({"language":"en","extensions":{"https://example.org/e":{"timestamp":"opaque","id":"old","duration":"PT1.011S"}}});
    template["timestamp"] = json!("2020-01-01T00:00:00Z");
    let mutations = [
        ("/result/duration", json!("PT1.011S"), true),
        ("/verb/id", json!("HTTPS://example.org/Verb"), false),
        (
            "/context/extensions/https:~1~1example.org~1e/duration",
            json!("PT1.019S"),
            true,
        ),
        (
            "/actor/member",
            json!([{"mbox":"mailto:b@example.org"},{"mbox":"mailto:a@example.org"}]),
            true,
        ),
        ("/actor/mbox", json!("mailto:group@example.org"), false),
        (
            "/actor/member/0/mbox",
            json!("mailto:other@example.org"),
            false,
        ),
        ("/verb/id", json!("https://example.org/verb"), false),
        ("/verb/id", json!("https://EXAMPLE.org/Verb"), false),
        ("/verb/id", json!("https://example.org/%56erb"), false),
        ("/verb/id", json!("https://example.org/a/../Verb"), false),
        ("/verb/id", json!("https://example.org:443/Verb"), false),
        ("/object/id", json!("https://example.org/activity"), false),
        ("/verb/display", json!({"ja":"changed"}), true),
        ("/object/definition", json!({"name":{"en":"changed"}}), true),
        ("/result/duration", json!("PT1.019S"), true),
        ("/result/duration", json!("PT1.01S"), true),
        ("/result/duration", json!("PT1.02S"), false),
        ("/result/score/raw", json!(999), true),
        ("/result/response", json!("new"), true),
        (
            "/context",
            json!({"language":"ja","extensions":{"https://example.org/e":{"timestamp":"other","id":"changed"}}}),
            true,
        ),
        ("/timestamp", json!("2021-01-01T00:00:00Z"), true),
    ];
    for nested in [false, true] {
        for batch in [false, true] {
            for (pointer, replacement, equivalent) in &mutations {
                let mut received = template.clone();
                let id = Uuid::new_v4();
                received["id"] = json!(id);
                let pointer = if nested {
                    received.as_object_mut().ok_or("SubStatement")?.remove("id");
                    received["objectType"] = json!("SubStatement");
                    let mut root = statement(id);
                    root["object"] = received;
                    received = root;
                    format!("/object{pointer}")
                } else {
                    (*pointer).to_owned()
                };
                let mut payload = received.clone();
                *payload.pointer_mut(&pointer).ok_or("signature mutation")? = replacement.clone();
                let jws = sign_payload(&payload)?;
                let response =
                    send_comparison(client, base, received.clone(), &payload, batch, *equivalent)
                        .await?;
                assert_eq!(
                    response.status(),
                    if *equivalent {
                        if batch {
                            StatusCode::OK
                        } else {
                            StatusCode::NO_CONTENT
                        }
                    } else {
                        StatusCode::BAD_REQUEST
                    },
                    "{pointer}/{replacement}/{batch}"
                );
                let saved = state
                    .xapi
                    .statements
                    .get(id)
                    .await
                    .map_err(|e| format!("{e:?}"))?;
                assert_eq!(saved.is_some(), *equivalent);
                if let Some(saved) = saved {
                    assert_eq!(
                        saved
                            .attachments
                            .values()
                            .next()
                            .ok_or("raw signature")?
                            .body,
                        jws.as_bytes()
                    );
                    let read = evidence::received(
                        client
                            .get(format!("{base}/statements?statementId={id}"))
                            .header("X-Experience-API-Version", "2.0.0")
                            .send()
                            .await
                            .map_err(|e| e.to_string())?,
                    )
                    .await?;
                    assert_eq!(read.status(), StatusCode::OK);
                    let value: Value = read.json().await.map_err(|e| e.to_string())?;
                    // Signed payload exclusions never authorize replacing the
                    // actual received Statement with the signed variant.
                    assert_eq!(value.pointer(&pointer), received.pointer(&pointer));
                }
            }
        }
    }
    // Exact primitive-integration paper pair, separate from the1.011 core
    // comparison vectors above. Original signed bytes remain cryptographic input.
    for nested in [false, true] {
        for post in [false, true] {
            for (signed_duration, equivalent) in [("PT1.239S", true), ("PT1.24S", false)] {
                let id = Uuid::new_v4();
                let mut received = template.clone();
                received["id"] = json!(id);
                received["result"]["duration"] = json!("PT1.231S");
                if nested {
                    received.as_object_mut().ok_or("Statement")?.remove("id");
                    received["objectType"] = json!("SubStatement");
                    let mut root = statement(id);
                    root["object"] = received;
                    received = root;
                }
                let pointer = if nested {
                    "/object/result/duration"
                } else {
                    "/result/duration"
                };
                let mut payload = received.clone();
                *payload.pointer_mut(pointer).ok_or("duration")? = json!(signed_duration);
                let reply =
                    send_comparison(client, base, received, &payload, post, equivalent).await?;
                assert_eq!(
                    reply.status(),
                    if equivalent {
                        if post {
                            StatusCode::OK
                        } else {
                            StatusCode::NO_CONTENT
                        }
                    } else {
                        StatusCode::BAD_REQUEST
                    }
                );
                let saved = state
                    .xapi
                    .statements
                    .get(id)
                    .await
                    .map_err(|e| format!("{e:?}"))?;
                assert_eq!(saved.is_some(), equivalent);
                if let Some(saved) = saved {
                    assert_eq!(saved.statement.pointer(pointer), Some(&json!("PT1.231S")));
                }
            }
        }
    }
    for field in [
        "id",
        "timestamp",
        "stored",
        "authority",
        "version",
        "attachments",
    ] {
        let received = statement(Uuid::new_v4());
        let mut payload = received.clone();
        payload[field] = match field {
            "id" => json!(Uuid::new_v4()),
            "timestamp" | "stored" => json!("2020-01-01T00:00:00Z"),
            "authority" => json!({"mbox":"mailto:other-authority@example.org"}),
            "version" => json!("1.0.3"),
            _ => json!([]),
        };
        let response = send_comparison(client, base, received, &payload, false, true).await?;
        assert_eq!(
            response.status(),
            StatusCode::NO_CONTENT,
            "root exclusion {field}"
        );
    }
    for default in ["actor-agent", "member-agent", "object-activity"] {
        let mut received = template.clone();
        received["id"] = json!(Uuid::new_v4());
        if default == "actor-agent" {
            received["actor"] = json!({"mbox":"mailto:a@example.org"});
        }
        let mut payload = received.clone();
        match default {
            "actor-agent" => payload["actor"]["objectType"] = json!("Agent"),
            "member-agent" => payload["actor"]["member"][0]["objectType"] = json!("Agent"),
            _ => payload["object"]["objectType"] = json!("Activity"),
        }
        let response = send_comparison(client, base, received, &payload, false, true).await?;
        assert_eq!(
            response.status(),
            StatusCode::NO_CONTENT,
            "equivalent schema default: {default}"
        );
    }
    for object_type in ["Agent", "Group"] {
        for changed in [false, true] {
            let mut received = template.clone();
            received["id"] = json!(Uuid::new_v4());
            received["object"] =
                json!({"objectType":object_type,"mbox":"mailto:Object@example.org"});
            if object_type == "Group" {
                received["object"]["member"] =
                    json!([{"mbox":"mailto:a@example.org"},{"mbox":"mailto:b@example.org"}]);
            }
            let mut payload = received.clone();
            if changed {
                payload["object"]["mbox"] = json!("mailto:object@example.org");
            } else if object_type == "Group" {
                payload["object"]["member"] = json!([{"objectType":"Agent","mbox":"mailto:b@example.org"},{"mbox":"mailto:a@example.org"}]);
            }
            let response =
                send_comparison(client, base, received, &payload, false, !changed).await?;
            assert_eq!(
                response.status(),
                if changed {
                    StatusCode::BAD_REQUEST
                } else {
                    StatusCode::NO_CONTENT
                },
                "Object {object_type}/{changed}"
            );
        }
    }
    for invalid in ["key-case", "enum-case"] {
        let mut received = template.clone();
        let id = Uuid::new_v4();
        received["id"] = json!(id);
        let mut payload = received.clone();
        if invalid == "key-case" {
            let verb = payload["verb"].as_object_mut().ok_or("Verb")?;
            let id = verb.remove("id").ok_or("Verb ID")?;
            verb.insert("Id".into(), id);
        } else {
            payload["actor"]["objectType"] = json!("group");
        }
        let response = send(
            client,
            base,
            &[(received, sign_payload(&payload)?)],
            false,
            false,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{invalid}");
        assert!(
            state
                .xapi
                .statements
                .get(id)
                .await
                .map_err(|e| format!("{e:?}"))?
                .is_none()
        );
    }
    evidence::observation(
        "signature-iri-comparison",
        &json!({"selected_method":"RFC 3987 simple string comparison","scheme_host_percent_dot_default_port":"exact strings, no normalization or resolution","path_and_mailbox_local_case":"significant","invalid_json_key_and_enum_case":"rejected before equivalence"}),
    );
    Ok(())
}

#[tokio::test]
async fn memory_xapi_no_certificate_signatures_distinguish_known_key_and_structure()
-> Result<(), String> {
    verify_known_keys(memory_test_state_with_auth(AuthConfig::default())).await
}

#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_no_certificate_signatures_distinguish_known_key_and_structure()
-> Result<(), String> {
    verify_known_keys(
        postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

async fn verify_known_keys(mut state: AppState) -> Result<(), String> {
    use x509_parser::prelude::{FromDer, X509Certificate};
    let mut config: Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/controls/kiva_certificate_policy.json"
    )))
    .map_err(|e| e.to_string())?;
    let cert = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-certificate.der"
    ));
    let spki = X509Certificate::from_der(cert)
        .map_err(|e| e.to_string())?
        .1
        .public_key()
        .raw
        .to_vec();
    config["trusted_jws_keys"] = json!({"fixture":STANDARD.encode(spki)});
    state.xapi.signature_policy =
        SignaturePolicy::from_json(config.to_string().as_bytes()).map_err(|e| e.message)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let base = format!(
        "http://{}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    let router = evidence::router(&state);
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    for algorithm in [Algorithm::RS256, Algorithm::RS384, Algorithm::RS512] {
        for (cause, accepted) in [
            ("no-certificate-valid", true),
            ("no-certificate-unknown", true),
            ("no-certificate-signature", false),
            ("no-certificate-key-mismatch", false),
            ("no-certificate-malformed", false),
        ] {
            for batch in [false, true] {
                let items = vec![signed_statement(algorithm, cause)?];
                let reply = send(&client, &base, &items, batch, false).await?;
                assert_eq!(
                    reply.status(),
                    if accepted {
                        if batch {
                            StatusCode::OK
                        } else {
                            StatusCode::NO_CONTENT
                        }
                    } else {
                        StatusCode::BAD_REQUEST
                    },
                    "{algorithm:?}/{cause}/{batch}"
                );
                assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
                assert!(
                    reply
                        .headers()
                        .contains_key("x-experience-api-consistent-through")
                );
                let id = Uuid::parse_str(items[0].0["id"].as_str().ok_or("id")?)
                    .map_err(|e| e.to_string())?;
                let saved = state
                    .xapi
                    .statements
                    .get(id)
                    .await
                    .map_err(|e| format!("{e:?}"))?;
                assert_eq!(saved.is_some(), accepted);
            }
        }
    }
    server.abort();
    Ok(())
}

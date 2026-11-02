#![allow(clippy::wildcard_imports)]
use super::*;
use crate::core::{response::hex_lower, signature_policy::SignaturePolicy};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use sha2::{Digest, Sha256};

#[tokio::test]
async fn memory_xapi_independent_rsa_provider_boundaries() -> Result<(), String> {
    verify(memory_test_state_with_auth(AuthConfig::default())).await
}
#[tokio::test]
#[ignore = "requires KIVA_POSTGRES_TEST_DATABASE_URL with all migrations applied"]
async fn postgres_xapi_independent_rsa_provider_boundaries() -> Result<(), String> {
    verify(
        postgres_test_state()
            .await?
            .ok_or("PostgreSQL URL must be set")?,
    )
    .await
}

#[allow(clippy::too_many_lines)]
async fn verify(mut state: AppState) -> Result<(), String> {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../tests/conformance/fixtures/jws-provider.json"
    ))
    .map_err(|e| e.to_string())?;
    let baseline: Value = serde_json::from_slice(include_bytes!(
        "../../../../../tests/conformance/controls/kiva_certificate_policy.json"
    ))
    .map_err(|e| e.to_string())?;
    let weak_cert = include_bytes!(
        "../../../../../tests/conformance/fixtures/signing/synthetic-2047-certificate.der"
    );
    let mut weak = baseline.clone();
    weak["trusted_jws_keys"] = json!({"fixture2048":fixture["keys"]["2048"]["spki_der_base64"]});
    weak["trust_anchors_der"] = json!([STANDARD.encode(weak_cert)]);
    weak["out_of_band_status"] = json!([{"certificate_sha256":hex_lower(&Sha256::digest(weak_cert)),"issuer_spki_sha256":hex_lower(&Sha256::digest(STANDARD.decode(fixture["keys"]["2047"]["spki_der_base64"].as_str().ok_or("SPKI")?).map_err(|e|e.to_string())?)),"authenticated_source":"independent synthetic key boundary fixture","valid_from":"2025-01-01T00:00:00Z","valid_until":"2035-01-01T00:00:00Z","status":"good"}]);
    weak["validation_time"] = json!("2026-10-28T00:00:00Z");
    let mut strong = baseline.clone();
    strong["trusted_jws_keys"] = json!({"fixture2048":fixture["keys"]["2048"]["spki_der_base64"]});
    let mut configured_weak = baseline.clone();
    configured_weak["trusted_jws_keys"] =
        json!({"fixture2047":fixture["keys"]["2047"]["spki_der_base64"]});
    assert!(
        SignaturePolicy::from_json(configured_weak.to_string().as_bytes()).is_err(),
        "no-x5c configured keys retain the exact modulus minimum"
    );
    // Weak x5c has valid DER, mathematical signature, pre-existing anchor,
    // current bound good status and a valid complete payload. Only the JWS
    // minimum public modulus length is violated.
    for bits in [2047, 2048] {
        state.xapi.signature_policy = SignaturePolicy::from_json(
            if bits == 2047 { &weak } else { &strong }
                .to_string()
                .as_bytes(),
        )
        .map_err(|e| e.message)?;
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
        for vector in fixture["vectors"]
            .as_array()
            .ok_or("vectors")?
            .iter()
            .filter(|v| v["bits"] == bits)
        {
            let mut jws = format!(
                "{}.{}",
                vector["message_ascii"].as_str().ok_or("message")?,
                vector["signature_base64url"].as_str().ok_or("signature")?
            );
            if bits == 2047 {
                // This variant binds the independently generated weak key to
                // x5c. The fixture's no-x5c original bytes remain separately
                // preserved for direct known-key provider controls below.
                let header = json!({"alg":vector["alg"],"x5c":[STANDARD.encode(weak_cert)]});
                let payload = vector["message_ascii"]
                    .as_str()
                    .ok_or("message")?
                    .split('.')
                    .nth(1)
                    .ok_or("payload")?;
                let message = format!("{}.{payload}", URL_SAFE_NO_PAD.encode(header.to_string()));
                // Ring signing itself rejects a small key. Use the independent
                // producer's x5c variant from the frozen fixture instead.
                jws = vector["x5c_jws"]
                    .as_str()
                    .ok_or("independent weak x5c signature")?
                    .to_owned();
                assert_eq!(jws.rsplit_once('.').ok_or("JWS")?.0, message);
            }
            for batch in [false, true] {
                let id = Uuid::new_v4();
                let mut value = fixture["payload"].clone();
                value["id"] = json!(id);
                let before = state
                    .xapi
                    .statements
                    .settled_snapshot()
                    .await
                    .map_err(|e| format!("{e:?}"))?;
                let items = if batch {
                    vec![
                        (statement(Uuid::new_v4()), valid_sibling(&fixture)?),
                        (value.clone(), jws.clone()),
                        (statement(Uuid::new_v4()), valid_sibling(&fixture)?),
                    ]
                } else {
                    vec![(value.clone(), jws.clone())]
                };
                // Sibling payload identities must match the independently
                // signed payload, even though their Statement ids are fresh.
                let items = items
                    .into_iter()
                    .map(|(value, sig)| {
                        let mut received = fixture["payload"].clone();
                        received["id"] = value["id"].clone();
                        (received, sig)
                    })
                    .collect::<Vec<_>>();
                crate::core::signature_policy::take_jws_observations();
                let reply = signatures::send(&client, &base, &items, batch, false).await?;
                let provider_calls = crate::core::signature_policy::take_jws_observations();
                let accepted = vector["jws_permitted"].as_bool().ok_or("expected")?;
                if bits == 2048 {
                    let digest = hex_lower(&Sha256::digest(
                        vector["message_ascii"]
                            .as_str()
                            .ok_or("message")?
                            .as_bytes(),
                    ));
                    let signature_digest = hex_lower(&Sha256::digest(
                        vector["signature_base64url"]
                            .as_str()
                            .ok_or("fixture signature")?
                            .as_bytes(),
                    ));
                    let relevant = provider_calls
                        .iter()
                        .filter(|v| {
                            v["message_sha256"] == digest
                                && v["signature_sha256"] == signature_digest
                        })
                        .collect::<Vec<_>>();
                    assert!(!relevant.is_empty(), "actual provider was called");
                    assert!(
                        relevant
                            .iter()
                            .all(|v| v["outcome"] == if accepted { "verified" } else { "failed" })
                    );
                }
                evidence::observation("actual-jws-provider-calls", &json!(provider_calls));
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
                    "{}/batch={batch}",
                    vector["id"]
                );
                assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
                assert!(
                    reply
                        .headers()
                        .contains_key("x-experience-api-consistent-through")
                );
                let _ = reply.bytes().await.map_err(|e| e.to_string())?;
                if !accepted {
                    assert_eq!(
                        before,
                        state
                            .xapi
                            .statements
                            .settled_snapshot()
                            .await
                            .map_err(|e| format!("{e:?}"))?
                    );
                }
                for (received, signature) in &items {
                    let reply = evidence::received(
                        client
                            .get(format!(
                                "{base}/statements?statementId={}&attachments=true",
                                received["id"].as_str().ok_or("id")?
                            ))
                            .header("X-Experience-API-Version", "2.0.0")
                            .send()
                            .await
                            .map_err(|e| e.to_string())?,
                    )
                    .await?;
                    assert_eq!(
                        reply.status(),
                        if accepted {
                            StatusCode::OK
                        } else {
                            StatusCode::NOT_FOUND
                        }
                    );
                    if accepted {
                        let parts = attachment_boundaries::decoded(reply).await?;
                        assert_eq!(parts.len(), 2);
                        assert_eq!(parts[1].1, signature.as_bytes());
                    }
                }
                evidence::observation(
                    "independent-rsa-interface",
                    &json!({"vector":vector,"batch":batch,"openssl":fixture["openssl"],"jws_minimum_bits":2048,"source_boundary":"Mathematical validity is distinct from JWS key-size and signature-length eligibility; producer and target implementations differ"}),
                );
            }
        }
        if bits == 2048 {
            verify_jose(&state, &client, &base, &fixture).await?;
        }
        server.abort();
    }
    verify_fractional_certificate(state, &fixture, strong).await
}

async fn verify_fractional_certificate(
    mut state: AppState,
    fixture: &Value,
    mut policy: Value,
) -> Result<(), String> {
    let precise = &fixture["fractional_certificate"];
    let encoded = precise["der_base64"]
        .as_str()
        .ok_or("precise certificate")?;
    let bytes = STANDARD.decode(encoded).map_err(|e| e.to_string())?;
    policy["trust_anchors_der"] = json!([encoded]);
    policy["out_of_band_status"][0]["certificate_sha256"] =
        json!(hex_lower(&Sha256::digest(&bytes)));
    for clock in precise["clock_cases"].as_array().ok_or("clock cases")? {
        policy["validation_time"] = clock["time"].clone();
        state.xapi.signature_policy =
            SignaturePolicy::from_json(policy.to_string().as_bytes()).map_err(|e| e.message)?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| e.to_string())?;
        let base = format!(
            "http://{}",
            listener.local_addr().map_err(|e| e.to_string())?
        );
        let router = evidence::router(&state);
        let server = tokio::spawn(async move { axum::serve(listener, router).await });
        let mut run = fixture.clone();
        run["jose_vectors"] = precise["vectors"].clone();
        for v in run["jose_vectors"]
            .as_array_mut()
            .ok_or("fraction vectors")?
        {
            v["accepted"] = clock["accepted"].clone();
        }
        evidence::observation(
            "original-certificate-time",
            &json!({"clock":clock,"fixture":precise}),
        );
        verify_jose(&state, &reqwest::Client::new(), &base, &run).await?;
        server.abort();
    }
    Ok(())
}

fn valid_sibling(fixture: &Value) -> Result<String, String> {
    let v = fixture["vectors"]
        .as_array()
        .ok_or("vectors")?
        .iter()
        .find(|v| v["id"] == "2048-RS256-valid")
        .ok_or("positive sibling")?;
    Ok(format!(
        "{}.{}",
        v["message_ascii"].as_str().ok_or("message")?,
        v["signature_base64url"].as_str().ok_or("signature")?
    ))
}

#[allow(clippy::too_many_lines)]
async fn verify_jose(
    state: &AppState,
    client: &reqwest::Client,
    base: &str,
    fixture: &Value,
) -> Result<(), String> {
    for vector in fixture["jose_vectors"].as_array().ok_or("JOSE vectors")? {
        let signature = vector["jws"].as_str().ok_or("compact JWS")?;
        let accepted = vector["accepted"].as_bool().ok_or("declared result")?;
        for batch in [false, true] {
            let mut received = fixture["payload"].clone();
            received["id"] = json!(Uuid::new_v4());
            let mut items = vec![(received.clone(), signature.to_owned())];
            if batch {
                for at in [0, 2] {
                    let mut sibling = fixture["payload"].clone();
                    sibling["id"] = json!(Uuid::new_v4());
                    items.insert(at, (sibling, valid_sibling(fixture)?));
                }
            }
            let baseline = if accepted {
                None
            } else {
                Some(
                    state
                        .xapi
                        .statements
                        .settled_snapshot()
                        .await
                        .map_err(|e| format!("{e:?}"))?,
                )
            };
            crate::core::signature_policy::take_jws_observations();
            let reply = signatures::send(client, base, &items, batch, false).await?;
            let provider_calls = crate::core::signature_policy::take_jws_observations();
            if vector["role"] == "D10-unknown-not-evaluated" {
                let matching = provider_calls
                    .iter()
                    .filter(|v| v["kid"] == "Fixture2048")
                    .collect::<Vec<_>>();
                assert_eq!(matching.len(), 1);
                assert_eq!(matching[0]["outcome"], "not-evaluated");
            }
            evidence::observation("actual-jws-provider-calls", &json!(provider_calls));
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
                "{}/batch={batch}",
                vector["id"]
            );
            assert_eq!(reply.headers()["x-experience-api-version"], "2.0.0");
            assert!(
                reply
                    .headers()
                    .contains_key("x-experience-api-consistent-through")
            );
            let _ = reply.bytes().await.map_err(|e| e.to_string())?;
            if let Some(baseline) = baseline {
                assert_eq!(
                    baseline,
                    state
                        .xapi
                        .statements
                        .settled_snapshot()
                        .await
                        .map_err(|e| format!("{e:?}"))?
                );
            }
            for (value, raw) in &items {
                let reply = evidence::received(
                    client
                        .get(format!(
                            "{base}/statements?statementId={}&attachments=true",
                            value["id"].as_str().ok_or("id")?
                        ))
                        .header("X-Experience-API-Version", "2.0.0")
                        .send()
                        .await
                        .map_err(|e| e.to_string())?,
                )
                .await?;
                assert_eq!(
                    reply.status(),
                    if accepted {
                        StatusCode::OK
                    } else {
                        StatusCode::NOT_FOUND
                    }
                );
                if accepted {
                    let parts = attachment_boundaries::decoded(reply).await?;
                    assert_eq!(parts.len(), 2);
                    assert_eq!(parts[1].1, raw.as_bytes());
                }
            }
            // Identical malformed JWS octets are ordinary opaque data when
            // the attachment's usageType does not select signature semantics.
            if !accepted {
                let id = Uuid::new_v4();
                let mut ordinary = fixture["payload"].clone();
                ordinary["id"] = json!(id);
                let hash = hex_lower(&Sha256::digest(signature.as_bytes()));
                ordinary["attachments"] = json!([{"usageType":"https://example.org/opaque-not-signature","display":{"en":"opaque bytes"},"contentType":"application/octet-stream","length":signature.len(),"sha2":hash}]);
                let request = if batch {
                    client.post(format!("{base}/statements"))
                } else {
                    client.put(format!("{base}/statements?statementId={id}"))
                };
                let reply = evidence::received(
                    request
                        .header("X-Experience-API-Version", "2.0.0")
                        .header("Content-Type", "multipart/mixed; boundary=sha2-control")
                        .body(attachment_boundaries::encoded(
                            &ordinary,
                            signature.as_bytes(),
                            &hash,
                            batch,
                        ))
                        .send()
                        .await
                        .map_err(|e| e.to_string())?,
                )
                .await?;
                assert_eq!(
                    reply.status(),
                    if batch {
                        StatusCode::OK
                    } else {
                        StatusCode::NO_CONTENT
                    },
                    "ordinary attachment {}",
                    vector["id"]
                );
                let reply = evidence::received(
                    client
                        .get(format!(
                            "{base}/statements?statementId={id}&attachments=true"
                        ))
                        .header("X-Experience-API-Version", "2.0.0")
                        .send()
                        .await
                        .map_err(|e| e.to_string())?,
                )
                .await?;
                assert_eq!(reply.status(), StatusCode::OK);
                let parts = attachment_boundaries::decoded(reply).await?;
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[1].1, signature.as_bytes());
            }
            evidence::observation(
                "jose-character-and-structure",
                &json!({"vector_id":vector["id"],"role":vector["role"],"accepted":accepted,"batch":batch,"original_bytes_retained":true,"independent_input_sha256":fixture["input_sha256"],"kid_case":"exact decoded names; unknown key is D10 crypto not-evaluated","crit":"unsupported critical rejected; malformed crit rejected under receiver MAY","duplicates":"D11 rejects decoded duplicates, including non-BMP escape equivalents"}),
            );
        }
    }
    Ok(())
}

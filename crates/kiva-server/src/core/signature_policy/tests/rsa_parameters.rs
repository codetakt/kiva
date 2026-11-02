//! DC-DER / DV-A06: distinguish ASN.1 NULL from empty tag-5 lookalikes.
#![allow(clippy::unwrap_used)]

use super::{SignaturePolicy, chain, config, encoded, extension_schema::ExtensionCase, fixtures};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, EncodingKey};
use serde_json::json;
use sha2::{Digest, Sha256};

// Splice original TLVs, including invalid parameters that typed encoders reject.
fn content(bytes: &[u8]) -> &[u8] {
    &bytes[if bytes[1] < 128 {
        2
    } else {
        2 + usize::from(bytes[1] & 127)
    }..]
}

fn fields(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut body = content(bytes);
    let mut result = Vec::new();
    while !body.is_empty() {
        let header = if body[1] < 128 {
            2
        } else {
            2 + usize::from(body[1] & 127)
        };
        let length = if header == 2 {
            usize::from(body[1])
        } else {
            body[2..header]
                .iter()
                .fold(0_usize, |n, b| n * 256 + usize::from(*b))
        };
        result.push(body[..header + length].to_vec());
        body = &body[header + length..];
    }
    result
}

fn replace_parameters(algorithm: &[u8], parameter: &[u8]) -> Vec<u8> {
    fixtures::wrap(48, &[fields(algorithm)[0].as_slice(), parameter].concat())
}

fn parameters(certificate: &[u8], location: &str, parameter: &[u8]) -> Vec<u8> {
    let mut outer = fields(certificate);
    let mut tbs = fields(&outer[0]);
    if matches!(location, "inner" | "both") {
        tbs[2] = replace_parameters(&tbs[2], parameter);
    }
    if matches!(location, "outer" | "both") {
        outer[1] = replace_parameters(&outer[1], parameter);
    }
    if location == "spki" {
        let mut key = fields(&tbs[6]);
        key[0] = replace_parameters(&key[0], parameter);
        tbs[6] = fixtures::wrap(48, &key.concat());
    }
    outer[0] = fixtures::wrap(48, &tbs.concat());
    let key = EncodingKey::from_rsa_pem(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-key.pem"
    )))
    .unwrap();
    let signature = jsonwebtoken::crypto::sign(&outer[0], &key, Algorithm::RS256).unwrap();
    outer[2] = fixtures::wrap(
        3,
        &[vec![0], URL_SAFE_NO_PAD.decode(signature).unwrap()].concat(),
    );
    fixtures::wrap(48, &outer.concat())
}

pub(crate) fn integration_cases() -> Vec<ExtensionCase> {
    let (root, original) = chain();
    let variants: [(&str, &[u8], bool); 11] = [
        ("null", &[5, 0], true),
        ("absent", &[], true),
        ("context-primitive-5", &[0x85, 0], false),
        ("context-constructed-5", &[0xa5, 0], false),
        ("application-primitive-5", &[0x45, 0], false),
        ("application-constructed-5", &[0x65, 0], false),
        ("private-primitive-5", &[0xc5, 0], false),
        ("private-constructed-5", &[0xe5, 0], false),
        ("constructed-universal-null", &[0x25, 0], false),
        ("nonempty-null", &[5, 1, 0], false),
        ("empty-octet-string", &[4, 0], false),
    ];
    let mut cases = Vec::new();
    for at in 0..2 {
        for location in ["inner", "outer", "both", "spki"] {
            for (name, parameter, valid) in variants {
                let mut path = original.clone();
                path[at] = parameters(&path[at], location, parameter);
                let mut policy = config(&root, &path);
                // Bind status to the actual issuer SPKI, including negative DER.
                for (i, issuer) in [&path[1], &root].into_iter().enumerate() {
                    let issuer_spki = fields(&fields(issuer)[0])[6].clone();
                    policy["out_of_band_status"][i]["issuer_spki_sha256"] =
                        json!(format!("{:x}", Sha256::digest(issuer_spki)));
                }
                cases.push(ExtensionCase {
                    label: format!("{at}/{location}/{name}"),
                    policy,
                    chain: path.iter().map(encoded).collect(),
                    accepted: valid && (location != "spki" || name != "absent"),
                });
            }
        }
    }
    cases
}

#[test]
fn rsa_parameter_types_govern_path_validation() {
    let mut failures = Vec::new();
    for case in integration_cases() {
        let result = SignaturePolicy::from_json(case.policy.to_string().as_bytes())
            .unwrap()
            .validate_chain(&case.chain)
            .map(|_| ());
        if result.is_ok() != case.accepted {
            failures.push(format!("{}: {result:?}", case.label));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

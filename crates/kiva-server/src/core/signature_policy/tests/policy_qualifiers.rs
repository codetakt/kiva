//! Signed receiver controls for RFC5280 `CertificatePolicies` qualifiers.
#![allow(clippy::unwrap_used)]

use super::{SignaturePolicy, certificate, chain, config, encoded, fixtures};
use ::der::{
    Decode, Encode,
    asn1::{ObjectIdentifier, OctetString},
};
use jsonwebtoken::Algorithm;
use serde_json::{Value, json};

const POLICY: &str = "1.2.3.4";
const CPS: &str = "1.3.6.1.5.5.7.2.1";
const NOTICE: &str = "1.3.6.1.5.5.7.2.2";
const UNKNOWN: &str = "1.2.3.999";

pub(crate) struct QualifierCase {
    pub label: String,
    pub policy: Value,
    pub chain: Vec<String>,
    pub accepted: bool,
}

fn seq(parts: &[Vec<u8>]) -> Vec<u8> {
    fixtures::wrap(48, &parts.concat())
}
fn oid(value: &str) -> Vec<u8> {
    ObjectIdentifier::new(value).unwrap().to_der().unwrap()
}
fn qualifier(identifier: &str, value: Vec<u8>) -> Vec<u8> {
    seq(&[oid(identifier), value])
}
fn policies(qualifiers: &[Vec<u8>]) -> Vec<u8> {
    seq(&[seq(&[oid(POLICY), seq(qualifiers)])])
}

struct Vector {
    label: &'static str,
    bytes: Vec<u8>,
    critical: bool,
    noncritical: bool,
}

#[allow(clippy::too_many_lines)] // Keep the independently enumerated fixture table together.
fn vectors() -> Vec<Vector> {
    let text = fixtures::wrap(12, "署名の通知".as_bytes());
    let reference = seq(&[
        fixtures::wrap(12, b"Review organization"),
        seq(&[fixtures::wrap(2, &[0xff]), fixtures::wrap(2, &[1; 24])]),
    ]);
    let mut vectors = Vec::new();
    for (label, identifier, value, valid) in [
        (
            "cps-uri",
            CPS,
            fixtures::wrap(22, b"https://example.org/cps"),
            true,
        ),
        (
            "cps-uri-without-fetch",
            CPS,
            fixtures::wrap(22, b"urn:example:cps"),
            true,
        ),
        ("cps-integer", CPS, fixtures::wrap(2, &[1]), false),
        (
            "cps-relative",
            CPS,
            fixtures::wrap(22, b"relative/cps"),
            false,
        ),
        ("notice-empty", NOTICE, seq(&[]), true),
        (
            "notice-utf8",
            NOTICE,
            seq(std::slice::from_ref(&text)),
            true,
        ),
        (
            "notice-ia5",
            NOTICE,
            seq(&[fixtures::wrap(22, b"notice")]),
            true,
        ),
        (
            "notice-visible",
            NOTICE,
            seq(&[fixtures::wrap(26, b"notice")]),
            true,
        ),
        (
            "notice-bmp",
            NOTICE,
            seq(&[fixtures::wrap(30, &[0x65, 0xe5, 0x67, 0x2c])]),
            true,
        ),
        // Issuer advice is not a receiver rejection rule. RFC5280 specifically
        // recommends handling explicitText longer than 200 characters.
        (
            "notice-long",
            NOTICE,
            seq(&[fixtures::wrap(12, &vec![b'n'; 201])]),
            true,
        ),
        (
            "notice-control",
            NOTICE,
            seq(&[fixtures::wrap(12, b"notice\0")]),
            true,
        ),
        (
            "notice-reference",
            NOTICE,
            seq(std::slice::from_ref(&reference)),
            true,
        ),
        (
            "notice-reference-and-text",
            NOTICE,
            seq(&[reference, text.clone()]),
            true,
        ),
        (
            "notice-empty-numbers",
            NOTICE,
            seq(&[seq(&[text.clone(), seq(&[])])]),
            true,
        ),
        ("notice-integer", NOTICE, fixtures::wrap(2, &[1]), false),
        (
            "notice-text-integer",
            NOTICE,
            seq(&[fixtures::wrap(2, &[1])]),
            false,
        ),
        (
            "notice-empty-text",
            NOTICE,
            seq(&[fixtures::wrap(12, b"")]),
            false,
        ),
        (
            "notice-missing-numbers",
            NOTICE,
            seq(&[seq(std::slice::from_ref(&text))]),
            false,
        ),
        (
            "notice-number-string",
            NOTICE,
            seq(&[seq(&[text.clone(), seq(std::slice::from_ref(&text))])]),
            false,
        ),
        (
            "notice-repeated-text",
            NOTICE,
            seq(&[text.clone(), text]),
            false,
        ),
        // Upstream x509-cert incorrectly types noticeRef as GeneralizedTime.
        (
            "notice-time-reference",
            NOTICE,
            seq(&[fixtures::wrap(24, b"20260101000000Z")]),
            false,
        ),
    ] {
        vectors.push(Vector {
            label,
            bytes: policies(&[qualifier(identifier, value)]),
            critical: valid,
            noncritical: valid,
        });
    }
    let unknown = qualifier(UNKNOWN, fixtures::wrap(12, b"unknown policy meaning"));
    let good = qualifier(CPS, fixtures::wrap(22, b"https://example.org/cps"));
    vectors.extend([
        Vector {
            label: "unknown",
            bytes: policies(std::slice::from_ref(&unknown)),
            critical: false,
            noncritical: true,
        },
        Vector {
            label: "known-then-unknown",
            bytes: policies(&[good.clone(), unknown.clone()]),
            critical: false,
            noncritical: true,
        },
        Vector {
            label: "unknown-then-malformed-known",
            bytes: policies(&[unknown, qualifier(CPS, fixtures::wrap(2, &[1]))]),
            critical: false,
            noncritical: false,
        },
        Vector {
            label: "two-known",
            bytes: policies(&[good, qualifier(NOTICE, seq(&[]))]),
            critical: true,
            noncritical: true,
        },
        Vector {
            label: "missing-value",
            bytes: policies(&[seq(&[oid(CPS)])]),
            critical: false,
            noncritical: false,
        },
        Vector {
            label: "missing-unknown-value",
            bytes: policies(&[seq(&[oid(UNKNOWN)])]),
            critical: false,
            noncritical: false,
        },
        Vector {
            label: "empty-qualifiers",
            bytes: policies(&[]),
            critical: false,
            noncritical: false,
        },
        Vector {
            label: "empty-policies",
            bytes: seq(&[]),
            critical: false,
            noncritical: false,
        },
    ]);
    vectors
}

fn with_policies(bytes: &[u8], value: &[u8], critical: bool) -> Vec<u8> {
    let mut certificate = x509_cert::Certificate::from_der(bytes).unwrap();
    let extension = certificate
        .tbs_certificate
        .extensions
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|extension| extension.extn_id.to_string() == "2.5.29.32")
        .unwrap();
    extension.critical = critical;
    extension.extn_value = OctetString::new(value).unwrap();
    fixtures::resign(certificate, Algorithm::RS256)
}

fn cases(tree_present: bool) -> Vec<QualifierCase> {
    let (root, _) = chain();
    let mut upper_extensions = vec![
        ("basicConstraints", "critical,CA:TRUE"),
        ("keyUsage", "keyCertSign"),
    ];
    if tree_present {
        upper_extensions.push(("certificatePolicies", POLICY));
    }
    let upper = certificate("Qualifier upper CA", Some(&root), &[30], &upper_extensions);
    let ca = certificate(
        "Qualifier CA",
        Some(&upper),
        &[31],
        &[
            ("basicConstraints", "critical,CA:TRUE"),
            ("keyUsage", "keyCertSign"),
            ("certificatePolicies", POLICY),
        ],
    );
    let leaf = certificate(
        "Qualifier signer",
        Some(&ca),
        &[32],
        &[
            ("keyUsage", "digitalSignature"),
            ("certificatePolicies", POLICY),
        ],
    );
    let mut cases = Vec::new();
    for vector in vectors() {
        for critical in [false, true] {
            for at in 0..2 {
                let mut path = vec![leaf.clone(), ca.clone(), upper.clone()];
                path[at] = with_policies(&path[at], &vector.bytes, critical);
                let mut policy = config(&root, &path);
                policy["initial_policy_set"] = json!([POLICY]);
                policy["explicit_policy"] = json!(tree_present);
                cases.push(QualifierCase {
                    label: format!(
                        "{}/{at}/critical={critical}/tree={tree_present}",
                        vector.label
                    ),
                    policy,
                    chain: path.iter().map(encoded).collect(),
                    accepted: if critical {
                        vector.critical
                    } else {
                        vector.noncritical
                    },
                });
            }
        }
    }
    cases
}

pub(crate) fn integration_cases() -> Vec<QualifierCase> {
    cases(true)
}

#[test]
fn policy_qualifiers_obey_receiver_contract_even_without_policy_tree() {
    for case in cases(true).into_iter().chain(cases(false)) {
        let result = SignaturePolicy::from_json(case.policy.to_string().as_bytes())
            .unwrap()
            .validate_chain(&case.chain);
        assert_eq!(result.is_ok(), case.accepted, "{}: {result:?}", case.label);
    }
}

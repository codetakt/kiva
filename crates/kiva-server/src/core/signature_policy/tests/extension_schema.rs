//! Signed controls for extension cardinalities and unbounded constraint integers.
#![allow(clippy::unwrap_used)]

use super::{SignaturePolicy, certificate, chain, config, encoded, fixtures};
use ::der::{
    Decode, Encode,
    asn1::{ObjectIdentifier, OctetString},
};
use jsonwebtoken::Algorithm;
use serde_json::{Value, json};

const POLICY: &str = "1.2.3.4";

pub(crate) struct ExtensionCase {
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

fn replace(bytes: &[u8], identifier: &str, value: &[u8], critical: bool) -> Vec<u8> {
    let mut cert = x509_cert::Certificate::from_der(bytes).unwrap();
    let extensions = cert.tbs_certificate.extensions.as_mut().unwrap();
    let identifier = ObjectIdentifier::new(identifier).unwrap();
    extensions.retain(|extension| extension.extn_id != identifier);
    extensions.push(x509_cert::ext::Extension {
        extn_id: identifier,
        critical,
        extn_value: OctetString::new(value).unwrap(),
    });
    fixtures::resign(cert, Algorithm::RS256)
}

struct Vector {
    label: String,
    oid: &'static str,
    bytes: Vec<u8>,
    accepted: bool,
    leaf: bool,
}

fn integer_vectors() -> Vec<Vector> {
    let mut result = Vec::new();
    for (label, bytes, accepted) in [
        ("zero", vec![0], true),
        ("one", vec![1], true),
        ("127", vec![127], true),
        ("128", vec![0, 128], true),
        ("255", vec![0, 255], true),
        ("256", vec![1, 0], true),
        ("u32-max", vec![0, 255, 255, 255, 255], true),
        ("u32-plus-one", vec![1, 0, 0, 0, 0], true),
        ("u64-max", [vec![0], vec![255; 8]].concat(), true),
        ("u64-plus-one", [vec![1], vec![0; 8]].concat(), true),
        ("256-bits", vec![1; 33], true),
        ("negative", vec![255], false),
        ("empty", vec![], false),
        ("nonminimal", vec![0, 1], false),
        ("nonminimal-large", vec![0, 1, 0, 0, 0, 0], false),
    ] {
        for (field, identifier, value) in [
            (
                "pathlen",
                "2.5.29.19",
                seq(&[vec![1, 1, 255], fixtures::wrap(2, &bytes)]),
            ),
            ("explicit", "2.5.29.36", seq(&[fixtures::wrap(128, &bytes)])),
            ("mapping", "2.5.29.36", seq(&[fixtures::wrap(129, &bytes)])),
            ("any", "2.5.29.54", fixtures::wrap(2, &bytes)),
        ] {
            result.push(Vector {
                label: format!("{field}/{label}"),
                oid: identifier,
                bytes: value,
                accepted,
                leaf: field != "pathlen",
            });
        }
    }
    result
}

fn collection_vectors() -> Vec<Vector> {
    let dns = fixtures::wrap(130, b"review.example");
    let subtree = seq(std::slice::from_ref(&dns));
    let mut result = Vec::new();
    for (label, identifier, bytes, accepted, leaf) in [
        ("san-empty", "2.5.29.17", seq(&[]), false, true),
        (
            "san-one",
            "2.5.29.17",
            seq(std::slice::from_ref(&dns)),
            true,
            true,
        ),
        ("ian-empty", "2.5.29.18", seq(&[]), false, true),
        (
            "ian-one",
            "2.5.29.18",
            seq(std::slice::from_ref(&dns)),
            true,
            true,
        ),
        ("eku-empty", "2.5.29.37", seq(&[]), false, true),
        (
            "eku-any",
            "2.5.29.37",
            seq(&[oid("2.5.29.37.0")]),
            true,
            true,
        ),
        ("mappings-empty", "2.5.29.33", seq(&[]), false, false),
        (
            "mappings-one",
            "2.5.29.33",
            seq(&[seq(&[oid(POLICY), oid(POLICY)])]),
            true,
            false,
        ),
        (
            "nc-permitted-empty",
            "2.5.29.30",
            seq(&[fixtures::wrap(160, &[])]),
            false,
            false,
        ),
        (
            "nc-permitted-one",
            "2.5.29.30",
            seq(&[fixtures::wrap(160, &subtree)]),
            true,
            false,
        ),
        (
            "nc-excluded-empty",
            "2.5.29.30",
            seq(&[fixtures::wrap(161, &[])]),
            false,
            false,
        ),
        (
            "nc-excluded-one",
            "2.5.29.30",
            seq(&[fixtures::wrap(161, &subtree)]),
            true,
            false,
        ),
        (
            "nc-both-empty",
            "2.5.29.30",
            seq(&[fixtures::wrap(160, &[]), fixtures::wrap(161, &[])]),
            false,
            false,
        ),
        // Both fields are optional in the ASN.1 schema. This is distinct from
        // a PRESENT field violating GeneralSubtrees' minimum size.
        ("nc-both-absent", "2.5.29.30", seq(&[]), true, false),
    ] {
        result.push(Vector {
            label: label.into(),
            oid: identifier,
            bytes,
            accepted,
            leaf,
        });
    }
    result
}

fn shape_vectors() -> Vec<Vector> {
    let mut result = Vec::new();
    for (label, identifier, bytes, accepted) in [
        (
            "basic-default-false",
            "2.5.29.19",
            seq(&[vec![1, 1, 0]]),
            false,
        ),
        (
            "basic-extra-field",
            "2.5.29.19",
            seq(&[vec![1, 1, 255], vec![2, 1, 0], vec![2, 1, 0]]),
            false,
        ),
        (
            "policy-duplicate",
            "2.5.29.36",
            seq(&[vec![128, 1, 0], vec![128, 1, 0]]),
            false,
        ),
        (
            "policy-reversed",
            "2.5.29.36",
            seq(&[vec![129, 1, 0], vec![128, 1, 0]]),
            false,
        ),
        (
            "policy-unknown-field",
            "2.5.29.36",
            seq(&[vec![130, 1, 0]]),
            false,
        ),
        (
            "policy-explicit-tag",
            "2.5.29.36",
            seq(&[fixtures::wrap(160, &[2, 1, 0])]),
            false,
        ),
        (
            "policy-both-fields",
            "2.5.29.36",
            seq(&[vec![128, 1, 0], vec![129, 1, 1]]),
            true,
        ),
        ("any-wrong-type", "2.5.29.54", vec![4, 1, 0], false),
        ("unknown-empty-sequence", "1.2.3.999", seq(&[]), true),
    ] {
        result.push(Vector {
            label: label.into(),
            oid: identifier,
            bytes,
            accepted,
            leaf: true,
        });
    }
    result
}

pub(crate) fn integration_cases() -> Vec<ExtensionCase> {
    let (root, _) = chain();
    let ca = certificate(
        "Extension CA",
        Some(&root),
        &[40],
        &[
            ("basicConstraints", "critical,CA:TRUE"),
            ("keyUsage", "keyCertSign"),
            ("certificatePolicies", POLICY),
        ],
    );
    let leaf = certificate(
        "Extension signer",
        Some(&ca),
        &[41],
        &[
            ("keyUsage", "digitalSignature"),
            ("certificatePolicies", POLICY),
        ],
    );
    let mut cases = Vec::new();
    for vector in integer_vectors()
        .into_iter()
        .chain(collection_vectors())
        .chain(shape_vectors())
    {
        for critical in [false, true] {
            // IssuerAltName is not in the provider's critical-extension
            // allowlist. Exercise its typed schema as a noncritical extension.
            if vector.oid == "2.5.29.18" && critical {
                continue;
            }
            for at in if vector.leaf { 0..2 } else { 1..2 } {
                let mut path = vec![leaf.clone(), ca.clone()];
                path[at] = replace(&path[at], vector.oid, &vector.bytes, critical);
                let mut policy = config(&root, &path);
                policy["initial_policy_set"] = json!([POLICY]);
                policy["explicit_policy"] = json!(true);
                cases.push(ExtensionCase {
                    label: format!("{}/{at}/critical={critical}", vector.label),
                    policy,
                    chain: path.iter().map(encoded).collect(),
                    accepted: vector.accepted && (vector.oid != "1.2.3.999" || !critical),
                });
            }
        }
    }
    // Actual path-length violations still reject after widening the integer.
    let lower = certificate(
        "Lower CA",
        Some(&ca),
        &[42],
        &[
            ("basicConstraints", "critical,CA:TRUE"),
            ("keyUsage", "keyCertSign"),
            ("certificatePolicies", POLICY),
        ],
    );
    let signer = certificate(
        "Deep signer",
        Some(&lower),
        &[43],
        &[
            ("keyUsage", "digitalSignature"),
            ("certificatePolicies", POLICY),
        ],
    );
    for limit in [0, 1] {
        let upper = replace(
            &ca,
            "2.5.29.19",
            &seq(&[vec![1, 1, 255], fixtures::wrap(2, &[limit])]),
            true,
        );
        let path = vec![signer.clone(), lower.clone(), upper];
        cases.push(ExtensionCase {
            label: format!("actual-depth/{limit}"),
            policy: config(&root, &path),
            chain: path.iter().map(encoded).collect(),
            accepted: limit == 1,
        });
    }
    cases
}

#[test]
fn extension_schema_handles_collection_sizes_and_unbounded_constraints() {
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

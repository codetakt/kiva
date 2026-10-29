#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use super::{HttpError, SignaturePolicy, hex_lower, provider, timestamp};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
mod fixtures;
pub(crate) mod name_constraints;
use fixtures::{X509, certificate, encoded, spki};

fn config(anchor: &X509, path: &[X509]) -> Value {
    let issuer_hash = hex_lower(&Sha256::digest(spki()));
    json!({
        "trust_anchors_der": [encoded(anchor)],
        "intended_service": "integrity-or-non-repudiation",
        "allowed_extended_key_usages": [],
        "initial_policy_set": ["2.5.29.32.0"],
        "validation_time": "2026-10-27T00:00:00Z",
        "out_of_band_status": path.iter().map(|certificate| json!({
            "certificate_sha256": hex_lower(&Sha256::digest(certificate)),
            "issuer_spki_sha256": issuer_hash,
            "authenticated_source": "local test authority",
            "valid_from": "2025-01-01T00:00:00Z", "valid_until": "2035-01-01T00:00:00Z",
            "status": "good"
        })).collect::<Vec<_>>()
    })
}

fn validate(config: &Value, path: &[X509]) -> Result<(), HttpError> {
    SignaturePolicy::from_json(&serde_json::to_vec(config).unwrap())?
        .validate_chain(&path.iter().map(encoded).collect::<Vec<_>>())
        .map(|_| ())
}

fn chain() -> (X509, Vec<X509>) {
    let ca = [
        ("basicConstraints", "critical,CA:TRUE"),
        ("keyUsage", "critical,keyCertSign,cRLSign"),
    ];
    let root = certificate("Root", None, &[1], &ca);
    let intermediate = certificate("Intermediate", Some(&root), &[2], &ca);
    let signer = certificate(
        "Signer",
        Some(&intermediate),
        &[3],
        &[("keyUsage", "digitalSignature")],
    );
    (root, vec![signer, intermediate])
}

#[test]
fn actual_path_requires_configured_anchor_and_every_link() {
    let (root, path) = chain();
    let policy = config(&root, &path);
    assert!(validate(&policy, &path).is_ok());
    let mut with_root = path.clone();
    with_root.push(root);
    assert!(validate(&policy, &with_root).is_ok());
    assert!(
        SignaturePolicy::default()
            .validate_chain(&path.iter().map(encoded).collect::<Vec<_>>())
            .is_err()
    );
    let other_root = certificate("Other root", None, &[4], &[("basicConstraints", "CA:TRUE")]);
    assert!(validate(&config(&other_root, &path), &path).is_err());
    assert!(validate(&policy, &path[..1]).is_err());
    assert!(validate(&policy, &[path[1].clone(), path[0].clone()]).is_err());
    assert!(
        validate(
            &policy,
            &[path[0].clone(), path[1].clone(), path[1].clone()]
        )
        .is_err()
    );
    let mut broken = path[1].clone();
    *broken.last_mut().unwrap() ^= 1;
    assert!(validate(&policy, &[path[0].clone(), broken]).is_err());
}

#[test]
fn validity_endpoints_are_inclusive_and_status_is_bound_and_current() {
    let (root, path) = chain();
    let original = config(&root, &path);
    for instant in ["2025-01-01T00:00:00Z", "2035-01-01T00:00:00Z"] {
        let mut policy = original.clone();
        policy["validation_time"] = json!(instant);
        assert!(validate(&policy, &path).is_ok(), "{instant}");
    }
    for instant in ["2024-12-31T23:59:59Z", "2035-01-01T00:00:00.000001Z"] {
        let mut policy = original.clone();
        policy["validation_time"] = json!(instant);
        assert!(validate(&policy, &path).is_err(), "{instant}");
    }
    for index in 0..2 {
        for status in ["revoked", "unknown"] {
            let mut policy = original.clone();
            policy["out_of_band_status"][index]["status"] = json!(status);
            assert!(validate(&policy, &path).is_err(), "{index}/{status}");
        }
        for (field, value) in [
            ("certificate_sha256", "0".repeat(64)),
            ("issuer_spki_sha256", "0".repeat(64)),
            ("valid_from", "2027-01-01T00:00:00Z".into()),
            ("valid_until", "2026-01-01T00:00:00Z".into()),
        ] {
            let mut policy = original.clone();
            policy["out_of_band_status"][index][field] = json!(value);
            assert!(validate(&policy, &path).is_err(), "{index}/{field}");
        }
        let mut policy = original.clone();
        policy["out_of_band_status"]
            .as_array_mut()
            .unwrap()
            .remove(index);
        assert!(validate(&policy, &path).is_err());
    }
}

#[test]
fn authenticated_status_configuration_keeps_scope_freshness_and_outcomes_distinct() {
    let (root, path) = chain();
    let baseline = config(&root, &path);
    // This provider selects authenticated local out-of-band status, not CRL
    // or OCSP wire processing. Those mechanisms' DP/reason/delta predicates
    // are not asserted executed or waived for a deployment selecting them.
    for index in 0..path.len() {
        for (field, value) in [
            ("authenticated_source", json!("")),
            ("authenticated_source", json!(" \t")),
            ("certificate_sha256", json!("0".repeat(63))),
            ("issuer_spki_sha256", json!("G".repeat(64))),
            ("valid_from", json!("2036-01-01T00:00:00Z")),
            ("status", json!(true)),
        ] {
            let mut policy = baseline.clone();
            policy["out_of_band_status"][index][field] = value;
            assert!(
                SignaturePolicy::from_json(policy.to_string().as_bytes()).is_err(),
                "{index}/{field}"
            );
        }
        let mut duplicate = baseline.clone();
        let repeated = duplicate["out_of_band_status"][index].clone();
        duplicate["out_of_band_status"]
            .as_array_mut()
            .unwrap()
            .push(repeated);
        assert!(SignaturePolicy::from_json(duplicate.to_string().as_bytes()).is_err());
        for (instant, valid) in [
            ("2026-01-01T00:00:00Z", true),
            ("2027-01-01T00:00:00Z", true),
            ("2025-12-31T23:59:59.999999999Z", false),
            ("2027-01-01T00:00:00.000000001Z", false),
        ] {
            let mut policy = baseline.clone();
            policy["validation_time"] = json!(instant);
            policy["out_of_band_status"][index]["valid_from"] = json!("2026-01-01T00:00:00Z");
            policy["out_of_band_status"][index]["valid_until"] = json!("2027-01-01T00:00:00Z");
            assert_eq!(
                validate(&policy, &path).is_ok(),
                valid,
                "closed OOB interval, not CRL nextUpdate: {index}/{instant}"
            );
        }
        for (status, reason) in [("revoked", "is revoked"), ("unknown", "indeterminate")] {
            let mut policy = baseline.clone();
            policy["out_of_band_status"][index]["status"] = json!(status);
            assert!(
                validate(&policy, &path)
                    .unwrap_err()
                    .message
                    .contains(reason)
            );
            // A stale assertion does not provide a current authenticated
            // revocation conclusion, even when its old label was revoked.
            policy["out_of_band_status"][index]["valid_until"] = json!("2026-01-01T00:00:00Z");
            assert!(
                validate(&policy, &path)
                    .unwrap_err()
                    .message
                    .contains("indeterminate")
            );
        }
    }
    let mut no_evidence = baseline;
    no_evidence["out_of_band_status"] = json!([]);
    assert!(
        validate(&no_evidence, &path)
            .unwrap_err()
            .message
            .contains("unavailable")
    );
    assert!(
        SignaturePolicy::default()
            .validate_chain(&path.iter().map(encoded).collect::<Vec<_>>())
            .unwrap_err()
            .message
            .contains("locally configured")
    );
}

#[test]
fn intended_use_accepts_both_signature_services_without_universal_eku() {
    let (root, path) = chain();
    for (ku, accepted) in [
        ("digitalSignature", true),
        ("nonRepudiation", true),
        ("digitalSignature,nonRepudiation", true),
        ("keyCertSign,cRLSign", false),
    ] {
        let signer = certificate("Signer", Some(&path[1]), &[5], &[("keyUsage", ku)]);
        let trial = vec![signer, path[1].clone()];
        let mut policy = config(&root, &trial);
        assert_eq!(validate(&policy, &trial).is_ok(), accepted, "{ku}");
        policy["intended_service"] = json!("non-repudiation");
        assert_eq!(
            validate(&policy, &trial).is_ok(),
            ku.contains("nonRepudiation")
        );
        policy["intended_service"] = json!("integrity");
        assert_eq!(
            validate(&policy, &trial).is_ok(),
            ku.contains("digitalSignature")
        );
    }
    for eku in [
        "codeSigning",
        "serverAuth",
        "1.2.3.4",
        "anyExtendedKeyUsage",
    ] {
        let signer = certificate("Signer", Some(&path[1]), &[6], &[("extendedKeyUsage", eku)]);
        let trial = vec![signer, path[1].clone()];
        let mut policy = config(&root, &trial);
        assert_eq!(
            validate(&policy, &trial).is_ok(),
            eku == "anyExtendedKeyUsage"
        );
        policy["allowed_extended_key_usages"] =
            json!(["1.3.6.1.5.5.7.3.3", "1.3.6.1.5.5.7.3.1", "1.2.3.4"]);
        assert!(validate(&policy, &trial).is_ok(), "{eku}");
    }
    let signer = certificate("Signer", Some(&path[1]), &[7], &[]);
    let trial = vec![signer, path[1].clone()];
    assert!(validate(&config(&root, &trial), &trial).is_ok());
}

#[test]
fn intermediate_constraints_and_unknown_critical_extensions_are_enforced() {
    let (root, _) = chain();
    for (extensions, accepted) in [
        (
            vec![
                ("basicConstraints", "critical,CA:TRUE,pathlen:0"),
                ("keyUsage", "keyCertSign"),
            ],
            true,
        ),
        (
            vec![
                ("basicConstraints", "critical,CA:FALSE"),
                ("keyUsage", "keyCertSign"),
            ],
            false,
        ),
        (
            vec![
                ("basicConstraints", "critical,CA:TRUE"),
                ("keyUsage", "digitalSignature"),
            ],
            false,
        ),
    ] {
        let intermediate = certificate("Intermediate", Some(&root), &[8], &extensions);
        let signer = certificate("Signer", Some(&intermediate), &[9], &[]);
        let path = vec![signer, intermediate];
        assert_eq!(
            validate(&config(&root, &path), &path).is_ok(),
            accepted,
            "{extensions:?}"
        );
        if accepted {
            let extra = certificate(
                "Extra CA",
                Some(&path[1]),
                &[10],
                &[("basicConstraints", "critical,CA:TRUE")],
            );
            let signer = certificate("Signer", Some(&extra), &[11], &[]);
            let path = vec![signer, extra, path[1].clone()];
            assert!(validate(&config(&root, &path), &path).is_err());
        }
    }
    for (value, accepted) in [("DER:05:00", true), ("critical,DER:05:00", false)] {
        for on_intermediate in [false, true] {
            let mut ca = vec![("basicConstraints", "critical,CA:TRUE")];
            if on_intermediate {
                ca.push(("1.2.3.4", value));
            }
            let intermediate = certificate("Intermediate", Some(&root), &[12], &ca);
            let extension = if on_intermediate {
                vec![]
            } else {
                vec![("1.2.3.4", value)]
            };
            let signer = certificate("Signer", Some(&intermediate), &[13], &extension);
            let path = vec![signer, intermediate];
            assert_eq!(
                validate(&config(&root, &path), &path).is_ok(),
                accepted,
                "{value}/{on_intermediate}"
            );
        }
    }
}

#[test]
fn noncritical_san_names_are_subject_to_name_constraints() {
    let (root, _) = chain();
    let intermediate = certificate(
        "Intermediate",
        Some(&root),
        &[14],
        &[
            ("basicConstraints", "critical,CA:TRUE"),
            (
                "nameConstraints",
                "critical,permitted;DNS:.example.org,excluded;DNS:.blocked.example.org",
            ),
        ],
    );
    for (name, accepted) in [
        ("DNS:allowed.example.org", true),
        ("DNS:other.example.net", false),
        ("DNS:sub.blocked.example.org", false),
    ] {
        let signer = certificate(
            "Signer",
            Some(&intermediate),
            &[15],
            &[("subjectAltName", name)],
        );
        let path = vec![signer, intermediate.clone()];
        assert_eq!(
            validate(&config(&root, &path), &path).is_ok(),
            accepted,
            "{name}"
        );
    }
}

#[test]
fn positive_twenty_octet_serials_retain_identity() {
    let (root, path) = chain();
    for length in [1, 8, 9, 19, 20] {
        let mut serial = vec![0; length];
        serial[0] = 1;
        let signer = certificate("Signer", Some(&path[1]), &serial, &[]);
        let parsed = x509_parser::parse_x509_certificate(&signer).unwrap().1;
        assert_eq!(parsed.raw_serial(), serial);
        let trial = vec![signer, path[1].clone()];
        assert!(
            validate(&config(&root, &trial), &trial).is_ok(),
            "serial {length}"
        );
    }
    let mut serial = [0; 20];
    serial[0] = 1;
    serial[19] = 1;
    let signer = certificate("Signer", Some(&path[1]), &serial, &[]);
    let first = vec![signer, path[1].clone()];
    let policy = config(&root, &first);
    assert!(validate(&policy, &first).is_ok());
    serial[19] = 2;
    let signer = certificate("Signer", Some(&path[1]), &serial, &[]);
    let second = vec![signer, path[1].clone()];
    assert!(
        validate(&policy, &second).is_err(),
        "status for the first serial must not authorize the second"
    );
    assert!(validate(&config(&root, &second), &second).is_ok());
}

#[test]
fn isolated_intermediate_validity_and_legacy_ca_policy_are_checked() {
    use ::der::Decode;
    use x509_cert::certificate::Version;
    let (root, _) = chain();
    // The signer, trust input and authenticated status remain valid. Only the
    // intermediate interval changes, including its two inclusive endpoints.
    for (before, after, accepted) in [
        ("260601000000Z", "270601000000Z", true),
        ("250601000000Z", "260601000000Z", true),
        ("260601000001Z", "270601000000Z", false),
        ("250601000000Z", "260531235959Z", false),
    ] {
        let intermediate = fixtures::certificate_with_validity(
            "Intermediate",
            Some(&root),
            &[61],
            &[("basicConstraints", "critical,CA:TRUE")],
            [(23, before), (23, after)],
        );
        let leaf = certificate("Signer", Some(&intermediate), &[62], &[]);
        let path = vec![leaf, intermediate];
        let mut policy = config(&root, &path);
        policy["validation_time"] = json!("2026-06-01T00:00:00Z");
        assert_eq!(
            validate(&policy, &path).is_ok(),
            accepted,
            "{before}/{after}"
        );
    }
    // No independent legacy-CA confirmation service is selected. RFC5280
    // permits rejecting v1/v2 intermediates rather than guessing CA status.
    for version in [Version::V1, Version::V2, Version::V3] {
        let bytes = certificate("Legacy CA", Some(&root), &[63], &[]);
        let mut intermediate = x509_cert::Certificate::from_der(&bytes).unwrap();
        intermediate.tbs_certificate.version = version;
        let intermediate = fixtures::resign(intermediate, jsonwebtoken::Algorithm::RS256);
        let leaf = certificate("Signer", Some(&intermediate), &[64], &[]);
        let path = vec![leaf, intermediate];
        assert!(
            validate(&config(&root, &path), &path).is_err(),
            "{version:?}"
        );
    }
}

#[test]
fn original_signature_algorithms_and_final_signer_parameters_are_bound() {
    use ::der::{
        Decode,
        asn1::{Any, ObjectIdentifier},
    };
    use jsonwebtoken::Algorithm;
    let (root, path) = chain();
    for (oid, algorithm) in [
        ("1.2.840.113549.1.1.11", Algorithm::RS256),
        ("1.2.840.113549.1.1.12", Algorithm::RS384),
        ("1.2.840.113549.1.1.13", Algorithm::RS512),
    ] {
        for (inner_null, outer_null) in [(true, true), (true, false), (false, true), (false, false)]
        {
            let mut leaf = x509_cert::Certificate::from_der(&path[0]).unwrap();
            leaf.tbs_certificate.signature.oid = ObjectIdentifier::new(oid).unwrap();
            leaf.tbs_certificate.signature.parameters = inner_null.then_some(Any::null());
            leaf.signature_algorithm.oid = ObjectIdentifier::new(oid).unwrap();
            leaf.signature_algorithm.parameters = outer_null.then_some(Any::null());
            let bytes = fixtures::resign(leaf.clone(), algorithm);
            let trial = vec![bytes, path[1].clone()];
            let policy = config(&root, &trial);
            assert!(
                validate(&policy, &trial).is_ok(),
                "{algorithm:?}/{inner_null}/{outer_null}"
            );
            let parsed_policy = SignaturePolicy::from_json(policy.to_string().as_bytes()).unwrap();
            let result = provider::validate(
                &trial,
                std::slice::from_ref(&root),
                parsed_policy.config.as_ref().unwrap(),
                timestamp("2026-06-01T00:00:00Z").unwrap(),
            )
            .unwrap();
            assert_eq!(
                result.leaf_spki,
                leaf.tbs_certificate.subject_public_key_info
            );
            // Valid mathematical signature, but outer and signed algorithm OIDs
            // disagree. This fails independently of TBS signature corruption.
            leaf.signature_algorithm.oid =
                ObjectIdentifier::new(if algorithm == Algorithm::RS256 {
                    "1.2.840.113549.1.1.12"
                } else {
                    "1.2.840.113549.1.1.11"
                })
                .unwrap();
            let trial = vec![fixtures::resign(leaf, algorithm), path[1].clone()];
            assert!(validate(&config(&root, &trial), &trial).is_err());
        }
    }
}

#[test]
fn certificate_policy_tree_uses_initial_set_mapping_and_inhibition() {
    let (root, _) = chain();
    for (ca_policy, leaf_policy, mapping, inhibit_any, inhibit_mapping, expected) in [
        ("1.2.3.4", "1.2.3.4", None, false, false, true),
        ("1.2.3.4", "1.2.3.5", None, false, false, false),
        ("2.5.29.32.0", "1.2.3.4", None, false, false, true),
        ("2.5.29.32.0", "1.2.3.4", None, true, false, false),
        (
            "1.2.3.4",
            "1.2.3.5",
            Some("1.2.3.4:1.2.3.5"),
            false,
            false,
            true,
        ),
        (
            "1.2.3.4",
            "1.2.3.5",
            Some("1.2.3.4:1.2.3.5"),
            false,
            true,
            false,
        ),
    ] {
        let mut extensions = vec![
            ("basicConstraints", "critical,CA:TRUE"),
            ("certificatePolicies", ca_policy),
        ];
        if let Some(mapping) = mapping {
            extensions.push(("policyMappings", mapping));
        }
        let intermediate = certificate("Policy intermediate", Some(&root), &[16], &extensions);
        let signer = certificate(
            "Policy signer",
            Some(&intermediate),
            &[17],
            &[("certificatePolicies", leaf_policy)],
        );
        let path = vec![signer, intermediate];
        let mut policy = config(&root, &path);
        policy["initial_policy_set"] = json!(["1.2.3.4"]);
        policy["explicit_policy"] = json!(true);
        policy["inhibit_any_policy"] = json!(inhibit_any);
        policy["inhibit_policy_mapping"] = json!(inhibit_mapping);
        let result = validate(&policy, &path);
        assert_eq!(
            result.is_ok(),
            expected,
            "{ca_policy}/{leaf_policy}/{mapping:?}/{inhibit_any}/{inhibit_mapping}: {result:?}"
        );
        if expected {
            let parsed = SignaturePolicy::from_json(policy.to_string().as_bytes()).unwrap();
            let result = provider::validate(
                &path,
                std::slice::from_ref(&root),
                parsed.config.as_ref().unwrap(),
                timestamp("2026-10-27T00:00:00Z").unwrap(),
            )
            .unwrap();
            let tree = result.valid_policy_tree.unwrap();
            assert!(
                tree.iter()
                    .any(|node| node.depth == 2 && node.valid_policy.to_string() == leaf_policy)
            );
            assert_eq!(result.anchor_index, 0);
        }
    }
}

#[test]
fn empty_policy_tree_is_permitted_until_explicit_policy_is_required() {
    let (root, path) = chain();
    let mut policy = config(&root, &path);
    policy["initial_policy_set"] = json!(["1.2.3.4"]);
    assert!(validate(&policy, &path).is_ok());
    policy["explicit_policy"] = json!(true);
    assert!(validate(&policy, &path).is_err());
    let intermediate = certificate(
        "Policy intermediate",
        Some(&root),
        &[18],
        &[
            ("basicConstraints", "critical,CA:TRUE"),
            ("policyConstraints", "critical,requireExplicitPolicy:0"),
        ],
    );
    let signer = certificate("Policy signer", Some(&intermediate), &[19], &[]);
    let path = vec![signer, intermediate];
    assert!(validate(&config(&root, &path), &path).is_err());
}

#[test]
fn directory_names_use_unicode32_preparation_and_combining_mark_space_rules() {
    use ::der::{Decode, Encode, asn1::ObjectIdentifier};
    use fixtures::wrap;
    let name = |text: &str, tag: u8| {
        let content = if tag == 0x1e {
            text.encode_utf16()
                .flat_map(u16::to_be_bytes)
                .collect::<Vec<_>>()
        } else {
            text.as_bytes().to_vec()
        };
        let mut attribute = ObjectIdentifier::new("2.5.4.3").unwrap().to_der().unwrap();
        attribute.extend(wrap(tag, &content));
        x509_cert::name::Name::from_der(&wrap(0x30, &wrap(0x31, &wrap(0x30, &attribute)))).unwrap()
    };
    for (left, right) in [
        ("Straße", "STRASSE"),
        ("Σςσ", "σσσ"),
        ("é", "e\u{301}"),
        ("ﬁ", "fi"),
        ("\u{a0} ALICE\t Bob\u{200b}", "alice bob"),
        ("C\u{34f}", "c"),
        ("\u{212a}", "k"),
        ("A\0B", "ab"),
        ("\u{1d400}", "a"),
        ("가", "\u{1100}\u{1161}"),
        ("각", "\u{1100}\u{1161}\u{11a8}"),
        (" \u{301}A", "  \u{301}A"),
        ("\u{2f868}", "\u{2136a}"),
        ("aאב", "Aאב"), // RFC4518 has no stringprep bidi rejection.
    ] {
        assert!(
            pkix_path::names_match(&name(left, 12), &name(right, 12)),
            "{left:?}/{right:?}"
        );
    }
    assert!(pkix_path::names_match(
        &name("École", 0x1e),
        &name("e\u{301}cole", 12)
    ));
    for (left, right) in [
        (" \u{301}A", "\u{301}A"),
        ("\u{2f868}", "\u{36fc}"),
        ("alice", "bob"),
    ] {
        assert!(
            !pkix_path::names_match(&name(left, 12), &name(right, 12)),
            "{left:?}/{right:?}"
        );
    }
    for value in ["\u{fffd}", "\u{e000}", "\u{1f600}", "\u{1e9e}"] {
        assert!(
            !pkix_path::names_match(&name(value, 12), &name(value, 12)),
            "failed preparation is not equality: {value:?}"
        );
    }
}

#[test]
fn directory_preparation_matches_pinned_unicode32_reference_vectors() {
    use std::io::Read;
    let compressed = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/pkix-unicode32.json.gz"
    ));
    let mut reference = Vec::new();
    flate2::read::GzDecoder::new(&compressed[..])
        .read_to_end(&mut reference)
        .unwrap();
    let vectors: Vec<Value> = serde_json::from_slice(&reference).unwrap();
    assert_eq!(vectors.len(), 84967);
    for vector in vectors {
        assert_eq!(
            pkix_path::normalize_unicode32(vector["input"].as_str().unwrap()),
            vector["nfkc"].as_str().unwrap(),
            "NFKC: {vector}"
        );
        let result = pkix_path::prepare_directory_string(vector["input"].as_str().unwrap());
        let expected = vector["prepared"].as_str().map(str::to_owned);
        assert_eq!(result, expected, "{vector}");
        assert_eq!(
            pkix_path::normalize_nfc_unicode32(vector["input"].as_str().unwrap()),
            vector["nfc"].as_str().unwrap(),
            "NFC: {vector}"
        );
    }
}

#[test]
fn name_attribute_oid_controls_directory_and_domain_comparison() {
    use ::der::{Decode, Encode, asn1::ObjectIdentifier};
    let name = |oid: &str, tag: u8, value: &str| {
        let mut attribute = ObjectIdentifier::new(oid).unwrap().to_der().unwrap();
        attribute.extend(fixtures::wrap(tag, value.as_bytes()));
        x509_cert::name::Name::from_der(&fixtures::wrap(
            0x30,
            &fixtures::wrap(0x31, &fixtures::wrap(0x30, &attribute)),
        ))
        .unwrap()
    };
    let dc = "0.9.2342.19200300.100.1.25";
    for (left, right, accepted) in [
        ("EXAMPLE", "example", true),
        ("1-example", "1-EXAMPLE", true),
        ("x", "X", true),
        ("xn--bcher-kva", "XN--BCHER-KVA", true),
        (" example ", "example", false),
        ("example.com", "example.com", false),
        ("_srv", "_srv", false),
        ("-edge", "-edge", false),
        ("edge-", "edge-", false),
        ("", "", false),
        ("a b", "a  b", false),
    ] {
        assert_eq!(
            pkix_path::names_match(&name(dc, 22, left), &name(dc, 22, right)),
            accepted,
            "{left}/{right}"
        );
    }
    for length in [63, 64] {
        assert_eq!(
            pkix_path::names_match(
                &name(dc, 22, &"a".repeat(length)),
                &name(dc, 22, &"A".repeat(length))
            ),
            length == 63
        );
    }
    assert!(!pkix_path::names_match(
        &name(dc, 12, "example"),
        &name(dc, 22, "example")
    ));
    assert!(!pkix_path::names_match(
        &name("2.5.4.3", 22, "Alice"),
        &name("2.5.4.3", 22, "Alice")
    ));
    assert!(!pkix_path::names_match(
        &name("2.5.4.6", 12, "US"),
        &name("2.5.4.6", 19, "US")
    ));
    assert!(pkix_path::names_match(
        &name("2.5.4.6", 19, "US"),
        &name("2.5.4.6", 19, "us")
    ));
    for (left, right, accepted) in [
        ("Literal", "Literal", true),
        ("Literal", "literal", false),
        ("a  b", "a b", false),
    ] {
        assert_eq!(
            pkix_path::names_match(&name("1.2.3.4", 12, left), &name("1.2.3.4", 12, right)),
            accepted
        );
    }
    assert!(!pkix_path::names_match(
        &name("1.2.3.4", 12, "same"),
        &name("1.2.3.4", 19, "same")
    ));
    assert!(!pkix_path::names_match(
        &name("2.5.4.3", 12, "same"),
        &name("2.5.4.10", 12, "same")
    ));
    assert!(pkix_path::names_match(
        &name("2.5.4.3", 12, "Alice"),
        &name("2.5.4.3", 19, "ALICE")
    ));

    // Each contrast has a fresh signature and bound good status for the exact
    // certificate bytes. A failed link cannot be credited to broken crypto.
    let (original_root, _) = chain();
    for (oid, left, right, accepted) in [
        (dc, "EXAMPLE", "example", true),
        (dc, "example", " example ", false),
        ("1.2.3.4", "Literal", "Literal", true),
        ("1.2.3.4", "Literal", "literal", false),
    ] {
        let tag = if oid == dc { 22 } else { 12 };
        let issuer = name(oid, tag, left);
        let root = fixtures::with_names(&original_root, &issuer, &issuer);
        let signer = certificate("Signer", Some(&root), &[91], &[]);
        let signer = fixtures::with_names(
            &signer,
            &name(oid, tag, right),
            &name("2.5.4.3", 12, "Signer"),
        );
        let path = vec![signer];
        let result = validate(&config(&root, &path), &path);
        assert_eq!(result.is_ok(), accepted, "{oid}/{left}/{right}: {result:?}");
    }
}

#[test]
fn original_certificate_times_support_1950_pivot_and_fractional_endpoints() {
    let (root, _) = chain();
    for (validity, instant, accepted) in [
        (
            [(23, "500101000000Z"), (23, "600101000000Z")],
            "1950-01-01T00:00:00Z",
            true,
        ),
        (
            [(23, "500101000000Z"), (23, "600101000000Z")],
            "1949-12-31T23:59:59Z",
            false,
        ),
        (
            [(23, "490101000000Z"), (23, "491231235959Z")],
            "2049-01-01T00:00:00Z",
            true,
        ),
        (
            [(23, "490101000000Z"), (23, "491231235959Z")],
            "1949-01-01T00:00:00Z",
            false,
        ),
        (
            [(24, "19920722132100.3Z"), (24, "19920722132100.7Z")],
            "1992-07-22T13:21:00.3Z",
            true,
        ),
        (
            [(24, "19920722132100.3Z"), (24, "19920722132100.7Z")],
            "1992-07-22T13:21:00.7Z",
            true,
        ),
        (
            [(24, "19920722132100.3Z"), (24, "19920722132100.7Z")],
            "1992-07-22T13:21:00.299999999Z",
            false,
        ),
        (
            [(24, "19920722132100.3Z"), (24, "19920722132100.7Z")],
            "1992-07-22T13:21:00.700000001Z",
            false,
        ),
        (
            [(24, "19920722132100.123456789Z"), (24, "19920722132100.7Z")],
            "1992-07-22T13:21:00.123456789Z",
            true,
        ),
        (
            [
                (24, "19920722132100.1234567891Z"),
                (24, "19920722132100.7Z"),
            ],
            "1992-07-22T13:21:00.123456789Z",
            false,
        ),
        (
            [
                (24, "19920722132100.1234567891Z"),
                (24, "19920722132100.7Z"),
            ],
            "1992-07-22T13:21:00.123456790Z",
            true,
        ),
        (
            [
                (24, "19920722132100.3Z"),
                (24, "19920722132100.7000000001Z"),
            ],
            "1992-07-22T13:21:00.7Z",
            true,
        ),
        (
            [
                (24, "19920722132100.3Z"),
                (24, "19920722132100.7000000001Z"),
            ],
            "1992-07-22T13:21:00.700000001Z",
            false,
        ),
    ] {
        let signer = fixtures::certificate_with_validity(
            "Historical signer",
            Some(&root),
            &[20],
            &[],
            validity,
        );
        let path = vec![signer];
        let mut policy = config(&root, &path);
        policy["validation_time"] = json!(instant);
        policy["out_of_band_status"][0]["valid_from"] = json!("1940-01-01T00:00:00Z");
        policy["out_of_band_status"][0]["valid_until"] = json!("2060-01-01T00:00:00Z");
        let result = validate(&policy, &path);
        assert_eq!(
            result.is_ok(),
            accepted,
            "{validity:?}/{instant}: {result:?}"
        );
    }
}

#[test]
fn path_time_view_never_authorizes_a_signature_over_different_original_bytes() {
    let (root, path) = chain();
    let mut changed = path[0].clone();
    let offset = changed
        .windows(13)
        .position(|bytes| bytes == b"250101000000Z")
        .unwrap();
    changed[offset] = b'2';
    changed[offset + 1] = b'4';
    // Both validity intervals include the fixed verification instant. The
    // modified bytes get their own authenticated status fixture, isolating the
    // original-byte signature check from time/status failures.
    let path = vec![changed, path[1].clone()];
    let result = validate(&config(&root, &path), &path).unwrap_err();
    assert!(result.message.contains("signature"), "{}", result.message);
}

#[test]
fn self_issued_rollover_accepts_a_new_key_only_when_signed_by_trusted_old_key() {
    let (root, _) = chain();
    let new_spki = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-rollover-spki.der"
    ));
    assert_ne!(&spki()[..], &new_spki[..]);
    let ca = [
        ("basicConstraints", "critical,CA:TRUE,pathlen:0"),
        ("keyUsage", "critical,keyCertSign"),
    ];
    for self_signed in [false, true] {
        let rollover = fixtures::certificate_with_keys(
            "Root",
            Some(&root),
            &[31],
            &ca,
            [(23, "250101000000Z"), (23, "350101000000Z")],
            new_spki,
            self_signed,
        );
        let signer = fixtures::certificate_with_keys(
            "Signer",
            Some(&rollover),
            &[32],
            &[],
            [(23, "250101000000Z"), (23, "350101000000Z")],
            &spki(),
            true,
        );
        let path = vec![signer, rollover];
        let mut policy = config(&root, &path);
        policy["out_of_band_status"][0]["issuer_spki_sha256"] =
            json!(hex_lower(&Sha256::digest(new_spki)));
        let result = validate(&policy, &path);
        assert_eq!(
            result.is_ok(),
            !self_signed,
            "self_signed={self_signed}: {result:?}"
        );
    }
}

#[test]
fn name_constraints_preserve_each_name_forms_semantics() {
    let (root, _) = chain();
    for (constraint, name, accepted) in [
        ("email:Alice@example.test", "email:Alice@EXAMPLE.TEST", true),
        (
            "email:Alice@example.test",
            "email:alice@example.test",
            false,
        ),
        ("email:example.test", "email:a@example.test", true),
        ("email:example.test", "email:a@sub.example.test", false),
        ("email:.example.test", "email:a@sub.example.test", true),
        ("email:.example.test", "email:a@example.test", false),
        ("DNS:example.test", "DNS:sub.EXAMPLE.TEST", true),
        ("DNS:host.example.test", "DNS:HOST.EXAMPLE.TEST", true),
        ("DNS:host.example.test", "DNS:www.host.example.test", true),
        ("DNS:host.example.test", "DNS:a.b.host.example.test", true),
        ("DNS:host.example.test", "DNS:evilhost.example.test", false),
        ("DNS:host.example.test", "DNS:host1.example.test", false),
        (
            "DNS:xn--bcher-kva.example",
            "DNS:XN--BCHER-KVA.EXAMPLE",
            true,
        ),
        ("DNS:example.test", "DNS:badexample.test", false),
        ("DNS:.example.test", "DNS:example.test", false),
        (
            "URI:example.test",
            "URI:https://user:pass@EXAMPLE.TEST:443/path?x=@elsewhere.test",
            true,
        ),
        (
            "URI:example.test",
            "URI:https://attacker.test/path?x=@example.test",
            false,
        ),
        ("URI:example.test", "URI:https://sub.example.test/", false),
        (
            "URI:example.test",
            "URI:http://%65xample.test:80/a/../b",
            true,
        ),
        ("URI:.example.test", "URI:ftp://sub.example.test/", true),
        ("URI:example.test", "URI:urn:example.test", false),
        ("URI:192.0.2.1", "URI:https://192.0.2.1/", false),
        ("IP:c0:00:02:00:ff:ff:ff:00", "IP:c0:00:02:7f", true),
        ("IP:c0:00:02:00:ff:ff:ff:00", "IP:c0:00:03:7f", false),
        ("IP:c0:00:02:00:ff:ff:ff:00", "IP:c0:00:14:01", false),
        ("IP:c0:00:02:00:ff:ff:ff:ff", "IP:c0:00:02:00", true),
        ("IP:c0:00:02:00:ff:ff:ff:ff", "IP:c0:00:02:01", false),
        ("IP:c0:00:02:00:00:00:00:00", "IP:cb:00:71:01", true),
        (
            "IP:20:01:0d:b8:00:00:00:00:00:00:00:00:00:00:00:00:ff:ff:ff:ff:00:00:00:00:00:00:00:00:00:00:00:00",
            "IP:20:01:0d:b8:00:00:00:00:00:00:00:00:00:00:00:01",
            true,
        ),
    ] {
        let permitted = format!("critical,permitted;{constraint}");
        let intermediate = certificate(
            "Intermediate",
            Some(&root),
            &[33],
            &[
                ("basicConstraints", "critical,CA:TRUE"),
                ("nameConstraints", &permitted),
            ],
        );
        let signer = certificate(
            "Signer",
            Some(&intermediate),
            &[34],
            &[("subjectAltName", name)],
        );
        let path = vec![signer, intermediate];
        let result = validate(&config(&root, &path), &path);
        assert_eq!(result.is_ok(), accepted, "{constraint}/{name}: {result:?}");
    }
    for (constraint, subject, accepted) in
        [("école", "E\u{301}COLE", true), ("école", "ecole", false)]
    {
        let permitted = format!("critical,permitted;dirName:{constraint}");
        let intermediate = certificate(
            "Intermediate",
            Some(&root),
            &[35],
            &[
                ("basicConstraints", "critical,CA:TRUE"),
                ("nameConstraints", &permitted),
            ],
        );
        let signer = certificate(subject, Some(&intermediate), &[36], &[]);
        let path = vec![signer, intermediate];
        let result = validate(&config(&root, &path), &path);
        assert_eq!(
            result.is_ok(),
            accepted,
            "{constraint}/{subject}: {result:?}"
        );
    }
}

#[test]
fn certificate_uri_comparison_has_its_own_syntax_and_scheme_contract() {
    for (left, right, equal) in [
        ("http://EXAMPLE.COM/a/./b", "http://example.com/a/b", true),
        ("http://example.com/%7e", "http://example.com/~", true),
        ("http://example.com/a%2fb", "http://example.com/a%2Fb", true),
        ("http://example.com/a%2fb", "http://example.com/a/b", false),
        ("http://example.com:80", "http://example.com/", true),
        ("http://example.com:", "http://example.com/", true),
        ("https://example.com:443", "https://example.com/", true),
        ("ftp://example.com:21", "ftp://example.com/", true),
        ("http://example.com:81", "http://example.com/", false),
        ("http://example.com/A", "http://example.com/a", false),
        ("http://u@example.com/", "http://U@example.com/", false),
        ("http://u:p@example.com/", "http://u:q@example.com/", false),
        ("http://example.com/?A", "http://example.com/?a", false),
        ("http://example.com/#A", "http://example.com/#a", false),
        ("http://example.com/", "http://example.com/?", false),
        ("http://example.com/", "http://example.com/#", false),
        (
            "ftp://user@example.com/x;type=a",
            "ftp://user@example.com/x;type=i",
            false,
        ),
        (
            "ftp://example.com/x",
            "ftp://anonymous@example.com/x",
            false,
        ),
        (
            "ldap://example.com",
            "ldap://EXAMPLE.COM:389/??base?(objectClass=*)",
            true,
        ),
        (
            "ldap://example.com",
            "ldap://example.com/?*?base?%28objectClass%3D%2A%29?",
            true,
        ),
        ("ldap://example.com", "ldap://example.com/??sub", false),
        (
            "ldap://example.com",
            "ldap://example.com/???(objectClass=person)",
            false,
        ),
        ("ldap://example.com/?cn", "ldap://example.com/", false),
        ("ldap://example.com/????!x", "ldap://example.com/", false),
        ("http://example.com/u%3Fv", "http://example.com/u?v", false),
        ("urn:example:x", "URN:example:x", true),
        (
            "https://b%C3%BCcher.example/",
            "https://xn--bcher-kva.example/",
            true,
        ),
        ("https://fa%C3%9F.example/", "https://fass.example/", true),
        (
            "http://user@b%C3%BCcher.example/%C3%BC?q=%C3%BC#%C3%BC",
            "http://user@xn--bcher-kva.example/%C3%BC?q=%C3%BC#%C3%BC",
            true,
        ),
    ] {
        assert_eq!(
            pkix_path::certificate_uris_match(left, right),
            Some(equal),
            "{left}/{right}"
        );
    }
    for malformed in [
        "https://bücher.example/",
        "http://example.com/%ZZ",
        "relative/path",
        "http://a b/",
    ] {
        assert_eq!(
            pkix_path::certificate_uris_match(malformed, malformed),
            None
        );
    }
    // No URI constraint means no invented DNS-host requirement: an opaque URN
    // is allowed supplementary SAN data. With a URI constraint it fails above.
    let (root, _) = chain();
    let leaf = certificate(
        "Signer",
        Some(&root),
        &[65],
        &[("subjectAltName", "URI:urn:example:x")],
    );
    let path = vec![leaf];
    assert!(validate(&config(&root, &path), &path).is_ok());
}

#[test]
fn certificate_idna2003_and_iri_producer_calls_keep_their_profiles() {
    use pkix_path::{
        certificate_dns_to_ascii as dns, certificate_iri_to_uri as iri,
        certificate_label_to_ascii as label,
    };
    for (input, output) in [
        ("bücher.example", "xn--bcher-kva.example"),
        ("faß.example", "fass.example"),
        (
            "BÜCHER\u{3002}example\u{ff0e}com\u{ff61}",
            "xn--bcher-kva.example.com.",
        ),
        ("ab\u{ad}\u{fe00}\u{200b}cd", "abcd"),
        ("EXAMPLE", "EXAMPLE"),
        ("1-example", "1-example"),
    ] {
        assert_eq!(dns(input).as_deref(), Some(output), "{input}");
    }
    for input in [
        "",
        "-edge",
        "edge-",
        "_srv",
        "a..b",
        "\u{e000}",
        "\u{fffd}",
        "\u{1f600}",
        "\u{1e9e}",
        "aאב",
        "אב1",
        "1אב",
        "אב\u{301}",
    ] {
        assert_eq!(dns(input), None, "{input:?}");
    }
    assert!(dns("אב1אב").is_some());
    assert!(
        dns("\u{13a0}").is_some(),
        "Unicode3.2 Cherokee is not modern lowercase"
    );
    for length in [1, 63, 64] {
        assert_eq!(dns(&"x".repeat(length)).is_some(), length <= 63);
    }
    assert_eq!(label("_srv", false).as_deref(), Some("_srv"));
    assert!(label("_srv", true).is_none());
    for (input, output) in [
        (
            "https://bücher.example/e\u{301}/😀?x=é#é",
            "https://b%C3%BCcher.example/%C3%A9/%F0%9F%98%80?x=%C3%A9#%C3%A9",
        ),
        ("https://example.test/é", "https://example.test/%C3%A9"),
        ("https://example.test/a%2Fb", "https://example.test/a%2Fb"),
        ("https://example.test/Ａ", "https://example.test/%EF%BC%A1"), // NFC, not NFKC.
    ] {
        assert_eq!(iri(input).as_deref(), Some(output));
    }
    assert_eq!(iri("https://example.test/a b"), None);
    // Required issuer conversion does not become an unrelated LRS HTTP
    // rejection. The actual signed-path test consumes ACE and IA5 bytes.
}

#[test]
fn selected_punycode_primitive_preserves_reference_bytes_and_fails_overflow() {
    // RFC3492 7.1 L/S, basic-only and scalar limits. Boundary encodings were
    // also calculated with Python's independent punycode implementation.
    for (decoded, encoded) in [
        ("3年B組金八先生", "3B-ww4c5e180e575a65lsy2b"),
        ("-> $1.00 <-", "-> $1.00 <--"),
        ("abc", "abc-"),
        ("\u{80}", "a"),
        ("\u{9f}", "5a"),
        ("\u{10ffff}", "dn32g"),
    ] {
        assert_eq!(
            idna::punycode::encode_str(decoded).as_deref(),
            Some(encoded)
        );
        assert_eq!(
            idna::punycode::decode_to_string(encoded).as_deref(),
            Some(decoded)
        );
    }
    for malformed in [
        "z",
        "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        "é",
        "abc-!",
    ] {
        assert!(
            idna::punycode::decode_to_string(malformed).is_none(),
            "{malformed}"
        );
    }
}

#[test]
fn legacy_subject_mail_constraints_apply_when_san_is_absent() {
    use ::der::{Decode, Encode, asn1::ObjectIdentifier};
    let (root, _) = chain();
    let intermediate = certificate(
        "Intermediate",
        Some(&root),
        &[67],
        &[
            ("basicConstraints", "critical,CA:TRUE"),
            ("nameConstraints", "critical,permitted;email:example.test"),
        ],
    );
    for (mail, san, accepted) in [
        ("User@example.test", None, true),
        ("User@outside.test", None, false),
        ("User@outside.test", Some("email:User@example.test"), true),
        ("User@example.test", Some("email:User@outside.test"), false),
        ("User@outside.test", Some("DNS:host.example.test"), true),
    ] {
        let mut attribute = ObjectIdentifier::new("1.2.840.113549.1.9.1")
            .unwrap()
            .to_der()
            .unwrap();
        attribute.extend(fixtures::wrap(22, mail.as_bytes()));
        let subject = x509_cert::name::Name::from_der(&fixtures::wrap(
            0x30,
            &fixtures::wrap(0x31, &fixtures::wrap(0x30, &attribute)),
        ))
        .unwrap();
        let extensions = san.map(|s| vec![("subjectAltName", s)]).unwrap_or_default();
        let leaf = certificate("Signer", Some(&intermediate), &[68], &extensions);
        let issuer = x509_cert::Certificate::from_der(&intermediate)
            .unwrap()
            .tbs_certificate
            .subject;
        let path = vec![
            fixtures::with_names(&leaf, &issuer, &subject),
            intermediate.clone(),
        ];
        assert_eq!(
            validate(&config(&root, &path), &path).is_ok(),
            accepted,
            "{mail}/{san:?}"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn legal_rdn_sets_and_ordered_dns_do_not_share_string_prefix_semantics() {
    use ::der::{Decode, Encode, asn1::ObjectIdentifier};
    let name = |rdns: &[Vec<(&str, u8, &str)>]| {
        let mut sequence = Vec::new();
        for rdn in rdns {
            let mut attributes = rdn
                .iter()
                .map(|(oid, tag, value)| {
                    let mut bytes = ObjectIdentifier::new(oid).unwrap().to_der().unwrap();
                    bytes.extend(fixtures::wrap(*tag, value.as_bytes()));
                    fixtures::wrap(0x30, &bytes)
                })
                .collect::<Vec<_>>();
            attributes.sort(); // DER order; logical RDN order is independent.
            sequence.extend(fixtures::wrap(0x31, &attributes.concat()));
        }
        x509_cert::name::Name::from_der(&fixtures::wrap(0x30, &sequence)).unwrap()
    };
    let cn = ("2.5.4.3", 12, "Alice");
    let org = ("2.5.4.10", 19, "EXAMPLE");
    let a = name(&[vec![cn, org]]);
    let b = name(&[vec![org, ("2.5.4.3", 19, "ALICE")]]);
    assert!(pkix_path::names_match(&a, &b));
    assert!(!pkix_path::names_match(
        &name(&[vec![cn], vec![org]]),
        &name(&[vec![org], vec![cn]])
    ));
    assert!(!pkix_path::names_match(&a, &name(&[vec![cn]])));
    for tag in [12, 19, 0x14] {
        let name = name(&[vec![("2.5.4.3", tag, "Alice")]]);
        assert_eq!(
            pkix_path::names_match(&name, &name),
            tag != 0x14,
            "unsupported Teletex must not pass required issuer-name comparison"
        );
    }
    // A valid UTF8/Printable name link is tested through the actual path, not
    // inferred from an isolated comparator result.
    let (root, _) = chain();
    let root = fixtures::with_names(&root, &a, &a);
    let leaf = certificate("Signer", Some(&root), &[66], &[]);
    let leaf = fixtures::with_names(&leaf, &b, &name(&[vec![cn]]));
    let path = vec![leaf];
    assert!(validate(&config(&root, &path), &path).is_ok());
    // Directory subtrees compare ordered RDN prefixes, never the display
    // string. A trailing subordinate RDN is permitted; prepending it is not.
    let base = name(&[vec![org]]);
    let subtree = fixtures::wrap(0x30, &fixtures::wrap(0xa4, &base.to_der().unwrap()));
    let constraints = fixtures::wrap(0x30, &fixtures::wrap(0xa0, &subtree));
    let extension = format!(
        "critical,DER:{}",
        constraints
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(":")
    );
    let intermediate = certificate(
        "Intermediate",
        Some(&root),
        &[69],
        &[
            ("basicConstraints", "critical,CA:TRUE"),
            ("2.5.29.30", &extension),
        ],
    );
    for (subject, accepted) in [
        (base, true),
        (name(&[vec![org], vec![cn]]), true),
        (name(&[vec![cn], vec![org]]), false),
        (name(&[vec![("2.5.4.3", 19, "EXAMPLE")], vec![cn]]), false),
    ] {
        let leaf = certificate("Signer", Some(&intermediate), &[70], &[]);
        let issuer = x509_cert::Certificate::from_der(&intermediate)
            .unwrap()
            .tbs_certificate
            .subject;
        let path = vec![
            fixtures::with_names(&leaf, &issuer, &subject),
            intermediate.clone(),
        ];
        assert_eq!(validate(&config(&root, &path), &path).is_ok(), accepted);
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn successive_name_constraints_intersect_and_self_issued_exceptions_are_position_bound() {
    let (root, _) = chain();
    let first = certificate(
        "Constrained CA",
        Some(&root),
        &[40],
        &[
            ("basicConstraints", "critical,CA:TRUE"),
            ("nameConstraints", "critical,permitted;DNS:example.org"),
        ],
    );
    for (next, names, accepted) in [
        (
            "critical,permitted;DNS:foo.example.org",
            "DNS:a.foo.example.org",
            true,
        ),
        (
            "critical,permitted;DNS:foo.example.org",
            "DNS:other.example.org",
            false,
        ),
        ("critical,permitted;DNS:other.net", "DNS:other.net", false),
        (
            "critical,permitted;URI:example.net",
            "DNS:a.example.org,URI:https://example.net/",
            true,
        ),
        (
            "critical,permitted;URI:example.net",
            "DNS:a.other.org,URI:https://example.net/",
            false,
        ),
        (
            "critical,permitted;DNS:example.org",
            "DNS:a.example.org,DNS:bad.other.net",
            false,
        ),
    ] {
        let second = certificate(
            "Next CA",
            Some(&first),
            &[41],
            &[
                ("basicConstraints", "critical,CA:TRUE"),
                ("nameConstraints", next),
            ],
        );
        let leaf = certificate("Signer", Some(&second), &[42], &[("subjectAltName", names)]);
        let path = vec![leaf, second, first.clone()];
        let result = validate(&config(&root, &path), &path);
        assert_eq!(
            result.is_ok(),
            accepted,
            "intersection/omitted type {next}/{names}: {result:?}"
        );
    }
    for (name, accepted) in [
        ("critical,DNS:a.example.org", true),
        ("critical,DNS:bad.other.net", false),
    ] {
        let leaf = certificate("", Some(&first), &[43], &[("subjectAltName", name)]);
        let path = vec![leaf, first.clone()];
        assert_eq!(
            validate(&config(&root, &path), &path).is_ok(),
            accepted,
            "empty subject with SAN"
        );
    }
    let rollover = certificate(
        "Constrained CA",
        Some(&first),
        &[44],
        &[
            ("basicConstraints", "critical,CA:TRUE"),
            ("keyUsage", "digitalSignature,keyCertSign"),
            ("subjectAltName", "DNS:outside.example.net"),
        ],
    );
    let leaf = certificate(
        "Signer",
        Some(&rollover),
        &[45],
        &[("subjectAltName", "DNS:inside.example.org")],
    );
    let path = vec![leaf, rollover.clone(), first.clone()];
    assert!(
        validate(&config(&root, &path), &path).is_ok(),
        "nonfinal self-issued name exception"
    );
    let path = vec![rollover, first];
    assert!(
        validate(&config(&root, &path), &path).is_err(),
        "same certificate as final signer must satisfy constraints"
    );
    for (tag, count) in [(0x80, 1), (0x81, 0)] {
        let base = fixtures::wrap(0x82, b"example.org");
        let distance = fixtures::wrap(tag, &[count]);
        let subtree = fixtures::wrap(0x30, &[base, distance].concat());
        let constraint = fixtures::wrap(0x30, &fixtures::wrap(0xa0, &subtree));
        let raw = format!(
            "critical,DER:{}",
            constraint
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(":")
        );
        let ca = certificate(
            "Distance CA",
            Some(&root),
            &[46],
            &[
                ("basicConstraints", "critical,CA:TRUE"),
                ("2.5.29.30", &raw),
            ],
        );
        for relevant_name in [false, true] {
            let ext = if relevant_name {
                vec![("subjectAltName", "DNS:a.example.org")]
            } else {
                vec![]
            };
            let leaf = certificate("Signer", Some(&ca), &[47], &ext);
            let path = vec![leaf, ca.clone()];
            assert!(
                validate(&config(&root, &path), &path).is_err(),
                "unsupported critical subtree distance tag={tag}/{relevant_name}"
            );
        }
    }
}

#[test]
fn path_policy_extension_counters_and_self_issued_decrement_rules_are_effective() {
    let (root, _) = chain();
    for counter in [0, 1] {
        let text = counter.to_string();
        let ca = certificate(
            "Any CA",
            Some(&root),
            &[48],
            &[
                ("basicConstraints", "critical,CA:TRUE"),
                ("certificatePolicies", "1.2.3.4"),
                ("inhibitAnyPolicy", &text),
            ],
        );
        let leaf = certificate(
            "Signer",
            Some(&ca),
            &[49],
            &[("certificatePolicies", "2.5.29.32.0")],
        );
        let path = vec![leaf, ca];
        let mut policy = config(&root, &path);
        policy["initial_policy_set"] = json!(["1.2.3.4"]);
        policy["explicit_policy"] = json!(true);
        assert_eq!(
            validate(&policy, &path).is_ok(),
            counter == 1,
            "inhibitAnyPolicy={counter}"
        );
        let constraint = format!("critical,inhibitPolicyMapping:{counter}");
        let first = certificate(
            "Policy CA",
            Some(&root),
            &[50],
            &[
                ("basicConstraints", "critical,CA:TRUE"),
                ("certificatePolicies", "1.2.3.4"),
                ("policyConstraints", &constraint),
            ],
        );
        for self_issued in [None, Some(false), Some(true)] {
            let mut intermediates = vec![first.clone()];
            if let Some(same_name) = self_issued {
                let rollover = certificate(
                    if same_name {
                        "Policy CA"
                    } else {
                        "Distinct CA"
                    },
                    Some(&first),
                    &[51],
                    &[
                        ("basicConstraints", "critical,CA:TRUE"),
                        ("certificatePolicies", "1.2.3.4"),
                    ],
                );
                intermediates.insert(0, rollover);
            }
            let mapper = certificate(
                "Mapper",
                Some(&intermediates[0]),
                &[52],
                &[
                    ("basicConstraints", "critical,CA:TRUE"),
                    ("certificatePolicies", "1.2.3.4"),
                    ("policyMappings", "1.2.3.4:1.2.3.5"),
                ],
            );
            let leaf = certificate(
                "Signer",
                Some(&mapper),
                &[53],
                &[("certificatePolicies", "1.2.3.5")],
            );
            let mut path = vec![leaf, mapper];
            path.extend(intermediates);
            let mut policy = config(&root, &path);
            policy["initial_policy_set"] = json!(["1.2.3.4"]);
            policy["explicit_policy"] = json!(true);
            let result = validate(&policy, &path);
            assert_eq!(
                result.is_ok(),
                counter == 1 && self_issued != Some(false),
                "mapping counter={counter}/self-issued={self_issued:?}: {result:?}"
            );
        }
    }
}

#[test]
fn critical_san_identity_never_waives_issuer_dn_chaining() {
    let (root, _) = chain();
    let intermediate = certificate(
        "",
        Some(&root),
        &[37],
        &[
            ("basicConstraints", "critical,CA:TRUE"),
            ("subjectAltName", "critical,DNS:ca.example.test"),
        ],
    );
    // The key is deliberately shared with the root, so signature mathematics
    // succeeds. Only the issuer/subject DN link discriminates this path.
    let signer = certificate("Signer", Some(&root), &[38], &[]);
    let path = vec![signer, intermediate];
    let error = validate(&config(&root, &path), &path).unwrap_err();
    assert!(error.message.contains("chain"), "{}", error.message);
}

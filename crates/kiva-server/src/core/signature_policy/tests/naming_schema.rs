//! Signed, single-cause Name and nested collection schema controls.
#![allow(clippy::unwrap_used)]

use super::{SignaturePolicy, chain, config, encoded, extension_schema::ExtensionCase, fixtures};
use ::der::{
    Decode, Encode,
    asn1::{ObjectIdentifier, OctetString},
};
use jsonwebtoken::Algorithm;

fn seq(parts: &[Vec<u8>]) -> Vec<u8> {
    fixtures::wrap(48, &parts.concat())
}
fn oid(value: &str) -> Vec<u8> {
    ObjectIdentifier::new(value).unwrap().to_der().unwrap()
}
fn ava(identifier: &str, tag: u8, bytes: &[u8]) -> Vec<u8> {
    seq(&[oid(identifier), fixtures::wrap(tag, bytes)])
}
fn name(identifier: &str, tag: u8, bytes: &[u8]) -> Vec<u8> {
    seq(&[fixtures::wrap(49, &ava(identifier, tag, bytes))])
}

#[allow(clippy::too_many_lines)]
fn names() -> Vec<(String, Vec<u8>, bool)> {
    let good = ava("2.5.4.3", 12, b"Signer");
    let mut multi = [good.clone(), ava("2.5.4.10", 12, b"Organization")];
    multi.sort();
    let mut values = vec![
        ("empty-name".into(), seq(&[]), true),
        ("empty-rdn".into(), seq(&[fixtures::wrap(49, &[])]), false),
        (
            "empty-before-valid".into(),
            seq(&[fixtures::wrap(49, &[]), fixtures::wrap(49, &good)]),
            false,
        ),
        (
            "empty-after-valid".into(),
            seq(&[fixtures::wrap(49, &good), fixtures::wrap(49, &[])]),
            false,
        ),
        (
            "multiple-avas".into(),
            seq(&[fixtures::wrap(49, &multi.concat())]),
            true,
        ),
        ("unknown-integer".into(), name("1.2.3.999", 2, &[7]), true),
    ];
    for identifier in [
        "2.5.4.3", "2.5.4.4", "2.5.4.7", "2.5.4.8", "2.5.4.10", "2.5.4.11", "2.5.4.12", "2.5.4.41",
        "2.5.4.42", "2.5.4.43", "2.5.4.44", "2.5.4.65",
    ] {
        for (label, tag, bytes, accepted) in [
            ("utf8", 12, b"Signer".as_slice(), true),
            ("printable", 19, b"Signer", true),
            ("integer", 2, &[7], false),
            ("octets", 4, b"Signer", false),
            ("ia5", 22, b"Signer", false),
            ("empty", 12, b"", false),
        ] {
            values.push((
                format!("{identifier}/{label}"),
                name(identifier, tag, bytes),
                accepted,
            ));
        }
    }
    for (label, identifier, tag, bytes, accepted) in [
        ("country", "2.5.4.6", 19, b"JP".to_vec(), true),
        ("country-short", "2.5.4.6", 19, b"J".to_vec(), false),
        ("country-long", "2.5.4.6", 19, b"JPN".to_vec(), false),
        ("country-utf8", "2.5.4.6", 12, b"JP".to_vec(), false),
        ("country-integer", "2.5.4.6", 2, vec![7], false),
        (
            "dc",
            "0.9.2342.19200300.100.1.25",
            22,
            b"example".to_vec(),
            true,
        ),
        (
            "dc-integer",
            "0.9.2342.19200300.100.1.25",
            2,
            vec![7],
            false,
        ),
        ("serial", "2.5.4.5", 19, b"123".to_vec(), true),
        ("serial-empty", "2.5.4.5", 19, vec![], false),
        ("qualifier-empty", "2.5.4.46", 19, vec![], true),
        ("qualifier-integer", "2.5.4.46", 2, vec![7], false),
        (
            "email",
            "1.2.840.113549.1.9.1",
            22,
            b"review@example.org".to_vec(),
            true,
        ),
        ("email-empty", "1.2.840.113549.1.9.1", 22, vec![], false),
        (
            "email-utf8",
            "1.2.840.113549.1.9.1",
            12,
            b"review@example.org".to_vec(),
            false,
        ),
        (
            "cn-64-scalars",
            "2.5.4.3",
            12,
            "😀".repeat(64).into_bytes(),
            true,
        ),
        (
            "cn-65-scalars",
            "2.5.4.3",
            12,
            "😀".repeat(65).into_bytes(),
            false,
        ),
        ("cn-bmp", "2.5.4.3", 30, vec![0, 65], true),
        ("cn-bmp-empty", "2.5.4.3", 30, vec![], false),
        ("cn-teletex-ascii", "2.5.4.3", 20, b"Signer".to_vec(), true),
    ] {
        values.push((label.into(), name(identifier, tag, &bytes), accepted));
    }
    values
}

fn extension(cert: &mut x509_cert::Certificate, identifier: &str, bytes: &[u8], critical: bool) {
    let identifier = ObjectIdentifier::new(identifier).unwrap();
    let extensions = cert.tbs_certificate.extensions.get_or_insert_with(Vec::new);
    extensions.retain(|ext| ext.extn_id != identifier);
    extensions.push(x509_cert::ext::Extension {
        extn_id: identifier,
        critical,
        extn_value: OctetString::new(bytes).unwrap(),
    });
}

#[allow(clippy::too_many_lines)]
pub(crate) fn integration_cases() -> Vec<ExtensionCase> {
    let (root, original) = chain();
    let mut cases = Vec::new();
    let mut push = |label: String, path: Vec<Vec<u8>>, accepted| {
        cases.push(ExtensionCase {
            label,
            policy: config(&root, &path),
            chain: path.iter().map(encoded).collect(),
            accepted,
        });
    };
    for (label, bytes, accepted) in names() {
        let mut leaf = x509_cert::Certificate::from_der(&original[0]).unwrap();
        leaf.tbs_certificate.subject = x509_cert::name::Name::from_der(&bytes).unwrap();
        extension(
            &mut leaf,
            "2.5.29.17",
            &seq(&[fixtures::wrap(130, b"review.example")]),
            true,
        );
        push(
            format!("subject/{label}"),
            vec![
                fixtures::resign(leaf, Algorithm::RS256),
                original[1].clone(),
            ],
            accepted,
        );
    }
    // Mirrored intermediate subject and leaf issuer establish valid linkage
    // apart from schema, including invalid names that would compare equal.
    for (label, bytes, accepted) in [
        ("valid", name("2.5.4.3", 12, b"CA"), true),
        ("empty-rdn", seq(&[fixtures::wrap(49, &[])]), false),
        ("integer", name("2.5.4.3", 2, &[7]), false),
    ] {
        let dn = x509_cert::name::Name::from_der(&bytes).unwrap();
        let mut leaf = x509_cert::Certificate::from_der(&original[0]).unwrap();
        let mut ca = x509_cert::Certificate::from_der(&original[1]).unwrap();
        leaf.tbs_certificate.issuer = dn.clone();
        ca.tbs_certificate.subject = dn;
        push(
            format!("issuer/{label}"),
            vec![
                fixtures::resign(leaf, Algorithm::RS256),
                fixtures::resign(ca, Algorithm::RS256),
            ],
            accepted,
        );
    }
    for (label, dn, accepted) in [
        ("valid", name("2.5.4.3", 12, b"Signer"), true),
        ("empty-rdn", seq(&[fixtures::wrap(49, &[])]), false),
        ("integer", name("2.5.4.3", 2, &[7]), false),
    ] {
        let general = fixtures::wrap(164, &dn);
        for (location, identifier, bytes) in [
            ("san", "2.5.29.17", seq(std::slice::from_ref(&general))),
            ("ian", "2.5.29.18", seq(std::slice::from_ref(&general))),
            (
                "constraints",
                "2.5.29.30",
                seq(&[fixtures::wrap(160, &seq(std::slice::from_ref(&general)))]),
            ),
            (
                "aki",
                "2.5.29.35",
                seq(&[fixtures::wrap(161, &general), vec![130, 1, 1]]),
            ),
            (
                "aia",
                "1.3.6.1.5.5.7.1.1",
                seq(&[seq(&[oid("1.3.6.1.5.5.7.48.1"), general.clone()])]),
            ),
            (
                "sia",
                "1.3.6.1.5.5.7.1.11",
                seq(&[seq(&[oid("1.3.6.1.5.5.7.48.5"), general.clone()])]),
            ),
            (
                "crldp",
                "2.5.29.31",
                seq(&[seq(&[fixtures::wrap(160, &fixtures::wrap(160, &general))])]),
            ),
            (
                "freshest",
                "2.5.29.46",
                seq(&[seq(&[fixtures::wrap(160, &fixtures::wrap(160, &general))])]),
            ),
        ] {
            for at in 0..2 {
                let mut path = original.clone();
                let mut cert = x509_cert::Certificate::from_der(&path[at]).unwrap();
                extension(&mut cert, identifier, &bytes, false);
                path[at] = fixtures::resign(cert, Algorithm::RS256);
                push(format!("{location}/{at}/{label}"), path, accepted);
            }
        }
    }
    for (label, extensions, accepted) in [("absent", None, true), ("empty", Some(vec![]), false)] {
        let mut leaf = x509_cert::Certificate::from_der(&original[0]).unwrap();
        leaf.tbs_certificate.extensions = extensions;
        push(
            format!("extensions/{label}"),
            vec![
                fixtures::resign(leaf, Algorithm::RS256),
                original[1].clone(),
            ],
            accepted,
        );
    }
    for identifier in [
        "1.3.6.1.5.5.7.1.1",
        "1.3.6.1.5.5.7.1.11",
        "2.5.29.31",
        "2.5.29.46",
    ] {
        for at in 0..2 {
            let mut path = original.clone();
            let mut cert = x509_cert::Certificate::from_der(&path[at]).unwrap();
            extension(&mut cert, identifier, &seq(&[]), false);
            path[at] = fixtures::resign(cert, Algorithm::RS256);
            push(format!("collection/{identifier}/{at}/empty"), path, false);
        }
    }
    for (label, identifier, bytes, accepted) in [
        (
            "relative-valid",
            "2.5.29.31",
            seq(&[seq(&[fixtures::wrap(
                160,
                &fixtures::wrap(161, &ava("2.5.4.3", 12, b"Signer")),
            )])]),
            true,
        ),
        (
            "relative-empty",
            "2.5.29.31",
            seq(&[seq(&[fixtures::wrap(160, &fixtures::wrap(161, &[]))])]),
            false,
        ),
        (
            "full-empty",
            "2.5.29.31",
            seq(&[seq(&[fixtures::wrap(160, &fixtures::wrap(160, &[]))])]),
            false,
        ),
        (
            "crl-issuer-empty",
            "2.5.29.31",
            seq(&[seq(&[fixtures::wrap(162, &[])])]),
            false,
        ),
        (
            "aki-issuer-empty",
            "2.5.29.35",
            seq(&[fixtures::wrap(161, &[]), vec![130, 1, 1]]),
            false,
        ),
        (
            "aki-issuer-absent",
            "2.5.29.35",
            seq(&[fixtures::wrap(128, b"key")]),
            true,
        ),
        ("unknown-empty", "1.2.3.999", seq(&[]), true),
    ] {
        let mut path = original.clone();
        let mut cert = x509_cert::Certificate::from_der(&path[0]).unwrap();
        extension(&mut cert, identifier, &bytes, false);
        path[0] = fixtures::resign(cert, Algorithm::RS256);
        push(label.into(), path, accepted);
    }
    cases
}

#[test]
fn certificate_naming_schema_preserves_types_sizes_and_optional_absence() {
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

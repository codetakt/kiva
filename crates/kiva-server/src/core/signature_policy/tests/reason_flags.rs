//! Signed DER contrasts for IMPLICIT `ReasonFlags` in both distribution extensions.
#![allow(clippy::unwrap_used)]

use super::{SignaturePolicy, chain, config, encoded, extension_schema::ExtensionCase, fixtures};
use ::der::{
    Decode,
    asn1::{BitString, ObjectIdentifier, OctetString},
};
use jsonwebtoken::Algorithm;

fn point(bits: Option<&[u8]>) -> Vec<u8> {
    let mut fields = fixtures::wrap(
        160,
        &fixtures::wrap(160, &fixtures::wrap(134, b"http://review.example/list")),
    );
    if let Some(bits) = bits {
        fields.extend(fixtures::wrap(129, bits));
    }
    fixtures::wrap(48, &fields)
}

fn vectors() -> Vec<(String, Vec<u8>, bool)> {
    let mut values = vec![
        ("absent".into(), point(None), true),
        ("empty".into(), point(Some(&[0])), true),
    ];
    // Every defined named bit, including the ninth bit, plus boundaries beyond
    // machine-sized flag sets. ASN.1 BIT STRING has no width constraint here.
    for bit in (0..9).chain([15, 16, 255]) {
        let mut content = vec![7 - bit % 8];
        content.extend(vec![0; usize::from(bit / 8)]);
        content.push(0x80 >> (bit % 8));
        values.push((format!("bit-{bit}"), point(Some(&content)), true));
    }
    for (label, content) in [
        ("trailing-zero-bits", vec![0, 64]),
        ("trailing-zero-octet", vec![0, 64, 0]),
        ("nonminimal-empty", vec![0, 0]),
        ("all-zero-padded", vec![7, 0]),
        ("bad-unused-count", vec![8, 64]),
        ("empty-with-unused", vec![1]),
        ("missing-count", vec![]),
        ("too-few-unused", vec![5, 64]),
    ] {
        values.push((label.into(), point(Some(&content)), false));
    }
    for unused in 1..8 {
        // Change just one padding bit of a canonically named one-bit value.
        let content = [unused, (1 << unused) | 1];
        values.push((
            format!("nonzero-unused-{unused}"),
            point(Some(&content)),
            false,
        ));
    }
    for bad_first in [true, false] {
        let good = point(Some(&[6, 64]));
        let bad = point(Some(&[6, 65]));
        values.push((
            format!("multiple-points/bad-first-{bad_first}"),
            if bad_first {
                [bad, good].concat()
            } else {
                [good, bad].concat()
            },
            false,
        ));
    }
    values
}

pub(crate) fn integration_cases() -> Vec<ExtensionCase> {
    let (root, original) = chain();
    let mut cases = Vec::new();
    let mut push = |label, path: Vec<Vec<u8>>, accepted| {
        cases.push(ExtensionCase {
            label,
            policy: config(&root, &path),
            chain: path.iter().map(encoded).collect(),
            accepted,
        });
    };
    for identifier in ["2.5.29.31", "2.5.29.46", "1.2.3.999"] {
        let values = if identifier == "1.2.3.999" {
            // An opaque unknown noncritical extension has no ReasonFlags type.
            vec![("opaque-unknown".into(), point(Some(&[6, 65])), true)]
        } else {
            vectors()
        };
        for at in 0..2 {
            for (label, points, accepted) in &values {
                let mut path = original.clone();
                let mut cert = x509_cert::Certificate::from_der(&path[at]).unwrap();
                cert.tbs_certificate
                    .extensions
                    .as_mut()
                    .unwrap()
                    .push(x509_cert::ext::Extension {
                        extn_id: ObjectIdentifier::new(identifier).unwrap(),
                        critical: false,
                        extn_value: OctetString::new(fixtures::wrap(48, points)).unwrap(),
                    });
                path[at] = fixtures::resign(cert, Algorithm::RS256);
                push(format!("{identifier}/{at}/{label}"), path, *accepted);
            }
        }
    }
    for at in 0..2 {
        let mut path = original.clone();
        let mut cert = x509_cert::Certificate::from_der(&path[at]).unwrap();
        // UniqueIdentifier is an ordinary BIT STRING, so trailing zero bits
        // are significant and must not receive named-bit-list shortening rules.
        cert.tbs_certificate.subject_unique_id = Some(BitString::new(0, vec![64, 0]).unwrap());
        path[at] = fixtures::resign(cert, Algorithm::RS256);
        push(format!("ordinary-bit-string/{at}"), path, true);
    }
    cases
}

#[test]
fn certificate_reason_flags_enforce_der_without_narrowing_bit_strings() {
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

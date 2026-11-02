//! OID codec and original-byte signed-path boundaries from DC-DER/DV-D08.
#![allow(clippy::unwrap_used)]

use super::{SignaturePolicy, chain, config, encoded, extension_schema::ExtensionCase, fixtures};
use ::der::{Decode, Encode, asn1::ObjectIdentifier};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, EncodingKey};
use serde_json::json;

// Literal base128 controls avoid using the production encoder as the oracle.
fn vectors() -> Vec<(String, Vec<u8>)> {
    let mut values = [
        ("0.0", vec![0]),
        ("0.39", vec![39]),
        ("1.0", vec![40]),
        ("1.39", vec![79]),
        ("2.0", vec![80]),
        ("2.39", vec![119]),
        ("2.40", vec![120]),
        ("2.47", vec![127]),
        ("2.48", vec![129, 0]),
        ("2.999", vec![136, 55]),
        ("1.2.3", vec![42, 3]),
        ("1.2.3.4", vec![42, 3, 4]),
        ("1.2.128", vec![42, 129, 0]),
        ("1.2.16384", vec![42, 129, 128, 0]),
        ("1.2.268435455", vec![42, 255, 255, 255, 127]),
        ("2.268435455", vec![129, 128, 128, 128, 79]),
        ("1.2.4294967295", vec![42, 143, 255, 255, 255, 127]),
        ("2.4294967295", vec![144, 128, 128, 128, 79]),
    ]
    .into_iter()
    .map(|(text, bytes)| (text.to_owned(), bytes))
    .collect::<Vec<_>>();
    for (count, tail) in [(12, vec![129, 0]), (13, vec![])] {
        values.push((
            format!(
                "1.2{}{}",
                ".16384".repeat(count),
                if tail.is_empty() { "" } else { ".128" }
            ),
            [vec![42], [129, 128, 0].repeat(count), tail].concat(),
        ));
    }
    // Exactly 20 arcs / 100 decimal characters; all arcs below 2^28.
    values.push((
        format!("1.2{}.1.1.10", ".16384".repeat(15)),
        [vec![42], [129, 128, 0].repeat(15), vec![1, 1, 10]].concat(),
    ));
    values
}

fn malformed() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("empty", vec![]),
        ("truncated-first", vec![129]),
        ("truncated-later", vec![42, 129]),
        ("nonminimal-first", vec![128, 80]),
        ("nonminimal-later", vec![42, 128, 1]),
        ("second-arc-overflow", vec![144, 128, 128, 128, 80]),
        ("later-arc-overflow", vec![42, 144, 128, 128, 128, 0]),
    ]
}

#[test]
fn oid_domain_codec_preserves_identity_and_rejects_malformed_values() {
    let values = vectors();
    for (text, bytes) in &values {
        let value = ObjectIdentifier::new(text).unwrap();
        assert_eq!(value.as_bytes(), bytes, "{text}");
        let decoded = ObjectIdentifier::from_bytes(bytes).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(decoded.to_string(), *text);
        assert_eq!(ObjectIdentifier::from_arcs(decoded.arcs()).unwrap(), value);
        let der = fixtures::wrap(6, bytes);
        assert_eq!(ObjectIdentifier::from_der(&der).unwrap(), value);
        assert_eq!(value.to_der().unwrap(), der);
        assert_eq!(value.push_arc(128).unwrap().parent(), Some(value));
        for (other, _) in &values {
            assert_eq!(
                value == ObjectIdentifier::new(other).unwrap(),
                text == other
            );
        }
    }
    for (name, bytes) in malformed() {
        assert!(ObjectIdentifier::from_bytes(&bytes).is_err(), "{name}");
        assert!(
            ObjectIdentifier::from_der(&fixtures::wrap(6, &bytes)).is_err(),
            "{name}"
        );
    }
    for text in [
        "",
        "1",
        "3.0",
        "0.40",
        "1.40",
        "1..2",
        ".1.2",
        "1.2.",
        "2.4294967296",
        "1.2.4294967296",
        "1.2.18446744073709551616",
    ] {
        assert!(ObjectIdentifier::new(text).is_err(), "{text}");
    }
    // Keep the published finite capacity coherent across all constructors.
    for length in [39, 40, 77, 128, 255] {
        let bytes = vec![1; length];
        let value = ObjectIdentifier::from_bytes(&bytes).unwrap();
        assert_eq!(ObjectIdentifier::new(&value.to_string()).unwrap(), value);
        assert_eq!(
            ObjectIdentifier::from_der(&value.to_der().unwrap()).unwrap(),
            value
        );
        if length == 255 {
            assert!(value.push_arc(1).is_err());
        }
    }
    assert!(ObjectIdentifier::from_bytes(&[1; 256]).is_err());
}

// These helpers splice and sign actual fixture octets, including malformed OIDs.
// They do not round-trip a negative certificate through the typed decoder.
fn elements(mut bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut values = Vec::new();
    while !bytes.is_empty() {
        let (at, length) = if bytes[1] < 128 {
            (2, usize::from(bytes[1]))
        } else {
            let width = usize::from(bytes[1] & 127);
            (
                2 + width,
                bytes[2..2 + width]
                    .iter()
                    .fold(0_usize, |n, b| n * 256 + usize::from(*b)),
            )
        };
        let end = at + length;
        values.push(bytes[..end].to_vec());
        bytes = &bytes[end..];
    }
    values
}
fn content(bytes: &[u8]) -> &[u8] {
    let at = if bytes[1] < 128 {
        2
    } else {
        2 + usize::from(bytes[1] & 127)
    };
    &bytes[at..]
}
fn add_extension(bytes: &[u8], oid: &[u8], critical: bool) -> Vec<u8> {
    let outer = elements(content(bytes));
    let mut fields = elements(content(&outer[0]));
    let field = fields.iter_mut().find(|f| f[0] == 163).unwrap();
    let mut extensions = content(content(field)).to_vec();
    let extension = [
        fixtures::wrap(6, oid),
        if critical { vec![1, 1, 255] } else { vec![] },
        fixtures::wrap(4, &fixtures::wrap(4, b"opaque")),
    ]
    .concat();
    extensions.extend(fixtures::wrap(48, &extension));
    *field = fixtures::wrap(163, &fixtures::wrap(48, &extensions));
    let tbs = fixtures::wrap(48, &fields.concat());
    let key = EncodingKey::from_rsa_pem(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-key.pem"
    )))
    .unwrap();
    let signature = jsonwebtoken::crypto::sign(&tbs, &key, Algorithm::RS256).unwrap();
    let bits = [vec![0], URL_SAFE_NO_PAD.decode(signature).unwrap()].concat();
    fixtures::wrap(
        48,
        &[tbs, outer[1].clone(), fixtures::wrap(3, &bits)].concat(),
    )
}

fn known_extension(bytes: &[u8], identifier: &str, value: Vec<u8>) -> Vec<u8> {
    let mut cert = x509_cert::Certificate::from_der(bytes).unwrap();
    cert.tbs_certificate
        .extensions
        .as_mut()
        .unwrap()
        .push(x509_cert::ext::Extension {
            extn_id: ObjectIdentifier::new(identifier).unwrap(),
            critical: true,
            extn_value: ::der::asn1::OctetString::new(value).unwrap(),
        });
    fixtures::resign(cert, Algorithm::RS256)
}

pub(crate) fn integration_cases() -> Vec<ExtensionCase> {
    let (root, original) = chain();
    let mut cases = Vec::new();
    let mut push = |label, at: usize, oid: &[u8], critical, accepted| {
        let mut path = original.clone();
        path[at] = add_extension(&path[at], oid, critical);
        cases.push(ExtensionCase {
            label,
            policy: config(&root, &path),
            chain: path.iter().map(encoded).collect(),
            accepted,
        });
    };
    for at in 0..2 {
        for (text, bytes) in vectors() {
            for critical in [false, true] {
                push(
                    format!("{at}/{text}/critical-{critical}"),
                    at,
                    &bytes,
                    critical,
                    !critical,
                );
            }
        }
        for (name, bytes) in malformed() {
            push(format!("{at}/malformed-{name}"), at, &bytes, false, false);
        }
    }
    // OIDs also carry identity inside recognized extensions and issuer names.
    // Equal/different policy, EKU and Name controls prohibit a lossy surrogate.
    for (text, bytes) in vectors()
        .into_iter()
        .filter(|(text, bytes)| text == "1.2.3" || text == "2.999" || bytes.len() == 40)
    {
        let oid = fixtures::wrap(6, &bytes);
        for accepted in [true, false] {
            let selected = if accepted {
                text.clone()
            } else {
                format!("{text}.1")
            };
            let path = original
                .iter()
                .map(|cert| {
                    known_extension(
                        cert,
                        "2.5.29.32",
                        fixtures::wrap(48, &fixtures::wrap(48, &oid)),
                    )
                })
                .collect::<Vec<_>>();
            let mut policy = config(&root, &path);
            policy["initial_policy_set"] = json!([selected]);
            policy["explicit_policy"] = json!(true);
            cases.push(ExtensionCase {
                label: format!("policy/{text}/match-{accepted}"),
                policy,
                chain: path.iter().map(encoded).collect(),
                accepted,
            });

            let mut path = original.clone();
            path[0] = known_extension(&path[0], "2.5.29.37", fixtures::wrap(48, &oid));
            let mut policy = config(&root, &path);
            policy["allowed_extended_key_usages"] = json!([selected]);
            cases.push(ExtensionCase {
                label: format!("eku/{text}/match-{accepted}"),
                policy,
                chain: path.iter().map(encoded).collect(),
                accepted,
            });

            let mut issuer = x509_cert::Certificate::from_der(&original[1]).unwrap();
            let mut leaf = x509_cert::Certificate::from_der(&original[0]).unwrap();
            let rdn = |identifier: &[u8]| {
                x509_cert::name::RelativeDistinguishedName::from_der(&fixtures::wrap(
                    49,
                    &fixtures::wrap(
                        48,
                        &[identifier.to_vec(), fixtures::wrap(4, b"identity")].concat(),
                    ),
                ))
                .unwrap()
            };
            issuer.tbs_certificate.subject.0.push(rdn(&oid));
            let selected = ObjectIdentifier::new(&selected).unwrap().to_der().unwrap();
            leaf.tbs_certificate.issuer.0.push(rdn(&selected));
            let path = vec![
                fixtures::resign(leaf, Algorithm::RS256),
                fixtures::resign(issuer, Algorithm::RS256),
            ];
            cases.push(ExtensionCase {
                label: format!("issuer-name/{text}/match-{accepted}"),
                policy: config(&root, &path),
                chain: path.iter().map(encoded).collect(),
                accepted,
            });
        }
    }
    cases
}

#[test]
fn certificate_oid_domain_governs_path_validation() {
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

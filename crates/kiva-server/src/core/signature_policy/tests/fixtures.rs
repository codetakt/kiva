//! Typed, deterministically signed certificates using the public test RSA key.
#![allow(clippy::unwrap_used, clippy::panic)]

use ::der::{Encode, asn1::ObjectIdentifier};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use jsonwebtoken::{Algorithm, EncodingKey};
use x509_parser::prelude::{FromDer, X509Certificate};

pub(super) type X509 = Vec<u8>;

pub(super) fn wrap(tag: u8, bytes: &[u8]) -> Vec<u8> {
    let mut result = vec![tag];
    if bytes.len() < 128 {
        result.push(u8::try_from(bytes.len()).unwrap());
    } else {
        let encoded = bytes.len().to_be_bytes();
        let encoded = &encoded[encoded.iter().position(|byte| *byte != 0).unwrap()..];
        result.push(0x80 | u8::try_from(encoded.len()).unwrap());
        result.extend_from_slice(encoded);
    }
    result.extend_from_slice(bytes);
    result
}

fn sequence(parts: &[Vec<u8>]) -> Vec<u8> {
    wrap(0x30, &parts.concat())
}
fn oid(value: &str) -> Vec<u8> {
    ObjectIdentifier::new(value).unwrap().to_der().unwrap()
}

fn general_name(value: &str) -> Vec<u8> {
    let (kind, value) = value.split_once(':').unwrap();
    match kind {
        "DNS" => wrap(0x82, value.as_bytes()),
        "email" => wrap(0x81, value.as_bytes()),
        "URI" => wrap(0x86, value.as_bytes()),
        "IP" => wrap(
            0x87,
            &value
                .split(':')
                .map(|v| u8::from_str_radix(v, 16).unwrap())
                .collect::<Vec<_>>(),
        ),
        "dirName" => wrap(
            0xa4,
            &sequence(&[wrap(
                0x31,
                &sequence(&[oid("2.5.4.3"), wrap(12, value.as_bytes())]),
            )]),
        ),
        _ => panic!("unknown fixture GeneralName {kind}"),
    }
}

pub(super) fn spki() -> Vec<u8> {
    let bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-certificate.der"
    ));
    X509Certificate::from_der(bytes)
        .unwrap()
        .1
        .public_key()
        .raw
        .to_vec()
}

pub(super) fn encoded(cert: &X509) -> String {
    STANDARD.encode(cert)
}

pub(super) fn with_names(
    cert: &X509,
    issuer: &x509_cert::name::Name,
    subject: &x509_cert::name::Name,
) -> X509 {
    use ::der::Decode;
    let mut cert = x509_cert::Certificate::from_der(cert).unwrap();
    cert.tbs_certificate.issuer = issuer.clone();
    cert.tbs_certificate.subject = subject.clone();
    resign(cert, Algorithm::RS256)
}

pub(super) fn resign(mut cert: x509_cert::Certificate, algorithm: Algorithm) -> X509 {
    use ::der::asn1::BitString;
    let key = EncodingKey::from_rsa_pem(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-key.pem"
    )))
    .unwrap();
    let signature =
        jsonwebtoken::crypto::sign(&cert.tbs_certificate.to_der().unwrap(), &key, algorithm)
            .unwrap();
    cert.signature = BitString::from_bytes(&URL_SAFE_NO_PAD.decode(signature).unwrap()).unwrap();
    cert.to_der().unwrap()
}

pub(super) fn certificate(
    name: &str,
    issuer: Option<&X509>,
    serial: &[u8],
    extensions: &[(&str, &str)],
) -> X509 {
    certificate_with_validity(
        name,
        issuer,
        serial,
        extensions,
        [(23, "250101000000Z"), (23, "350101000000Z")],
    )
}

pub(super) fn certificate_with_validity(
    name: &str,
    issuer: Option<&X509>,
    serial: &[u8],
    extensions: &[(&str, &str)],
    validity: [(u8, &str); 2],
) -> X509 {
    certificate_with_keys(name, issuer, serial, extensions, validity, &spki(), false)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn certificate_with_keys(
    name: &str,
    issuer: Option<&X509>,
    serial: &[u8],
    extensions: &[(&str, &str)],
    validity: [(u8, &str); 2],
    subject_spki: &[u8],
    sign_with_rollover: bool,
) -> X509 {
    let subject = if name.is_empty() {
        sequence(&[])
    } else {
        sequence(&[wrap(
            0x31,
            &sequence(&[oid("2.5.4.3"), wrap(12, name.as_bytes())]),
        )])
    };
    let issuer = issuer.map_or_else(
        || subject.clone(),
        |bytes| {
            X509Certificate::from_der(bytes)
                .unwrap()
                .1
                .subject()
                .as_raw()
                .to_vec()
        },
    );
    let mut positive = serial.to_vec();
    if positive[0] & 0x80 != 0 {
        positive.insert(0, 0);
    }
    let algorithm = sequence(&[oid("1.2.840.113549.1.1.11"), vec![5, 0]]);
    let mut fields = vec![
        vec![0xa0, 3, 2, 1, 2],
        wrap(2, &positive),
        algorithm.clone(),
        issuer,
        sequence(&validity.map(|(tag, text)| wrap(tag, text.as_bytes()))),
        subject,
        subject_spki.to_vec(),
    ];
    if !extensions.is_empty() {
        fields.push(wrap(
            0xa3,
            &sequence(
                &extensions
                    .iter()
                    .map(|(name, value)| extension(name, value))
                    .collect::<Vec<_>>(),
            ),
        ));
    }
    let tbs = sequence(&fields);
    let ordinary = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-key.pem"
    ));
    let rollover = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/fixtures/signing/synthetic-rollover-key.pem"
    ));
    let key = EncodingKey::from_rsa_pem(if sign_with_rollover {
        &rollover[..]
    } else {
        &ordinary[..]
    })
    .unwrap();
    let signature = jsonwebtoken::crypto::sign(&tbs, &key, Algorithm::RS256).unwrap();
    let mut bits = vec![0];
    bits.extend(URL_SAFE_NO_PAD.decode(signature).unwrap());
    sequence(&[tbs, algorithm, wrap(3, &bits)])
}

#[allow(clippy::too_many_lines)] // The fixture encoder keeps each ASN.1 extension shape explicit.
fn extension(name: &str, value: &str) -> Vec<u8> {
    let (critical, value) = value
        .strip_prefix("critical,")
        .map_or((false, value), |value| (true, value));
    let (identifier, content) = match name {
        "basicConstraints" => {
            let mut parts = if value.contains("CA:TRUE") {
                vec![vec![1, 1, 0xff]]
            } else {
                vec![]
            };
            if let Some((_, limit)) = value.split_once("pathlen:") {
                parts.push(vec![2, 1, limit.parse::<u8>().unwrap()]);
            }
            ("2.5.29.19", sequence(&parts))
        }
        "keyUsage" => {
            let mut bits = 0_u8;
            for usage in value.split(',') {
                bits |= match usage {
                    "digitalSignature" => 128,
                    "nonRepudiation" => 64,
                    "keyCertSign" => 4,
                    "cRLSign" => 2,
                    _ => panic!("unknown fixture KU {usage}"),
                };
            }
            (
                "2.5.29.15",
                wrap(3, &[u8::try_from(bits.trailing_zeros()).unwrap(), bits]),
            )
        }
        "extendedKeyUsage" => (
            "2.5.29.37",
            sequence(
                &value
                    .split(',')
                    .map(|purpose| {
                        oid(match purpose {
                            "codeSigning" => "1.3.6.1.5.5.7.3.3",
                            "serverAuth" => "1.3.6.1.5.5.7.3.1",
                            "anyExtendedKeyUsage" => "2.5.29.37.0",
                            other => other,
                        })
                    })
                    .collect::<Vec<_>>(),
            ),
        ),
        "subjectAltName" => (
            "2.5.29.17",
            sequence(&value.split(',').map(general_name).collect::<Vec<_>>()),
        ),
        "nameConstraints" => {
            let mut permitted = Vec::new();
            let mut excluded = Vec::new();
            for item in value.split(',') {
                let (kind, name) = item.split_once(';').unwrap();
                let subtree = sequence(&[general_name(name)]);
                if kind == "permitted" {
                    permitted.extend(subtree);
                } else {
                    excluded.extend(subtree);
                }
            }
            let mut parts = Vec::new();
            if !permitted.is_empty() {
                parts.push(wrap(0xa0, &permitted));
            }
            if !excluded.is_empty() {
                parts.push(wrap(0xa1, &excluded));
            }
            ("2.5.29.30", sequence(&parts))
        }
        "certificatePolicies" => (
            "2.5.29.32",
            sequence(
                &value
                    .split(',')
                    .map(|policy| sequence(&[oid(policy)]))
                    .collect::<Vec<_>>(),
            ),
        ),
        "policyMappings" => (
            "2.5.29.33",
            sequence(
                &value
                    .split(',')
                    .map(|mapping| {
                        let (a, b) = mapping.split_once(':').unwrap();
                        sequence(&[oid(a), oid(b)])
                    })
                    .collect::<Vec<_>>(),
            ),
        ),
        "policyConstraints" => (
            "2.5.29.36",
            sequence(
                &value
                    .split(',')
                    .map(|constraint| {
                        let (name, n) = constraint.split_once(':').unwrap();
                        wrap(
                            if name == "requireExplicitPolicy" {
                                0x80
                            } else {
                                0x81
                            },
                            &[n.parse::<u8>().unwrap()],
                        )
                    })
                    .collect::<Vec<_>>(),
            ),
        ),
        "inhibitAnyPolicy" => ("2.5.29.54", wrap(2, &[value.parse::<u8>().unwrap()])),
        other => (
            other,
            value
                .strip_prefix("DER:")
                .unwrap()
                .split(':')
                .map(|octet| u8::from_str_radix(octet, 16).unwrap())
                .collect(),
        ),
    };
    let mut fields = vec![oid(identifier)];
    if critical {
        fields.push(vec![1, 1, 0xff]);
    }
    fields.push(wrap(4, &content));
    sequence(&fields)
}

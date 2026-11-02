//! Signed-path controls with an independently enumerated finite name-set model.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{SignaturePolicy, X509, certificate, chain, config, encoded, validate};
use serde_json::Value;

pub(crate) struct NameConstraintCase {
    pub label: &'static str,
    pub policy: Value,
    pub chain: Vec<String>,
    pub accepted: bool,
}

pub(crate) fn integration_cases() -> Vec<NameConstraintCase> {
    let mut cases = [
        (
            "empty DNS intersection stays empty",
            vec![
                "permitted;DNS:example.org",
                "permitted;DNS:other.net",
                "permitted;DNS:other.net",
            ],
            "DNS:a.other.net",
            false,
        ),
        (
            "identical URI constraints",
            vec!["permitted;URI:example.org"; 2],
            "URI:https://example.org/x",
            true,
        ),
        (
            "identical email constraints",
            vec!["permitted;email:example.org"; 2],
            "email:a@example.org",
            true,
        ),
        (
            "identical IP constraints",
            vec!["permitted;IP:c0:00:02:00:ff:ff:ff:00"; 2],
            "IP:c0:00:02:05",
            true,
        ),
        (
            "excluded URI with hostless SAN",
            vec!["excluded;URI:example.org"],
            "URI:urn:example:signer",
            false,
        ),
        (
            "excluded URI with IPv4 SAN",
            vec!["excluded;URI:example.org"],
            "URI:https://192.0.2.1/x",
            false,
        ),
        (
            "excluded URI with IPv6 SAN",
            vec!["excluded;URI:example.org"],
            "URI:https://[2001:db8::1]/x",
            false,
        ),
        (
            "unconstrained hostless SAN",
            vec![],
            "URI:urn:example:signer",
            true,
        ),
        (
            "excluded matching URI",
            vec!["excluded;URI:example.org"],
            "URI:https://example.org/x",
            false,
        ),
        (
            "excluded unrelated URI",
            vec!["excluded;URI:example.org"],
            "URI:https://other.net/x",
            true,
        ),
    ]
    .into_iter()
    .map(|(label, constraints, san, accepted)| signed_case(label, &constraints, san, accepted))
    .collect::<Vec<_>>();
    cases.extend(unsupported_cases());
    cases
}

fn signed_case(
    label: &'static str,
    constraints: &[&str],
    san: &str,
    accepted: bool,
) -> NameConstraintCase {
    let (root, _) = chain();
    let mut issuer = root.clone();
    let mut path = Vec::new();
    for (index, constraint) in constraints.iter().enumerate() {
        let ca = certificate(
            &format!("Constraint CA {index}"),
            Some(&issuer),
            &[1, u8::try_from(index).unwrap()],
            &[
                ("basicConstraints", "critical,CA:TRUE"),
                ("nameConstraints", &format!("critical,{constraint}")),
            ],
        );
        issuer = ca.clone();
        path.push(ca);
    }
    path.push(certificate(
        "Signer",
        Some(&issuer),
        &[2, 1],
        &[("subjectAltName", san)],
    ));
    path.reverse();
    NameConstraintCase {
        label,
        policy: config(&root, &path),
        chain: path.iter().map(encoded).collect(),
        accepted,
    }
}

#[test]
fn name_constraint_review_counterexamples_have_correct_path_results() {
    for case in integration_cases() {
        let result = SignaturePolicy::from_json(case.policy.to_string().as_bytes())
            .unwrap()
            .validate_chain(&case.chain);
        assert_eq!(result.is_ok(), case.accepted, "{}: {result:?}", case.label);
    }
}

struct NameModel<'a> {
    names: &'a [&'a str],
    // Explicit membership masks over `names`; no production name comparator
    // computes the oracle. Each certificate contributes one such allowed set.
    groups: &'a [(&'a str, u8)],
}

fn check_model(model: &NameModel<'_>) {
    let (root, _) = chain();
    let mut issuer = root.clone();
    let mut stages: Vec<Vec<X509>> = Vec::new();
    for depth in 0..3 {
        let stage = model
            .groups
            .iter()
            .enumerate()
            .map(|(index, (constraints, _))| {
                let mut extensions = vec![("basicConstraints", "critical,CA:TRUE")];
                if !constraints.is_empty() {
                    extensions.push(("nameConstraints", constraints));
                }
                certificate(
                    &format!("Model CA {depth}"),
                    Some(&issuer),
                    &[
                        u8::try_from(depth + 1).unwrap(),
                        u8::try_from(index).unwrap(),
                    ],
                    &extensions,
                )
            })
            .collect::<Vec<_>>();
        issuer = stage[0].clone();
        stages.push(stage);
    }
    let leaves = model
        .names
        .iter()
        .enumerate()
        .map(|(index, san)| {
            certificate(
                "Signer",
                Some(&issuer),
                &[4, u8::try_from(index).unwrap()],
                &[("subjectAltName", san)],
            )
        })
        .collect::<Vec<_>>();
    for (a, (_, mask_a)) in model.groups.iter().enumerate() {
        for (b, (_, mask_b)) in model.groups.iter().enumerate() {
            for (c, (_, mask_c)) in model.groups.iter().enumerate() {
                let allowed = mask_a & mask_b & mask_c;
                for (index, leaf) in leaves.iter().enumerate() {
                    let path = vec![
                        leaf.clone(),
                        stages[2][c].clone(),
                        stages[1][b].clone(),
                        stages[0][a].clone(),
                    ];
                    let result = validate(&config(&root, &path), &path);
                    assert_eq!(
                        result.is_ok(),
                        allowed & (1 << index) != 0,
                        "groups {a}/{b}/{c}; name {}: {result:?}",
                        model.names[index]
                    );
                }
            }
        }
    }
}

#[test]
fn name_constraint_dns_sets_follow_intersection_and_exclusion_model() {
    check_model(&NameModel {
        names: &[
            "DNS:example.org",
            "DNS:a.example.org",
            "DNS:a.sub.example.org",
            "DNS:a.other.net",
        ],
        groups: &[
            ("", 0b1111),
            ("critical,permitted;DNS:example.org", 0b0111),
            ("critical,permitted;DNS:.example.org", 0b0110),
            ("critical,permitted;DNS:sub.example.org", 0b0100),
            ("critical,permitted;DNS:other.net", 0b1000),
            (
                "critical,permitted;DNS:example.org,permitted;DNS:other.net",
                0b1111,
            ),
            ("critical,excluded;DNS:sub.example.org", 0b1011),
            ("critical,permitted;URI:unrelated.test", 0b1111),
        ],
    });
}

#[test]
fn name_constraint_uri_sets_follow_exact_host_and_subdomain_model() {
    check_model(&NameModel {
        names: &[
            "URI:https://example.org/",
            "URI:https://a.example.org/",
            "URI:ftp://a.sub.example.org/",
            "URI:https://other.net/",
        ],
        groups: &[
            ("", 0b1111),
            ("critical,permitted;URI:example.org", 0b0001),
            ("critical,permitted;URI:.example.org", 0b0110),
            ("critical,permitted;URI:.sub.example.org", 0b0100),
            ("critical,permitted;URI:other.net", 0b1000),
            (
                "critical,permitted;URI:example.org,permitted;URI:.example.org",
                0b0111,
            ),
            ("critical,excluded;URI:.sub.example.org", 0b1011),
            ("critical,permitted;DNS:unrelated.test", 0b1111),
        ],
    });
}

#[test]
fn name_constraint_mail_sets_preserve_local_case_and_domain_roles() {
    check_model(&NameModel {
        names: &[
            "email:Alice@example.org",
            "email:alice@example.org",
            "email:a@sub.example.org",
            "email:a@other.net",
        ],
        groups: &[
            ("", 0b1111),
            ("critical,permitted;email:example.org", 0b0011),
            ("critical,permitted;email:Alice@EXAMPLE.ORG", 0b0001),
            ("critical,permitted;email:.example.org", 0b0100),
            ("critical,permitted;email:other.net", 0b1000),
            (
                "critical,permitted;email:example.org,permitted;email:.example.org",
                0b0111,
            ),
            ("critical,excluded;email:alice@example.org", 0b1101),
            ("critical,permitted;URI:unrelated.test", 0b1111),
        ],
    });
}

#[test]
fn name_constraint_ip_sets_preserve_network_masks_and_address_families() {
    check_model(&NameModel {
        names: &[
            "IP:c0:00:02:05",
            "IP:c0:00:02:85",
            "IP:c6:33:64:05",
            "IP:20:01:0d:b8:00:00:00:00:00:00:00:00:00:00:00:01",
        ],
        groups: &[
            ("", 0b1111),
            ("critical,permitted;IP:c0:00:02:00:ff:ff:ff:00", 0b0011),
            ("critical,permitted;IP:c0:00:02:00:ff:ff:ff:80", 0b0001),
            ("critical,permitted;IP:c6:33:64:00:ff:ff:ff:00", 0b0100),
            (
                "critical,permitted;IP:20:01:0d:b8:00:00:00:00:00:00:00:00:00:00:00:00:ff:ff:ff:ff:00:00:00:00:00:00:00:00:00:00:00:00",
                0b1000,
            ),
            (
                "critical,permitted;IP:c0:00:02:00:ff:ff:ff:00,permitted;IP:c6:33:64:00:ff:ff:ff:00",
                0b0111,
            ),
            ("critical,excluded;IP:c0:00:02:80:ff:ff:ff:80", 0b1101),
            ("critical,permitted;DNS:unrelated.test", 0b1111),
        ],
    });
}

fn unsupported_cases() -> Vec<NameConstraintCase> {
    use super::fixtures::wrap;
    let (root, _) = chain();
    // registeredID 1.2.3.4 is a legal GeneralName, with no matcher in this provider.
    let registered = wrap(0x88, &[42, 3, 4]);
    let der = |bytes: &[u8]| {
        format!(
            "DER:{}",
            bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(":")
        )
    };
    let san = der(&wrap(0x30, &registered));
    let mut cases = Vec::new();
    for excluded in [false, true] {
        let nc = wrap(
            0x30,
            &wrap(if excluded { 0xa1 } else { 0xa0 }, &wrap(0x30, &registered)),
        );
        for critical in [false, true] {
            let constraint = format!("{}{}", if critical { "critical," } else { "" }, der(&nc));
            let ca = certificate(
                "Registered CA",
                Some(&root),
                &[6, 1],
                &[
                    ("basicConstraints", "critical,CA:TRUE"),
                    ("2.5.29.30", &constraint),
                ],
            );
            for present in [false, true] {
                let extensions = if present {
                    vec![("2.5.29.17", san.as_str())]
                } else {
                    vec![]
                };
                let leaf = certificate("Signer", Some(&ca), &[6, 2], &extensions);
                let path = vec![leaf, ca.clone()];
                cases.push(NameConstraintCase {
                    label: if critical && present {
                        "unsupported critical form encountered"
                    } else {
                        "unsupported form outside mandatory applicability"
                    },
                    policy: config(&root, &path),
                    chain: path.iter().map(encoded).collect(),
                    accepted: !(critical && present),
                });
            }
        }
    }
    cases
}

#[test]
fn name_constraint_unsupported_critical_forms_cannot_silently_succeed() {
    for case in unsupported_cases() {
        let result = SignaturePolicy::from_json(case.policy.to_string().as_bytes())
            .unwrap()
            .validate_chain(&case.chain);
        assert_eq!(result.is_ok(), case.accepted, "{}: {result:?}", case.label);
    }
}

#[test]
fn name_constraint_unsupported_forms_keep_self_issued_exception_position_bound() {
    let (root, _) = chain();
    let first = certificate(
        "Registered CA",
        Some(&root),
        &[7, 1],
        &[
            ("basicConstraints", "critical,CA:TRUE"),
            // critical permitted registeredID 1.2.3.4
            ("2.5.29.30", "critical,DER:30:09:a0:07:30:05:88:03:2a:03:04"),
        ],
    );
    for self_issued in [false, true] {
        let next = certificate(
            if self_issued {
                "Registered CA"
            } else {
                "Next CA"
            },
            Some(&first),
            &[7, 2],
            &[
                ("basicConstraints", "critical,CA:TRUE"),
                ("2.5.29.17", "DER:30:05:88:03:2a:03:04"),
            ],
        );
        for present in [false, true] {
            let extensions = if present {
                vec![("2.5.29.17", "DER:30:05:88:03:2a:03:04")]
            } else {
                vec![]
            };
            let leaf = certificate("Signer", Some(&next), &[7, 3], &extensions);
            let path = vec![leaf, next.clone(), first.clone()];
            assert_eq!(
                validate(&config(&root, &path), &path).is_ok(),
                self_issued && !present,
                "self-issued={self_issued}, final-name-present={present}"
            );
        }
        let path = vec![next, first.clone()];
        assert!(
            validate(&config(&root, &path), &path).is_err(),
            "a final signer has no self-issued name exception"
        );
    }
}

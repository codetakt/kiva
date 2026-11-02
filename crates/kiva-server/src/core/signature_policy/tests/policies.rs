//! Independent finite policy-set/relation models and signed boundary witnesses.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::{
    SignaturePolicy, X509, certificate, chain, config, encoded, provider, timestamp, validate,
};
use serde_json::{Value, json};

const P: &str = "1.2.3.4";
const Q: &str = "1.2.3.5";
const ANY: &str = "2.5.29.32.0";

pub(crate) struct PolicyCase {
    pub label: &'static str,
    pub policy: Value,
    pub chain: Vec<String>,
    pub accepted: bool,
}

fn signed_case(
    label: &'static str,
    levels: &[(&str, &str)],
    initial: &str,
    accepted: bool,
) -> PolicyCase {
    let (root, _) = chain();
    let mut issuer = root.clone();
    let mut path = Vec::new();
    for (depth, (policies, mappings)) in levels.iter().enumerate() {
        let name = format!("Policy certificate {depth}");
        let mut extensions = if depth + 1 < levels.len() {
            vec![
                ("basicConstraints", "critical,CA:TRUE"),
                ("keyUsage", "keyCertSign"),
            ]
        } else {
            vec![("keyUsage", "digitalSignature")]
        };
        if !policies.is_empty() {
            extensions.push(("certificatePolicies", policies));
        }
        if !mappings.is_empty() {
            extensions.push(("policyMappings", mappings));
        }
        let cert = certificate(
            &name,
            Some(&issuer),
            &[9, u8::try_from(depth).unwrap()],
            &extensions,
        );
        issuer = cert.clone();
        path.push(cert);
    }
    path.reverse();
    let mut policy = config(&root, &path);
    policy["initial_policy_set"] = json!(initial.split(',').collect::<Vec<_>>());
    policy["explicit_policy"] = json!(true);
    PolicyCase {
        label,
        policy,
        chain: path.iter().map(encoded).collect(),
        accepted,
    }
}

pub(crate) fn integration_cases() -> Vec<PolicyCase> {
    let mut cases = vec![
        signed_case("matching policy", &[(P, ""), (P, "")], P, true),
        signed_case("incompatible policy", &[(Q, ""), (Q, "")], P, false),
        signed_case(
            "any CA cannot bypass initial set",
            &[(ANY, ""), (Q, "")],
            P,
            false,
        ),
        signed_case(
            "matching child under any CA",
            &[(ANY, ""), (P, "")],
            P,
            true,
        ),
        signed_case("explicit any initial set", &[(P, ""), (P, "")], ANY, true),
        signed_case("explicit any direct signer", &[(P, "")], ANY, true),
        signed_case(
            "self mapping first",
            &[(P, "1.2.3.4:1.2.3.4,1.2.3.4:1.2.3.5"), (P, "")],
            P,
            true,
        ),
        signed_case(
            "self mapping last",
            &[(P, "1.2.3.4:1.2.3.5,1.2.3.4:1.2.3.4"), (P, "")],
            P,
            true,
        ),
        signed_case(
            "mapped other policy",
            &[(P, "1.2.3.4:1.2.3.4,1.2.3.4:1.2.3.5"), (Q, "")],
            P,
            true,
        ),
        signed_case(
            "parallel any and mapped branches",
            &[
                ("1.2.3.4,2.5.29.32.0", "1.2.3.4:1.2.3.5"),
                ("1.2.3.5,2.5.29.32.0", ""),
                (Q, ""),
            ],
            P,
            true,
        ),
        signed_case(
            "mapping synthesized beside any",
            &[(ANY, "1.2.3.4:1.2.3.5"), (Q, "")],
            P,
            true,
        ),
        signed_case(
            "mapping cannot authorize another initial policy",
            &[(Q, "1.2.3.5:1.2.3.4"), (P, "")],
            P,
            false,
        ),
    ];
    let mut absent = signed_case(
        "absent policies without explicit requirement",
        &[("", ""), ("", "")],
        P,
        true,
    );
    absent.policy["explicit_policy"] = json!(false);
    cases.push(absent);
    cases.push(signed_case(
        "absent policies with explicit requirement",
        &[("", ""), ("", "")],
        P,
        false,
    ));
    let mut inhibited = signed_case(
        "inhibited mapping",
        &[(P, "1.2.3.4:1.2.3.5"), (Q, "")],
        P,
        false,
    );
    inhibited.policy["inhibit_policy_mapping"] = json!(true);
    cases.push(inhibited);
    let mut inhibited = signed_case(
        "inhibited any certificate",
        &[(ANY, ""), (P, "")],
        ANY,
        false,
    );
    inhibited.policy["inhibit_any_policy"] = json!(true);
    cases.push(inhibited);
    cases.extend(wrap_up_cases());
    cases
}

#[test]
fn policy_review_counterexamples_have_correct_path_results() {
    for case in integration_cases() {
        let result = SignaturePolicy::from_json(case.policy.to_string().as_bytes())
            .unwrap()
            .validate_chain(&case.chain);
        assert_eq!(result.is_ok(), case.accepted, "{}: {result:?}", case.label);
    }
}

fn wrap_up_cases() -> Vec<PolicyCase> {
    let (root, _) = chain();
    let mut cases = Vec::new();
    for counter in [0, 1, 2] {
        for self_issued in [false, true] {
            for ca_policy in [false, true] {
                for leaf_policy in [false, true] {
                    let constraint = format!("critical,requireExplicitPolicy:{counter}");
                    let mut extensions = vec![
                        ("basicConstraints", "critical,CA:TRUE"),
                        ("policyConstraints", constraint.as_str()),
                    ];
                    if ca_policy {
                        extensions.push(("certificatePolicies", P));
                    }
                    let ca = certificate("Counter CA", Some(&root), &[19], &extensions);
                    let extensions = if leaf_policy {
                        vec![("certificatePolicies", P)]
                    } else {
                        vec![]
                    };
                    let leaf = certificate(
                        if self_issued {
                            "Counter CA"
                        } else {
                            "Distinct signer"
                        },
                        Some(&ca),
                        &[20],
                        &extensions,
                    );
                    let path = vec![leaf, ca];
                    cases.push(PolicyCase {
                        label: if self_issued {
                            "self-issued final policy counter"
                        } else {
                            "ordinary final policy counter"
                        },
                        policy: config(&root, &path),
                        chain: path.iter().map(encoded).collect(),
                        accepted: counter >= 2 || ca_policy && leaf_policy,
                    });
                }
            }
        }
    }
    cases
}

// No production OID comparison or tree routine computes this oracle. Each bit
// denotes one of two literal policies. anyPolicy permits both when not inhibited.
#[test]
fn policy_sets_follow_independent_intersection_model() {
    let sets = [
        "",
        P,
        Q,
        "1.2.3.4,1.2.3.5",
        ANY,
        "1.2.3.4,2.5.29.32.0",
        "1.2.3.5,2.5.29.32.0",
        "1.2.3.4,1.2.3.5,2.5.29.32.0",
    ];
    let ordinary = [0_u8, 1, 2, 3, 0, 1, 2, 3];
    let expanded = [0_u8, 1, 2, 3, 3, 3, 3, 3];
    let (root, _) = chain();
    let mut issuer = root.clone();
    let mut stages = Vec::new();
    for depth in 0..3 {
        let stage = sets
            .iter()
            .enumerate()
            .map(|(index, policies)| {
                let mut extensions = vec![];
                if depth < 2 {
                    extensions.push(("basicConstraints", "critical,CA:TRUE"));
                }
                if !policies.is_empty() {
                    extensions.push(("certificatePolicies", *policies));
                }
                certificate(
                    &format!("Set model {depth}"),
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
    for inhibit in [false, true] {
        let masks = if inhibit { ordinary } else { expanded };
        for (initial, mask) in [(P, 1), (Q, 2), ("1.2.3.4,1.2.3.5", 3), (ANY, 3)] {
            for a in 0..8 {
                for b in 0..8 {
                    for c in 0..8 {
                        let path = vec![
                            stages[2][c].clone(),
                            stages[1][b].clone(),
                            stages[0][a].clone(),
                        ];
                        let mut policy = config(&root, &path);
                        policy["initial_policy_set"] =
                            json!(initial.split(',').collect::<Vec<_>>());
                        policy["explicit_policy"] = json!(true);
                        policy["inhibit_any_policy"] = json!(inhibit);
                        let result = validate(&policy, &path);
                        assert_eq!(
                            result.is_ok(),
                            mask & masks[a] & masks[b] & masks[c] != 0,
                            "initial={initial} inhibit={inhibit} levels={a}/{b}/{c}: {result:?}"
                        );
                    }
                }
            }
        }
    }
}

// Relation bits: P->P, P->Q, Q->P, Q->Q. No outgoing edge leaves that policy
// unchanged; inhibition deletes the policies with outgoing mappings.
fn image(mask: u8, relation: u8, inhibit: bool) -> u8 {
    let mut output = 0;
    for index in 0..2 {
        if mask & (1 << index) == 0 {
            continue;
        }
        let targets = (relation >> (index * 2)) & 3;
        output |= if targets == 0 {
            1 << index
        } else if inhibit {
            0
        } else {
            targets
        };
    }
    output
}

fn mapping(relation: u8) -> String {
    [
        "1.2.3.4:1.2.3.4",
        "1.2.3.4:1.2.3.5",
        "1.2.3.5:1.2.3.4",
        "1.2.3.5:1.2.3.5",
    ]
    .into_iter()
    .enumerate()
    .filter(|(index, _)| relation & (1 << index) != 0)
    .map(|(_, s)| s)
    .collect::<Vec<_>>()
    .join(",")
}

#[test]
fn policy_mappings_follow_independent_relation_composition() {
    let sets = [P, Q, "1.2.3.4,1.2.3.5"];
    let (root, _) = chain();
    let mut stages: Vec<Vec<X509>> = Vec::new();
    let mut issuer = root.clone();
    for depth in 0..2 {
        let mut stage = Vec::new();
        for policies in sets {
            for relation in 0..16 {
                let mappings = mapping(relation);
                let mut extensions = vec![
                    ("basicConstraints", "critical,CA:TRUE"),
                    ("certificatePolicies", policies),
                ];
                if !mappings.is_empty() {
                    extensions.push(("policyMappings", &mappings));
                }
                stage.push(certificate(
                    &format!("Relation model {depth}"),
                    Some(&issuer),
                    &[
                        u8::try_from(depth + 1).unwrap(),
                        u8::try_from(stage.len()).unwrap(),
                    ],
                    &extensions,
                ));
            }
        }
        issuer = stage[0].clone();
        stages.push(stage);
    }
    let leaves = sets
        .iter()
        .enumerate()
        .map(|(index, policies)| {
            certificate(
                "Relation signer",
                Some(&issuer),
                &[3, u8::try_from(index).unwrap()],
                &[("certificatePolicies", policies)],
            )
        })
        .collect::<Vec<_>>();
    // First CA includes both policies, allowing branches to split and converge.
    for inhibit in [false, true] {
        for initial in 1..=3 {
            for first in 0..16 {
                for second in 0..16 {
                    for middle in 1..=3 {
                        for leaf in 1..=3 {
                            let path = vec![
                                leaves[usize::from(leaf - 1)].clone(),
                                stages[1][usize::from((middle - 1) * 16 + second)].clone(),
                                stages[0][usize::from(32 + first)].clone(),
                            ];
                            let expected =
                                image(image(initial, first, inhibit) & middle, second, inhibit)
                                    & leaf
                                    != 0;
                            let mut policy = config(&root, &path);
                            policy["initial_policy_set"] = json!(
                                sets[usize::from(initial - 1)]
                                    .split(',')
                                    .collect::<Vec<_>>()
                            );
                            policy["explicit_policy"] = json!(true);
                            policy["inhibit_policy_mapping"] = json!(inhibit);
                            let result = validate(&policy, &path);
                            assert_eq!(
                                result.is_ok(),
                                expected,
                                "initial={initial} maps={first}/{second} middle={middle} leaf={leaf} inhibit={inhibit}: {result:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn policy_tree_retains_distinct_parents_and_leaf_qualifiers() {
    use ::der::{
        Decode, Encode,
        asn1::{Any, Ia5String, ObjectIdentifier, OctetString},
    };
    use x509_cert::ext::pkix::certpolicy::{CertificatePolicies, PolicyQualifierInfo};
    let (root, _) = chain();
    let ca = certificate(
        "Branch CA",
        Some(&root),
        &[21],
        &[
            ("basicConstraints", "CA:TRUE"),
            ("certificatePolicies", "1.2.3.4,1.2.3.5"),
            ("policyMappings", "1.2.3.4:1.2.3.5"),
        ],
    );
    let leaf = certificate(
        "Branch signer",
        Some(&ca),
        &[22],
        &[("certificatePolicies", Q)],
    );
    let mut leaf = x509_cert::Certificate::from_der(&leaf).unwrap();
    let extension = leaf
        .tbs_certificate
        .extensions
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|e| e.extn_id.to_string() == "2.5.29.32")
        .unwrap();
    let mut policies = CertificatePolicies::from_der(extension.extn_value.as_bytes()).unwrap();
    let qualifier = PolicyQualifierInfo {
        policy_qualifier_id: ObjectIdentifier::new("1.3.6.1.5.5.7.2.1").unwrap(),
        qualifier: Some(
            Any::from_der(
                &Ia5String::new("https://example.org/policy")
                    .unwrap()
                    .to_der()
                    .unwrap(),
            )
            .unwrap(),
        ),
    };
    policies.0[0].policy_qualifiers = Some(vec![qualifier.clone()]);
    extension.extn_value = OctetString::new(policies.to_der().unwrap()).unwrap();
    let leaf = super::fixtures::resign(leaf, jsonwebtoken::Algorithm::RS256);
    let path = vec![leaf, ca];
    for (initial, expected) in [("1.2.3.4,1.2.3.5", 2), (P, 1), (Q, 1), (ANY, 2)] {
        let mut policy = config(&root, &path);
        policy["initial_policy_set"] = json!(initial.split(',').collect::<Vec<_>>());
        policy["explicit_policy"] = json!(true);
        let policy = SignaturePolicy::from_json(policy.to_string().as_bytes()).unwrap();
        let result = provider::validate(
            &path,
            std::slice::from_ref(&root),
            policy.config.as_ref().unwrap(),
            timestamp("2026-10-27T00:00:00Z").unwrap(),
        )
        .unwrap();
        let leaves = result
            .valid_policy_tree
            .unwrap()
            .into_iter()
            .filter(|n| n.depth == 2)
            .collect::<Vec<_>>();
        assert_eq!(leaves.len(), expected);
        assert!(leaves.iter().all(|n|n.valid_policy.to_string()==Q && n.qualifiers==vec![qualifier.clone()]));
    }
}

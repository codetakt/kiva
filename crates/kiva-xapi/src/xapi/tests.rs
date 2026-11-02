use std::collections::BTreeMap;

use kiva_domain::{Activity, Actor, Iri, Statement, StatementId};
use time::OffsetDateTime;
use uuid::Uuid;

use serde_json::{Value, json};

use super::{
    AUTHORITY_ACCOUNT_HOME_PAGE, StatementFilterError, XapiError, accepted_language_ranges,
    admit_statement_batch, authority_agent, canonicalize_statement_language_maps,
    filter_statements,
};

fn statement(id: StatementId) -> Statement {
    let actor = match Actor::new("learner:1") {
        Ok(actor) => actor,
        Err(error) => panic!("test actor must be valid: {error}"),
    };
    let verb = match Iri::parse("https://adlnet.gov/expapi/verbs/experienced") {
        Ok(verb) => verb,
        Err(error) => panic!("test verb must be valid: {error}"),
    };
    let activity = match Iri::parse("https://example.edu/activity/1") {
        Ok(activity) => Activity::new(activity),
        Err(error) => panic!("test activity must be valid: {error}"),
    };
    Statement::new(id, actor, verb, activity, OffsetDateTime::UNIX_EPOCH)
}

#[test]
fn rejects_duplicate_statement_ids() {
    let id = StatementId::from_uuid(Uuid::nil());
    let result = admit_statement_batch([statement(id), statement(id)]);

    assert_eq!(result, Err(XapiError::DuplicateStatementId(id)));
}

#[test]
fn authority_agent_is_deterministic() {
    assert_eq!(
        authority_agent("writer").pointer("/account/homePage"),
        Some(&json!(AUTHORITY_ACCOUNT_HOME_PAGE))
    );
}

#[test]
fn language_map_canonicalization_uses_accept_language_weight() {
    let languages = accepted_language_ranges(Some("fr-CA;q=0.9, en-US"));
    let statement = canonicalize_statement_language_maps(
        json!({
            "verb": {
                "display": {
                    "en-US": "experienced",
                    "fr-CA": "a vecu"
                }
            }
        }),
        &languages,
    );

    assert_eq!(
        statement.pointer("/verb/display/en-US"),
        Some(&json!("experienced"))
    );
    assert_eq!(statement.pointer("/verb/display/fr-CA"), None);
}

#[test]
fn statement_metadata_extracts_promoted_fields() {
    let metadata = super::StatementMetadata::from_statement(
        &json!({
            "actor": {
                "objectType": "Agent",
                "account": {
                    "homePage": "https://example.edu",
                    "name": "learner-1"
                }
            },
            "verb": {
                "id": "https://adlnet.gov/expapi/verbs/experienced"
            },
            "object": {
                "id": "https://example.edu/activity/1"
            },
            "authority": {
                "objectType": "Agent",
                "mbox": "mailto:lrs@example.edu"
            }
        }),
        "2.0.0",
    );

    assert_eq!(
        metadata.actor_ifi.as_deref(),
        Some("account:https://example.edu:learner-1")
    );
    assert_eq!(
        metadata.verb_id.as_deref(),
        Some("https://adlnet.gov/expapi/verbs/experienced")
    );
    assert_eq!(
        metadata.object_id.as_deref(),
        Some("https://example.edu/activity/1")
    );
    assert_eq!(metadata.version, "2.0.0");
}

#[test]
fn statement_filtering_excludes_voided_targets_and_orders_by_stored_time() {
    let target_id = "00000000-0000-4000-8000-000000000001";
    let retained_id = "00000000-0000-4000-8000-000000000002";
    let voiding_id = "00000000-0000-4000-8000-000000000003";
    let statements = vec![
        json!({
            "id": target_id,
            "stored": "2025-01-01T00:00:00Z",
            "verb": {"id": "https://adlnet.gov/expapi/verbs/experienced"},
            "object": {"id": "https://example.edu/activity/1"}
        }),
        json!({
            "id": retained_id,
            "stored": "2025-01-02T00:00:00Z",
            "verb": {"id": "https://adlnet.gov/expapi/verbs/experienced"},
            "object": {"id": "https://example.edu/activity/2"}
        }),
        json!({
            "id": voiding_id,
            "stored": "2025-01-03T00:00:00Z",
            "verb": {"id": "http://adlnet.gov/expapi/verbs/voided"},
            "object": {
                "objectType": "StatementRef",
                "id": target_id
            }
        }),
    ];

    let filtered = match filter_statements(statements, &BTreeMap::new(), None) {
        Ok(filtered) => filtered,
        Err(error) => panic!("filter should accept empty query: {error}"),
    };

    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].get("id"), Some(&json!(voiding_id)));
    assert_eq!(filtered[1].get("id"), Some(&json!(retained_id)));
}

#[test]
fn statement_filtering_reports_agent_json_errors() {
    let mut query = BTreeMap::new();
    query.insert("agent".to_owned(), "{not-json".to_owned());

    let result = filter_statements(Vec::new(), &query, None);

    assert_eq!(result, Err(StatementFilterError::InvalidAgentJson));
}

#[test]
fn iri_validation_checks_the_complete_identifier() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../tests/conformance/fixtures/identifier-grammar.json"
    ))
    .expect("pinned identifier corpus");
    for case in corpus["cases"].as_array().expect("identifier cases") {
        assert_eq!(
            super::is_absolute_iri(case["value"].as_str().expect("IRI")),
            case["iri"].as_bool().expect("independent IRI expectation"),
            "{}",
            case["id"]
        );
    }
    for valid in [
        "urn:example:lesson",
        "https://例え.テスト/学習#項目",
        "https://example.org/a%20b",
    ] {
        assert!(super::is_absolute_iri(valid), "{valid}");
    }
    for invalid in [
        "relative",
        "https://example.org/a b",
        "https://example.org/%ZZ",
        "https://example.org/\n",
    ] {
        assert!(!super::is_absolute_iri(invalid), "{invalid:?}");
    }
}

#[test]
fn related_agent_filter_reads_relation_actors_in_statements_and_substatements() {
    for (collection, property, actor) in [
        (
            "contextAgents",
            "agent",
            json!({"mbox": "mailto:related@example.org"}),
        ),
        (
            "contextGroups",
            "group",
            json!({"objectType": "Group", "mbox": "mailto:related@example.org"}),
        ),
    ] {
        for nested in [false, true] {
            let mut value = json!({
                "actor": {"mbox": "mailto:other@example.org"},
                "verb": {"id": "https://example.org/v"},
                "object": {"id": "https://example.org/a"},
                "context": {(collection): [{(property): actor}]}
            });
            if nested {
                value["objectType"] = json!("SubStatement");
                value = json!({"actor": {"mbox": "mailto:other@example.org"}, "object": value});
            }
            let query = BTreeMap::from([
                ("agent".to_owned(), actor.to_string()),
                ("related_agents".to_owned(), "true".to_owned()),
            ]);
            value["stored"] = json!("2025-01-01T00:00:00Z");
            let result = filter_statements(vec![value], &query, None);
            assert!(matches!(result, Ok(ref values) if values.len() == 1));
        }
    }
}

#[test]
fn statement_equivalence_ignores_lrs_managed_fields() {
    assert!(super::statements_equivalent(
        &json!({
            "id": "00000000-0000-4000-8000-000000000001",
            "stored": "2025-01-01T00:00:00Z",
            "timestamp": "2025-01-01T00:00:00Z",
            "authority": {"objectType": "Agent", "mbox": "mailto:a@example.org"},
            "result": {"duration": "PT1.231S"}
        }),
        &json!({
            "id": "00000000-0000-4000-8000-000000000001",
            "stored": "2026-01-01T00:00:00Z",
            "timestamp": "2026-01-01T00:00:00Z",
            "authority": {"objectType": "Agent", "mbox": "mailto:b@example.org"},
            "result": {"duration": "PT1.23S"}
        }),
    ));
}

#[test]
fn statement_equivalence_normalizes_full_duration_precision() {
    assert!(super::statements_equivalent(
        &json!({"result": {"duration": "P1DT12H36M0.1237S"}}),
        &json!({"result": {"duration": "P1DT12H36M0.12S"}}),
    ));
    assert!(!super::statements_equivalent(
        &json!({"result": {"duration": "P1DT12H36M0.1237S"}}),
        &json!({"result": {"duration": "P1DT12H36M0.13S"}}),
    ));
}

#[test]
fn duration_decimal_separators_obey_lowest_component_grammar() {
    for duration in ["PT1,234567S", "PT1.234567S", "P1,5W", "P1Y2,5M", "P2DT0,5H"] {
        assert!(super::validation::is_duration(duration), "{duration}");
    }
    for duration in [
        "PT1,S", "PT1.S", "PT,5S", "PT1,2.3S", "PT1,5H2M", "P1,5DT1H", "PT1S2M", "P1W1D",
    ] {
        assert!(!super::validation::is_duration(duration), "{duration}");
    }
}

#[test]
fn comma_duration_comparison_ignores_only_sub_centiseconds() {
    for nested in [false, true] {
        let statement = |duration: &str| {
            if nested {
                json!({"object": {"objectType": "SubStatement", "result": {"duration": duration}}})
            } else {
                json!({"result": {"duration": duration}})
            }
        };
        assert!(super::statements_equivalent(
            &statement("PT1,239S"),
            &statement("PT1.23S")
        ));
        assert!(!super::statements_equivalent(
            &statement("PT1,239S"),
            &statement("PT1.24S")
        ));
    }
}

#[test]
fn duration_equality_is_exact_before_centisecond_truncation() {
    for nested in [false, true] {
        let statement = |duration: &str| {
            let mut statement = json!({"actor":{"mbox":"mailto:a@example.test"},
                "verb":{"id":"urn:verb"},"object":{"id":"urn:activity"},
                "result":{"duration":duration}});
            if nested {
                statement["objectType"] = json!("SubStatement");
                json!({"actor":{"mbox":"mailto:a@example.test"},"verb":{"id":"urn:verb"},"object":statement})
            } else {
                statement
            }
        };
        for (left, right) in [
            ("PT1S", "PT1.00S"),
            ("PT01.0100S", "PT1,019999S"),
            ("PT60S", "PT1M"),
            ("PT1.5H", "PT90M"),
            ("PT1H0M0S", "PT3600S"),
            ("PT0.00001H", "PT0.039S"),
            ("PT0.000001M", "PT0S"),
            ("PT9007199254740993.011S", "PT9007199254740993.019S"),
            ("P01Y02.500M", "P1Y2,5M"),
            ("P00Y0M1DT1S", "P1DT1.0099S"),
        ] {
            assert!(
                super::statements_equivalent(&statement(left), &statement(right)),
                "{left}/{right}/{nested}"
            );
        }
        for (left, right) in [
            ("PT1.009S", "PT1.01S"),
            ("PT0.00001H", "PT0.04S"),
            ("PT9007199254740993S", "PT9007199254740992S"),
            ("P1M", "P30D"),
            ("P1Y", "P365D"),
            ("PT1.S", "PT1.S"),
            ("PT1H2", "PT1H2"),
            ("PT1H2", "PT1H"),
        ] {
            assert!(
                !super::statements_equivalent(&statement(left), &statement(right)),
                "{left}/{right}/{nested}"
            );
        }
    }
}

use std::collections::BTreeMap;

use kiva_domain::{Activity, Actor, Iri, Statement, StatementId};
use time::OffsetDateTime;
use uuid::Uuid;

use serde_json::json;

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
fn language_map_canonicalization_uses_accept_language_order() {
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
        statement.pointer("/verb/display/fr-CA"),
        Some(&json!("a vecu"))
    );
    assert_eq!(statement.pointer("/verb/display/en-US"), None);
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

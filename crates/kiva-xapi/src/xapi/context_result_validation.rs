use serde_json::{Map, Value};
use thiserror::Error;
use uuid::Uuid;

use super::{
    actor_validation::{ActorRole, validate_actor_value},
    object_validation::validate_activity,
    validation::{is_absolute_iri, is_duration, is_language_tag},
};

const RESULT_PROPERTIES: [&str; 6] = [
    "score",
    "success",
    "completion",
    "response",
    "duration",
    "extensions",
];
const SCORE_PROPERTIES: [&str; 4] = ["scaled", "raw", "min", "max"];
const CONTEXT_PROPERTIES: [&str; 11] = [
    "registration",
    "instructor",
    "team",
    "contextActivities",
    "contextAgents",
    "contextGroups",
    "revision",
    "platform",
    "language",
    "statement",
    "extensions",
];
const CONTEXT_ACTIVITY_KEYS: [&str; 4] = ["parent", "grouping", "category", "other"];

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum ContextResultValidationError {
    #[error("{path} must be a JSON object")]
    ExpectedObject { path: String },
    #[error("{path} contains an unsupported property")]
    UnsupportedProperty { path: String },
    #[error("{path} must be a string")]
    ExpectedString { path: String },
    #[error("{path} must be a boolean")]
    ExpectedBoolean { path: String },
    #[error("{path} must be a number")]
    ExpectedNumber { path: String },
    #[error("{path} must be an array")]
    ExpectedArray { path: String },
    #[error("{path} must be a UUID string")]
    InvalidUuid { path: String },
    #[error("{path} must be an RFC 5646 language tag")]
    InvalidLanguageTag { path: String },
    #[error("{path} must be an ISO 8601 duration")]
    InvalidDuration { path: String },
    #[error("{path} score scaled must be between -1 and 1")]
    InvalidScaledScore { path: String },
    #[error("{path} score raw must be within min and max when present")]
    InvalidRawScore { path: String },
    #[error("{path} score min must be less than max")]
    InvalidScoreBounds { path: String },
    #[error("{path} contextActivities key is not permitted")]
    InvalidContextActivityKey { path: String },
    #[error("{path} contextActivities value must be an Activity or Activity array")]
    InvalidContextActivity { path: String },
    #[error("{path} objectType is invalid")]
    InvalidObjectType { path: String },
    #[error("{path} relevantTypes must contain one or more IRI strings")]
    InvalidRelevantTypes { path: String },
    #[error("{path} actor relation is invalid")]
    InvalidActorRelation { path: String },
    #[error("{path} extension keys must be absolute IRIs")]
    InvalidExtensionKey { path: String },
}

pub(crate) fn validate_statement_context_result(
    statement: &Value,
) -> Result<(), ContextResultValidationError> {
    let Some(statement) = statement.as_object() else {
        return Ok(());
    };
    validate_statement_like(statement, "Statement")
}

fn validate_statement_like(
    statement: &Map<String, Value>,
    path: &str,
) -> Result<(), ContextResultValidationError> {
    validate_result(statement.get("result"), &format!("{path} result"))?;
    validate_context(statement.get("context"), &format!("{path} context"))?;
    if let Some(object) = statement.get("object").and_then(Value::as_object)
        && object.get("objectType").and_then(Value::as_str) == Some("SubStatement")
    {
        validate_statement_like(object, &format!("{path} object"))?;
    }
    Ok(())
}

fn validate_result(result: Option<&Value>, path: &str) -> Result<(), ContextResultValidationError> {
    let Some(result) = result else {
        return Ok(());
    };
    let result = expect_object(result, path)?;
    validate_allowed_properties(result, &RESULT_PROPERTIES, path)?;
    validate_score(result.get("score"), &format!("{path} score"))?;
    validate_bool(result.get("success"), &format!("{path} success"))?;
    validate_bool(result.get("completion"), &format!("{path} completion"))?;
    validate_string(result.get("response"), &format!("{path} response"))?;
    validate_duration(result.get("duration"), &format!("{path} duration"))?;
    validate_extensions(result.get("extensions"), &format!("{path} extensions"))
}

fn validate_score(score: Option<&Value>, path: &str) -> Result<(), ContextResultValidationError> {
    let Some(score) = score else {
        return Ok(());
    };
    let score = expect_object(score, path)?;
    validate_allowed_properties(score, &SCORE_PROPERTIES, path)?;
    let scaled = optional_number(score.get("scaled"), &format!("{path} scaled"))?;
    if let Some(scaled) = scaled
        && !(-1.0..=1.0).contains(&scaled)
    {
        return Err(ContextResultValidationError::InvalidScaledScore {
            path: path.to_owned(),
        });
    }
    let raw = optional_number(score.get("raw"), &format!("{path} raw"))?;
    let min = optional_number(score.get("min"), &format!("{path} min"))?;
    let max = optional_number(score.get("max"), &format!("{path} max"))?;
    if let (Some(min), Some(max)) = (min, max)
        && min >= max
    {
        return Err(ContextResultValidationError::InvalidScoreBounds {
            path: path.to_owned(),
        });
    }
    if let (Some(raw), Some(min)) = (raw, min)
        && raw < min
    {
        return Err(ContextResultValidationError::InvalidRawScore {
            path: path.to_owned(),
        });
    }
    if let (Some(raw), Some(max)) = (raw, max)
        && raw > max
    {
        return Err(ContextResultValidationError::InvalidRawScore {
            path: path.to_owned(),
        });
    }
    Ok(())
}

fn validate_context(
    context: Option<&Value>,
    path: &str,
) -> Result<(), ContextResultValidationError> {
    let Some(context) = context else {
        return Ok(());
    };
    let context = expect_object(context, path)?;
    validate_allowed_properties(context, &CONTEXT_PROPERTIES, path)?;
    validate_uuid(context.get("registration"), &format!("{path} registration"))?;
    validate_context_activities(
        context.get("contextActivities"),
        &format!("{path} contextActivities"),
    )?;
    validate_context_relations(
        context.get("contextAgents"),
        &format!("{path} contextAgents"),
        "contextAgent",
        "agent",
        ActorRole::Agent,
    )?;
    validate_context_relations(
        context.get("contextGroups"),
        &format!("{path} contextGroups"),
        "contextGroup",
        "group",
        ActorRole::Group,
    )?;
    validate_string(context.get("revision"), &format!("{path} revision"))?;
    validate_string(context.get("platform"), &format!("{path} platform"))?;
    validate_language(context.get("language"), &format!("{path} language"))?;
    validate_statement_ref(context.get("statement"), &format!("{path} statement"))?;
    validate_extensions(context.get("extensions"), &format!("{path} extensions"))
}

fn validate_context_activities(
    value: Option<&Value>,
    path: &str,
) -> Result<(), ContextResultValidationError> {
    let Some(value) = value else {
        return Ok(());
    };
    let activities = expect_object(value, path)?;
    if let Some(key) = activities
        .keys()
        .find(|key| !CONTEXT_ACTIVITY_KEYS.contains(&key.as_str()))
    {
        return Err(ContextResultValidationError::InvalidContextActivityKey {
            path: format!("{path} {key}"),
        });
    }
    for (key, value) in activities {
        let activity_path = format!("{path} {key}");
        if let Some(array) = value.as_array() {
            for (index, activity) in array.iter().enumerate() {
                validate_context_activity(activity, &format!("{path} {key}[{index}]"))?;
            }
        } else {
            validate_context_activity(value, &activity_path)?;
        }
    }
    Ok(())
}

fn validate_context_activity(
    value: &Value,
    path: &str,
) -> Result<(), ContextResultValidationError> {
    let activity =
        value
            .as_object()
            .ok_or_else(|| ContextResultValidationError::InvalidContextActivity {
                path: path.to_owned(),
            })?;
    validate_activity(activity, path).map_err(|_error| {
        ContextResultValidationError::InvalidContextActivity {
            path: path.to_owned(),
        }
    })
}

fn validate_context_relations(
    value: Option<&Value>,
    path: &str,
    object_type: &str,
    actor_property: &str,
    actor_role: ActorRole,
) -> Result<(), ContextResultValidationError> {
    let Some(value) = value else {
        return Ok(());
    };
    let relations =
        value
            .as_array()
            .ok_or_else(|| ContextResultValidationError::ExpectedArray {
                path: path.to_owned(),
            })?;
    for (index, relation) in relations.iter().enumerate() {
        let relation_path = format!("{path}[{index}]");
        let relation = expect_object(relation, &relation_path)?;
        validate_allowed_properties(
            relation,
            &["objectType", actor_property, "relevantTypes"],
            &relation_path,
        )?;
        if relation.get("objectType").and_then(Value::as_str) != Some(object_type) {
            return Err(ContextResultValidationError::InvalidObjectType {
                path: format!("{relation_path} objectType"),
            });
        }
        let Some(actor) = relation.get(actor_property) else {
            return Err(ContextResultValidationError::InvalidActorRelation {
                path: format!("{relation_path} {actor_property}"),
            });
        };
        validate_actor_value(
            actor,
            &format!("{relation_path} {actor_property}"),
            actor_role,
        )
        .map_err(
            |_error| ContextResultValidationError::InvalidActorRelation {
                path: format!("{relation_path} {actor_property}"),
            },
        )?;
        validate_relevant_types(
            relation.get("relevantTypes"),
            &format!("{relation_path} relevantTypes"),
        )?;
    }
    Ok(())
}

fn validate_relevant_types(
    value: Option<&Value>,
    path: &str,
) -> Result<(), ContextResultValidationError> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some(values) = value.as_array() else {
        return Err(ContextResultValidationError::ExpectedArray {
            path: path.to_owned(),
        });
    };
    if values.is_empty()
        || values
            .iter()
            .any(|value| value.as_str().is_none_or(|value| !is_absolute_iri(value)))
    {
        return Err(ContextResultValidationError::InvalidRelevantTypes {
            path: path.to_owned(),
        });
    }
    Ok(())
}

fn validate_statement_ref(
    value: Option<&Value>,
    path: &str,
) -> Result<(), ContextResultValidationError> {
    let Some(value) = value else {
        return Ok(());
    };
    let statement_ref = expect_object(value, path)?;
    validate_allowed_properties(statement_ref, &["objectType", "id"], path)?;
    if statement_ref.get("objectType").and_then(Value::as_str) != Some("StatementRef") {
        return Err(ContextResultValidationError::InvalidObjectType {
            path: format!("{path} objectType"),
        });
    }
    if !statement_ref.contains_key("id") {
        return Err(ContextResultValidationError::InvalidUuid {
            path: format!("{path} id"),
        });
    }
    validate_uuid(statement_ref.get("id"), &format!("{path} id"))
}

fn validate_extensions(
    value: Option<&Value>,
    path: &str,
) -> Result<(), ContextResultValidationError> {
    let Some(value) = value else {
        return Ok(());
    };
    let extensions = expect_object(value, path)?;
    if extensions.keys().any(|key| !is_absolute_iri(key)) {
        return Err(ContextResultValidationError::InvalidExtensionKey {
            path: path.to_owned(),
        });
    }
    Ok(())
}

fn validate_uuid(value: Option<&Value>, path: &str) -> Result<(), ContextResultValidationError> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some(value) = value.as_str() else {
        return Err(ContextResultValidationError::InvalidUuid {
            path: path.to_owned(),
        });
    };
    if value.len() != 36 {
        return Err(ContextResultValidationError::InvalidUuid {
            path: path.to_owned(),
        });
    }
    Uuid::parse_str(value).map(|_uuid| ()).map_err(|_error| {
        ContextResultValidationError::InvalidUuid {
            path: path.to_owned(),
        }
    })
}

fn validate_bool(value: Option<&Value>, path: &str) -> Result<(), ContextResultValidationError> {
    if value.is_some_and(|value| !value.is_boolean()) {
        return Err(ContextResultValidationError::ExpectedBoolean {
            path: path.to_owned(),
        });
    }
    Ok(())
}

fn validate_string(value: Option<&Value>, path: &str) -> Result<(), ContextResultValidationError> {
    if value.is_some_and(|value| !value.is_string()) {
        return Err(ContextResultValidationError::ExpectedString {
            path: path.to_owned(),
        });
    }
    Ok(())
}

fn validate_language(
    value: Option<&Value>,
    path: &str,
) -> Result<(), ContextResultValidationError> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some(value) = value.as_str() else {
        return Err(ContextResultValidationError::ExpectedString {
            path: path.to_owned(),
        });
    };
    if !is_language_tag(value) {
        return Err(ContextResultValidationError::InvalidLanguageTag {
            path: path.to_owned(),
        });
    }
    Ok(())
}

fn validate_duration(
    value: Option<&Value>,
    path: &str,
) -> Result<(), ContextResultValidationError> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some(value) = value.as_str() else {
        return Err(ContextResultValidationError::ExpectedString {
            path: path.to_owned(),
        });
    };
    if !is_duration(value) {
        return Err(ContextResultValidationError::InvalidDuration {
            path: path.to_owned(),
        });
    }
    Ok(())
}

fn optional_number(
    value: Option<&Value>,
    path: &str,
) -> Result<Option<f64>, ContextResultValidationError> {
    value.map_or(Ok(None), |value| {
        value
            .as_f64()
            .map(Some)
            .ok_or_else(|| ContextResultValidationError::ExpectedNumber {
                path: path.to_owned(),
            })
    })
}

fn expect_object<'a>(
    value: &'a Value,
    path: &str,
) -> Result<&'a Map<String, Value>, ContextResultValidationError> {
    value
        .as_object()
        .ok_or_else(|| ContextResultValidationError::ExpectedObject {
            path: path.to_owned(),
        })
}

fn validate_allowed_properties(
    object: &Map<String, Value>,
    allowed: &[&str],
    path: &str,
) -> Result<(), ContextResultValidationError> {
    if object.keys().any(|name| !allowed.contains(&name.as_str())) {
        return Err(ContextResultValidationError::UnsupportedProperty {
            path: path.to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{CONTEXT_ACTIVITY_KEYS, validate_statement_context_result};

    fn statement_with(property: &str, value: Value) -> Value {
        let mut statement = json!({
            "actor": {"objectType": "Agent", "mbox": "mailto:learner@example.org"},
            "verb": {"id": "https://example.org/verbs/experienced"},
            "object": {"objectType": "Activity", "id": "https://example.org/activity"}
        });
        statement[property] = value;
        statement
    }

    #[test]
    fn validates_context_activity_definitions_and_allows_empty_arrays() {
        for key in CONTEXT_ACTIVITY_KEYS {
            let empty = statement_with("context", json!({"contextActivities": {key: []}}));
            assert!(validate_statement_context_result(&empty).is_ok());
            for activity in [
                json!({"id": "https://example.org/a", "extra": true}),
                json!({"id": "https://example.org/a", "objectType": "Agent"}),
                json!({"id": "https://example.org/a", "definition": {"name": 1}}),
                json!({"id": "https://example.org/a", "definition": {"moreInfo": "relative"}}),
            ] {
                for value in [activity.clone(), json!([activity])] {
                    let statement =
                        statement_with("context", json!({"contextActivities": {key: value}}));
                    assert!(validate_statement_context_result(&statement).is_err());
                }
            }
        }
        let missing_id = statement_with(
            "context",
            json!({"statement": {"objectType": "StatementRef"}}),
        );
        assert!(validate_statement_context_result(&missing_id).is_err());
    }

    #[test]
    fn accepts_valid_context_and_result_properties() {
        let statement = json!({
            "actor": {"objectType": "Agent", "mbox": "mailto:learner@example.org"},
            "verb": {"id": "https://example.org/verbs/experienced"},
            "object": {"objectType": "Activity", "id": "https://example.org/activity"},
            "result": {
                "success": true,
                "completion": false,
                "response": "ok",
                "duration": "PT1H0M0.1S",
                "score": {"scaled": 0.75, "raw": 75, "min": 0, "max": 100},
                "extensions": {"https://example.org/ext/result": {"kept": true}}
            },
            "context": {
                "registration": "10000000-0000-4000-8000-000000000001",
                "revision": "1",
                "platform": "web",
                "language": "en-US",
                "statement": {
                    "objectType": "StatementRef",
                    "id": "10000000-0000-4000-8000-000000000002"
                },
                "contextActivities": {
                    "parent": {"objectType": "Activity", "id": "https://example.org/parent"},
                    "other": [{"objectType": "Activity", "id": "https://example.org/other"}]
                },
                "contextAgents": [{
                    "objectType": "contextAgent",
                    "agent": {"objectType": "Agent", "mbox": "mailto:coach@example.org"},
                    "relevantTypes": ["https://example.org/relevant/coach"]
                }],
                "contextGroups": [{
                    "objectType": "contextGroup",
                    "group": {"objectType": "Group", "member": [{"mbox": "mailto:member@example.org"}]}
                }],
                "extensions": {"https://example.org/ext/context": null}
            }
        });
        assert_eq!(validate_statement_context_result(&statement), Ok(()));
    }

    #[test]
    fn rejects_invalid_result_property_shapes() {
        for result in [
            json!("not-object"),
            json!({"success": "true"}),
            json!({"completion": "false"}),
            json!({"response": 1}),
            json!({"duration": "P4W1D"}),
            json!({"score": "not-object"}),
            json!({"score": {"scaled": 1.01}}),
            json!({"score": {"raw": 3, "min": 4}}),
            json!({"score": {"min": 2, "max": 1}}),
            json!({"extensions": {"not-an-iri": true}}),
        ] {
            assert!(validate_statement_context_result(&statement_with("result", result)).is_err());
        }
    }

    #[test]
    fn rejects_invalid_context_property_shapes() {
        for context in [
            json!("not-object"),
            json!({"registration": "not-a-uuid"}),
            json!({"contextActivities": "not-object"}),
            json!({"contextActivities": {"invalid": {"id": "https://example.org/a"}}}),
            json!({"contextActivities": {"parent": [{"key": "not-activity"}]}}),
            json!({"revision": 1}),
            json!({"platform": {}}),
            json!({"language": "a12345"}),
            json!({"statement": {"objectType": "statementref"}}),
            json!({"statement": {"objectType": "StatementRef", "id": "not-a-uuid"}}),
            json!({"extensions": {"not-an-iri": true}}),
        ] {
            assert!(
                validate_statement_context_result(&statement_with("context", context)).is_err()
            );
        }
    }

    #[test]
    fn rejects_invalid_context_agent_and_group_relations() {
        for context in [
            json!({"contextAgents": "not-array"}),
            json!({"contextAgents": [{"objectType": "bad", "agent": {"mbox": "mailto:a@example.org"}}]}),
            json!({"contextAgents": [{"objectType": "contextAgent", "agent": {"key": "bad"}}]}),
            json!({"contextAgents": [{"objectType": "contextAgent", "agent": {"mbox": "mailto:a@example.org"}, "relevantTypes": []}]}),
            json!({"contextAgents": [{"objectType": "contextAgent", "agent": {"mbox": "mailto:a@example.org"}, "relevantTypes": ["relative"]}]}),
            json!({"contextGroups": "not-array"}),
            json!({"contextGroups": [{"objectType": "bad", "group": {"objectType": "Group", "member": [{"mbox": "mailto:a@example.org"}]}}]}),
            json!({"contextGroups": [{"objectType": "contextGroup", "group": {"mbox": "mailto:a@example.org"}}]}),
            json!({"contextGroups": [{"objectType": "contextGroup", "group": {"objectType": "Group", "member": [{"mbox": "mailto:a@example.org"}]}, "relevantTypes": ["relative"]}]}),
        ] {
            assert!(
                validate_statement_context_result(&statement_with("context", context)).is_err()
            );
        }
    }

    #[test]
    fn validates_substatement_context_and_result_recursively() {
        let statement = json!({
            "actor": {"objectType": "Agent", "mbox": "mailto:learner@example.org"},
            "verb": {"id": "https://example.org/verbs/experienced"},
            "object": {
                "objectType": "SubStatement",
                "actor": {"objectType": "Agent", "mbox": "mailto:nested@example.org"},
                "verb": {"id": "https://example.org/verbs/experienced"},
                "object": {"objectType": "Activity", "id": "https://example.org/activity"},
                "result": {"duration": "not-a-duration"}
            }
        });
        assert!(validate_statement_context_result(&statement).is_err());
    }
}

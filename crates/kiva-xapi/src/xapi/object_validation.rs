use serde_json::{Map, Value};
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use super::validation::{is_absolute_iri, is_language_tag};

const INTERACTION_PROPERTIES: [&str; 7] = [
    "interactionType",
    "correctResponsesPattern",
    "choices",
    "scale",
    "source",
    "target",
    "steps",
];
const SUBSTATEMENT_PROPERTIES: [&str; 9] = [
    "objectType",
    "actor",
    "verb",
    "object",
    "result",
    "context",
    "timestamp",
    "attachments",
    "extensions",
];
const ATTACHMENT_PROPERTIES: [&str; 7] = [
    "usageType",
    "display",
    "description",
    "contentType",
    "length",
    "sha2",
    "fileUrl",
];

/// A Statement Object, Verb, `SubStatement`, timestamp, or Attachment violation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct ObjectValidationError {
    message: String,
}

impl ObjectValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Validates Statement Object-family values without changing the supplied JSON.
pub fn validate_statement_object_model(statement: &Value) -> Result<(), ObjectValidationError> {
    reject_nulls(statement, "Statement", false)?;
    let statement = expect_object(statement, "Statement")?;
    validate_verb(statement.get("verb"), "Statement verb")?;
    validate_object(statement.get("object"), "Statement object", false)?;
    validate_timestamp(statement.get("timestamp"), "Statement timestamp")?;
    validate_version(statement.get("version"), "Statement version")?;
    validate_attachments(statement.get("attachments"), "Statement attachments")
}

fn reject_nulls(
    value: &Value,
    path: &str,
    in_extensions: bool,
) -> Result<(), ObjectValidationError> {
    match value {
        Value::Null if !in_extensions => Err(ObjectValidationError::new(format!(
            "{path} must not be null"
        ))),
        Value::Object(object) => {
            for (name, value) in object {
                reject_nulls(
                    value,
                    &format!("{path} {name}"),
                    in_extensions || name == "extensions",
                )?;
            }
            Ok(())
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                reject_nulls(value, &format!("{path}[{index}]"), in_extensions)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_verb(value: Option<&Value>, path: &str) -> Result<(), ObjectValidationError> {
    let verb = required_object(value, path)?;
    allowed_properties(verb, &["id", "display"], path)?;
    required_absolute_iri(verb.get("id"), &format!("{path} id"))?;
    validate_language_map(verb.get("display"), &format!("{path} display"), false)
}

fn validate_object(
    value: Option<&Value>,
    path: &str,
    inside_substatement: bool,
) -> Result<(), ObjectValidationError> {
    let object = required_object(value, path)?;
    let object_type = match object.get("objectType") {
        None => "Activity",
        Some(Value::String(value)) => value.as_str(),
        Some(_) => {
            return Err(ObjectValidationError::new(format!(
                "{path} objectType must be a string"
            )));
        }
    };
    match object_type {
        "Activity" => validate_activity(object, path),
        "Agent" | "Group" => Ok(()),
        "StatementRef" => validate_statement_ref(object, path),
        "SubStatement" if !inside_substatement => validate_substatement(object, path),
        "SubStatement" => Err(ObjectValidationError::new(
            "SubStatement must not contain a nested SubStatement",
        )),
        _ => Err(ObjectValidationError::new(format!(
            "{path} has an unsupported objectType"
        ))),
    }
}

fn validate_activity(
    activity: &Map<String, Value>,
    path: &str,
) -> Result<(), ObjectValidationError> {
    allowed_properties(activity, &["objectType", "id", "definition"], path)?;
    required_absolute_iri(activity.get("id"), &format!("{path} id"))?;
    let Some(definition) = activity.get("definition") else {
        return Ok(());
    };
    validate_activity_definition(
        expect_object(definition, &format!("{path} definition"))?,
        &format!("{path} definition"),
    )
}

fn validate_activity_definition(
    definition: &Map<String, Value>,
    path: &str,
) -> Result<(), ObjectValidationError> {
    allowed_properties(
        definition,
        &[
            "name",
            "description",
            "type",
            "moreInfo",
            "interactionType",
            "correctResponsesPattern",
            "choices",
            "scale",
            "source",
            "target",
            "steps",
            "extensions",
        ],
        path,
    )?;
    validate_language_map(definition.get("name"), &format!("{path} name"), false)?;
    validate_language_map(
        definition.get("description"),
        &format!("{path} description"),
        false,
    )?;
    optional_absolute_iri(definition.get("type"), &format!("{path} type"))?;
    optional_absolute_iri(definition.get("moreInfo"), &format!("{path} moreInfo"))?;

    let has_interaction_data = INTERACTION_PROPERTIES[1..]
        .iter()
        .any(|name| definition.contains_key(*name));
    let interaction_type = match definition.get("interactionType") {
        Some(Value::String(value)) => Some(value.as_str()),
        Some(_) => {
            return Err(ObjectValidationError::new(format!(
                "{path} interactionType must be a string"
            )));
        }
        None => None,
    };
    if has_interaction_data && interaction_type.is_none() {
        return Err(ObjectValidationError::new(format!(
            "{path} interactionType is required for an Interaction Activity"
        )));
    }
    if interaction_type.is_some_and(|value| {
        !matches!(
            value,
            "true-false"
                | "choice"
                | "fill-in"
                | "long-fill-in"
                | "matching"
                | "performance"
                | "sequencing"
                | "likert"
                | "numeric"
                | "other"
        )
    }) {
        return Err(ObjectValidationError::new(format!(
            "{path} interactionType is invalid"
        )));
    }
    validate_interaction_arrays(definition, path)
}

fn validate_interaction_arrays(
    definition: &Map<String, Value>,
    path: &str,
) -> Result<(), ObjectValidationError> {
    if let Some(patterns) = definition.get("correctResponsesPattern") {
        let Some(patterns) = patterns.as_array() else {
            return Err(ObjectValidationError::new(format!(
                "{path} correctResponsesPattern must be an array"
            )));
        };
        if patterns.iter().any(|value| !value.is_string()) {
            return Err(ObjectValidationError::new(format!(
                "{path} correctResponsesPattern must contain strings"
            )));
        }
    }
    for name in ["choices", "scale", "source", "target", "steps"] {
        let Some(components) = definition.get(name) else {
            continue;
        };
        let Some(components) = components.as_array() else {
            return Err(ObjectValidationError::new(format!(
                "{path} {name} must be an array"
            )));
        };
        for (index, component) in components.iter().enumerate() {
            let component_path = format!("{path} {name}[{index}]");
            let component = expect_object(component, &component_path)?;
            allowed_properties(component, &["id", "description"], &component_path)?;
            if component.get("id").and_then(Value::as_str).is_none() {
                return Err(ObjectValidationError::new(format!(
                    "{component_path} id must be a string"
                )));
            }
            validate_language_map(
                component.get("description"),
                &format!("{component_path} description"),
                false,
            )?;
        }
    }
    Ok(())
}

fn validate_statement_ref(
    statement_ref: &Map<String, Value>,
    path: &str,
) -> Result<(), ObjectValidationError> {
    allowed_properties(statement_ref, &["objectType", "id"], path)?;
    let id = required_string(statement_ref.get("id"), &format!("{path} id"))?;
    Uuid::parse_str(id)
        .map(|_id| ())
        .map_err(|_error| ObjectValidationError::new(format!("{path} id must be a UUID")))
}

fn validate_substatement(
    substatement: &Map<String, Value>,
    path: &str,
) -> Result<(), ObjectValidationError> {
    allowed_properties(substatement, &SUBSTATEMENT_PROPERTIES, path)?;
    for name in ["actor", "verb", "object"] {
        if !substatement.contains_key(name) {
            return Err(ObjectValidationError::new(format!(
                "{path} {name} is required"
            )));
        }
    }
    validate_verb(substatement.get("verb"), &format!("{path} verb"))?;
    validate_object(substatement.get("object"), &format!("{path} object"), true)?;
    validate_timestamp(substatement.get("timestamp"), &format!("{path} timestamp"))?;
    validate_attachments(
        substatement.get("attachments"),
        &format!("{path} attachments"),
    )
}

fn validate_timestamp(value: Option<&Value>, path: &str) -> Result<(), ObjectValidationError> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some(value) = value.as_str() else {
        return Err(ObjectValidationError::new(format!(
            "{path} must be a string"
        )));
    };
    if value.ends_with("-00:00") || OffsetDateTime::parse(value, &Rfc3339).is_err() {
        return Err(ObjectValidationError::new(format!(
            "{path} must be an RFC3339 timestamp with a known offset"
        )));
    }
    Ok(())
}

fn validate_version(value: Option<&Value>, path: &str) -> Result<(), ObjectValidationError> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some(value) = value.as_str() else {
        return Err(ObjectValidationError::new(format!(
            "{path} must be a string"
        )));
    };
    if !is_semver_1(value) {
        return Err(ObjectValidationError::new(format!(
            "{path} must use Semantic Versioning 1.0.0 format"
        )));
    }
    Ok(())
}

fn is_semver_1(value: &str) -> bool {
    let (core, prerelease) = value
        .split_once('-')
        .map_or((value, None), |(core, suffix)| (core, Some(suffix)));
    let mut numbers = core.split('.');
    if !(valid_semver_number(numbers.next())
        && valid_semver_number(numbers.next())
        && valid_semver_number(numbers.next())
        && numbers.next().is_none())
    {
        return false;
    }
    prerelease.is_none_or(|suffix| {
        !suffix.is_empty()
            && suffix.split('.').all(|identifier| {
                !identifier.is_empty()
                    && identifier
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            })
    })
}

fn valid_semver_number(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        !value.is_empty()
            && value.bytes().all(|byte| byte.is_ascii_digit())
            && (value == "0" || !value.starts_with('0'))
    })
}

fn validate_attachments(value: Option<&Value>, path: &str) -> Result<(), ObjectValidationError> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some(attachments) = value.as_array() else {
        return Err(ObjectValidationError::new(format!(
            "{path} must be an array"
        )));
    };
    for (index, attachment) in attachments.iter().enumerate() {
        let attachment_path = format!("{path}[{index}]");
        let attachment = expect_object(attachment, &attachment_path)?;
        allowed_properties(attachment, &ATTACHMENT_PROPERTIES, &attachment_path)?;
        required_absolute_iri(
            attachment.get("usageType"),
            &format!("{attachment_path} usageType"),
        )?;
        validate_language_map(
            attachment.get("display"),
            &format!("{attachment_path} display"),
            true,
        )?;
        validate_language_map(
            attachment.get("description"),
            &format!("{attachment_path} description"),
            false,
        )?;
        let content_type = required_string(
            attachment.get("contentType"),
            &format!("{attachment_path} contentType"),
        )?;
        let media_type = content_type.split(';').next().unwrap_or_default().trim();
        if !media_type
            .split_once('/')
            .is_some_and(|(kind, subtype)| !kind.is_empty() && !subtype.is_empty())
        {
            return Err(ObjectValidationError::new(format!(
                "{attachment_path} contentType is invalid"
            )));
        }
        if attachment.get("length").and_then(Value::as_u64).is_none() {
            return Err(ObjectValidationError::new(format!(
                "{attachment_path} length must be a non-negative integer"
            )));
        }
        let sha2 = required_string(attachment.get("sha2"), &format!("{attachment_path} sha2"))?;
        if sha2.len() != 64 || !sha2.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ObjectValidationError::new(format!(
                "{attachment_path} sha2 must be a SHA-256 hexadecimal value"
            )));
        }
        optional_absolute_iri(
            attachment.get("fileUrl"),
            &format!("{attachment_path} fileUrl"),
        )?;
    }
    Ok(())
}

fn validate_language_map(
    value: Option<&Value>,
    path: &str,
    required: bool,
) -> Result<(), ObjectValidationError> {
    let Some(value) = value else {
        return if required {
            Err(ObjectValidationError::new(format!("{path} is required")))
        } else {
            Ok(())
        };
    };
    let map = expect_object(value, path)?;
    if map.is_empty()
        || map
            .iter()
            .any(|(language, value)| !is_language_tag(language) || !value.is_string())
    {
        return Err(ObjectValidationError::new(format!(
            "{path} must be a non-empty language map"
        )));
    }
    Ok(())
}

fn required_absolute_iri(value: Option<&Value>, path: &str) -> Result<(), ObjectValidationError> {
    let value = required_string(value, path)?;
    if !is_absolute_iri(value) {
        return Err(ObjectValidationError::new(format!(
            "{path} must be an absolute IRI"
        )));
    }
    Ok(())
}

fn optional_absolute_iri(value: Option<&Value>, path: &str) -> Result<(), ObjectValidationError> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some(value) = value.as_str() else {
        return Err(ObjectValidationError::new(format!(
            "{path} must be a string"
        )));
    };
    if !is_absolute_iri(value) {
        return Err(ObjectValidationError::new(format!(
            "{path} must be an absolute IRI"
        )));
    }
    Ok(())
}

fn required_string<'a>(
    value: Option<&'a Value>,
    path: &str,
) -> Result<&'a str, ObjectValidationError> {
    value
        .and_then(Value::as_str)
        .ok_or_else(|| ObjectValidationError::new(format!("{path} must be a string")))
}

fn required_object<'a>(
    value: Option<&'a Value>,
    path: &str,
) -> Result<&'a Map<String, Value>, ObjectValidationError> {
    value
        .ok_or_else(|| ObjectValidationError::new(format!("{path} is required")))
        .and_then(|value| expect_object(value, path))
}

fn expect_object<'a>(
    value: &'a Value,
    path: &str,
) -> Result<&'a Map<String, Value>, ObjectValidationError> {
    value
        .as_object()
        .ok_or_else(|| ObjectValidationError::new(format!("{path} must be a JSON object")))
}

fn allowed_properties(
    object: &Map<String, Value>,
    allowed: &[&str],
    path: &str,
) -> Result<(), ObjectValidationError> {
    if object.keys().any(|name| !allowed.contains(&name.as_str())) {
        return Err(ObjectValidationError::new(format!(
            "{path} contains an unsupported property"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::validate_statement_object_model;

    fn statement(object: &serde_json::Value) -> serde_json::Value {
        json!({
            "actor": {"objectType": "Agent", "mbox": "mailto:learner@example.org"},
            "verb": {"id": "https://example.org/verbs/experienced"},
            "object": object
        })
    }

    #[test]
    fn rejects_interaction_data_without_interaction_type() {
        let value = statement(&json!({
            "id": "https://example.org/activity",
            "definition": {"choices": [{"id": "a"}]}
        }));
        assert!(validate_statement_object_model(&value).is_err());
    }

    #[test]
    fn accepts_valid_interaction_activity() {
        let value = statement(&json!({
            "id": "https://example.org/activity",
            "definition": {"interactionType": "choice", "choices": [{"id": "a"}]}
        }));
        assert!(validate_statement_object_model(&value).is_ok());
    }

    #[test]
    fn rejects_substatement_managed_properties_and_missing_required_fields() {
        for invalid in [
            json!({"objectType": "SubStatement", "actor": {}, "verb": {}, "object": {}, "stored": "2025-01-01T00:00:00Z"}),
            json!({"objectType": "SubStatement", "actor": {}, "verb": {"id": "https://example.org/v"}}),
        ] {
            assert!(validate_statement_object_model(&statement(&invalid)).is_err());
        }
    }

    #[test]
    fn rejects_invalid_substatement_timestamp() {
        let value = statement(&json!({
            "objectType": "SubStatement",
            "actor": {"objectType": "Agent", "mbox": "mailto:learner@example.org"},
            "verb": {"id": "https://example.org/verbs/experienced"},
            "object": {"id": "https://example.org/activity"},
            "timestamp": "2025-01-01T00:00:00-00:00"
        }));
        assert!(validate_statement_object_model(&value).is_err());
    }

    #[test]
    fn accepts_complete_attachment_metadata() {
        let mut value = statement(&json!({"id": "https://example.org/activity"}));
        value["attachments"] = json!([{
            "usageType": "https://example.org/attachment",
            "display": {"en": "evidence"},
            "contentType": "text/plain",
            "length": 5,
            "sha2": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
            "fileUrl": "https://example.org/evidence.txt"
        }]);
        assert!(validate_statement_object_model(&value).is_ok());
    }

    #[test]
    fn validates_statement_version_as_semver_1() {
        for version in ["0.9.9", "1.1.0", "2.0.0", "2.0.0-rc.1"] {
            let mut value = statement(&json!({"id": "https://example.org/activity"}));
            value["version"] = json!(version);
            assert!(validate_statement_object_model(&value).is_ok(), "{version}");
        }
        for version in ["should fail", "1.0", "01.0.0", "1.0.0+"] {
            let mut value = statement(&json!({"id": "https://example.org/activity"}));
            value["version"] = json!(version);
            assert!(
                validate_statement_object_model(&value).is_err(),
                "{version}"
            );
        }
    }
}

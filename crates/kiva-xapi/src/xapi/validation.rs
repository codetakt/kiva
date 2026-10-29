#![allow(clippy::wildcard_imports)]

use super::context_result_validation::validate_statement_context_result as validate_context_result_model;
use super::*;

/// xAPI Statement shape validation error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct StatementValidationError {
    message: String,
}

impl StatementValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the HTTP-facing validation message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Validates the supported xAPI Statement JSON shape without depending on HTTP adapters.
pub fn validate_statement_shape(statement: &Value) -> Result<(), StatementValidationError> {
    validate_statement_object_model(statement)
        .map_err(|error| StatementValidationError::new(error.to_string()))?;
    let object = statement_object(statement)?;
    if let Some(id) = object.get("id")
        && !id
            .as_str()
            .is_some_and(|id| id.len() == 36 && Uuid::parse_str(id).is_ok())
    {
        return Err(StatementValidationError::new("Statement id must be a UUID"));
    }
    validate_allowed_properties(
        object,
        &[
            "id",
            "actor",
            "verb",
            "object",
            "result",
            "context",
            "timestamp",
            "stored",
            "authority",
            "version",
            "attachments",
        ],
        "Statement",
    )?;
    validate_required_property(object, "actor")?;
    validate_required_property(object, "verb")?;
    validate_required_property(object, "object")?;
    validate_statement_actors(statement)
        .map_err(|error| StatementValidationError::new(error.to_string()))?;
    validate_verb(object.get("verb"))?;
    validate_statement_object(object.get("object"))?;
    validate_voiding_statement(object)?;
    validate_attachments_metadata(object.get("attachments"))?;
    Ok(())
}

/// Validates Statement Context, Result, and nested `SubStatement` Context/Result data.
pub fn validate_statement_context_result_shape(
    statement: &Value,
) -> Result<(), StatementValidationError> {
    validate_context_result_model(statement)
        .map_err(|error| StatementValidationError::new(error.to_string()))
}

fn validate_voiding_statement(object: &Map<String, Value>) -> Result<(), StatementValidationError> {
    if object
        .get("verb")
        .and_then(|verb| verb.get("id"))
        .and_then(Value::as_str)
        == Some("http://adlnet.gov/expapi/verbs/voided")
        && object
            .get("object")
            .and_then(|statement_object| statement_object.get("objectType"))
            .and_then(Value::as_str)
            != Some("StatementRef")
    {
        return Err(StatementValidationError::new(
            "voiding Statements must use a StatementRef object",
        ));
    }
    Ok(())
}

fn statement_object(statement: &Value) -> Result<&Map<String, Value>, StatementValidationError> {
    statement
        .as_object()
        .ok_or_else(|| StatementValidationError::new("statement must be a JSON object"))
}

fn validate_allowed_properties(
    object: &Map<String, Value>,
    allowed: &[&str],
    label: &str,
) -> Result<(), StatementValidationError> {
    if let Some(name) = object.keys().find(|name| !allowed.contains(&name.as_str())) {
        return Err(StatementValidationError::new(format!(
            "{label} contains an unsupported property: {name}"
        )));
    }
    Ok(())
}

fn validate_required_property(
    object: &Map<String, Value>,
    name: &str,
) -> Result<(), StatementValidationError> {
    match object.get(name) {
        Some(value) if !value.is_null() => Ok(()),
        Some(_) => Err(StatementValidationError::new(format!(
            "Statement {name} must not be null"
        ))),
        None => Err(StatementValidationError::new(format!(
            "Statement {name} is required"
        ))),
    }
}

fn validate_verb(verb: Option<&Value>) -> Result<(), StatementValidationError> {
    let Some(verb) = verb else {
        return Ok(());
    };
    let Some(verb) = verb.as_object() else {
        return Err(StatementValidationError::new(
            "Statement verb must be a JSON object",
        ));
    };
    let id = verb
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| StatementValidationError::new("Statement verb id is required"))?;
    if !is_absolute_iri(id) {
        return Err(StatementValidationError::new(
            "Statement verb id must be an absolute IRI",
        ));
    }
    validate_language_map(verb.get("display"), "verb display")
}

fn validate_statement_object(object: Option<&Value>) -> Result<(), StatementValidationError> {
    let Some(object) = object else {
        return Ok(());
    };
    let Some(object) = object.as_object() else {
        return Err(StatementValidationError::new(
            "Statement object must be a JSON object",
        ));
    };
    let object_type = object
        .get("objectType")
        .and_then(Value::as_str)
        .unwrap_or("Activity");
    match object_type {
        "Activity" => validate_activity_object(object),
        "Agent" | "Group" => Ok(()),
        "StatementRef" => validate_statement_ref(object),
        "SubStatement" => validate_substatement(object),
        _ => Err(StatementValidationError::new(
            "Statement object has an unsupported objectType",
        )),
    }
}

pub(crate) fn validate_activity_object(
    activity: &Map<String, Value>,
) -> Result<(), StatementValidationError> {
    let id = activity
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| StatementValidationError::new("Activity id is required"))?;
    if !is_absolute_iri(id) {
        return Err(StatementValidationError::new(
            "Activity id must be an absolute IRI",
        ));
    }
    if let Some(definition) = activity.get("definition") {
        validate_activity_definition(definition)?;
    }
    Ok(())
}

fn validate_activity_definition(definition: &Value) -> Result<(), StatementValidationError> {
    let Some(definition) = definition.as_object() else {
        return Err(StatementValidationError::new(
            "Activity definition must be a JSON object",
        ));
    };
    if let Some(interaction_type) = definition.get("interactionType").and_then(Value::as_str)
        && !matches!(
            interaction_type,
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
    {
        return Err(StatementValidationError::new(
            "Activity definition interactionType is invalid",
        ));
    }
    validate_language_map(definition.get("name"), "Activity definition name")?;
    validate_language_map(
        definition.get("description"),
        "Activity definition description",
    )?;
    validate_extensions(definition.get("extensions"))?;
    Ok(())
}

fn validate_statement_ref(object: &Map<String, Value>) -> Result<(), StatementValidationError> {
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| StatementValidationError::new("StatementRef id is required"))?;
    Uuid::parse_str(id)
        .map(|_id| ())
        .map_err(|_error| StatementValidationError::new("statementId must be a UUID"))
}

fn validate_substatement(object: &Map<String, Value>) -> Result<(), StatementValidationError> {
    if object
        .get("object")
        .and_then(|statement_object| statement_object.get("objectType"))
        .and_then(Value::as_str)
        == Some("SubStatement")
    {
        return Err(StatementValidationError::new(
            "SubStatement must not contain a nested SubStatement",
        ));
    }
    validate_verb(object.get("verb"))?;
    validate_statement_object(object.get("object"))
}

pub(crate) fn validate_extensions(
    extensions: Option<&Value>,
) -> Result<(), StatementValidationError> {
    let Some(extensions) = extensions else {
        return Ok(());
    };
    let Some(extensions) = extensions.as_object() else {
        return Err(StatementValidationError::new(
            "extensions must be a JSON object",
        ));
    };
    if extensions.keys().any(|key| !is_absolute_iri(key)) {
        return Err(StatementValidationError::new(
            "extension keys must be absolute IRIs",
        ));
    }
    Ok(())
}

fn validate_attachments_metadata(
    attachments: Option<&Value>,
) -> Result<(), StatementValidationError> {
    let Some(attachments) = attachments else {
        return Ok(());
    };
    if !attachments.is_array() {
        return Err(StatementValidationError::new(
            "Statement attachments must be an array",
        ));
    }
    Ok(())
}

fn validate_language_map(
    language_map: Option<&Value>,
    label: &str,
) -> Result<(), StatementValidationError> {
    let Some(language_map) = language_map else {
        return Ok(());
    };
    let Some(language_map) = language_map.as_object() else {
        return Err(StatementValidationError::new(format!(
            "{label} must be a language map object"
        )));
    };
    if language_map.keys().any(|key| !is_language_tag(key)) {
        return Err(StatementValidationError::new(format!(
            "{label} contains an invalid language tag"
        )));
    }
    Ok(())
}

pub(crate) fn is_language_tag(value: &str) -> bool {
    super::language_tag::valid_language_tag(value)
}

/// Checks the complete syntax of an IRI with a scheme, without normalizing it.
#[must_use]
pub fn is_absolute_iri(value: &str) -> bool {
    iri_string::types::IriStr::new(value).is_ok()
}

/// Checks locator syntax, including the required host of HTTP(S) locators.
/// This does not resolve the locator or restrict ordinary identifier fields.
#[must_use]
pub fn is_absolute_irl(value: &str) -> bool {
    iri_string::types::IriStr::new(value).is_ok_and(|iri| {
        !matches!(
            iri.scheme_str().to_ascii_lowercase().as_str(),
            "http" | "https"
        ) || iri
            .authority_components()
            .is_some_and(|authority| !authority.host().is_empty())
    })
}

pub(crate) fn is_duration(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('P') else {
        return false;
    };
    if rest.is_empty() || rest.contains('-') || rest.contains(':') {
        return false;
    }
    if let Some(week) = rest.strip_suffix('W') {
        return is_duration_number(week);
    }

    let mut parts = rest.split('T');
    let date = parts.next().unwrap_or_default();
    let time = parts.next();
    if parts.next().is_some() {
        return false;
    }
    if time.is_some() && date.contains(['.', ',']) {
        // ISO 8601 allows a fraction only on the lowest-order component.
        return false;
    }

    let date_consumed = parse_duration_components(date, &['Y', 'M', 'D']);
    let Some((date_valid, date_count)) = date_consumed else {
        return false;
    };
    let time_count = match time {
        Some("") => return false,
        Some(time) => match parse_duration_components(time, &['H', 'M', 'S']) {
            Some((true, count)) => count,
            _ => return false,
        },
        None => 0,
    };
    date_valid && date_count + time_count > 0
}

fn parse_duration_components(value: &str, designators: &[char]) -> Option<(bool, usize)> {
    let mut rest = value;
    let mut count = 0;
    for designator in designators {
        let Some(index) = rest.find(*designator) else {
            continue;
        };
        let number = &rest[..index];
        if !is_duration_number(number) {
            return None;
        }
        rest = &rest[index + designator.len_utf8()..];
        if number.contains(['.', ',']) && !rest.is_empty() {
            return None;
        }
        count += 1;
    }
    Some((rest.is_empty(), count))
}

fn is_duration_number(value: &str) -> bool {
    let Some((whole, fraction)) = value.split_once(['.', ',']) else {
        return !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit());
    };
    !whole.is_empty()
        && !fraction.is_empty()
        && whole.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
}

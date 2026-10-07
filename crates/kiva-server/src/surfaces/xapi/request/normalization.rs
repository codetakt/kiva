//! xAPI Statement normalization before repository admission.

use std::collections::{BTreeMap, BTreeSet};

use axum::http::StatusCode;
use kiva_xapi::{StatementValidationError, admit_json_object, validate_statement_shape};
use serde_json::{Map, Value};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::{
    core::{
        config::XAPI_VERSION,
        response::{HttpError, rfc3339},
    },
    surfaces::xapi::model::{AttachmentRecord, StoredStatement},
};

use super::{attachments::attachments_for_statement, query::parse_statement_id};

pub(crate) fn normalize_statement_batch(
    body: Value,
    stored_at: OffsetDateTime,
    authority: &Value,
    attachments: &BTreeMap<String, AttachmentRecord>,
) -> Result<Vec<(Uuid, StoredStatement)>, HttpError> {
    let values = match body {
        Value::Array(values) => values,
        value => vec![value],
    };
    if values.is_empty() {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "statement batch must not be empty",
        ));
    }

    let mut seen = BTreeSet::new();
    let mut statements = Vec::with_capacity(values.len());
    for value in values {
        let statement_id = statement_id_or_new(&value)?;
        if !seen.insert(statement_id) {
            return Err(HttpError::new(
                StatusCode::BAD_REQUEST,
                "statement batch must not contain duplicate ids",
            ));
        }
        let statement = normalized_statement(value, statement_id, stored_at, authority)?;
        let statement_attachments = attachments_for_statement(&statement, attachments)?;
        statements.push((
            statement_id,
            StoredStatement::new(statement, stored_at, statement_attachments),
        ));
    }
    Ok(statements)
}

fn statement_id_or_new(value: &Value) -> Result<Uuid, HttpError> {
    let Some(object) = value.as_object() else {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "statement must be a JSON object",
        ));
    };
    object.get("id").map_or_else(
        || Ok(Uuid::new_v4()),
        |id| {
            id.as_str().map_or_else(
                || {
                    Err(HttpError::new(
                        StatusCode::BAD_REQUEST,
                        "statement id must be a UUID",
                    ))
                },
                parse_statement_id,
            )
        },
    )
}

pub(crate) fn normalized_statement(
    mut value: Value,
    statement_id: Uuid,
    stored_at: OffsetDateTime,
    authority: &Value,
) -> Result<Value, HttpError> {
    admit_json_object(&value).map_err(|_error| {
        HttpError::new(StatusCode::BAD_REQUEST, "statement must be a JSON object")
    })?;
    validate_statement_shape(&value).map_err(|error| statement_validation_error(&error))?;
    let Some(object) = value.as_object_mut() else {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "statement must be a JSON object",
        ));
    };
    match object.get("id").and_then(Value::as_str) {
        Some(existing_id) if parse_statement_id(existing_id)? != statement_id => {
            return Err(HttpError::new(
                StatusCode::BAD_REQUEST,
                "statement id must match statementId",
            ));
        }
        Some(_) => {}
        None => {
            object.insert("id".to_owned(), Value::String(statement_id.to_string()));
        }
    }
    object
        .entry("version".to_owned())
        .or_insert_with(|| Value::String(XAPI_VERSION.to_owned()));
    let stored = rfc3339(stored_at);
    object.insert("stored".to_owned(), Value::String(stored.clone()));
    object
        .entry("timestamp".to_owned())
        .or_insert_with(|| Value::String(stored));
    normalize_statement_timestamp(object)?;
    object.insert("authority".to_owned(), authority.clone());
    Ok(value)
}

fn statement_validation_error(error: &StatementValidationError) -> HttpError {
    HttpError::new(StatusCode::BAD_REQUEST, error.message().to_owned())
}

fn normalize_statement_timestamp(object: &mut Map<String, Value>) -> Result<(), HttpError> {
    let Some(timestamp) = object.get("timestamp") else {
        return Ok(());
    };
    let Some(timestamp) = timestamp.as_str() else {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "statement timestamp must be a string",
        ));
    };
    let timestamp = OffsetDateTime::parse(timestamp, &Rfc3339).map_err(|_error| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            "statement timestamp must be an RFC3339 timestamp",
        )
    })?;
    object.insert("timestamp".to_owned(), Value::String(rfc3339(timestamp)));
    Ok(())
}

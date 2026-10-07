//! xAPI `OpenAPI` request DTO admission.

use axum::http::StatusCode;
use kiva_xapi::{validate_statement_actors, validate_statement_context_result_shape};
use kiva_xapi_openapi_contract as xapi_contract;
use serde_json::Value;

use crate::core::response::HttpError;

pub(crate) fn statement_body_from_openapi_contract(
    value: Value,
    label: &'static str,
) -> Result<Value, HttpError> {
    if value.is_array() {
        serde_json::from_value::<Vec<xapi_contract::Statement>>(value.clone()).map_err(
            |_error| {
                HttpError::new(
                    StatusCode::BAD_REQUEST,
                    format!("{label} must match the xAPI Statement OpenAPI contract"),
                )
            },
        )?;
        validate_statement_actor_contract(&value, label)?;
        validate_statement_context_result_contract(&value, label)?;
        return Ok(value);
    }

    serde_json::from_value::<xapi_contract::Statement>(value.clone()).map_err(|_error| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            format!("{label} must match the xAPI Statement OpenAPI contract"),
        )
    })?;
    validate_statement_actor_contract(&value, label)?;
    validate_statement_context_result_contract(&value, label)?;
    Ok(value)
}

fn validate_statement_actor_contract(value: &Value, label: &str) -> Result<(), HttpError> {
    let values: Vec<&Value> = match value {
        Value::Array(values) => values.iter().collect(),
        value => vec![value],
    };
    for statement in values {
        validate_statement_actors(statement).map_err(|_error| {
            HttpError::new(
                StatusCode::BAD_REQUEST,
                format!("{label} violates xAPI Actor requirements"),
            )
        })?;
    }
    Ok(())
}

fn validate_statement_context_result_contract(value: &Value, label: &str) -> Result<(), HttpError> {
    let values: Vec<&Value> = match value {
        Value::Array(values) => values.iter().collect(),
        value => vec![value],
    };
    for statement in values {
        validate_statement_context_result_shape(statement).map_err(|_error| {
            HttpError::new(
                StatusCode::BAD_REQUEST,
                format!("{label} violates xAPI Context/Result requirements"),
            )
        })?;
    }
    Ok(())
}

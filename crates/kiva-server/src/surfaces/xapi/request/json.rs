//! JSON request helpers for xAPI payloads.

use axum::http::StatusCode;
use serde_json::Value;

use crate::core::response::HttpError;

pub(crate) fn parse_json_without_duplicate_properties(
    body: &[u8],
    label: &str,
) -> Result<Value, HttpError> {
    kiva_xapi::validate_unique_json_properties(body).map_err(|_error| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            format!("{label} must be valid JSON without duplicate object properties"),
        )
    })?;
    serde_json::from_slice::<crate::surfaces::xapi::lossless::StoredJson>(body)
        .map(|value| value.0)
        .map_err(|_error| {
            HttpError::new(
                StatusCode::BAD_REQUEST,
                format!("{label} must be valid JSON"),
            )
        })
}

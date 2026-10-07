//! HTTP request extraction helpers.

use axum::http::StatusCode;
use serde::de;

use crate::core::response::HttpError;

/// Parses a JSON request body into a typed request payload.
pub(crate) fn parse_json_request<T>(body: &[u8], label: &str) -> Result<T, HttpError>
where
    T: de::DeserializeOwned,
{
    serde_json::from_slice::<T>(body).map_err(|_error| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            format!("{label} must be valid JSON"),
        )
    })
}

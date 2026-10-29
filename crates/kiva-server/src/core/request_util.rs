//! Shared HTTP request parsing helpers.

use std::collections::BTreeMap;

use axum::http::{HeaderMap, HeaderValue, StatusCode};
use kiva_confidential::primitive::SecureAggregateTranscriptId;
use uuid::Uuid;

use crate::core::response::{HttpError, fixed_hex_32};

pub(crate) fn header_string(
    headers: &HeaderMap,
    name: ::http::header::HeaderName,
) -> Option<String> {
    headers.get(name).map(header_text)
}

/// RFC 2616 TEXT is an octet field, including ISO-8859-1 in quoted strings.
/// Keep a one-to-one octet/character mapping even for non-UTF-8 values; ASCII
/// `HeaderValue::to_str` and lossy UTF-8 would discard or conflate valid fields.
/// Stored values must use `latin1_header_value` when emitted back onto the wire.
pub(crate) fn header_text(value: &HeaderValue) -> String {
    value.as_bytes().iter().copied().map(char::from).collect()
}

pub(crate) fn key_component(name: &str, value: &str) -> String {
    format!("{name}:{}:{value}|", value.len())
}

pub(crate) fn required_query<'a>(
    query: &'a BTreeMap<String, String>,
    name: &str,
) -> Result<&'a str, HttpError> {
    match query.get(name).map(String::as_str) {
        Some(value) if !value.is_empty() => Ok(value),
        Some(_) => Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            format!("{name} must not be empty"),
        )),
        None => Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            format!("{name} is required"),
        )),
    }
}

pub(crate) fn optional_uuid_query(
    query: &BTreeMap<String, String>,
    name: &str,
) -> Result<Option<Uuid>, HttpError> {
    query
        .get(name)
        .map(|value| {
            Uuid::parse_str(value).map_err(|_error| {
                HttpError::new(StatusCode::BAD_REQUEST, format!("{name} must be a UUID"))
            })
        })
        .transpose()
}

pub(crate) fn required_transcript_id_query(
    query: &BTreeMap<String, String>,
) -> Result<SecureAggregateTranscriptId, HttpError> {
    let value = required_query(query, "transcriptId")?;
    SecureAggregateTranscriptId::new(fixed_hex_32(value, "transcriptId")?).map_err(|_error| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            "transcriptId must not be the reserved empty identifier",
        )
    })
}

pub(crate) fn optional_hex_32_query(
    query: &BTreeMap<String, String>,
    name: &str,
) -> Result<Option<SecureAggregateTranscriptId>, HttpError> {
    query
        .get(name)
        .map(|value| {
            SecureAggregateTranscriptId::new(fixed_hex_32(value, name)?).map_err(|_error| {
                HttpError::new(
                    StatusCode::BAD_REQUEST,
                    format!("{name} must not be the reserved empty identifier"),
                )
            })
        })
        .transpose()
}

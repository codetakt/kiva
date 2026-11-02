//! Shared HTTP response helpers.

use axum::{
    Json,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use std::fmt::Debug;
use time::{Month, OffsetDateTime, UtcOffset, Weekday, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::core::{
    config::{XAPI_VERSION, XAPI_VERSION_HEADER},
    runtime::RuntimeLimits,
};

#[derive(Debug)]
pub(crate) struct HttpError {
    pub(crate) status: StatusCode,
    pub(crate) message: String,
}

impl HttpError {
    pub(crate) fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        json_response(
            self.status,
            json!({
                "error": self.message
            }),
        )
    }
}

pub(crate) fn new_etag() -> String {
    format!("\"{}\"", Uuid::new_v4())
}

pub(crate) fn insert_header(
    headers: &mut HeaderMap,
    name: ::http::header::HeaderName,
    value: &str,
) {
    if let Ok(value) = HeaderValue::from_str(value) {
        headers.insert(name, value);
    }
}

/// Inverse of `request_util::header_text` for stored HTTP field octets.
/// Do not serialize the internal ISO-8859-1 character mapping as UTF-8.
pub(crate) fn latin1_header_value(value: &str) -> Result<HeaderValue, HttpError> {
    let invalid = || {
        HttpError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "stored HTTP header could not be serialized",
        )
    };
    let bytes = value
        .chars()
        .map(u8::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid())?;
    HeaderValue::from_bytes(&bytes).map_err(|_| invalid())
}

pub(crate) fn http_date(value: OffsetDateTime) -> String {
    format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
        weekday_name(value.weekday()),
        value.day(),
        month_name(value.month()),
        value.year(),
        value.hour(),
        value.minute(),
        value.second()
    )
}

pub(crate) const fn weekday_name(value: Weekday) -> &'static str {
    match value {
        Weekday::Monday => "Mon",
        Weekday::Tuesday => "Tue",
        Weekday::Wednesday => "Wed",
        Weekday::Thursday => "Thu",
        Weekday::Friday => "Fri",
        Weekday::Saturday => "Sat",
        Weekday::Sunday => "Sun",
    }
}

pub(crate) const fn month_name(value: Month) -> &'static str {
    match value {
        Month::January => "Jan",
        Month::February => "Feb",
        Month::March => "Mar",
        Month::April => "Apr",
        Month::May => "May",
        Month::June => "Jun",
        Month::July => "Jul",
        Month::August => "Aug",
        Month::September => "Sep",
        Month::October => "Oct",
        Month::November => "Nov",
        Month::December => "Dec",
    }
}

pub(crate) fn fixed_hex_32(value: &str, name: &str) -> Result<[u8; 32], HttpError> {
    if value.len() != 64 {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            format!("{name} must be 64 lowercase or uppercase hexadecimal characters"),
        ));
    }
    let mut bytes = [0_u8; 32];
    for (offset, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let Some(high) = hex_value(chunk[0]) else {
            return Err(invalid_hex_response(name));
        };
        let Some(low) = hex_value(chunk[1]) else {
            return Err(invalid_hex_response(name));
        };
        bytes[offset] = (high << 4) | low;
    }
    Ok(bytes)
}

pub(crate) const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(crate) fn invalid_hex_response(name: &str) -> HttpError {
    HttpError::new(
        StatusCode::BAD_REQUEST,
        format!("{name} must be 64 lowercase or uppercase hexadecimal characters"),
    )
}

pub(crate) fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

pub(crate) async fn rate_limit_response(limits: &RuntimeLimits, path: &str) -> Option<Response> {
    if limits.allows_path(path).await {
        return None;
    }
    Some(
        HttpError::new(StatusCode::TOO_MANY_REQUESTS, "request rate limit exceeded")
            .into_response(),
    )
}

pub(crate) fn json_response(status: StatusCode, body: Value) -> Response {
    (status, version_headers(), Json(body)).into_response()
}

pub(crate) fn json_bytes_response(
    status: StatusCode,
    body: Result<Vec<u8>, serde_json::Error>,
) -> Response {
    match body {
        Ok(body) => {
            let mut headers = version_headers();
            headers.insert(
                ::http::header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            (status, headers, body).into_response()
        }
        Err(error) => backend_error_response(
            &error,
            "HTTP response projection failed",
            "HTTP response projection failed",
        ),
    }
}

pub(crate) fn json_projection_response(
    status: StatusCode,
    body: Result<Value, serde_json::Error>,
) -> Response {
    match body {
        Ok(body) => json_response(status, body),
        Err(error) => backend_error_response(
            &error,
            "HTTP response projection failed",
            "HTTP response projection failed",
        ),
    }
}

pub(crate) fn backend_error_response(
    source: &impl Debug,
    log_message: &'static str,
    response_message: &'static str,
) -> Response {
    tracing::error!(error = ?source, "{log_message}");
    HttpError::new(StatusCode::INTERNAL_SERVER_ERROR, response_message).into_response()
}

pub(crate) fn empty_response(status: StatusCode) -> Response {
    (status, version_headers()).into_response()
}

pub(crate) fn version_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(XAPI_VERSION_HEADER, HeaderValue::from_static(XAPI_VERSION));
    headers
}

pub(crate) fn statement_result_headers() -> HeaderMap {
    // The Statement route middleware supplies the repository watermark captured
    // BEFORE the response's reads. Wall time here can race an unfinished write.
    version_headers()
}

pub(crate) fn statement_resource_response(response: impl IntoResponse) -> Response {
    let mut response = response.into_response();
    response.headers_mut().extend(statement_result_headers());
    response
}

pub(crate) fn rfc3339(value: OffsetDateTime) -> String {
    value
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .unwrap_or_else(|_error| "1970-01-01T00:00:00Z".to_owned())
}

pub(crate) fn postgres_microsecond_timestamp(value: OffsetDateTime) -> OffsetDateTime {
    let nanosecond = value.nanosecond();
    value
        .replace_nanosecond((nanosecond / 1_000) * 1_000)
        .unwrap_or(value)
}

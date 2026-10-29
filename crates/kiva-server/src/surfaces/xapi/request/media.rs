//! HTTP error mapping and parameter equivalence for structured media fields.

use crate::core::response::HttpError;
use axum::http::StatusCode;
use kiva_xapi::MediaType;

pub(super) fn invalid() -> HttpError {
    HttpError::new(
        StatusCode::BAD_REQUEST,
        "invalid or unsupported structured MIME field",
    )
}

pub(super) fn media_type(text: &str, mime: bool) -> Result<MediaType, HttpError> {
    kiva_xapi::parse_media_type(text, mime).map_err(|_| invalid())
}

pub(super) fn transfer_encoding(text: &str) -> Result<(), HttpError> {
    kiva_xapi::validate_binary_transfer_encoding(text).map_err(|_| invalid())
}

/// Establish the required relation using supported parameter definitions.
/// Unknown parameter semantics are not arbitrarily lowercased or ignored:
/// Kiva accepts identical decoded values and rejects other unestablished
/// relations as an explicit unsupported-input policy.
pub(crate) fn matching_types(left: &str, right: &str) -> Result<bool, HttpError> {
    let mut left = media_type(left, true)?;
    let mut right = media_type(right, true)?;
    if left.essence != right.essence {
        return Ok(false);
    }
    for media in [&mut left, &mut right] {
        if media.essence == "text/plain" {
            media
                .parameters
                .entry("charset".into())
                .or_insert_with(|| "us-ascii".into());
        }
        for (name, value) in &mut media.parameters {
            if name == "charset" && media.essence.starts_with("text/")
                || name == "access-type" && media.essence == "message/external-body"
            {
                value.make_ascii_lowercase();
            }
        }
    }
    let mut unsupported = false;
    for name in left.parameters.keys().chain(right.parameters.keys()) {
        let a = left.parameters.get(name);
        let b = right.parameters.get(name);
        if a == b {
            continue;
        }
        if name == "boundary" && left.essence.starts_with("multipart/")
            || a.is_some()
                && b.is_some()
                && (name == "charset" && left.essence.starts_with("text/")
                    || name == "access-type" && left.essence == "message/external-body")
        {
            return Ok(false);
        }
        unsupported = true;
    }
    if unsupported {
        Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "attachment media parameter equivalence is not supported for these values",
        ))
    } else {
        Ok(true)
    }
}

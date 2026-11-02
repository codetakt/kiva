//! Incorporated RFC2616 conditions and the selected D-04 mixed-condition policy.

use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use kiva_http_core::{etag_list_matches, etag_list_matches_weak};
use time::OffsetDateTime;

use crate::{core::response::HttpError, surfaces::xapi::model::Preconditions};

pub(crate) fn mixed_condition_response(headers: &HeaderMap) -> Option<Response> {
    Preconditions::from_headers(headers)
        .undefined_combination
        .then(undefined_condition_response)
}

pub(crate) fn undefined_condition_response() -> Response {
    let mut response=HttpError::new(StatusCode::BAD_REQUEST,
        "The supplied conditional headers have an undefined combination; send separate supported conditions.").into_response();
    // HEAD has no entity; retain the diagnostic without inventing an IEEE code.
    response.headers_mut().insert(
        "x-kiva-error",
        axum::http::HeaderValue::from_static("undefined-precondition-combination"),
    );
    response
}

/// Conditions concern the selected representation, including a document ID list.
/// Lists have an `ETag` but no recorded modification history; unknown dates must
/// not be synthesized from surviving members (which cannot describe deletions).
pub(crate) fn document_read_status(
    etag: &str,
    modified_at: Option<OffsetDateTime>,
    headers: &HeaderMap,
) -> StatusCode {
    let conditions = Preconditions::from_headers(headers);
    if conditions
        .if_match
        .as_deref()
        .is_some_and(|value| !etag_list_matches(value, etag))
        || conditions
            .if_unmodified_since
            .zip(modified_at)
            .is_some_and(|(date, modified)| modified.unix_timestamp() > date.unix_timestamp())
    {
        return StatusCode::PRECONDITION_FAILED;
    }
    let unchanged = conditions
        .if_modified_since
        .zip(modified_at)
        .map(|(date, modified)| modified.unix_timestamp() <= date.unix_timestamp());
    if let Some(tags) = conditions.if_none_match {
        // Only read If-None-Match permits weak comparison. Preserve quoted
        // tag boundaries with the same parser used by strong conditions.
        let matches = etag_list_matches_weak(&tags, etag);
        if matches && unchanged != Some(false) {
            return StatusCode::NOT_MODIFIED;
        }
        // A nonmatching tag explicitly disables If-Modified-Since (14.26).
        return StatusCode::OK;
    }
    if unchanged == Some(true) {
        StatusCode::NOT_MODIFIED
    } else {
        StatusCode::OK
    }
}

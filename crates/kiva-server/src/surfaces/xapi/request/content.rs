//! xAPI Statement and document content-type request parsing.

use axum::http::{HeaderMap, StatusCode, header::CONTENT_TYPE};

use crate::{
    core::{
        config::{DOCUMENT_JSON_CONTENT_TYPE, MULTIPART_MIXED_CONTENT_TYPE},
        request_util::header_string,
        response::HttpError,
    },
    surfaces::xapi::model::{DocumentContent, StatementPayload},
};

use super::{
    json::parse_json_without_duplicate_properties, multipart::multipart_statement_payload,
    openapi::statement_body_from_openapi_contract,
};

pub(crate) fn statement_payload_from_request(
    headers: &HeaderMap,
    body: &[u8],
) -> Result<StatementPayload, HttpError> {
    let content_type = header_string(headers, CONTENT_TYPE).unwrap_or_default();
    if content_type_matches(&content_type, DOCUMENT_JSON_CONTENT_TYPE) || content_type.is_empty() {
        return parse_json_without_duplicate_properties(body, "statement body")
            .and_then(|body| statement_body_from_openapi_contract(body, "statement body"))
            .map(StatementPayload::json);
    }
    if content_type_matches(&content_type, MULTIPART_MIXED_CONTENT_TYPE) {
        return multipart_statement_payload(&content_type, body);
    }

    Err(HttpError::new(
        StatusCode::BAD_REQUEST,
        "Statement request content type must be application/json or multipart/mixed",
    ))
}

pub(crate) fn document_content_from_request(
    headers: &HeaderMap,
    body: &[u8],
) -> Result<DocumentContent, HttpError> {
    DocumentContent::from_request(headers, body)
}

pub(crate) fn document_request_content_type(headers: &HeaderMap) -> String {
    header_string(headers, CONTENT_TYPE)
        .filter(|content_type| !content_type.trim().is_empty())
        .unwrap_or_else(|| "application/octet-stream".to_owned())
}

pub(crate) fn content_type_matches(content_type: &str, expected: &str) -> bool {
    content_type
        .split(';')
        .next()
        .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case(expected))
}

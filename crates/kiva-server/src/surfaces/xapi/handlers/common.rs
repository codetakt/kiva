//! Common imports for xAPI HTTP handler resource modules.

pub(super) use std::collections::BTreeMap;

pub(super) use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
pub(super) use serde_json::json;
pub(super) use time::OffsetDateTime;

pub(super) use crate::core::{
    outcome::StoreOutcome,
    response::{
        HttpError, empty_response, json_projection_response, json_response,
        statement_resource_response,
    },
    state::XapiHttpContext,
};
pub(super) use crate::surfaces::xapi::{
    model::{
        DocumentAddress, DocumentIdRequirement, DocumentKind, DocumentSelection, Preconditions,
    },
    outcome::{DocumentDeleteOutcome, DocumentWriteOutcome, PutBatchError},
    request::{
        document_content_from_request, filtered_statements, formatted_single_statement,
        formatted_statement, normalize_statement_batch, normalized_statement_record,
        optional_limit, parse_statement_id, person_from_agent, required_agent, required_iri,
        statement_authority, statement_payload_from_request, validate_query_parameters,
        validate_statement_get_query,
    },
    response::{
        document_response, document_store_error_response, required_document_address,
        single_statement_response, statement_body_limit_response,
        statement_result_multipart_response, statement_result_response, store_error_response,
    },
    service as xapi_service, wire,
};

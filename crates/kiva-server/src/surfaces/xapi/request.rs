//! HTTP-dependent xAPI request parsing and validation helpers.

mod attachments;
mod content;
mod json;
mod media;
mod multipart;
mod normalization;
mod openapi;
mod pagination;
mod person;
mod preconditions;
mod projection;
mod query;
mod signature_comparison;

pub(crate) use attachments::attachments_for_statement;
pub(crate) use content::{
    content_type_matches, document_content_from_request, document_request_content_type,
    statement_payload_from_request,
};
pub(crate) use media::matching_types as matching_media_types;
#[cfg(test)]
pub(crate) use normalization::normalized_statement;
pub(crate) use normalization::{normalize_statement_batch, normalized_statement_record};
pub(crate) use pagination::{StatementCursor, statement_page};
pub(crate) use person::person_from_agent;
pub(crate) use preconditions::{
    document_read_status, mixed_condition_response, undefined_condition_response,
};
pub(crate) use projection::*;
pub(crate) use query::*;

//! HTTP-dependent xAPI request parsing and validation helpers.

mod attachments;
mod content;
mod json;
mod multipart;
mod normalization;
mod openapi;
mod person;
mod projection;
mod query;

pub(crate) use attachments::attachments_for_statement;
pub(crate) use content::{
    content_type_matches, document_content_from_request, document_request_content_type,
    statement_payload_from_request,
};
pub(crate) use normalization::{normalize_statement_batch, normalized_statement};
pub(crate) use person::person_from_agent;
pub(crate) use projection::*;
pub(crate) use query::*;

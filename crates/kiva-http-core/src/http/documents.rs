#![allow(clippy::wildcard_imports)]

use super::*;

/// Evaluates HTTP document write/delete preconditions.
#[must_use]
pub fn document_preconditions_pass(
    existing_etag: Option<&str>,
    if_match: Option<&str>,
    if_none_match: Option<&str>,
) -> bool {
    if let Some(if_match) = if_match {
        let Some(existing_etag) = existing_etag else {
            return false;
        };
        if !etag_list_matches(if_match, existing_etag) {
            return false;
        }
    }
    if let (Some(if_none_match), Some(existing_etag)) = (if_none_match, existing_etag)
        && etag_list_matches(if_none_match, existing_etag)
    {
        return false;
    }
    true
}

/// Pure PUT decision for xAPI document resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentWriteDecision {
    /// Write or replace the document.
    Write,
    /// Existing document is byte-for-byte or JSON-equal and no precondition was supplied.
    Noop,
    /// Existing document differs and the request did not supply a precondition.
    Conflict,
    /// At least one HTTP precondition failed.
    PreconditionFailed,
}

/// Pure DELETE decision for xAPI document resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentDeleteDecision {
    /// Delete the existing document or matching document scope.
    Delete,
    /// No existing document matched the delete target.
    NotFound,
    /// At least one HTTP precondition failed.
    PreconditionFailed,
}

/// Decides whether a document PUT should write, no-op, conflict, or fail.
#[must_use]
pub const fn document_put_decision(
    preconditions_pass: bool,
    has_preconditions: bool,
    existing_content_matches: Option<bool>,
) -> DocumentWriteDecision {
    if !preconditions_pass {
        return DocumentWriteDecision::PreconditionFailed;
    }
    match (existing_content_matches, has_preconditions) {
        (Some(false), false) => DocumentWriteDecision::Conflict,
        (Some(true), false) => DocumentWriteDecision::Noop,
        (None | Some(_), true) | (None, false) => DocumentWriteDecision::Write,
    }
}

/// Decides whether a document DELETE should delete, miss, or fail.
#[must_use]
pub const fn document_delete_decision(
    preconditions_pass: bool,
    existing_content_present: bool,
) -> DocumentDeleteDecision {
    if !preconditions_pass {
        return DocumentDeleteDecision::PreconditionFailed;
    }
    if existing_content_present {
        DocumentDeleteDecision::Delete
    } else {
        DocumentDeleteDecision::NotFound
    }
}

/// Content shape relevant to JSON document merge decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentContentShape {
    /// JSON object content.
    JsonObject,
    /// JSON content that is not an object.
    JsonNonObject,
    /// Non-JSON document content.
    Bytes,
}

/// Pure merge decision for xAPI document `POST` JSON merge behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentJsonMergeDecision {
    /// Merge the patch object into the existing object, or into an empty object
    /// when no existing document is present.
    Merge,
    /// The request body cannot be used as a JSON object patch.
    InvalidPatch,
    /// The existing document cannot be used as a JSON object merge base.
    InvalidExisting,
}

/// Decides whether a document `POST` JSON merge is structurally valid.
#[must_use]
pub const fn document_json_merge_decision(
    existing_shape: Option<DocumentContentShape>,
    patch_shape: DocumentContentShape,
) -> DocumentJsonMergeDecision {
    if !matches!(patch_shape, DocumentContentShape::JsonObject) {
        return DocumentJsonMergeDecision::InvalidPatch;
    }
    match existing_shape {
        None | Some(DocumentContentShape::JsonObject) => DocumentJsonMergeDecision::Merge,
        Some(DocumentContentShape::JsonNonObject | DocumentContentShape::Bytes) => {
            DocumentJsonMergeDecision::InvalidExisting
        }
    }
}

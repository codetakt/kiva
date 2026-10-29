//! xAPI authority and response-format projection helpers.

use std::collections::BTreeMap;

use axum::http::{HeaderMap, StatusCode, header::ACCEPT_LANGUAGE};
use kiva_identity::AuthenticatedPrincipal;
use kiva_xapi::{
    accepted_language_ranges, authority_agent as xapi_authority_agent,
    canonicalize_statement_language_maps, statement_ids_format,
    statements_equivalent as xapi_statements_equivalent,
};
use serde_json::Value;

use crate::{core::response::HttpError, surfaces::xapi::model::StoredStatement};

pub(crate) fn statement_authority(
    principal: Option<&AuthenticatedPrincipal>,
) -> Result<Value, HttpError> {
    match principal {
        Some(principal) if !principal.can_write_statements() => Err(HttpError::new(
            StatusCode::FORBIDDEN,
            "credential is not allowed to write Statements",
        )),
        Some(principal) => Ok(authority_agent(&principal.subject)),
        None => Ok(authority_agent("kiva-lrs")),
    }
}

pub(crate) fn authority_agent(name: &str) -> Value {
    xapi_authority_agent(name)
}

pub(crate) fn formatted_single_statement(
    mut statement: StoredStatement,
    query: &BTreeMap<String, String>,
    headers: &HeaderMap,
) -> StoredStatement {
    statement.statement = formatted_statement(statement.statement, query, headers);
    statement
}

pub(crate) fn formatted_statement(
    statement: Value,
    query: &BTreeMap<String, String>,
    headers: &HeaderMap,
) -> Value {
    match query.get("format").map(String::as_str) {
        Some("canonical") => canonical_statement(statement, headers),
        Some("ids") => statement_ids_format(statement),
        _ => statement,
    }
}

pub(crate) fn canonical_statement(mut statement: Value, headers: &HeaderMap) -> Value {
    let languages = accepted_languages(headers);
    statement = canonicalize_statement_language_maps(statement, &languages);
    statement
}

pub(crate) fn accepted_languages(headers: &HeaderMap) -> Vec<kiva_xapi::LanguagePreference> {
    let combined = headers
        .get_all(ACCEPT_LANGUAGE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .collect::<Vec<_>>()
        .join(",");
    accepted_language_ranges(Some(&combined))
}

pub(crate) fn statements_equivalent(left: &Value, right: &Value) -> bool {
    xapi_statements_equivalent(left, right)
}

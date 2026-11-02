//! Statement selection and optional mailbox-hash query expansion.

use std::collections::BTreeMap;

use axum::http::StatusCode;
use kiva_xapi::filter_statements_with_ifi_aliases;
use serde_json::Value;

use crate::core::response::HttpError;

pub(crate) fn filtered_statements(
    statements: Vec<Value>,
    query: &BTreeMap<String, String>,
    limit: Option<usize>,
    mailbox_hash_queries: bool,
) -> Result<Vec<Value>, HttpError> {
    let mut aliases = Vec::new();
    if mailbox_hash_queries
        && let Some(agent) = query
            .get("agent")
            .and_then(|json| serde_json::from_str::<Value>(json).ok())
        && let Some(mailbox) = agent.get("mbox").and_then(Value::as_str)
    {
        let hash =
            ring::digest::digest(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY, mailbox.as_bytes());
        aliases.push(format!(
            "mbox_sha1sum:{}",
            crate::core::response::hex_lower(hash.as_ref())
        ));
    }
    filter_statements_with_ifi_aliases(statements, query, limit, &aliases)
        .map_err(|error| HttpError::new(StatusCode::BAD_REQUEST, error.message()))
}

//! Stable continuation positions for live Statement collections.

use std::{cmp::Ordering, collections::BTreeMap};

use axum::http::StatusCode;
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::core::response::{HttpError, rfc3339};

pub(crate) struct StatementCursor {
    stored_at: OffsetDateTime,
    statement_id: String,
}

impl StatementCursor {
    pub(crate) fn from_query(query: &mut BTreeMap<String, String>) -> Result<Self, HttpError> {
        let invalid = || {
            HttpError::new(
                StatusCode::BAD_REQUEST,
                "invalid Statement continuation cursor",
            )
        };
        let stored = query.remove("cursorStored").ok_or_else(invalid)?;
        let statement_id = query.remove("cursorId").ok_or_else(invalid)?;
        let stored_at = OffsetDateTime::parse(&stored, &Rfc3339).map_err(|_| invalid())?;
        super::parse_statement_id(&statement_id)?;
        Ok(Self {
            stored_at,
            statement_id,
        })
    }

    fn from_statement(statement: &Value) -> Option<Self> {
        Some(Self {
            stored_at: kiva_xapi::statement_stored_at(statement)?,
            statement_id: statement.get("id")?.as_str()?.to_owned(),
        })
    }

    fn precedes(&self, statement: &Value, ascending: bool) -> bool {
        let Some(next) = Self::from_statement(statement) else {
            return false;
        };
        let ordering =
            (next.stored_at, &next.statement_id).cmp(&(self.stored_at, &self.statement_id));
        ordering
            == if ascending {
                Ordering::Greater
            } else {
                Ordering::Less
            }
    }
}

pub(crate) fn statement_page(
    mut statements: Vec<Value>,
    query: &BTreeMap<String, String>,
    limit: Option<usize>,
    cursor: Option<&StatementCursor>,
) -> (Vec<Value>, String) {
    if let Some(cursor) = cursor {
        let ascending = query.get("ascending").map(String::as_str) == Some("true");
        statements.retain(|statement| cursor.precedes(statement, ascending));
    }
    let page_len = limit.unwrap_or(usize::MAX).min(statements.len());
    let has_more = page_len < statements.len();
    statements.truncate(page_len);
    let more = if has_more
        && let Some(last) = statements.last().and_then(StatementCursor::from_statement)
    {
        let mut next = url::form_urlencoded::Serializer::new(String::new());
        next.extend_pairs(query);
        next.append_pair("cursorStored", &rfc3339(last.stored_at));
        next.append_pair("cursorId", &last.statement_id);
        format!("/statements/more?{}", next.finish())
    } else {
        String::new()
    };
    (statements, more)
}

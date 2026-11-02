//! Stable continuation positions for live Statement collections.

use std::{cmp::Ordering, collections::BTreeMap};

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::core::response::HttpError;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct StatementCursor {
    stored_at: OffsetDateTime,
    statement_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PageCursor {
    query: BTreeMap<String, String>,
    position: StatementCursor,
    snapshot: Option<Vec<String>>,
    expires_at: i64,
}

fn invalid() -> HttpError {
    HttpError::new(
        StatusCode::NOT_FOUND,
        "Statement continuation was not found or has expired",
    )
}

impl PageCursor {
    pub(crate) async fn resolve(
        state: &crate::core::state::XapiHttpContext,
        query: &mut BTreeMap<String, String>,
    ) -> Result<Self, HttpError> {
        if let Some(token) = query.remove("cursor") {
            if !query.is_empty() || uuid::Uuid::parse_str(&token).is_err() {
                return Err(invalid());
            }
            let record = state
                .capabilities()
                .store
                .get(&format!("cursor:{token}"))
                .await?
                .ok_or_else(invalid)?;
            let cursor: Self = serde_json::from_slice(&record.body).map_err(|_| invalid())?;
            if cursor.expires_at <= OffsetDateTime::now_utc().unix_timestamp() {
                return Err(invalid());
            }
            *query = cursor.query.clone();
            Ok(cursor)
        } else {
            // Retain the meaning of previously issued live continuation links.
            let position = StatementCursor::from_query(query)?;
            Ok(Self {
                query: query.clone(),
                position,
                snapshot: None,
                expires_at: 0,
            })
        }
    }
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

pub(crate) async fn statement_page(
    state: &crate::core::state::XapiHttpContext,
    mut statements: Vec<Value>,
    query: &BTreeMap<String, String>,
    limit: Option<usize>,
    cursor: Option<&PageCursor>,
) -> Result<(Vec<Value>, String), HttpError> {
    if let Some(cursor) = cursor {
        let ascending = query.get("ascending").map(String::as_str) == Some("true");
        statements.retain(|statement| cursor.position.precedes(statement, ascending));
        if let Some(ids) = &cursor.snapshot {
            let ids: std::collections::BTreeSet<_> = ids.iter().map(String::as_str).collect();
            statements.retain(|s| s["id"].as_str().is_some_and(|id| ids.contains(id)));
        }
    }
    let page_len = limit.unwrap_or(usize::MAX).min(statements.len());
    let has_more = page_len < statements.len();
    let snapshot = if state.capabilities().config.pagination
        == crate::surfaces::xapi::capabilities::config::PaginationMode::Snapshot
        || cursor.is_some_and(|c| c.snapshot.is_some())
    {
        Some(
            statements
                .iter()
                .skip(page_len)
                .filter_map(|s| s["id"].as_str().map(str::to_owned))
                .collect(),
        )
    } else {
        None
    };
    statements.truncate(page_len);
    let more = if has_more
        && let Some(last) = statements.last().and_then(StatementCursor::from_statement)
    {
        let prefix = state
            .capabilities()
            .config
            .public_base_url
            .as_deref()
            .and_then(|base| url::Url::parse(base).ok())
            .map(|url| url.path().trim_end_matches('/').to_owned())
            .unwrap_or_default();
        if snapshot.is_none() && state.capabilities().config.self_contained_live_cursors {
            let mut next = url::form_urlencoded::Serializer::new(String::new());
            next.extend_pairs(query);
            next.append_pair(
                "cursorStored",
                &crate::core::response::rfc3339(last.stored_at),
            );
            next.append_pair("cursorId", &last.statement_id);
            let more = format!("{prefix}/statements/more?{}", next.finish());
            // A deployment may choose stateless continuations, but large filters
            // always fall back to a short token rather than an oversized URL.
            if more.len() <= 2048 {
                return Ok((statements, more));
            }
        }
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let next = PageCursor {
            query: query.clone(),
            position: last,
            snapshot,
            expires_at: now + i64::from(state.capabilities().config.cursor_ttl_seconds),
        };
        let body = serde_json::to_vec(&next).map_err(|_| {
            HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "continuation encoding failed",
            )
        })?;
        let token = uuid::Uuid::new_v4();
        let maximum = state.capabilities().config.max_cursors;
        let byte_limit = state.capabilities().config.cursor_storage_bytes;
        state
            .capabilities()
            .store
            .mutate("cursor:", move |records| {
                records.retain(|_, r| {
                    serde_json::from_slice::<PageCursor>(&r.body).is_ok_and(|c| c.expires_at > now)
                });
                let used_bytes = records
                    .values()
                    .try_fold(body.len(), |sum, record| sum.checked_add(record.body.len()));
                if records.len() >= maximum || used_bytes.is_none_or(|bytes| bytes > byte_limit) {
                    return Err(HttpError::new(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "Statement continuation capacity reached; retry later",
                    ));
                }
                records.insert(
                    format!("cursor:{token}"),
                    crate::surfaces::xapi::capabilities::store::Record {
                        body,
                        etag: token.to_string(),
                    },
                );
                Ok(())
            })
            .await?;
        format!("{prefix}/statements/more?cursor={token}")
    } else {
        String::new()
    };
    Ok((statements, more))
}

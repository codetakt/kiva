//! Optional xAPI budgets keyed by the authenticated subject, never a raw token.

use std::{collections::BTreeMap, io, sync::Arc};

use axum::http::StatusCode;
use kiva_identity::AuthenticatedPrincipal;
use serde::Deserialize;
use time::{Duration, OffsetDateTime};
use tokio::sync::Mutex;

use crate::core::{recording_clock::RecordingClock, response::HttpError};

#[derive(Debug, Clone, Copy)]
pub(crate) enum BodyKind {
    Statement,
    Document,
    Attachment,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Budget {
    statement_body_bytes: Option<usize>,
    document_body_bytes: Option<usize>,
    attachment_bytes: Option<usize>,
    rate: Option<Rate>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rate {
    accepted: usize,
    window_seconds: u32,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    #[serde(default)]
    default: Budget,
    #[serde(default)]
    subjects: BTreeMap<String, Budget>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct XapiLimits {
    configuration: Configuration,
    windows: Arc<Mutex<BTreeMap<Option<String>, Window>>>,
    clock: RecordingClock,
}

#[derive(Debug, Clone)]
struct Window {
    until: OffsetDateTime,
    accepted: usize,
}

impl XapiLimits {
    pub(crate) fn from_json(json: &str) -> Result<Self, io::Error> {
        let configuration: Configuration = serde_json::from_str(json).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("KIVA_XAPI_LIMITS_JSON: {e}"),
            )
        })?;
        if configuration.subjects.keys().any(String::is_empty)
            || std::iter::once(&configuration.default)
                .chain(configuration.subjects.values())
                .any(|b| {
                    [
                        b.statement_body_bytes,
                        b.document_body_bytes,
                        b.attachment_bytes,
                    ]
                    .contains(&Some(0))
                        || b.rate
                            .is_some_and(|r| r.accepted == 0 || r.window_seconds == 0)
                })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "xAPI budgets require positive limits and nonempty subject identifiers",
            ));
        }
        Ok(Self {
            configuration,
            ..Self::default()
        })
    }

    fn budget(&self, principal: Option<&AuthenticatedPrincipal>) -> Budget {
        let default = self.configuration.default;
        let Some(overrides) = principal.and_then(|p| self.configuration.subjects.get(&p.subject))
        else {
            return default;
        };
        Budget {
            statement_body_bytes: overrides
                .statement_body_bytes
                .or(default.statement_body_bytes),
            document_body_bytes: overrides
                .document_body_bytes
                .or(default.document_body_bytes),
            attachment_bytes: overrides.attachment_bytes.or(default.attachment_bytes),
            rate: overrides.rate.or(default.rate),
        }
    }

    pub(crate) fn body(
        &self,
        principal: Option<&AuthenticatedPrincipal>,
        kind: BodyKind,
        bytes: usize,
    ) -> Result<(), HttpError> {
        let budget = self.budget(principal);
        let (limit, label) = match kind {
            BodyKind::Statement => (budget.statement_body_bytes, "Statement HTTP body"),
            BodyKind::Document => (budget.document_body_bytes, "Document HTTP body"),
            BodyKind::Attachment => (budget.attachment_bytes, "raw attachment part"),
        };
        if limit.is_some_and(|max| bytes > max) {
            return Err(HttpError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("{label} byte limit exceeded"),
            ));
        }
        Ok(())
    }

    pub(crate) async fn admit(
        &self,
        principal: Option<&AuthenticatedPrincipal>,
    ) -> Result<(), HttpError> {
        let Some(rate) = self.budget(principal).rate else {
            return Ok(());
        };
        let now = self.clock.now();
        let key = principal.map(|p| p.subject.clone());
        let mut windows = self.windows.lock().await;
        windows.retain(|_, v| v.until > now);
        let window = windows.entry(key).or_insert_with(|| Window {
            until: now.saturating_add(Duration::seconds(i64::from(rate.window_seconds))),
            accepted: 0,
        });
        let denied = window.accepted >= rate.accepted;
        if !denied {
            window.accepted += 1;
        }
        drop(windows);
        if denied {
            return Err(HttpError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "authenticated subject request rate limit exceeded",
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) const fn clock(&self) -> &RecordingClock {
        &self.clock
    }
}

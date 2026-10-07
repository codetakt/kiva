use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Outcome recorded for a learner-initiated portability export attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum PortabilityExportAuditOutcome {
    /// The governed export page was produced.
    #[serde(rename = "completed")]
    Completed,
    /// The governed export request was rejected after subject authentication.
    #[serde(rename = "rejected")]
    Rejected,
}

/// Admitted audit event for one learner-initiated portability export attempt.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortabilityExportAuditRecorded {
    event_id: Uuid,
    data_subject: String,
    outcome: PortabilityExportAuditOutcome,
    statement_count: u32,
    occurred_at: OffsetDateTime,
}

impl PortabilityExportAuditRecorded {
    /// Creates a portability export audit event controlled by the data subject.
    #[must_use]
    pub const fn new(
        event_id: Uuid,
        data_subject: String,
        outcome: PortabilityExportAuditOutcome,
        statement_count: u32,
        occurred_at: OffsetDateTime,
    ) -> Self {
        Self {
            event_id,
            data_subject,
            outcome,
            statement_count,
            occurred_at,
        }
    }

    /// Returns the audit event identifier.
    #[must_use]
    pub const fn event_id(&self) -> Uuid {
        self.event_id
    }

    /// Returns the controlling data subject.
    #[must_use]
    pub fn data_subject(&self) -> &str {
        &self.data_subject
    }

    /// Returns the recorded outcome.
    #[must_use]
    pub const fn outcome(&self) -> PortabilityExportAuditOutcome {
        self.outcome
    }

    /// Returns the number of Statement bodies released by this page.
    #[must_use]
    pub const fn statement_count(&self) -> u32 {
        self.statement_count
    }

    /// Returns the event timestamp.
    #[must_use]
    pub const fn occurred_at(&self) -> OffsetDateTime {
        self.occurred_at
    }
}

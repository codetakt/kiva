#![allow(clippy::wildcard_imports)]

use super::*;

/// xAPI boundary errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum XapiError {
    /// The batch is empty.
    #[error("statement batch must not be empty")]
    EmptyBatch,

    /// The same Statement id appears more than once in the batch.
    #[error("duplicate statement id in batch: {0}")]
    DuplicateStatementId(StatementId),

    /// The JSON payload is structurally invalid for the selected boundary.
    #[error("invalid JSON payload")]
    InvalidJsonPayload,
}

/// xAPI Statement query filter parse errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StatementFilterError {
    /// The `agent` query parameter is not valid JSON.
    #[error("agent must be valid JSON")]
    InvalidAgentJson,

    /// The `agent` query parameter lacks an inverse functional identifier.
    #[error("agent must include an inverse functional identifier")]
    AgentMissingInverseFunctionalIdentifier,

    /// The `registration` query parameter is not a UUID.
    #[error("registration must be a UUID")]
    InvalidRegistrationUuid,

    /// The `since` query parameter is not an RFC3339 timestamp.
    #[error("since must be an RFC3339 timestamp")]
    InvalidSinceTimestamp,

    /// The `until` query parameter is not an RFC3339 timestamp.
    #[error("until must be an RFC3339 timestamp")]
    InvalidUntilTimestamp,
}

impl StatementFilterError {
    /// Returns the existing HTTP-facing message for this parse error.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidAgentJson => "agent must be valid JSON",
            Self::AgentMissingInverseFunctionalIdentifier => {
                "agent must include an inverse functional identifier"
            }
            Self::InvalidRegistrationUuid => "registration must be a UUID",
            Self::InvalidSinceTimestamp => "since must be an RFC3339 timestamp",
            Self::InvalidUntilTimestamp => "until must be an RFC3339 timestamp",
        }
    }
}

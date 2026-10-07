//! xAPI repository outcomes and storage errors.

use kiva_http_core::DocumentWriteDecision;
use uuid::Uuid;

impl From<DocumentWriteDecision> for DocumentWriteOutcome {
    fn from(value: DocumentWriteDecision) -> Self {
        match value {
            DocumentWriteDecision::Write | DocumentWriteDecision::Noop => Self::Stored,
            DocumentWriteDecision::Conflict => Self::Conflict,
            DocumentWriteDecision::PreconditionFailed => Self::PreconditionFailed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DocumentWriteOutcome {
    Stored,
    Conflict,
    PreconditionFailed,
    InvalidMerge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DocumentDeleteOutcome {
    Deleted,
    NotFound,
    PreconditionFailed,
}
#[derive(Debug)]
pub(crate) struct StatementStoreError {
    pub(crate) source: sqlx::Error,
}

impl From<sqlx::Error> for StatementStoreError {
    fn from(source: sqlx::Error) -> Self {
        Self { source }
    }
}

#[derive(Debug)]
pub(crate) struct DocumentStoreError {
    pub(crate) source: sqlx::Error,
}

impl From<sqlx::Error> for DocumentStoreError {
    fn from(source: sqlx::Error) -> Self {
        Self { source }
    }
}
#[derive(Debug)]
pub(crate) enum PutBatchError {
    Conflict(Uuid),
    Store(StatementStoreError),
}

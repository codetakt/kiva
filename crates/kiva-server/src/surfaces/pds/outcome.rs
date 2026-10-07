//! PDS repository outcomes and storage errors.

use kiva_application::runtime::ApplicationRuntimeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PdsRevocationOutcome {
    Revoked,
    AlreadyRevoked,
    NotFound,
    Forbidden,
}
#[derive(Debug)]
pub(crate) enum PdsRepositoryError {
    Runtime(String),
    Store { source: sqlx::Error },
    InvalidRequest(String),
    InvalidStoredData,
}

impl PdsRepositoryError {
    pub(crate) const fn store(source: sqlx::Error) -> Self {
        Self::Store { source }
    }
}

impl From<ApplicationRuntimeError> for PdsRepositoryError {
    fn from(error: ApplicationRuntimeError) -> Self {
        Self::Runtime(error.message().to_owned())
    }
}

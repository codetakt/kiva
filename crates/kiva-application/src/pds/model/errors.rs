//! PDS application error types.

pub use crate::runtime::ApplicationRuntimeError;

/// Backend-neutral result of an application persistence attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationStoreOutcome {
    /// The requested state was stored.
    Stored,
    /// The requested state already existed with identical data.
    Unchanged,
    /// The requested state conflicted with existing durable state.
    Conflict,
}

/// Backend-neutral result of an application revocation attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationRevocationOutcome {
    /// The requested consent grant was revoked.
    Revoked,
    /// The requested consent grant was already revoked.
    AlreadyRevoked,
    /// The requested consent grant was not found.
    NotFound,
    /// The consent grant is owned by a different subject.
    Forbidden,
}

/// Backend-neutral result of appending one admitted PDS domain event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationWriteOutcome {
    /// The event produced a store-style state mutation.
    Store(ApplicationStoreOutcome),
    /// The event produced a revocation-style state mutation.
    Revocation(ApplicationRevocationOutcome),
}

impl ApplicationWriteOutcome {
    /// Returns the store outcome expected by store-style command workflows.
    pub const fn into_store(self) -> Result<ApplicationStoreOutcome, PdsApplicationStoreError> {
        match self {
            Self::Store(outcome) => Ok(outcome),
            Self::Revocation(_outcome) => Err(PdsApplicationStoreError::InvalidStoredData),
        }
    }

    /// Returns the revocation outcome expected by revocation command workflows.
    pub const fn into_revocation(
        self,
    ) -> Result<ApplicationRevocationOutcome, PdsApplicationStoreError> {
        match self {
            Self::Revocation(outcome) => Ok(outcome),
            Self::Store(_outcome) => Err(PdsApplicationStoreError::InvalidStoredData),
        }
    }
}

/// Error returned by the PDS record repository port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdsApplicationStoreError {
    /// The admitted application request could not be represented as valid PDS state.
    InvalidRequest(String),
    /// The backing store failed.
    Store,
    /// Stored data could not be projected into application records.
    InvalidStoredData,
}

/// Error returned by the xAPI statement metadata reader port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementReadError {
    /// The backing statement store failed.
    Store,
    /// The supplied continuation cursor is malformed or outside the subject partition.
    InvalidCursor,
}

/// Error returned by the release-artifact availability port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseArtifactAvailabilityError {
    /// The backing release-artifact store failed.
    Store,
    /// Stored release-artifact metadata was invalid.
    InvalidStoredData,
}

/// Error returned while building a PDS read-model response.
#[derive(Debug)]
pub enum PdsReadModelError {
    /// Application runtime could not start the read-model workflow boundary.
    Runtime(ApplicationRuntimeError),
    /// xAPI statement metadata could not be read.
    Statements(StatementReadError),
    /// PDS metadata could not be read.
    Pds(PdsApplicationStoreError),
}

/// Error returned while registering an approved PDS data product.
#[derive(Debug)]
pub enum RegisterDataProductError {
    /// Application runtime could not start the workflow boundary.
    Runtime(ApplicationRuntimeError),
    /// The referenced confidential release artifact is not available for this subject.
    ArtifactUnavailable,
    /// Release-artifact availability could not be determined.
    ArtifactStore(ReleaseArtifactAvailabilityError),
    /// PDS command admission or persistence failed.
    Pds(PdsApplicationStoreError),
}

/// Error returned while admitting and persisting a PDS command workflow.
#[derive(Debug)]
pub enum PdsApplicationWriteError {
    /// Application runtime could not start the workflow boundary.
    Runtime(ApplicationRuntimeError),
    /// PDS command admission or persistence failed.
    Pds(PdsApplicationStoreError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdsApplicationCommandError {
    InvalidRequest(String),
}

impl From<PdsApplicationCommandError> for RegisterDataProductError {
    fn from(error: PdsApplicationCommandError) -> Self {
        match error {
            PdsApplicationCommandError::InvalidRequest(message) => {
                Self::Pds(PdsApplicationStoreError::InvalidRequest(message))
            }
        }
    }
}

impl From<PdsApplicationWriteError> for RegisterDataProductError {
    fn from(error: PdsApplicationWriteError) -> Self {
        match error {
            PdsApplicationWriteError::Runtime(error) => Self::Runtime(error),
            PdsApplicationWriteError::Pds(error) => Self::Pds(error),
        }
    }
}

impl From<ApplicationRuntimeError> for RegisterDataProductError {
    fn from(error: ApplicationRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<ApplicationRuntimeError> for PdsApplicationWriteError {
    fn from(error: ApplicationRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<ApplicationRuntimeError> for PdsReadModelError {
    fn from(error: ApplicationRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<PdsApplicationCommandError> for PdsApplicationWriteError {
    fn from(error: PdsApplicationCommandError) -> Self {
        match error {
            PdsApplicationCommandError::InvalidRequest(message) => {
                Self::Pds(PdsApplicationStoreError::InvalidRequest(message))
            }
        }
    }
}

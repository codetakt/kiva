//! Confidential-computation repository outcomes and storage errors.

use kiva_application::confidential::ConfidentialApplicationError;
use kiva_confidential::mechanism::{
    additive::SecureAggregateError,
    differential_privacy::{
        DifferentialPrivacyBudgetError, DifferentialPrivacyCompositionError,
        DifferentialPrivacyNoiseError, DifferentialPrivacyPolicyError,
        DifferentialPrivacyReleaseError,
    },
    job::{ConfidentialJobAdmissionError, ConfidentialJobTransitionError},
    threshold::ThresholdReleaseTransitionError,
    workflow::{ConfidentialParticipantTransitionError, WorkflowError},
};

use crate::surfaces::confidential::model::{ConfidentialJobState, ConfidentialParticipantState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfidentialJobRegistrationOutcome {
    Stored(ConfidentialJobState),
    Unchanged(ConfidentialJobState),
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfidentialJobTransitionOutcome {
    Updated(ConfidentialJobState),
    Unchanged(ConfidentialJobState),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfidentialParticipantRegistrationOutcome {
    Stored(ConfidentialParticipantState),
    Unchanged(ConfidentialParticipantState),
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfidentialParticipantTransitionOutcome {
    Updated(ConfidentialParticipantState),
    Unchanged(ConfidentialParticipantState),
}
#[derive(Debug)]
pub(crate) enum ConfidentialAggregateError {
    ApplicationAdmission(String),
    Secure(SecureAggregateError),
    Store(ConfidentialAggregateStoreError),
    InactiveParticipant,
}

#[derive(Debug)]
pub(crate) enum ConfidentialShareSubmissionError {
    ApplicationAdmission(String),
    Secure(SecureAggregateError),
    Store(ConfidentialAggregateStoreError),
    Forbidden,
    InactiveParticipant,
}

#[derive(Debug)]
pub(crate) enum ConfidentialThresholdReleaseError {
    ApplicationAdmission(String),
    Secure(SecureAggregateError),
    Store(ConfidentialAggregateStoreError),
    InvalidAuthoritySet,
    NotFound,
    Forbidden,
    MissingComputedAggregate,
    MissingApproval,
}

#[derive(Debug)]
pub(crate) enum ConfidentialShamirReconstructionError {
    ApplicationAdmission(String),
    Workflow(WorkflowError),
    Secure(SecureAggregateError),
    Store(ConfidentialAggregateStoreError),
    NotFound,
    Conflict,
}

#[derive(Debug)]
pub(crate) enum ConfidentialDifferentialPrivacyBudgetError {
    ApplicationAdmission(String),
    Budget(DifferentialPrivacyBudgetError),
    Composition(DifferentialPrivacyCompositionError),
    Store(ConfidentialAggregateStoreError),
    NotFound,
    Forbidden,
}

#[derive(Debug)]
pub(crate) enum ConfidentialDifferentialPrivacyReleaseError {
    ApplicationAdmission(String),
    Budget(DifferentialPrivacyBudgetError),
    Noise(DifferentialPrivacyNoiseError),
    Policy(DifferentialPrivacyPolicyError),
    Release(DifferentialPrivacyReleaseError),
    Secure(SecureAggregateError),
    Store(ConfidentialAggregateStoreError),
    NotFound,
    Forbidden,
    MissingComputedAggregate,
    DataSubjectCountMismatch,
    RandomnessUnavailable,
    Conflict,
}

#[derive(Debug)]
pub(crate) enum ConfidentialJobError {
    Admission(ConfidentialJobAdmissionError),
    Transition(ConfidentialJobTransitionError),
    Store(ConfidentialAggregateStoreError),
    InvalidRequest(String),
    NotFound,
    Forbidden,
    Conflict,
}

#[derive(Debug)]
pub(crate) enum ConfidentialParticipantError {
    ApplicationAdmission(String),
    Transition(ConfidentialParticipantTransitionError),
    Store(ConfidentialAggregateStoreError),
    InvalidRequest(String),
    NotFound,
    Forbidden,
    Conflict,
    Inactive,
}

impl From<ConfidentialAggregateStoreError> for ConfidentialThresholdReleaseError {
    fn from(source: ConfidentialAggregateStoreError) -> Self {
        Self::Store(source)
    }
}

impl From<ThresholdReleaseTransitionError> for ConfidentialThresholdReleaseError {
    fn from(source: ThresholdReleaseTransitionError) -> Self {
        match source {
            ThresholdReleaseTransitionError::InvalidAuthoritySet => Self::InvalidAuthoritySet,
            ThresholdReleaseTransitionError::Forbidden => Self::Forbidden,
            ThresholdReleaseTransitionError::MissingComputedAggregate => {
                Self::MissingComputedAggregate
            }
            ThresholdReleaseTransitionError::MissingApproval => Self::MissingApproval,
        }
    }
}

#[derive(Debug)]
pub(crate) enum ConfidentialAggregateStoreError {
    Store { source: sqlx::Error },
    InvalidStoredArtifactLength,
    InvalidStoredReplayRecord(SecureAggregateError),
    InvalidStoredSigningKey(SecureAggregateError),
    InvalidStoredShamirReconstruction,
    InvalidStoredDifferentialPrivacyBudget(DifferentialPrivacyBudgetError),
    InvalidStoredDifferentialPrivacyRelease,
    InvalidStoredConfidentialJob,
    InvalidStoredConfidentialParticipant,
    ParticipantIndexTooLarge,
}

impl From<ConfidentialApplicationError> for ConfidentialAggregateError {
    fn from(error: ConfidentialApplicationError) -> Self {
        Self::ApplicationAdmission(confidential_application_error_message(&error))
    }
}

impl From<ConfidentialApplicationError> for ConfidentialShareSubmissionError {
    fn from(error: ConfidentialApplicationError) -> Self {
        Self::ApplicationAdmission(confidential_application_error_message(&error))
    }
}

impl From<ConfidentialApplicationError> for ConfidentialThresholdReleaseError {
    fn from(error: ConfidentialApplicationError) -> Self {
        Self::ApplicationAdmission(confidential_application_error_message(&error))
    }
}

impl From<ConfidentialApplicationError> for ConfidentialShamirReconstructionError {
    fn from(error: ConfidentialApplicationError) -> Self {
        Self::ApplicationAdmission(confidential_application_error_message(&error))
    }
}

impl From<ConfidentialApplicationError> for ConfidentialDifferentialPrivacyBudgetError {
    fn from(error: ConfidentialApplicationError) -> Self {
        Self::ApplicationAdmission(confidential_application_error_message(&error))
    }
}

impl From<ConfidentialApplicationError> for ConfidentialDifferentialPrivacyReleaseError {
    fn from(error: ConfidentialApplicationError) -> Self {
        Self::ApplicationAdmission(confidential_application_error_message(&error))
    }
}

impl From<ConfidentialApplicationError> for ConfidentialParticipantError {
    fn from(error: ConfidentialApplicationError) -> Self {
        Self::ApplicationAdmission(confidential_application_error_message(&error))
    }
}

fn confidential_application_error_message(error: &ConfidentialApplicationError) -> String {
    format!(
        "confidential application admission failed: {}",
        error.message()
    )
}

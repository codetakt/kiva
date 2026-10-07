//! Confidential job, workflow, and release-policy boundary.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Durable confidential job lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfidentialJobStatus {
    /// Job metadata is registered but not admitted for execution.
    Draft,
    /// Job passed admission checks and is queued for execution.
    Admitted,
    /// Job is leased by a worker or scheduler.
    Running,
    /// Job computation completed and waits for a governed release.
    AwaitingRelease,
    /// Job output was released through the applicable policy gate.
    Released,
    /// Job failed without a released result.
    Failed,
    /// Job was cancelled by an authorized actor.
    Cancelled,
    /// Job deadline elapsed before completion or release.
    Expired,
    /// Job metadata was retained only for audit and is not executable.
    Retired,
}

impl ConfidentialJobStatus {
    /// Returns the stable API label for this status.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Admitted => "admitted",
            Self::Running => "running",
            Self::AwaitingRelease => "awaitingRelease",
            Self::Released => "released",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
            Self::Retired => "retired",
        }
    }

    /// Parses a stable API status label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(Self::Draft),
            "admitted" => Some(Self::Admitted),
            "running" => Some(Self::Running),
            "awaitingRelease" => Some(Self::AwaitingRelease),
            "released" => Some(Self::Released),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            "expired" => Some(Self::Expired),
            "retired" => Some(Self::Retired),
            _ => None,
        }
    }

    /// Returns whether the status accepts no further execution or release work.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Released | Self::Failed | Self::Cancelled | Self::Expired | Self::Retired
        )
    }
}

/// Confidential job lifecycle validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ConfidentialJobTransitionError {
    /// The requested state transition is outside Kiva's strict lifecycle graph.
    #[error("confidential job lifecycle transition is invalid")]
    InvalidTransition,
}

/// Confidential job admission validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ConfidentialJobAdmissionError {
    /// A confidential job requires at least two participants.
    #[error("confidential job admission requires at least two participants")]
    TooFewParticipants,

    /// A confidential job must bind to a policy.
    #[error("confidential job admission requires a policy binding")]
    MissingPolicyBinding,

    /// A confidential job must have a future deadline at admission time.
    #[error("confidential job deadline is not in the future")]
    DeadlineNotFuture,

    /// The admission quota for the job class or tenant is exhausted.
    #[error("confidential job admission quota is exhausted")]
    AdmissionQuotaExceeded,

    /// The scheduler cannot accept additional work.
    #[error("confidential job scheduler is applying backpressure")]
    SchedulerBackpressure,

    /// The job has exhausted its configured retry budget.
    #[error("confidential job retry budget is exhausted")]
    RetryBudgetExceeded,
}

/// Admission facts for one durable confidential job.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfidentialJobAdmissionFacts {
    participant_count_at_least_two: bool,
    policy_binding_present: bool,
    deadline_future: bool,
    admission_quota_available: bool,
    scheduler_capacity_available: bool,
    backpressure_clear: bool,
    retry_budget_available: bool,
}

impl ConfidentialJobAdmissionFacts {
    /// Creates admission facts gathered by the API/repository boundary.
    #[allow(clippy::fn_params_excessive_bools)]
    #[must_use]
    pub const fn new(
        participant_count_at_least_two: bool,
        policy_binding_present: bool,
        deadline_future: bool,
        admission_quota_available: bool,
        scheduler_capacity_available: bool,
        backpressure_clear: bool,
        retry_budget_available: bool,
    ) -> Self {
        Self {
            participant_count_at_least_two,
            policy_binding_present,
            deadline_future,
            admission_quota_available,
            scheduler_capacity_available,
            backpressure_clear,
            retry_budget_available,
        }
    }
}

/// Returns whether a confidential job status transition is admitted.
#[allow(clippy::match_same_arms)]
#[must_use]
pub const fn confidential_job_transition_is_allowed(
    current: ConfidentialJobStatus,
    requested: ConfidentialJobStatus,
) -> bool {
    use ConfidentialJobStatus as Status;
    match (current, requested) {
        (left, right) if left as u8 == right as u8 => true,
        (Status::Draft, Status::Admitted | Status::Cancelled) => true,
        (Status::Admitted, Status::Running | Status::Cancelled) => true,
        (Status::Running, Status::AwaitingRelease | Status::Failed | Status::Cancelled) => true,
        (Status::AwaitingRelease, Status::Released | Status::Failed | Status::Cancelled) => true,
        (
            Status::Draft | Status::Admitted | Status::Running | Status::AwaitingRelease,
            Status::Expired,
        ) => true,
        (
            Status::Released | Status::Failed | Status::Cancelled | Status::Expired,
            Status::Retired,
        ) => true,
        _ => false,
    }
}

/// Applies a strict confidential job lifecycle transition.
pub const fn transition_confidential_job_status(
    current: ConfidentialJobStatus,
    requested: ConfidentialJobStatus,
) -> Result<ConfidentialJobStatus, ConfidentialJobTransitionError> {
    if confidential_job_transition_is_allowed(current, requested) {
        Ok(requested)
    } else {
        Err(ConfidentialJobTransitionError::InvalidTransition)
    }
}

/// Validates the pure admission gate for a durable confidential job.
pub const fn validate_confidential_job_admission(
    facts: ConfidentialJobAdmissionFacts,
) -> Result<(), ConfidentialJobAdmissionError> {
    if !facts.participant_count_at_least_two {
        return Err(ConfidentialJobAdmissionError::TooFewParticipants);
    }
    if !facts.policy_binding_present {
        return Err(ConfidentialJobAdmissionError::MissingPolicyBinding);
    }
    if !facts.deadline_future {
        return Err(ConfidentialJobAdmissionError::DeadlineNotFuture);
    }
    if !facts.admission_quota_available {
        return Err(ConfidentialJobAdmissionError::AdmissionQuotaExceeded);
    }
    if !facts.scheduler_capacity_available || !facts.backpressure_clear {
        return Err(ConfidentialJobAdmissionError::SchedulerBackpressure);
    }
    if !facts.retry_budget_available {
        return Err(ConfidentialJobAdmissionError::RetryBudgetExceeded);
    }
    Ok(())
}

pub use super::core::{
    ConfidentialMechanismKind, ConfidentialMechanismPlan, ReleasePolicy, Technique,
    ThresholdReleaseApprovalSet, ThresholdReleaseAuthoritySide, ThresholdReleaseTransitionError,
    WorkflowError, WorkflowPlan, classify_threshold_release_authority,
    record_threshold_release_approval, validate_threshold_release_approval_threshold,
    validate_threshold_release_authority_set, validate_threshold_release_completion,
    validate_threshold_release_computed_aggregate,
};

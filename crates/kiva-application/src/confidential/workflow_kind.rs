//! Confidential workflow kind labels.

use crate::{
    operation_catalog,
    runtime::{ApplicationWorkflowKind, ApplicationWorkflowSpec, WorkflowKind},
};

use super::ConfidentialWorkflowFamily;

/// Stable confidential application workflow kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidentialWorkflowKind {
    /// Lists share signing keys.
    ListShareSigningKeys,
    /// Lists additive replay records.
    ListReplayRecords,
    /// Lists submitted additive shares.
    ListSubmittedShares,
    /// Registers a share signing key.
    RegisterShareSigningKey,
    /// Revokes a share signing key.
    RevokeShareSigningKey,
    /// Rotates a share signing key.
    RotateShareSigningKey,
    /// Submits an additive share.
    SubmitAdditiveShare,
    /// Releases a signed additive aggregate.
    ReleaseSignedAdditiveAggregate,
    /// Registers a differential-privacy budget.
    RegisterDifferentialPrivacyBudget,
    /// Spends a differential-privacy budget.
    SpendDifferentialPrivacyBudget,
    /// Lists differential-privacy budgets.
    ListDifferentialPrivacyBudgets,
    /// Lists differential-privacy composition summaries.
    ListDifferentialPrivacyCompositionSummaries,
    /// Releases a differential-privacy aggregate.
    ReleaseDifferentialPrivacyAggregate,
    /// Releases a sampled differential-privacy aggregate.
    ReleaseSampledDifferentialPrivacyAggregate,
    /// Lists differential-privacy releases.
    ListDifferentialPrivacyReleases,
    /// Registers a confidential job.
    RegisterJob,
    /// Transitions a confidential job.
    TransitionJob,
    /// Lists confidential jobs.
    ListJobs,
    /// Lists confidential job audit events.
    ListJobAuditEvents,
    /// Registers a confidential participant.
    RegisterParticipant,
    /// Transitions a confidential participant.
    TransitionParticipant,
    /// Lists confidential participants.
    ListParticipants,
    /// Lists confidential participant audit events.
    ListParticipantAuditEvents,
    /// Reconstructs a Shamir threshold secret.
    ReconstructShamirThresholdSecret,
    /// Lists Shamir threshold reconstructions.
    ListShamirThresholdReconstructions,
    /// Registers a threshold release request.
    RegisterThresholdReleaseRequest,
    /// Approves a threshold release.
    ApproveThresholdRelease,
    /// Releases a threshold aggregate.
    ReleaseThresholdAggregate,
    /// Lists threshold release requests.
    ListThresholdReleaseRequests,
}

impl ConfidentialWorkflowKind {
    /// Returns the workflow family used by admission evidence.
    #[must_use]
    pub const fn family(self) -> ConfidentialWorkflowFamily {
        operation_catalog::confidential_workflow_family(self)
    }

    /// Returns the stable workflow label used by evidence and logs.
    #[must_use]
    pub const fn label(self) -> &'static str {
        self.workflow_spec().label()
    }

    /// Returns the Rust workflow marker used by drift evidence.
    #[must_use]
    pub const fn rust_kind(self) -> &'static str {
        operation_catalog::confidential_workflow_rust_kind(self)
    }

    /// Returns the application workflow specification.
    #[must_use]
    pub const fn workflow_spec(self) -> ApplicationWorkflowSpec {
        operation_catalog::confidential_workflow_spec(self)
    }
}

impl WorkflowKind for ConfidentialWorkflowKind {
    fn application_kind(self) -> ApplicationWorkflowKind {
        ApplicationWorkflowKind::new(self.label())
    }

    fn workflow_spec(self) -> ApplicationWorkflowSpec {
        self.workflow_spec()
    }
}

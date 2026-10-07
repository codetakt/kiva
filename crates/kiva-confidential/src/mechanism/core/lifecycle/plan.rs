#![allow(clippy::wildcard_imports)]

use crate::mechanism::registry::descriptor_for_mechanism;

use super::super::*;

/// Release policy for a confidential computation workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ReleasePolicyUnchecked", into = "ReleasePolicyUnchecked")]
pub struct ReleasePolicy {
    pub(in crate::mechanism::core) participant_count: NonZeroUsize,
    pub(in crate::mechanism::core) release_threshold: NonZeroUsize,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct ReleasePolicyUnchecked {
    participant_count: NonZeroUsize,
    release_threshold: NonZeroUsize,
}

impl ReleasePolicy {
    /// Creates a release policy after structural validation.
    pub const fn new(
        participant_count: NonZeroUsize,
        release_threshold: NonZeroUsize,
    ) -> Result<Self, WorkflowError> {
        if participant_count.get() < 2 {
            return Err(WorkflowError::TooFewParticipants);
        }
        if release_threshold.get() > participant_count.get() {
            return Err(WorkflowError::ThresholdExceedsParticipants);
        }
        Ok(Self {
            participant_count,
            release_threshold,
        })
    }

    /// Returns the participant count.
    #[must_use]
    pub const fn participant_count(self) -> NonZeroUsize {
        self.participant_count
    }

    /// Returns the release threshold.
    #[must_use]
    pub const fn release_threshold(self) -> NonZeroUsize {
        self.release_threshold
    }
}

impl TryFrom<ReleasePolicyUnchecked> for ReleasePolicy {
    type Error = WorkflowError;

    fn try_from(value: ReleasePolicyUnchecked) -> Result<Self, Self::Error> {
        Self::new(value.participant_count, value.release_threshold)
    }
}

impl From<ReleasePolicy> for ReleasePolicyUnchecked {
    fn from(value: ReleasePolicy) -> Self {
        Self {
            participant_count: value.participant_count(),
            release_threshold: value.release_threshold(),
        }
    }
}

/// Validated confidential mechanism plan for one computation transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "ConfidentialMechanismPlanUnchecked",
    into = "ConfidentialMechanismPlanUnchecked"
)]
pub struct ConfidentialMechanismPlan {
    mechanism: ConfidentialMechanismKind,
    release_policy: ReleasePolicy,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct ConfidentialMechanismPlanUnchecked {
    mechanism: ConfidentialMechanismKind,
    release_policy: ReleasePolicy,
}

impl ConfidentialMechanismPlan {
    /// Creates a mechanism plan after enforcing mechanism-specific threshold rules.
    #[allow(clippy::match_same_arms)]
    pub fn new(
        mechanism: ConfidentialMechanismKind,
        release_policy: ReleasePolicy,
    ) -> Result<Self, SecureAggregateError> {
        match mechanism {
            ConfidentialMechanismKind::AdditiveNOfN => {
                validate_additive_release_policy(release_policy)?;
            }
            ConfidentialMechanismKind::PrivateSetIntersection => {
                validate_psi_release_policy(release_policy)?;
            }
            ConfidentialMechanismKind::PrivateSetIntersectionOprfDh => {
                validate_psi_release_policy(release_policy)?;
            }
            ConfidentialMechanismKind::ShamirThreshold => {
                validate_shamir_release_policy(release_policy)?;
            }
            ConfidentialMechanismKind::ThresholdFrostSigning => {
                validate_shamir_release_policy(release_policy)?;
            }
            ConfidentialMechanismKind::TrustedExecutionEnvironment => {
                validate_tee_release_policy(release_policy)?;
            }
            ConfidentialMechanismKind::TrustedExecutionEnvironmentAwsNitro => {
                validate_tee_release_policy(release_policy)?;
            }
            ConfidentialMechanismKind::FheSumFeasibility => {
                validate_fhe_release_policy(release_policy)?;
            }
            ConfidentialMechanismKind::FheSumRuntime => {
                validate_fhe_release_policy(release_policy)?;
            }
        }
        Ok(Self {
            mechanism,
            release_policy,
        })
    }

    /// Returns the selected concrete mechanism.
    #[must_use]
    pub const fn mechanism(self) -> ConfidentialMechanismKind {
        self.mechanism
    }

    /// Returns the validated release policy.
    #[must_use]
    pub const fn release_policy(self) -> ReleasePolicy {
        self.release_policy
    }

    /// Returns the broader technique family represented by this mechanism.
    #[must_use]
    pub const fn technique(self) -> Technique {
        descriptor_for_mechanism(self.mechanism).technique()
    }

    /// Returns whether this mechanism requires every participant to contribute.
    #[must_use]
    pub const fn requires_all_participants(self) -> bool {
        self.release_policy.release_threshold().get()
            == self.release_policy.participant_count().get()
    }

    /// Returns whether fewer than all participants can reconstruct the value.
    #[must_use]
    pub const fn supports_partial_reconstruction(self) -> bool {
        self.release_policy.release_threshold().get()
            < self.release_policy.participant_count().get()
    }
}

impl TryFrom<ConfidentialMechanismPlanUnchecked> for ConfidentialMechanismPlan {
    type Error = SecureAggregateError;

    fn try_from(value: ConfidentialMechanismPlanUnchecked) -> Result<Self, Self::Error> {
        Self::new(value.mechanism, value.release_policy)
    }
}

impl From<ConfidentialMechanismPlan> for ConfidentialMechanismPlanUnchecked {
    fn from(value: ConfidentialMechanismPlan) -> Self {
        Self {
            mechanism: value.mechanism(),
            release_policy: value.release_policy(),
        }
    }
}

#[allow(clippy::redundant_pub_crate)]
pub(crate) const fn validate_additive_release_policy(
    release_policy: ReleasePolicy,
) -> Result<(), SecureAggregateError> {
    if release_policy.release_threshold().get() != release_policy.participant_count().get() {
        return Err(SecureAggregateError::ThresholdMustRequireAllParticipants);
    }
    Ok(())
}

#[allow(clippy::redundant_pub_crate)]
pub(crate) const fn validate_psi_release_policy(
    release_policy: ReleasePolicy,
) -> Result<(), SecureAggregateError> {
    if release_policy.release_threshold().get() != release_policy.participant_count().get() {
        return Err(SecureAggregateError::PsiRequiresAllParticipants);
    }
    Ok(())
}

#[allow(clippy::redundant_pub_crate)]
pub(crate) const fn validate_tee_release_policy(
    release_policy: ReleasePolicy,
) -> Result<(), SecureAggregateError> {
    if release_policy.release_threshold().get() != release_policy.participant_count().get() {
        return Err(SecureAggregateError::TeeRequiresAllParticipants);
    }
    Ok(())
}

#[allow(clippy::redundant_pub_crate)]
pub(crate) const fn validate_fhe_release_policy(
    release_policy: ReleasePolicy,
) -> Result<(), SecureAggregateError> {
    if release_policy.release_threshold().get() != release_policy.participant_count().get() {
        return Err(SecureAggregateError::FheRequiresAllParticipants);
    }
    Ok(())
}

#[allow(clippy::redundant_pub_crate)]
pub(crate) fn validate_shamir_release_policy(
    release_policy: ReleasePolicy,
) -> Result<(), SecureAggregateError> {
    if release_policy.release_threshold().get() < 2 {
        return Err(SecureAggregateError::ShamirThresholdTooSmall);
    }
    let Ok(participant_count) = u64::try_from(release_policy.participant_count().get()) else {
        return Err(SecureAggregateError::ShamirParticipantCountOutsideField);
    };
    if participant_count >= SECURE_AGGREGATE_FIELD_MODULUS {
        return Err(SecureAggregateError::ShamirParticipantCountOutsideField);
    }
    Ok(())
}

/// Planned confidential computation workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowPlan {
    technique: Technique,
    release_policy: ReleasePolicy,
}

impl WorkflowPlan {
    /// Creates a workflow plan.
    #[must_use]
    pub const fn new(technique: Technique, release_policy: ReleasePolicy) -> Self {
        Self {
            technique,
            release_policy,
        }
    }

    /// Returns the selected technique family.
    #[must_use]
    pub const fn technique(self) -> Technique {
        self.technique
    }

    /// Returns the release policy.
    #[must_use]
    pub const fn release_policy(self) -> ReleasePolicy {
        self.release_policy
    }
}

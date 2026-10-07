//! Differential privacy release-gate boundary.

use serde::{Deserialize, Serialize};

use super::{errors::DifferentialPrivacyReleaseError, policy::DifferentialPrivacyReleasePolicy};

/// Runtime facts required before releasing one DP aggregate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifferentialPrivacyReleaseContext {
    aggregate_computed: bool,
    data_subject_count: usize,
    epsilon_microunits: u64,
    noise_applied: bool,
}

impl DifferentialPrivacyReleaseContext {
    /// Creates a DP release context from already-normalized runtime facts.
    #[must_use]
    pub const fn new(
        aggregate_computed: bool,
        data_subject_count: usize,
        epsilon_microunits: u64,
        noise_applied: bool,
    ) -> Self {
        Self {
            aggregate_computed,
            data_subject_count,
            epsilon_microunits,
            noise_applied,
        }
    }

    /// Returns whether the release is bound to a computed aggregate.
    #[must_use]
    pub const fn aggregate_computed(self) -> bool {
        self.aggregate_computed
    }

    /// Returns the number of data subjects in the aggregate.
    #[must_use]
    pub const fn data_subject_count(self) -> usize {
        self.data_subject_count
    }

    /// Returns the epsilon budget in integer microunits.
    #[must_use]
    pub const fn epsilon_microunits(self) -> u64 {
        self.epsilon_microunits
    }

    /// Returns whether a noise mechanism was applied before release.
    #[must_use]
    pub const fn noise_applied(self) -> bool {
        self.noise_applied
    }
}

/// Validates the fail-closed release gate for a DP aggregate.
pub const fn validate_differential_privacy_release(
    policy: DifferentialPrivacyReleasePolicy,
    context: DifferentialPrivacyReleaseContext,
) -> Result<(), DifferentialPrivacyReleaseError> {
    if !context.aggregate_computed() {
        return Err(DifferentialPrivacyReleaseError::MissingComputedAggregate);
    }
    if context.data_subject_count() < policy.minimum_population().get() {
        return Err(DifferentialPrivacyReleaseError::PopulationBelowMinimum);
    }
    if context.epsilon_microunits() == 0 {
        return Err(DifferentialPrivacyReleaseError::EpsilonBudgetMustBeNonZero);
    }
    if context.epsilon_microunits() > policy.max_epsilon_microunits().get() {
        return Err(DifferentialPrivacyReleaseError::EpsilonBudgetExceeded);
    }
    if !context.noise_applied() {
        return Err(DifferentialPrivacyReleaseError::NoiseNotApplied);
    }
    Ok(())
}

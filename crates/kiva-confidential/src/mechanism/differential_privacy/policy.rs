//! Differential privacy policy and lifecycle state.

use std::num::{NonZeroU64, NonZeroUsize};

use serde::{Deserialize, Serialize};

use super::errors::{
    DifferentialPrivacyBudgetPolicyLifecycleError, DifferentialPrivacyPolicyError,
};

/// Differential privacy release policy for aggregate outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "DifferentialPrivacyReleasePolicyUnchecked",
    into = "DifferentialPrivacyReleasePolicyUnchecked"
)]
pub struct DifferentialPrivacyReleasePolicy {
    pub(crate) minimum_population: NonZeroUsize,
    pub(crate) max_epsilon_microunits: NonZeroU64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct DifferentialPrivacyReleasePolicyUnchecked {
    pub(crate) minimum_population: NonZeroUsize,
    pub(crate) max_epsilon_microunits: NonZeroU64,
}

impl DifferentialPrivacyReleasePolicy {
    /// Creates a DP release policy after rejecting single-subject publication.
    pub const fn new(
        minimum_population: NonZeroUsize,
        max_epsilon_microunits: NonZeroU64,
    ) -> Result<Self, DifferentialPrivacyPolicyError> {
        if minimum_population.get() < 2 {
            return Err(DifferentialPrivacyPolicyError::MinimumPopulationTooSmall);
        }
        Ok(Self {
            minimum_population,
            max_epsilon_microunits,
        })
    }

    /// Returns the minimum population required before aggregate release.
    #[must_use]
    pub const fn minimum_population(self) -> NonZeroUsize {
        self.minimum_population
    }

    /// Returns the maximum epsilon budget in integer microunits.
    #[must_use]
    pub const fn max_epsilon_microunits(self) -> NonZeroU64 {
        self.max_epsilon_microunits
    }
}

impl TryFrom<DifferentialPrivacyReleasePolicyUnchecked> for DifferentialPrivacyReleasePolicy {
    type Error = DifferentialPrivacyPolicyError;

    fn try_from(value: DifferentialPrivacyReleasePolicyUnchecked) -> Result<Self, Self::Error> {
        Self::new(value.minimum_population, value.max_epsilon_microunits)
    }
}

impl From<DifferentialPrivacyReleasePolicy> for DifferentialPrivacyReleasePolicyUnchecked {
    fn from(value: DifferentialPrivacyReleasePolicy) -> Self {
        Self {
            minimum_population: value.minimum_population(),
            max_epsilon_microunits: value.max_epsilon_microunits(),
        }
    }
}

/// Lifecycle state for one differential privacy budget policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DifferentialPrivacyBudgetPolicyStatus {
    /// Policy is being drafted and must not admit production releases.
    Draft,
    /// Policy is active and can admit releases.
    Active,
    /// Policy has been superseded by a newer active policy.
    Superseded,
    /// Policy has been retired and cannot be reactivated.
    Retired,
}

/// Validates a DP budget-policy lifecycle transition.
pub const fn transition_differential_privacy_budget_policy(
    current: DifferentialPrivacyBudgetPolicyStatus,
    requested: DifferentialPrivacyBudgetPolicyStatus,
) -> Result<DifferentialPrivacyBudgetPolicyStatus, DifferentialPrivacyBudgetPolicyLifecycleError> {
    let allowed = match current {
        DifferentialPrivacyBudgetPolicyStatus::Draft => matches!(
            requested,
            DifferentialPrivacyBudgetPolicyStatus::Draft
                | DifferentialPrivacyBudgetPolicyStatus::Active
                | DifferentialPrivacyBudgetPolicyStatus::Retired
        ),
        DifferentialPrivacyBudgetPolicyStatus::Active => matches!(
            requested,
            DifferentialPrivacyBudgetPolicyStatus::Active
                | DifferentialPrivacyBudgetPolicyStatus::Superseded
                | DifferentialPrivacyBudgetPolicyStatus::Retired
        ),
        DifferentialPrivacyBudgetPolicyStatus::Superseded => matches!(
            requested,
            DifferentialPrivacyBudgetPolicyStatus::Superseded
                | DifferentialPrivacyBudgetPolicyStatus::Retired
        ),
        DifferentialPrivacyBudgetPolicyStatus::Retired => {
            matches!(requested, DifferentialPrivacyBudgetPolicyStatus::Retired)
        }
    };
    if allowed {
        Ok(requested)
    } else {
        Err(DifferentialPrivacyBudgetPolicyLifecycleError::InvalidTransition)
    }
}

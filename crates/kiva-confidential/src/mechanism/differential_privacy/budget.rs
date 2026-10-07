//! Differential privacy budget ledger and composition accounting.

use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};

use super::{
    DIFFERENTIAL_PRIVACY_MICROUNITS_PER_UNIT,
    errors::{DifferentialPrivacyBudgetError, DifferentialPrivacyCompositionError},
};

/// Differential privacy epsilon-budget ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "DifferentialPrivacyBudgetLedgerUnchecked",
    into = "DifferentialPrivacyBudgetLedgerUnchecked"
)]
pub struct DifferentialPrivacyBudgetLedger {
    pub(crate) total_epsilon_microunits: NonZeroU64,
    pub(crate) spent_epsilon_microunits: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct DifferentialPrivacyBudgetLedgerUnchecked {
    pub(crate) total_epsilon_microunits: NonZeroU64,
    pub(crate) spent_epsilon_microunits: u64,
}

impl DifferentialPrivacyBudgetLedger {
    /// Creates a DP budget ledger after rejecting over-spent persisted state.
    pub const fn new(
        total_epsilon_microunits: NonZeroU64,
        spent_epsilon_microunits: u64,
    ) -> Result<Self, DifferentialPrivacyBudgetError> {
        if spent_epsilon_microunits > total_epsilon_microunits.get() {
            return Err(DifferentialPrivacyBudgetError::SpentBudgetExceedsTotal);
        }
        Ok(Self {
            total_epsilon_microunits,
            spent_epsilon_microunits,
        })
    }

    /// Returns the total epsilon budget in integer microunits.
    #[must_use]
    pub const fn total_epsilon_microunits(self) -> NonZeroU64 {
        self.total_epsilon_microunits
    }

    /// Returns the spent epsilon budget in integer microunits.
    #[must_use]
    pub const fn spent_epsilon_microunits(self) -> u64 {
        self.spent_epsilon_microunits
    }

    /// Returns the remaining epsilon budget in integer microunits.
    #[must_use]
    pub const fn remaining_epsilon_microunits(self) -> u64 {
        self.total_epsilon_microunits.get() - self.spent_epsilon_microunits
    }
}

impl TryFrom<DifferentialPrivacyBudgetLedgerUnchecked> for DifferentialPrivacyBudgetLedger {
    type Error = DifferentialPrivacyBudgetError;

    fn try_from(value: DifferentialPrivacyBudgetLedgerUnchecked) -> Result<Self, Self::Error> {
        Self::new(
            value.total_epsilon_microunits,
            value.spent_epsilon_microunits,
        )
    }
}

impl From<DifferentialPrivacyBudgetLedger> for DifferentialPrivacyBudgetLedgerUnchecked {
    fn from(value: DifferentialPrivacyBudgetLedger) -> Self {
        Self {
            total_epsilon_microunits: value.total_epsilon_microunits(),
            spent_epsilon_microunits: value.spent_epsilon_microunits(),
        }
    }
}

/// Applies one DP epsilon spend request to a valid ledger.
pub const fn spend_differential_privacy_budget(
    ledger: DifferentialPrivacyBudgetLedger,
    requested_epsilon_microunits: u64,
) -> Result<DifferentialPrivacyBudgetLedger, DifferentialPrivacyBudgetError> {
    if requested_epsilon_microunits == 0 {
        return Err(DifferentialPrivacyBudgetError::SpendRequestMustBeNonZero);
    }
    if requested_epsilon_microunits > ledger.remaining_epsilon_microunits() {
        return Err(DifferentialPrivacyBudgetError::SpendRequestExceedsRemaining);
    }
    Ok(DifferentialPrivacyBudgetLedger {
        total_epsilon_microunits: ledger.total_epsilon_microunits(),
        spent_epsilon_microunits: ledger.spent_epsilon_microunits() + requested_epsilon_microunits,
    })
}

/// Sequential-composition summary for releases charged to one budget ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifferentialPrivacyCompositionSummary {
    ledger: DifferentialPrivacyBudgetLedger,
    release_count: usize,
    release_spent_epsilon_microunits: u64,
    release_epsilon_square_sum_microunits2: u128,
    unattributed_spent_epsilon_microunits: u64,
    max_release_epsilon_microunits: u64,
}

impl DifferentialPrivacyCompositionSummary {
    /// Returns the source budget ledger.
    #[must_use]
    pub const fn ledger(self) -> DifferentialPrivacyBudgetLedger {
        self.ledger
    }

    /// Returns the number of releases included in the sequential composition.
    #[must_use]
    pub const fn release_count(self) -> usize {
        self.release_count
    }

    /// Returns epsilon spent by persisted releases in integer microunits.
    #[must_use]
    pub const fn release_spent_epsilon_microunits(self) -> u64 {
        self.release_spent_epsilon_microunits
    }

    /// Returns the sum of squared release epsilons in integer microunits squared.
    #[must_use]
    pub const fn release_epsilon_square_sum_microunits2(self) -> u128 {
        self.release_epsilon_square_sum_microunits2
    }

    /// Returns budget spend not attributable to persisted releases.
    #[must_use]
    pub const fn unattributed_spent_epsilon_microunits(self) -> u64 {
        self.unattributed_spent_epsilon_microunits
    }

    /// Returns the largest epsilon charged by one release.
    #[must_use]
    pub const fn max_release_epsilon_microunits(self) -> u64 {
        self.max_release_epsilon_microunits
    }
}

/// Advanced composition upper bound for pure DP releases charged to one budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifferentialPrivacyAdvancedCompositionBound {
    release_count: usize,
    delta_microunits: NonZeroU64,
    basic_epsilon_microunits: u64,
    epsilon_upper_bound_microunits: u64,
}

impl DifferentialPrivacyAdvancedCompositionBound {
    /// Returns the number of releases included in the bound.
    #[must_use]
    pub const fn release_count(self) -> usize {
        self.release_count
    }

    /// Returns the advanced-composition slack delta in integer microunits.
    #[must_use]
    pub const fn delta_microunits(self) -> NonZeroU64 {
        self.delta_microunits
    }

    /// Returns the basic sequential epsilon sum in integer microunits.
    #[must_use]
    pub const fn basic_epsilon_microunits(self) -> u64 {
        self.basic_epsilon_microunits
    }

    /// Returns the advanced-composition epsilon upper bound in integer microunits.
    #[must_use]
    pub const fn epsilon_upper_bound_microunits(self) -> u64 {
        self.epsilon_upper_bound_microunits
    }
}

/// Summarizes basic sequential composition for one subject-owned budget.
///
/// This is an accounting invariant, not a proof of distributional differential
/// privacy. It records the pure epsilon sum charged by persisted releases and
/// requires that sum to be covered by the persisted budget ledger.
pub fn summarize_differential_privacy_composition(
    ledger: DifferentialPrivacyBudgetLedger,
    release_epsilon_microunits: &[u64],
) -> Result<DifferentialPrivacyCompositionSummary, DifferentialPrivacyCompositionError> {
    let mut release_spent_epsilon_microunits = 0_u64;
    let mut release_epsilon_square_sum_microunits2 = 0_u128;
    let mut max_release_epsilon_microunits = 0_u64;
    for epsilon_microunits in release_epsilon_microunits {
        release_spent_epsilon_microunits = release_spent_epsilon_microunits
            .checked_add(*epsilon_microunits)
            .ok_or(DifferentialPrivacyCompositionError::ReleaseSpendOverflow)?;
        let epsilon_square = u128::from(*epsilon_microunits) * u128::from(*epsilon_microunits);
        release_epsilon_square_sum_microunits2 = release_epsilon_square_sum_microunits2
            .checked_add(epsilon_square)
            .ok_or(DifferentialPrivacyCompositionError::ReleaseSquareSpendOverflow)?;
        max_release_epsilon_microunits = max_release_epsilon_microunits.max(*epsilon_microunits);
    }
    if release_spent_epsilon_microunits > ledger.spent_epsilon_microunits() {
        return Err(DifferentialPrivacyCompositionError::ReleaseSpendExceedsBudgetSpend);
    }
    Ok(DifferentialPrivacyCompositionSummary {
        ledger,
        release_count: release_epsilon_microunits.len(),
        release_spent_epsilon_microunits,
        release_epsilon_square_sum_microunits2,
        unattributed_spent_epsilon_microunits: ledger.spent_epsilon_microunits()
            - release_spent_epsilon_microunits,
        max_release_epsilon_microunits,
    })
}

/// Computes a heterogeneous advanced-composition bound for pure DP releases.
///
/// The bound is:
/// `sum(eps_i * (exp(eps_i) - 1)) + sqrt(2 * ln(1 / delta') * sum(eps_i^2))`.
/// It is reported as an integer microunit upper bound and is often larger than
/// the basic sum for small release counts. The basic sum remains the enforced
/// budget ledger boundary.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub fn bound_differential_privacy_advanced_composition(
    release_epsilon_microunits: &[u64],
    delta_microunits: NonZeroU64,
) -> Result<DifferentialPrivacyAdvancedCompositionBound, DifferentialPrivacyCompositionError> {
    if delta_microunits.get() >= DIFFERENTIAL_PRIVACY_MICROUNITS_PER_UNIT {
        return Err(DifferentialPrivacyCompositionError::AdvancedCompositionDeltaOutOfRange);
    }

    let mut basic_epsilon_microunits = 0_u64;
    let mut linear_term = 0.0_f64;
    let mut square_sum = 0.0_f64;
    for epsilon_microunits in release_epsilon_microunits {
        basic_epsilon_microunits = basic_epsilon_microunits
            .checked_add(*epsilon_microunits)
            .ok_or(DifferentialPrivacyCompositionError::ReleaseSpendOverflow)?;
        let epsilon = *epsilon_microunits as f64 / DIFFERENTIAL_PRIVACY_MICROUNITS_PER_UNIT as f64;
        linear_term += epsilon * epsilon.exp_m1();
        square_sum += epsilon * epsilon;
    }

    let delta = delta_microunits.get() as f64 / DIFFERENTIAL_PRIVACY_MICROUNITS_PER_UNIT as f64;
    let advanced_epsilon = if release_epsilon_microunits.is_empty() {
        0.0
    } else {
        linear_term + (2.0 * (1.0 / delta).ln() * square_sum).sqrt()
    };
    let upper_bound = advanced_epsilon * DIFFERENTIAL_PRIVACY_MICROUNITS_PER_UNIT as f64;
    if !upper_bound.is_finite() || upper_bound < 0.0 || upper_bound > u64::MAX as f64 {
        return Err(DifferentialPrivacyCompositionError::AdvancedCompositionOverflow);
    }

    Ok(DifferentialPrivacyAdvancedCompositionBound {
        release_count: release_epsilon_microunits.len(),
        delta_microunits,
        basic_epsilon_microunits,
        epsilon_upper_bound_microunits: upper_bound.ceil() as u64,
    })
}

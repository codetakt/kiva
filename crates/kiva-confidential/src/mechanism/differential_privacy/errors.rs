//! Differential privacy validation errors.

use thiserror::Error;

/// Differential privacy release-policy validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DifferentialPrivacyPolicyError {
    /// Public aggregate releases must not be configured for a single subject.
    #[error("differential privacy minimum population must be at least two")]
    MinimumPopulationTooSmall,
}

/// Differential privacy privacy-budget validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DifferentialPrivacyBudgetError {
    /// Persisted budget state must not spend more than the configured total.
    #[error("differential privacy spent epsilon budget exceeds the total budget")]
    SpentBudgetExceedsTotal,

    /// A budget spend request must consume a non-zero epsilon amount.
    #[error("differential privacy epsilon spend request must be non-zero")]
    SpendRequestMustBeNonZero,

    /// A budget spend request exceeds the remaining epsilon budget.
    #[error("differential privacy epsilon spend request exceeds the remaining budget")]
    SpendRequestExceedsRemaining,
}

/// Differential privacy budget-policy lifecycle validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DifferentialPrivacyBudgetPolicyLifecycleError {
    /// Budget policies can move only through the configured lifecycle.
    #[error("differential privacy budget policy lifecycle transition is invalid")]
    InvalidTransition,
}

/// Differential privacy composition-accounting validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DifferentialPrivacyCompositionError {
    /// The release-derived epsilon sum overflowed unsigned integer bounds.
    #[error("differential privacy release epsilon composition overflowed")]
    ReleaseSpendOverflow,

    /// Release-derived epsilon cannot exceed the persisted budget ledger spend.
    #[error("differential privacy release epsilon composition exceeds the budget ledger spend")]
    ReleaseSpendExceedsBudgetSpend,

    /// Release-derived squared epsilon accounting overflowed unsigned integer bounds.
    #[error("differential privacy release squared epsilon composition overflowed")]
    ReleaseSquareSpendOverflow,

    /// Advanced composition delta must be greater than zero and less than one.
    #[error("differential privacy advanced composition delta is outside (0, 1)")]
    AdvancedCompositionDeltaOutOfRange,

    /// Advanced composition epsilon bound exceeded representable integer microunits.
    #[error("differential privacy advanced composition epsilon bound overflowed")]
    AdvancedCompositionOverflow,
}

/// Differential privacy release-gate validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DifferentialPrivacyReleaseError {
    /// A DP release must be bound to a persisted computed aggregate.
    #[error("differential privacy release requires a computed aggregate")]
    MissingComputedAggregate,

    /// The aggregate population is below the configured release minimum.
    #[error("differential privacy release population is below the policy minimum")]
    PopulationBelowMinimum,

    /// The release epsilon budget must be present and non-zero.
    #[error("differential privacy epsilon budget must be non-zero")]
    EpsilonBudgetMustBeNonZero,

    /// The release epsilon budget exceeds the configured policy maximum.
    #[error("differential privacy epsilon budget exceeds the policy maximum")]
    EpsilonBudgetExceeded,

    /// A DP release requires an explicitly applied noise mechanism.
    #[error("differential privacy release requires applied noise")]
    NoiseNotApplied,
}

/// Differential privacy noise-mechanism validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DifferentialPrivacyNoiseError {
    /// Adding the sampled noise to the aggregate exceeded signed integer bounds.
    #[error("differential privacy noised aggregate value overflowed")]
    NoisedValueOverflow,

    /// The requested sampling parameters are outside the supported numeric range.
    #[error("differential privacy noise parameters are outside the supported sampler range")]
    NoiseParameterOutOfRange,

    /// The sampled integer noise exceeded signed integer bounds.
    #[error("differential privacy sampled noise exceeded signed integer bounds")]
    NoiseSampleOutOfRange,
}

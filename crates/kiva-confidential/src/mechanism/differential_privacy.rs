//! Differential privacy boundary.

mod budget;
mod errors;
mod noise;
mod policy;
mod release;

pub use budget::{
    DifferentialPrivacyAdvancedCompositionBound, DifferentialPrivacyBudgetLedger,
    DifferentialPrivacyCompositionSummary, bound_differential_privacy_advanced_composition,
    spend_differential_privacy_budget, summarize_differential_privacy_composition,
};
pub use errors::{
    DifferentialPrivacyBudgetError, DifferentialPrivacyBudgetPolicyLifecycleError,
    DifferentialPrivacyCompositionError, DifferentialPrivacyNoiseError,
    DifferentialPrivacyPolicyError, DifferentialPrivacyReleaseError,
};
pub use noise::{
    DifferentialPrivacyAccountingScope, DifferentialPrivacyAdjacencyModel,
    DifferentialPrivacyNoiseMechanism, DifferentialPrivacyNoiseParameters,
    DifferentialPrivacyNoisedAggregate, apply_differential_privacy_integer_noise,
    sample_differential_privacy_discrete_laplace_noise_from_uniforms,
};
pub use policy::{
    DifferentialPrivacyBudgetPolicyStatus, DifferentialPrivacyReleasePolicy,
    transition_differential_privacy_budget_policy,
};
pub use release::{DifferentialPrivacyReleaseContext, validate_differential_privacy_release};

/// Integer microunits used to represent one epsilon or delta unit.
pub const DIFFERENTIAL_PRIVACY_MICROUNITS_PER_UNIT: u64 = 1_000_000;

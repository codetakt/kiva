//! Differential privacy noise application and sampling boundary.

use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};

use super::errors::DifferentialPrivacyNoiseError;

/// Concrete differential privacy noise mechanism applied to one aggregate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DifferentialPrivacyNoiseMechanism {
    /// Integer-valued Laplace-family noise represented as a signed sample.
    DiscreteLaplace,
}

/// Privacy-accounting scope for a DP noise application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DifferentialPrivacyAccountingScope {
    /// Account the epsilon spend against one subject-owned budget ledger.
    SubjectBudget,
}

/// Differential privacy neighboring-dataset relation used by Kiva's DP profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DifferentialPrivacyAdjacencyModel {
    /// Neighboring datasets differ by adding or removing one data subject.
    AddRemoveOneSubject,
}

/// Parameters attached to one DP noise application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifferentialPrivacyNoiseParameters {
    pub(crate) mechanism: DifferentialPrivacyNoiseMechanism,
    pub(crate) accounting_scope: DifferentialPrivacyAccountingScope,
    pub(crate) adjacency_model: DifferentialPrivacyAdjacencyModel,
    pub(crate) epsilon_microunits: NonZeroU64,
    pub(crate) l1_sensitivity: NonZeroU64,
}

impl DifferentialPrivacyNoiseParameters {
    /// Creates a DP noise-application parameter set.
    #[must_use]
    pub const fn new(
        mechanism: DifferentialPrivacyNoiseMechanism,
        accounting_scope: DifferentialPrivacyAccountingScope,
        epsilon_microunits: NonZeroU64,
        l1_sensitivity: NonZeroU64,
    ) -> Self {
        Self {
            mechanism,
            accounting_scope,
            adjacency_model: DifferentialPrivacyAdjacencyModel::AddRemoveOneSubject,
            epsilon_microunits,
            l1_sensitivity,
        }
    }

    /// Returns the noise mechanism.
    #[must_use]
    pub const fn mechanism(self) -> DifferentialPrivacyNoiseMechanism {
        self.mechanism
    }

    /// Returns the privacy-accounting scope.
    #[must_use]
    pub const fn accounting_scope(self) -> DifferentialPrivacyAccountingScope {
        self.accounting_scope
    }

    /// Returns the neighboring-dataset relation used for this noise application.
    #[must_use]
    pub const fn adjacency_model(self) -> DifferentialPrivacyAdjacencyModel {
        self.adjacency_model
    }

    /// Returns the epsilon spend in integer microunits.
    #[must_use]
    pub const fn epsilon_microunits(self) -> NonZeroU64 {
        self.epsilon_microunits
    }

    /// Returns the L1 sensitivity in aggregate units.
    #[must_use]
    pub const fn l1_sensitivity(self) -> NonZeroU64 {
        self.l1_sensitivity
    }
}

/// Evidence that one aggregate value has had DP noise applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifferentialPrivacyNoisedAggregate {
    parameters: DifferentialPrivacyNoiseParameters,
    aggregate_value: i64,
    noise_value: i64,
    noised_value: i64,
}

impl DifferentialPrivacyNoisedAggregate {
    /// Returns the parameters used for the noise application.
    #[must_use]
    pub const fn parameters(self) -> DifferentialPrivacyNoiseParameters {
        self.parameters
    }

    /// Returns the raw aggregate value before noise.
    #[must_use]
    pub const fn aggregate_value(self) -> i64 {
        self.aggregate_value
    }

    /// Returns the signed noise sample applied to the aggregate.
    #[must_use]
    pub const fn noise_value(self) -> i64 {
        self.noise_value
    }

    /// Returns the released aggregate value after noise.
    #[must_use]
    pub const fn noised_value(self) -> i64 {
        self.noised_value
    }

    /// Returns whether this value carries an explicit noise application.
    #[must_use]
    pub const fn noise_applied(self) -> bool {
        true
    }
}

/// Applies an already-sampled integer DP noise value to an aggregate.
///
/// This function validates the arithmetic and records mechanism evidence. It
/// does not sample randomness or prove distributional differential privacy.
pub const fn apply_differential_privacy_integer_noise(
    parameters: DifferentialPrivacyNoiseParameters,
    aggregate_value: i64,
    noise_value: i64,
) -> Result<DifferentialPrivacyNoisedAggregate, DifferentialPrivacyNoiseError> {
    let Some(released_value) = aggregate_value.checked_add(noise_value) else {
        return Err(DifferentialPrivacyNoiseError::NoisedValueOverflow);
    };
    Ok(DifferentialPrivacyNoisedAggregate {
        parameters,
        aggregate_value,
        noise_value,
        noised_value: released_value,
    })
}

/// Samples integer-valued discrete Laplace-family noise from two random words.
///
/// The random words are expected to come from a cryptographically secure random
/// source. Kiva samples two independent geometric magnitudes and subtracts
/// them, which yields a symmetric two-sided geometric distribution with
/// parameter `exp(-epsilon / l1_sensitivity)`. The function is deterministic
/// for a given pair of random words so tests and higher-level evidence can
/// exercise the sampling boundary without depending on OS randomness.
#[allow(clippy::cast_precision_loss)]
pub fn sample_differential_privacy_discrete_laplace_noise_from_uniforms(
    parameters: DifferentialPrivacyNoiseParameters,
    left_random: u64,
    right_random: u64,
) -> Result<i64, DifferentialPrivacyNoiseError> {
    let epsilon = parameters.epsilon_microunits().get() as f64 / 1_000_000.0;
    let sensitivity = parameters.l1_sensitivity().get() as f64;
    let alpha = (-epsilon / sensitivity).exp();
    let left = sample_geometric_magnitude(alpha, left_random)?;
    let right = sample_geometric_magnitude(alpha, right_random)?;
    left.checked_sub(right)
        .ok_or(DifferentialPrivacyNoiseError::NoiseSampleOutOfRange)
}

#[allow(clippy::cast_possible_truncation)]
fn sample_geometric_magnitude(
    alpha: f64,
    random_word: u64,
) -> Result<i64, DifferentialPrivacyNoiseError> {
    const I64_EXCLUSIVE_UPPER_BOUND_AS_F64: f64 = 9_223_372_036_854_775_808.0;

    if alpha <= 0.0 {
        return Ok(0);
    }
    if !alpha.is_finite() || alpha >= 1.0 {
        return Err(DifferentialPrivacyNoiseError::NoiseParameterOutOfRange);
    }

    let uniform = uniform_open_closed_unit(random_word);
    let magnitude = uniform.log(alpha).floor();
    if !magnitude.is_finite() || magnitude >= I64_EXCLUSIVE_UPPER_BOUND_AS_F64 {
        return Err(DifferentialPrivacyNoiseError::NoiseSampleOutOfRange);
    }
    Ok(magnitude as i64)
}

#[allow(clippy::cast_precision_loss)]
fn uniform_open_closed_unit(random_word: u64) -> f64 {
    const F64_MANTISSA_BITS: u32 = 53;
    const F64_RANDOM_DENOMINATOR: f64 = 9_007_199_254_740_992.0;

    let mantissa = (random_word >> (u64::BITS - F64_MANTISSA_BITS)) + 1;
    mantissa as f64 / F64_RANDOM_DENOMINATOR
}

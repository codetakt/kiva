#![allow(clippy::wildcard_imports)]

use super::*;

/// Scheme family admitted by the current FHE feasibility profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FheFeasibilitySchemeFamily {
    /// BFV-style integer batching for exact small sums.
    BfvStyleIntegerSum,
}

impl FheFeasibilitySchemeFamily {
    /// Returns the stable scheme-family label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::BfvStyleIntegerSum => "bfv-style-integer-sum",
        }
    }
}

/// Operation admitted by the current FHE feasibility profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FheFeasibilityOperation {
    /// Exact integer sum over bounded participant plaintexts.
    Sum,
}

impl FheFeasibilityOperation {
    /// Returns the stable operation label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sum => "sum",
        }
    }
}

/// Strict parameter set for the deterministic FHE sum feasibility profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "FheFeasibilityParameterSetUnchecked",
    into = "FheFeasibilityParameterSetUnchecked"
)]
pub struct FheFeasibilityParameterSet {
    scheme_family: FheFeasibilitySchemeFamily,
    operation: FheFeasibilityOperation,
    polynomial_modulus_degree: u32,
    plaintext_modulus: u64,
    ciphertext_modulus_bits: u16,
    security_level_bits: u16,
    max_participant_count: NonZeroUsize,
    max_plaintext_value: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct FheFeasibilityParameterSetUnchecked {
    scheme_family: FheFeasibilitySchemeFamily,
    operation: FheFeasibilityOperation,
    polynomial_modulus_degree: u32,
    plaintext_modulus: u64,
    ciphertext_modulus_bits: u16,
    security_level_bits: u16,
    max_participant_count: NonZeroUsize,
    max_plaintext_value: u64,
}

impl FheFeasibilityParameterSet {
    /// Creates the strict FHE feasibility parameter set after profile validation.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        scheme_family: FheFeasibilitySchemeFamily,
        operation: FheFeasibilityOperation,
        polynomial_modulus_degree: u32,
        plaintext_modulus: u64,
        ciphertext_modulus_bits: u16,
        security_level_bits: u16,
        max_participant_count: NonZeroUsize,
        max_plaintext_value: u64,
    ) -> Result<Self, SecureAggregateError> {
        let parameter_set = Self {
            scheme_family,
            operation,
            polynomial_modulus_degree,
            plaintext_modulus,
            ciphertext_modulus_bits,
            security_level_bits,
            max_participant_count,
            max_plaintext_value,
        };
        validate_fhe_feasibility_parameters(parameter_set)?;
        Ok(parameter_set)
    }

    /// Creates a parameter set without validation for Kani boundary proofs.
    #[cfg(kani)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn unchecked_for_verification(
        scheme_family: FheFeasibilitySchemeFamily,
        operation: FheFeasibilityOperation,
        polynomial_modulus_degree: u32,
        plaintext_modulus: u64,
        ciphertext_modulus_bits: u16,
        security_level_bits: u16,
        max_participant_count: NonZeroUsize,
        max_plaintext_value: u64,
    ) -> Self {
        Self {
            scheme_family,
            operation,
            polynomial_modulus_degree,
            plaintext_modulus,
            ciphertext_modulus_bits,
            security_level_bits,
            max_participant_count,
            max_plaintext_value,
        }
    }

    /// Returns the default CC6 feasibility profile.
    #[must_use]
    pub const fn sum_profile_v1() -> Self {
        let Some(max_participant_count) = NonZeroUsize::new(FHE_FEASIBILITY_MAX_PARTICIPANTS)
        else {
            return Self {
                scheme_family: FheFeasibilitySchemeFamily::BfvStyleIntegerSum,
                operation: FheFeasibilityOperation::Sum,
                polynomial_modulus_degree: FHE_FEASIBILITY_POLYNOMIAL_MODULUS_DEGREE,
                plaintext_modulus: FHE_FEASIBILITY_PLAINTEXT_MODULUS,
                ciphertext_modulus_bits: FHE_FEASIBILITY_CIPHERTEXT_MODULUS_BITS,
                security_level_bits: FHE_FEASIBILITY_SECURITY_LEVEL_BITS,
                max_participant_count: NonZeroUsize::MIN,
                max_plaintext_value: FHE_FEASIBILITY_MAX_PLAINTEXT_VALUE,
            };
        };
        Self {
            scheme_family: FheFeasibilitySchemeFamily::BfvStyleIntegerSum,
            operation: FheFeasibilityOperation::Sum,
            polynomial_modulus_degree: FHE_FEASIBILITY_POLYNOMIAL_MODULUS_DEGREE,
            plaintext_modulus: FHE_FEASIBILITY_PLAINTEXT_MODULUS,
            ciphertext_modulus_bits: FHE_FEASIBILITY_CIPHERTEXT_MODULUS_BITS,
            security_level_bits: FHE_FEASIBILITY_SECURITY_LEVEL_BITS,
            max_participant_count,
            max_plaintext_value: FHE_FEASIBILITY_MAX_PLAINTEXT_VALUE,
        }
    }

    /// Returns the scheme family.
    #[must_use]
    pub const fn scheme_family(self) -> FheFeasibilitySchemeFamily {
        self.scheme_family
    }

    /// Returns the supported operation.
    #[must_use]
    pub const fn operation(self) -> FheFeasibilityOperation {
        self.operation
    }

    /// Returns the polynomial modulus degree.
    #[must_use]
    pub const fn polynomial_modulus_degree(self) -> u32 {
        self.polynomial_modulus_degree
    }

    /// Returns the plaintext modulus.
    #[must_use]
    pub const fn plaintext_modulus(self) -> u64 {
        self.plaintext_modulus
    }

    /// Returns the ciphertext modulus bit budget.
    #[must_use]
    pub const fn ciphertext_modulus_bits(self) -> u16 {
        self.ciphertext_modulus_bits
    }

    /// Returns the documented security level in bits.
    #[must_use]
    pub const fn security_level_bits(self) -> u16 {
        self.security_level_bits
    }

    /// Returns the maximum admitted participant count.
    #[must_use]
    pub const fn max_participant_count(self) -> NonZeroUsize {
        self.max_participant_count
    }

    /// Returns the maximum admitted participant plaintext value.
    #[must_use]
    pub const fn max_plaintext_value(self) -> u64 {
        self.max_plaintext_value
    }
}

impl TryFrom<FheFeasibilityParameterSetUnchecked> for FheFeasibilityParameterSet {
    type Error = SecureAggregateError;

    fn try_from(value: FheFeasibilityParameterSetUnchecked) -> Result<Self, Self::Error> {
        Self::new(
            value.scheme_family,
            value.operation,
            value.polynomial_modulus_degree,
            value.plaintext_modulus,
            value.ciphertext_modulus_bits,
            value.security_level_bits,
            value.max_participant_count,
            value.max_plaintext_value,
        )
    }
}

impl From<FheFeasibilityParameterSet> for FheFeasibilityParameterSetUnchecked {
    fn from(value: FheFeasibilityParameterSet) -> Self {
        Self {
            scheme_family: value.scheme_family(),
            operation: value.operation(),
            polynomial_modulus_degree: value.polynomial_modulus_degree(),
            plaintext_modulus: value.plaintext_modulus(),
            ciphertext_modulus_bits: value.ciphertext_modulus_bits(),
            security_level_bits: value.security_level_bits(),
            max_participant_count: value.max_participant_count(),
            max_plaintext_value: value.max_plaintext_value(),
        }
    }
}

/// Plan for one deterministic FHE sum feasibility job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "FheFeasibilityPlanUnchecked",
    into = "FheFeasibilityPlanUnchecked"
)]
pub struct FheFeasibilityPlan {
    pub(in crate::mechanism::core) release_policy: ReleasePolicy,
    pub(in crate::mechanism::core) parameter_set: FheFeasibilityParameterSet,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct FheFeasibilityPlanUnchecked {
    release_policy: ReleasePolicy,
    parameter_set: FheFeasibilityParameterSet,
}

impl FheFeasibilityPlan {
    /// Creates an FHE feasibility plan after enforcing the strict CC6 boundary.
    pub fn new(
        release_policy: ReleasePolicy,
        parameter_set: FheFeasibilityParameterSet,
    ) -> Result<Self, SecureAggregateError> {
        validate_fhe_release_policy(release_policy)?;
        validate_fhe_feasibility_parameters(parameter_set)?;
        if release_policy.participant_count().get() > parameter_set.max_participant_count().get() {
            return Err(SecureAggregateError::FheParameterSetRejected);
        }
        Ok(Self {
            release_policy,
            parameter_set,
        })
    }

    /// Returns the release policy.
    #[must_use]
    pub const fn release_policy(self) -> ReleasePolicy {
        self.release_policy
    }

    /// Returns the admitted parameter set.
    #[must_use]
    pub const fn parameter_set(self) -> FheFeasibilityParameterSet {
        self.parameter_set
    }

    /// Returns the participant count for this feasibility job.
    #[must_use]
    pub const fn participant_count(self) -> NonZeroUsize {
        self.release_policy.participant_count()
    }
}

impl TryFrom<FheFeasibilityPlanUnchecked> for FheFeasibilityPlan {
    type Error = SecureAggregateError;

    fn try_from(value: FheFeasibilityPlanUnchecked) -> Result<Self, Self::Error> {
        Self::new(value.release_policy, value.parameter_set)
    }
}

impl From<FheFeasibilityPlan> for FheFeasibilityPlanUnchecked {
    fn from(value: FheFeasibilityPlan) -> Self {
        Self {
            release_policy: value.release_policy(),
            parameter_set: value.parameter_set(),
        }
    }
}

/// Validates the strict parameter set admitted by the CC6 FHE feasibility profile.
pub fn validate_fhe_feasibility_parameters(
    parameter_set: FheFeasibilityParameterSet,
) -> Result<(), SecureAggregateError> {
    if parameter_set.scheme_family() != FheFeasibilitySchemeFamily::BfvStyleIntegerSum {
        return Err(SecureAggregateError::FheUnsupportedProfile);
    }
    if parameter_set.operation() != FheFeasibilityOperation::Sum {
        return Err(SecureAggregateError::FheUnsupportedProfile);
    }
    if parameter_set.polynomial_modulus_degree() != FHE_FEASIBILITY_POLYNOMIAL_MODULUS_DEGREE
        || parameter_set.plaintext_modulus() != FHE_FEASIBILITY_PLAINTEXT_MODULUS
        || parameter_set.ciphertext_modulus_bits() != FHE_FEASIBILITY_CIPHERTEXT_MODULUS_BITS
        || parameter_set.security_level_bits() != FHE_FEASIBILITY_SECURITY_LEVEL_BITS
        || parameter_set.max_participant_count().get() != FHE_FEASIBILITY_MAX_PARTICIPANTS
        || parameter_set.max_plaintext_value() != FHE_FEASIBILITY_MAX_PLAINTEXT_VALUE
    {
        return Err(SecureAggregateError::FheParameterSetRejected);
    }

    let Ok(max_participant_count) = u64::try_from(parameter_set.max_participant_count().get())
    else {
        return Err(SecureAggregateError::FheParameterSetRejected);
    };
    let Some(max_sum) = parameter_set
        .max_plaintext_value()
        .checked_mul(max_participant_count)
    else {
        return Err(SecureAggregateError::FheParameterSetRejected);
    };
    if max_sum >= parameter_set.plaintext_modulus() {
        return Err(SecureAggregateError::FheParameterSetRejected);
    }

    Ok(())
}

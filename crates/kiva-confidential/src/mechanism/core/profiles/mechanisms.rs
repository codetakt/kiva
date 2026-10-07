//! Technique and mechanism identifiers for integrated confidential computation.

use super::super::*;

/// Confidential computation technique family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Technique {
    /// Secure multi-party computation.
    SecureMultiPartyComputation,
    /// N-of-N additive secure aggregation over a fixed finite field.
    AdditiveSecureAggregation,
    /// Private set intersection.
    PrivateSetIntersection,
    /// Threshold cryptography.
    ThresholdCryptography,
    /// Differential privacy.
    DifferentialPrivacy,
    /// Trusted execution environment.
    TrustedExecutionEnvironment,
    /// Fully homomorphic encryption.
    FullyHomomorphicEncryption,
}

/// Concrete confidential mechanism implemented by Kiva's current extension boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidentialMechanismKind {
    /// N-of-N additive secure aggregation over Kiva's fixed finite field.
    AdditiveNOfN,
    /// N-of-N private set intersection over pre-tokenized cohort identifiers.
    PrivateSetIntersection,
    /// Review-gated DH-OPRF PSI runtime boundary.
    PrivateSetIntersectionOprfDh,
    /// Shamir threshold reconstruction over Kiva's fixed finite field.
    ShamirThreshold,
    /// Review-gated FROST threshold-signing runtime boundary.
    ThresholdFrostSigning,
    /// Deployment-bound portable trusted execution environment evidence gate.
    TrustedExecutionEnvironment,
    /// Review-gated AWS Nitro Enclaves attestation verifier boundary.
    TrustedExecutionEnvironmentAwsNitro,
    /// Narrow deterministic FHE sum feasibility profile.
    FheSumFeasibility,
    /// Review-gated production FHE sum runtime boundary.
    FheSumRuntime,
}

impl ConfidentialMechanismKind {
    /// Returns the stable label used in contracts and evidence.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::AdditiveNOfN => "additive-n-of-n",
            Self::PrivateSetIntersection => "private-set-intersection",
            Self::PrivateSetIntersectionOprfDh => "private-set-intersection-oprf-dh",
            Self::ShamirThreshold => "shamir-threshold",
            Self::ThresholdFrostSigning => "threshold-frost-signing",
            Self::TrustedExecutionEnvironment => "trusted-execution-environment",
            Self::TrustedExecutionEnvironmentAwsNitro => "trusted-execution-environment-aws-nitro",
            Self::FheSumFeasibility => "fhe-sum-feasibility",
            Self::FheSumRuntime => "fhe-sum-runtime",
        }
    }

    /// Parses a stable confidential-mechanism label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "additive-n-of-n" => Some(Self::AdditiveNOfN),
            "private-set-intersection" => Some(Self::PrivateSetIntersection),
            "private-set-intersection-oprf-dh" => Some(Self::PrivateSetIntersectionOprfDh),
            "shamir-threshold" => Some(Self::ShamirThreshold),
            "threshold-frost-signing" => Some(Self::ThresholdFrostSigning),
            "trusted-execution-environment" => Some(Self::TrustedExecutionEnvironment),
            "trusted-execution-environment-aws-nitro" => {
                Some(Self::TrustedExecutionEnvironmentAwsNitro)
            }
            "fhe-sum-feasibility" => Some(Self::FheSumFeasibility),
            "fhe-sum-runtime" => Some(Self::FheSumRuntime),
            _ => None,
        }
    }
}

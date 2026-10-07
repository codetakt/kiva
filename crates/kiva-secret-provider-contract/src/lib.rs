//! Secret and confidential-compute provider contract vocabulary.
//!
//! This crate owns provider-kind, capability, and profile labels shared by
//! Kiva's confidential control plane, PDS provider-binding metadata, generated
//! registries, and external provider integration documents. It deliberately
//! contains no mechanism registry, runtime admission, cryptographic primitive,
//! storage, or HTTP dependencies.

mod kind_registry;

pub use kind_registry::{
    SECRET_COMPUTE_PROVIDER_KIND_REGISTRY, SecretComputeProviderKindRegistryEntry,
    provider_kind_from_label, provider_kind_label, registry_entry_for_provider_kind,
};

use serde::{Deserialize, Serialize};

/// Deterministic reference provider profile label.
pub const REFERENCE_PRIMITIVE_PROVIDER_PROFILE: &str = "kiva-reference-primitives-v1";

/// External secret/confidential-compute provider contract profile label.
pub const SECRET_COMPUTE_PROVIDER_CONTRACT_PROFILE: &str =
    "kiva-secret-compute-provider-contract-v1";

/// Provider family admitted by Kiva's secret/confidential-compute contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretComputeProviderKind {
    /// Deterministic in-repository reference primitives for tests and fixtures.
    ReferencePrimitive,
    /// External key-management service or HSM custody provider.
    SecretCustody,
    /// External TEE verifier or attested-execution provider.
    TrustedExecutionEnvironment,
    /// External FHE runtime provider.
    FullyHomomorphicEncryption,
    /// External PSI runtime provider.
    PrivateSetIntersection,
    /// External threshold cryptography runtime provider.
    ThresholdCryptography,
}

/// Provider kind admitted only for deterministic reference fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceProviderKind {
    /// Deterministic in-repository reference primitives for tests and fixtures.
    ReferencePrimitive,
}

/// Provider kind admitted only for opaque secret or key custody metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretCustodyProviderKind {
    /// External key-management service or HSM custody provider.
    SecretCustody,
}

/// Provider kind admitted for confidential-computation mechanism execution or attestation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComputeMechanismProviderKind {
    /// External TEE verifier or attested-execution provider.
    TrustedExecutionEnvironment,
    /// External FHE runtime provider.
    FullyHomomorphicEncryption,
    /// External PSI runtime provider.
    PrivateSetIntersection,
    /// External threshold cryptography runtime provider.
    ThresholdCryptography,
}

/// Marker for providers that only custody opaque secrets or keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretCustodyProvider;

/// Marker for deterministic fixture providers used by tests and verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferencePrimitiveProvider;

/// Marker for providers that execute or attest confidential computation mechanisms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComputeMechanismProvider;

/// Shared contract implemented by every provider-kind enum at this boundary.
pub trait SecretComputeProviderKindContract: Copy {
    /// Returns the stable label used in manifests and digest derivation.
    #[must_use]
    fn label(self) -> &'static str;

    /// Returns the shared manifest provider kind.
    #[must_use]
    fn as_secret_compute_provider_kind(self) -> SecretComputeProviderKind;
}

impl SecretComputeProviderKindContract for SecretComputeProviderKind {
    fn label(self) -> &'static str {
        Self::label(self)
    }

    fn as_secret_compute_provider_kind(self) -> SecretComputeProviderKind {
        self
    }
}

impl SecretComputeProviderKindContract for ReferenceProviderKind {
    fn label(self) -> &'static str {
        Self::label(self)
    }

    fn as_secret_compute_provider_kind(self) -> SecretComputeProviderKind {
        Self::as_secret_compute_provider_kind(self)
    }
}

impl SecretComputeProviderKindContract for SecretCustodyProviderKind {
    fn label(self) -> &'static str {
        Self::label(self)
    }

    fn as_secret_compute_provider_kind(self) -> SecretComputeProviderKind {
        Self::as_secret_compute_provider_kind(self)
    }
}

impl SecretComputeProviderKindContract for ComputeMechanismProviderKind {
    fn label(self) -> &'static str {
        Self::label(self)
    }

    fn as_secret_compute_provider_kind(self) -> SecretComputeProviderKind {
        Self::as_secret_compute_provider_kind(self)
    }
}

impl SecretComputeProviderKind {
    /// Returns the stable label used in manifests and digest derivation.
    #[must_use]
    pub fn label(self) -> &'static str {
        provider_kind_label(self)
    }

    /// Parses a stable provider-kind label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        provider_kind_from_label(value)
    }

    /// Returns whether this kind belongs to the secret-custody provider boundary.
    #[must_use]
    pub const fn is_secret_custody_provider(self) -> bool {
        matches!(self, Self::SecretCustody)
    }

    /// Returns whether this kind belongs to the deterministic reference-fixture boundary.
    #[must_use]
    pub const fn is_reference_primitive_provider(self) -> bool {
        matches!(self, Self::ReferencePrimitive)
    }

    /// Returns whether this kind belongs to the compute-mechanism provider boundary.
    #[must_use]
    pub const fn is_compute_mechanism_provider(self) -> bool {
        matches!(
            self,
            Self::TrustedExecutionEnvironment
                | Self::FullyHomomorphicEncryption
                | Self::PrivateSetIntersection
                | Self::ThresholdCryptography
        )
    }

    /// Returns the typed reference-provider kind when this is a reference fixture.
    #[must_use]
    pub const fn as_reference_provider_kind(self) -> Option<ReferenceProviderKind> {
        match self {
            Self::ReferencePrimitive => Some(ReferenceProviderKind::ReferencePrimitive),
            _ => None,
        }
    }

    /// Returns the typed custody-provider kind when this is a custody boundary.
    #[must_use]
    pub const fn as_secret_custody_provider_kind(self) -> Option<SecretCustodyProviderKind> {
        match self {
            Self::SecretCustody => Some(SecretCustodyProviderKind::SecretCustody),
            _ => None,
        }
    }

    /// Returns the typed compute-provider kind when this is a compute boundary.
    #[must_use]
    pub const fn as_compute_mechanism_provider_kind(self) -> Option<ComputeMechanismProviderKind> {
        match self {
            Self::TrustedExecutionEnvironment => {
                Some(ComputeMechanismProviderKind::TrustedExecutionEnvironment)
            }
            Self::FullyHomomorphicEncryption => {
                Some(ComputeMechanismProviderKind::FullyHomomorphicEncryption)
            }
            Self::PrivateSetIntersection => {
                Some(ComputeMechanismProviderKind::PrivateSetIntersection)
            }
            Self::ThresholdCryptography => {
                Some(ComputeMechanismProviderKind::ThresholdCryptography)
            }
            _ => None,
        }
    }
}

impl ReferenceProviderKind {
    /// Returns the stable label used in manifests.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReferencePrimitive => "reference-primitive",
        }
    }

    /// Returns the shared manifest provider kind.
    #[must_use]
    pub const fn as_secret_compute_provider_kind(self) -> SecretComputeProviderKind {
        match self {
            Self::ReferencePrimitive => SecretComputeProviderKind::ReferencePrimitive,
        }
    }
}

impl SecretCustodyProviderKind {
    /// Returns the stable label used in manifests.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::SecretCustody => "secret-custody",
        }
    }

    /// Returns the shared manifest provider kind.
    #[must_use]
    pub const fn as_secret_compute_provider_kind(self) -> SecretComputeProviderKind {
        match self {
            Self::SecretCustody => SecretComputeProviderKind::SecretCustody,
        }
    }
}

impl ComputeMechanismProviderKind {
    /// Returns the stable label used in manifests.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::TrustedExecutionEnvironment => "trusted-execution-environment",
            Self::FullyHomomorphicEncryption => "fully-homomorphic-encryption",
            Self::PrivateSetIntersection => "private-set-intersection",
            Self::ThresholdCryptography => "threshold-cryptography",
        }
    }

    /// Parses a compute-provider kind label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "trusted-execution-environment" => Some(Self::TrustedExecutionEnvironment),
            "fully-homomorphic-encryption" => Some(Self::FullyHomomorphicEncryption),
            "private-set-intersection" => Some(Self::PrivateSetIntersection),
            "threshold-cryptography" => Some(Self::ThresholdCryptography),
            _ => None,
        }
    }

    /// Returns the shared manifest provider kind.
    #[must_use]
    pub const fn as_secret_compute_provider_kind(self) -> SecretComputeProviderKind {
        match self {
            Self::TrustedExecutionEnvironment => {
                SecretComputeProviderKind::TrustedExecutionEnvironment
            }
            Self::FullyHomomorphicEncryption => {
                SecretComputeProviderKind::FullyHomomorphicEncryption
            }
            Self::PrivateSetIntersection => SecretComputeProviderKind::PrivateSetIntersection,
            Self::ThresholdCryptography => SecretComputeProviderKind::ThresholdCryptography,
        }
    }
}

/// Capability claimed by a provider binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretComputeProviderCapability {
    /// Envelope encryption or opaque secret custody.
    SecretCustody,
    /// TEE attestation or attested execution evidence.
    TeeAttestation,
    /// FHE ciphertext evaluation metadata.
    FheEvaluation,
    /// PSI evaluated element or transcript metadata.
    PsiEvaluation,
    /// Threshold signing or key-ceremony metadata.
    ThresholdSigning,
    /// Deterministic reference fixtures for Kiva boundary tests.
    ReferenceFixtures,
}

impl SecretComputeProviderCapability {
    /// Returns the stable label used in manifests and digest derivation.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::SecretCustody => "secret-custody",
            Self::TeeAttestation => "tee-attestation",
            Self::FheEvaluation => "fhe-evaluation",
            Self::PsiEvaluation => "psi-evaluation",
            Self::ThresholdSigning => "threshold-signing",
            Self::ReferenceFixtures => "reference-fixtures",
        }
    }

    /// Parses a stable provider-capability label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "secret-custody" => Some(Self::SecretCustody),
            "tee-attestation" => Some(Self::TeeAttestation),
            "fhe-evaluation" => Some(Self::FheEvaluation),
            "psi-evaluation" => Some(Self::PsiEvaluation),
            "threshold-signing" => Some(Self::ThresholdSigning),
            "reference-fixtures" => Some(Self::ReferenceFixtures),
            _ => None,
        }
    }
}

/// Contract profile for a secret/confidential-compute provider binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretComputeProviderProfile {
    /// Kiva's deterministic reference provider for tests, fixtures, and proofs.
    KivaReferencePrimitivesV1,
    /// A deployment-selected externally reviewed provider.
    ExternalReviewedProviderV1,
}

impl SecretComputeProviderProfile {
    /// Returns the stable profile identifier.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::KivaReferencePrimitivesV1 => REFERENCE_PRIMITIVE_PROVIDER_PROFILE,
            Self::ExternalReviewedProviderV1 => SECRET_COMPUTE_PROVIDER_CONTRACT_PROFILE,
        }
    }

    /// Parses a stable provider-profile label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            REFERENCE_PRIMITIVE_PROVIDER_PROFILE => Some(Self::KivaReferencePrimitivesV1),
            SECRET_COMPUTE_PROVIDER_CONTRACT_PROFILE => Some(Self::ExternalReviewedProviderV1),
            _ => None,
        }
    }
}

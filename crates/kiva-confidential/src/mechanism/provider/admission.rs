//! Secret-compute provider admission and release gates.

use serde::{Deserialize, Serialize};

use kiva_secret_provider_contract::{
    SecretComputeProviderCapability, SecretComputeProviderProfile,
};

use super::{
    binding::{
        SecretComputeProviderBinding, secret_compute_required_capability_for_mechanism,
        validate_secret_compute_provider_binding,
    },
    digest::derive_secret_compute_provider_binding_digest,
};
use crate::{
    SecureAggregateError, mechanism::workflow::ConfidentialMechanismKind,
    primitive::SecureAggregateCommitment,
};

/// Evidence gate state produced from a provider binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretComputeProviderEvidenceGate {
    /// Deterministic reference fixtures only; no production security claim.
    ReferenceFixture,
    /// External provider is admitted only for non-production use.
    ExternalNonProduction,
    /// External provider supplied completed review evidence.
    ExternalReviewed,
}

impl SecretComputeProviderEvidenceGate {
    /// Returns the stable label used in manifests and evidence bundles.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReferenceFixture => "reference-fixture",
            Self::ExternalNonProduction => "external-non-production",
            Self::ExternalReviewed => "external-reviewed",
        }
    }
}

/// Release gate admitted for a provider binding and mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretComputeProviderReleaseGate {
    /// Fixture-only boundary; cannot support production claims.
    FixtureOnly,
    /// Non-production external runtime boundary.
    NonProduction,
    /// Production external runtime boundary with review evidence.
    Production,
}

impl SecretComputeProviderReleaseGate {
    /// Returns the stable label used in manifests and evidence bundles.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::FixtureOnly => "fixture-only",
            Self::NonProduction => "non-production",
            Self::Production => "production",
        }
    }
}

/// Admitted provider evidence for one concrete confidential mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretComputeProviderAdmission {
    binding: SecretComputeProviderBinding,
    mechanism: ConfidentialMechanismKind,
    required_capability: SecretComputeProviderCapability,
    binding_digest: SecureAggregateCommitment,
    evidence_gate: SecretComputeProviderEvidenceGate,
    release_gate: SecretComputeProviderReleaseGate,
}

impl SecretComputeProviderAdmission {
    /// Returns the provider binding that was admitted.
    #[must_use]
    pub const fn binding(self) -> SecretComputeProviderBinding {
        self.binding
    }

    /// Returns the concrete mechanism admitted for this provider binding.
    #[must_use]
    pub const fn mechanism(self) -> ConfidentialMechanismKind {
        self.mechanism
    }

    /// Returns the mechanism's required non-reference capability.
    #[must_use]
    pub const fn required_capability(self) -> SecretComputeProviderCapability {
        self.required_capability
    }

    /// Returns the canonical digest of the provider binding.
    #[must_use]
    pub const fn binding_digest(self) -> SecureAggregateCommitment {
        self.binding_digest
    }

    /// Returns the evidence gate state.
    #[must_use]
    pub const fn evidence_gate(self) -> SecretComputeProviderEvidenceGate {
        self.evidence_gate
    }

    /// Returns the release gate admitted for this binding.
    #[must_use]
    pub const fn release_gate(self) -> SecretComputeProviderReleaseGate {
        self.release_gate
    }
}

/// Validates that a provider capability matches the mechanism that will use it.
pub fn validate_secret_compute_provider_for_mechanism(
    binding: SecretComputeProviderBinding,
    mechanism: ConfidentialMechanismKind,
) -> Result<(), SecureAggregateError> {
    let capability = binding.capability();
    let required_capability = secret_compute_required_capability_for_mechanism(mechanism);
    let accepted = match mechanism {
        ConfidentialMechanismKind::AdditiveNOfN => capability == required_capability,
        _other => {
            capability == required_capability
                || capability == SecretComputeProviderCapability::ReferenceFixtures
        }
    };
    if !accepted {
        return Err(SecureAggregateError::SecretComputeProviderCapabilityMismatch);
    }
    validate_secret_compute_provider_binding(binding)
}

/// Admits a provider binding and returns the normalized capability/evidence model.
pub fn admit_secret_compute_provider(
    binding: SecretComputeProviderBinding,
    mechanism: ConfidentialMechanismKind,
) -> Result<SecretComputeProviderAdmission, SecureAggregateError> {
    validate_secret_compute_provider_for_mechanism(binding, mechanism)?;
    let required_capability = secret_compute_required_capability_for_mechanism(mechanism);
    let binding_digest = derive_secret_compute_provider_binding_digest(binding);
    let evidence_gate = match binding.profile() {
        SecretComputeProviderProfile::KivaReferencePrimitivesV1 => {
            SecretComputeProviderEvidenceGate::ReferenceFixture
        }
        SecretComputeProviderProfile::ExternalReviewedProviderV1
            if binding.external_review_completed()
                && binding.review_evidence_digest().is_some() =>
        {
            SecretComputeProviderEvidenceGate::ExternalReviewed
        }
        SecretComputeProviderProfile::ExternalReviewedProviderV1 => {
            SecretComputeProviderEvidenceGate::ExternalNonProduction
        }
    };
    let release_gate = match binding.profile() {
        SecretComputeProviderProfile::KivaReferencePrimitivesV1 => {
            SecretComputeProviderReleaseGate::FixtureOnly
        }
        SecretComputeProviderProfile::ExternalReviewedProviderV1
            if binding.production_claim_requested() =>
        {
            SecretComputeProviderReleaseGate::Production
        }
        SecretComputeProviderProfile::ExternalReviewedProviderV1 => {
            SecretComputeProviderReleaseGate::NonProduction
        }
    };
    Ok(SecretComputeProviderAdmission {
        binding,
        mechanism,
        required_capability,
        binding_digest,
        evidence_gate,
        release_gate,
    })
}

//! Runtime boundary admitted from provider contract evidence.

use kiva_confidential_core::{ControlPlaneEvidence, ExternalDataPlaneBoundary};
use kiva_secret_provider_contract::{
    ComputeMechanismProviderKind, SecretComputeProviderCapability, SecretComputeProviderKind,
    SecretComputeProviderProfile,
};
use serde::{Deserialize, Serialize};

use super::{
    admission::{
        SecretComputeProviderAdmission, SecretComputeProviderEvidenceGate,
        SecretComputeProviderReleaseGate, admit_secret_compute_provider,
    },
    binding::{SecretComputeProviderBinding, SecretComputeProviderEvidence},
};
use crate::{
    SecureAggregateError, mechanism::workflow::ConfidentialMechanismKind,
    primitive::SecureAggregateCommitment,
};

/// Runtime claim admitted for a provider-backed confidential mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretComputeProviderRuntimeBoundaryClaim {
    /// Deterministic reference fixture boundary; never a production claim.
    Fixture,
    /// External provider boundary admitted for non-production use.
    ExternalNonProduction,
    /// External provider boundary admitted for production after completed review.
    ExternalReviewedProduction,
}

/// Coarse provider boundary class used by deployment and evidence tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretComputeProviderRuntimeClass {
    /// In-repository deterministic reference primitive boundary.
    Reference,
    /// External provider admitted for experiments or staging only.
    ExternalNonProduction,
    /// External provider admitted for production after review.
    ExternalReviewedProduction,
}

impl SecretComputeProviderRuntimeClass {
    /// Returns the stable label used in evidence and deployment manifests.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Reference => "reference",
            Self::ExternalNonProduction => "external-non-production",
            Self::ExternalReviewedProduction => "external-reviewed-production",
        }
    }
}

impl SecretComputeProviderRuntimeBoundaryClaim {
    /// Returns the stable label used in evidence and release packages.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fixture => "fixture",
            Self::ExternalNonProduction => "external-non-production",
            Self::ExternalReviewedProduction => "external-reviewed-production",
        }
    }
}

/// First-class runtime boundary produced after provider admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretComputeProviderRuntimeBoundary {
    admission: ControlPlaneEvidence<SecretComputeProviderAdmission>,
    claim: ControlPlaneEvidence<SecretComputeProviderRuntimeBoundaryClaim>,
    runtime_class: ExternalDataPlaneBoundary<SecretComputeProviderRuntimeClass>,
}

/// Common runtime contract shape exposed to provider integrations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretComputeProviderRuntimeContract {
    mechanism: ConfidentialMechanismKind,
    required_capability: SecretComputeProviderCapability,
    runtime_class: SecretComputeProviderRuntimeClass,
    claim: SecretComputeProviderRuntimeBoundaryClaim,
}

impl SecretComputeProviderRuntimeContract {
    /// Creates the common runtime contract view for an admitted boundary.
    #[must_use]
    pub fn from_boundary(boundary: SecretComputeProviderRuntimeBoundary) -> Self {
        let admission = boundary.admission();
        let claim = boundary.claim();
        let runtime_class = boundary.runtime_class();
        Self {
            mechanism: admission.mechanism(),
            required_capability: admission.required_capability(),
            runtime_class,
            claim,
        }
    }

    /// Returns the confidential mechanism governed by this runtime contract.
    #[must_use]
    pub const fn mechanism(self) -> ConfidentialMechanismKind {
        self.mechanism
    }

    /// Returns the provider capability required by this runtime contract.
    #[must_use]
    pub const fn required_capability(self) -> SecretComputeProviderCapability {
        self.required_capability
    }

    /// Returns the runtime class admitted for this provider boundary.
    #[must_use]
    pub const fn runtime_class(self) -> SecretComputeProviderRuntimeClass {
        self.runtime_class
    }

    /// Returns the admitted runtime boundary claim.
    #[must_use]
    pub const fn claim(self) -> SecretComputeProviderRuntimeBoundaryClaim {
        self.claim
    }

    /// Returns whether this runtime contract is admitted for production use.
    #[must_use]
    pub const fn production_ready(self) -> bool {
        matches!(
            self.claim,
            SecretComputeProviderRuntimeBoundaryClaim::ExternalReviewedProduction
        )
    }
}

impl SecretComputeProviderRuntimeBoundary {
    /// Creates a runtime boundary from an admitted provider record.
    pub fn new(admission: SecretComputeProviderAdmission) -> Result<Self, SecureAggregateError> {
        let claim = match admission.release_gate() {
            SecretComputeProviderReleaseGate::FixtureOnly => {
                SecretComputeProviderRuntimeBoundaryClaim::Fixture
            }
            SecretComputeProviderReleaseGate::NonProduction => {
                SecretComputeProviderRuntimeBoundaryClaim::ExternalNonProduction
            }
            SecretComputeProviderReleaseGate::Production
                if admission.evidence_gate()
                    == SecretComputeProviderEvidenceGate::ExternalReviewed =>
            {
                SecretComputeProviderRuntimeBoundaryClaim::ExternalReviewedProduction
            }
            SecretComputeProviderReleaseGate::Production => {
                return Err(SecureAggregateError::ExternalProviderReviewRequired);
            }
        };
        let runtime_class = match claim {
            SecretComputeProviderRuntimeBoundaryClaim::Fixture => {
                SecretComputeProviderRuntimeClass::Reference
            }
            SecretComputeProviderRuntimeBoundaryClaim::ExternalNonProduction => {
                SecretComputeProviderRuntimeClass::ExternalNonProduction
            }
            SecretComputeProviderRuntimeBoundaryClaim::ExternalReviewedProduction => {
                SecretComputeProviderRuntimeClass::ExternalReviewedProduction
            }
        };
        Ok(Self {
            admission: ControlPlaneEvidence::new(admission),
            claim: ControlPlaneEvidence::new(claim),
            runtime_class: ExternalDataPlaneBoundary::new(runtime_class),
        })
    }

    /// Returns the provider admission backing this runtime boundary.
    #[must_use]
    pub fn admission(self) -> SecretComputeProviderAdmission {
        self.admission.into_inner()
    }

    /// Returns the runtime claim admitted for this provider boundary.
    #[must_use]
    pub fn claim(self) -> SecretComputeProviderRuntimeBoundaryClaim {
        self.claim.into_inner()
    }

    /// Returns the coarse provider boundary class.
    #[must_use]
    pub fn runtime_class(self) -> SecretComputeProviderRuntimeClass {
        self.runtime_class.into_inner()
    }

    /// Returns the control-plane label attached to admission and release claim evidence.
    #[must_use]
    pub const fn control_plane_label(&self) -> &'static str {
        self.admission.plane_label()
    }

    /// Returns the external data-plane label attached to provider runtime execution.
    #[must_use]
    pub const fn data_plane_label(&self) -> &'static str {
        self.runtime_class.plane_label()
    }

    /// Returns whether this boundary is Kiva's deterministic reference provider.
    #[must_use]
    pub fn reference_provider(self) -> bool {
        matches!(
            self.runtime_class(),
            SecretComputeProviderRuntimeClass::Reference
        )
    }

    /// Returns whether this boundary admits production runtime use.
    #[must_use]
    pub fn production_ready(self) -> bool {
        matches!(
            self.claim(),
            SecretComputeProviderRuntimeBoundaryClaim::ExternalReviewedProduction
        )
    }

    /// Returns the common runtime contract view for provider integrations.
    #[must_use]
    pub fn runtime_contract(self) -> SecretComputeProviderRuntimeContract {
        SecretComputeProviderRuntimeContract::from_boundary(self)
    }
}

/// Admits a provider binding and returns the normalized runtime boundary.
pub fn admit_secret_compute_provider_runtime_boundary(
    binding: SecretComputeProviderBinding,
    mechanism: ConfidentialMechanismKind,
) -> Result<SecretComputeProviderRuntimeBoundary, SecureAggregateError> {
    SecretComputeProviderRuntimeBoundary::new(admit_secret_compute_provider(binding, mechanism)?)
}

/// Admits Kiva's fixed reference primitive provider for experiments and fixtures.
pub fn admit_reference_primitive_runtime_boundary(
    provider_id_digest: SecureAggregateCommitment,
    artifact_digest: SecureAggregateCommitment,
    mechanism: ConfidentialMechanismKind,
) -> Result<SecretComputeProviderRuntimeBoundary, SecureAggregateError> {
    let binding = SecretComputeProviderBinding::new_with_evidence(
        SecretComputeProviderKind::ReferencePrimitive,
        SecretComputeProviderCapability::ReferenceFixtures,
        SecretComputeProviderProfile::KivaReferencePrimitivesV1,
        provider_id_digest,
        artifact_digest,
        SecretComputeProviderEvidence::non_production(None),
    )?;
    admit_secret_compute_provider_runtime_boundary(binding, mechanism)
}

/// Admits an externally reviewed provider boundary for production use.
pub fn admit_reviewed_production_runtime_boundary(
    kind: ComputeMechanismProviderKind,
    capability: SecretComputeProviderCapability,
    provider_id_digest: SecureAggregateCommitment,
    artifact_digest: SecureAggregateCommitment,
    parameter_set_digest: Option<SecureAggregateCommitment>,
    review_evidence_digest: SecureAggregateCommitment,
    mechanism: ConfidentialMechanismKind,
) -> Result<SecretComputeProviderRuntimeBoundary, SecureAggregateError> {
    let binding = SecretComputeProviderBinding::new_with_evidence(
        kind.as_secret_compute_provider_kind(),
        capability,
        SecretComputeProviderProfile::ExternalReviewedProviderV1,
        provider_id_digest,
        artifact_digest,
        SecretComputeProviderEvidence::reviewed_production(
            parameter_set_digest,
            review_evidence_digest,
        ),
    )?;
    admit_secret_compute_provider_runtime_boundary(binding, mechanism)
}

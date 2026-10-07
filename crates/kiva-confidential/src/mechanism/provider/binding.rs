//! Secret-compute provider binding types and validation.

use serde::{Deserialize, Serialize};

use kiva_secret_provider_contract::{
    SecretComputeProviderCapability, SecretComputeProviderKind, SecretComputeProviderProfile,
};

use crate::{
    SecureAggregateError,
    mechanism::{registry::descriptor_for_mechanism, workflow::ConfidentialMechanismKind},
    primitive::SecureAggregateCommitment,
};

/// External review evidence state for a secret-compute provider binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretComputeProviderReviewEvidence {
    /// No completed external review evidence is attached.
    Pending,
    /// External review completed with a digest of the review evidence.
    Completed(SecureAggregateCommitment),
}

impl SecretComputeProviderReviewEvidence {
    /// Returns the optional review evidence digest.
    #[must_use]
    pub const fn digest(self) -> Option<SecureAggregateCommitment> {
        match self {
            Self::Pending => None,
            Self::Completed(digest) => Some(digest),
        }
    }

    /// Returns whether external review is complete.
    #[must_use]
    pub const fn completed(self) -> bool {
        matches!(self, Self::Completed(_digest))
    }
}

/// Usage claim requested for a secret-compute provider binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretComputeProviderUsageClaim {
    /// Binding is admitted only for fixture or non-production use.
    NonProduction,
    /// Binding requests a production security claim.
    Production,
}

impl SecretComputeProviderUsageClaim {
    /// Returns whether a production claim is requested.
    #[must_use]
    pub const fn production_requested(self) -> bool {
        matches!(self, Self::Production)
    }
}

/// Typed provider evidence used to construct a provider binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretComputeProviderEvidence {
    parameter_set_digest: Option<SecureAggregateCommitment>,
    review_evidence: SecretComputeProviderReviewEvidence,
    usage_claim: SecretComputeProviderUsageClaim,
}

impl SecretComputeProviderEvidence {
    /// Creates provider evidence from typed review and usage facts.
    #[must_use]
    pub const fn new(
        parameter_set_digest: Option<SecureAggregateCommitment>,
        review_evidence: SecretComputeProviderReviewEvidence,
        usage_claim: SecretComputeProviderUsageClaim,
    ) -> Self {
        Self {
            parameter_set_digest,
            review_evidence,
            usage_claim,
        }
    }

    /// Creates non-production provider evidence with no external review.
    #[must_use]
    pub const fn non_production(parameter_set_digest: Option<SecureAggregateCommitment>) -> Self {
        Self::new(
            parameter_set_digest,
            SecretComputeProviderReviewEvidence::Pending,
            SecretComputeProviderUsageClaim::NonProduction,
        )
    }

    /// Creates production provider evidence with completed external review.
    #[must_use]
    pub const fn reviewed_production(
        parameter_set_digest: Option<SecureAggregateCommitment>,
        review_evidence_digest: SecureAggregateCommitment,
    ) -> Self {
        Self::new(
            parameter_set_digest,
            SecretComputeProviderReviewEvidence::Completed(review_evidence_digest),
            SecretComputeProviderUsageClaim::Production,
        )
    }

    /// Returns the optional provider parameter-set digest.
    #[must_use]
    pub const fn parameter_set_digest(self) -> Option<SecureAggregateCommitment> {
        self.parameter_set_digest
    }

    /// Returns the optional external-review evidence digest.
    #[must_use]
    pub const fn review_evidence_digest(self) -> Option<SecureAggregateCommitment> {
        self.review_evidence.digest()
    }

    /// Returns whether external review is complete.
    #[must_use]
    pub const fn external_review_completed(self) -> bool {
        self.review_evidence.completed()
    }

    /// Returns whether a production claim is requested.
    #[must_use]
    pub const fn production_claim_requested(self) -> bool {
        self.usage_claim.production_requested()
    }
}

/// Metadata-only binding for a provider that handles secrets or confidential computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretComputeProviderBinding {
    kind: SecretComputeProviderKind,
    capability: SecretComputeProviderCapability,
    profile: SecretComputeProviderProfile,
    provider_id_digest: SecureAggregateCommitment,
    artifact_digest: SecureAggregateCommitment,
    parameter_set_digest: Option<SecureAggregateCommitment>,
    review_evidence_digest: Option<SecureAggregateCommitment>,
    external_review_completed: bool,
    production_claim_requested: bool,
}

impl SecretComputeProviderBinding {
    /// Creates a provider binding from typed evidence after enforcing profile and claim boundaries.
    pub fn new_with_evidence(
        kind: SecretComputeProviderKind,
        capability: SecretComputeProviderCapability,
        profile: SecretComputeProviderProfile,
        provider_id_digest: SecureAggregateCommitment,
        artifact_digest: SecureAggregateCommitment,
        evidence: SecretComputeProviderEvidence,
    ) -> Result<Self, SecureAggregateError> {
        let binding = Self {
            kind,
            capability,
            profile,
            provider_id_digest,
            artifact_digest,
            parameter_set_digest: evidence.parameter_set_digest(),
            review_evidence_digest: evidence.review_evidence_digest(),
            external_review_completed: evidence.external_review_completed(),
            production_claim_requested: evidence.production_claim_requested(),
        };
        validate_secret_compute_provider_binding(binding)?;
        Ok(binding)
    }

    /// Creates a provider binding after enforcing profile and claim boundaries.
    ///
    /// Prefer [`Self::new_with_evidence`] for new code so review state and
    /// production claims are represented by typed values rather than booleans.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: SecretComputeProviderKind,
        capability: SecretComputeProviderCapability,
        profile: SecretComputeProviderProfile,
        provider_id_digest: SecureAggregateCommitment,
        artifact_digest: SecureAggregateCommitment,
        parameter_set_digest: Option<SecureAggregateCommitment>,
        review_evidence_digest: Option<SecureAggregateCommitment>,
        external_review_completed: bool,
        production_claim_requested: bool,
    ) -> Result<Self, SecureAggregateError> {
        let review_evidence = if external_review_completed {
            review_evidence_digest.map_or(SecretComputeProviderReviewEvidence::Pending, |digest| {
                SecretComputeProviderReviewEvidence::Completed(digest)
            })
        } else {
            SecretComputeProviderReviewEvidence::Pending
        };
        let usage_claim = if production_claim_requested {
            SecretComputeProviderUsageClaim::Production
        } else {
            SecretComputeProviderUsageClaim::NonProduction
        };
        Self::new_with_evidence(
            kind,
            capability,
            profile,
            provider_id_digest,
            artifact_digest,
            SecretComputeProviderEvidence::new(parameter_set_digest, review_evidence, usage_claim),
        )
    }

    /// Creates a provider binding without validation from typed evidence for Kani boundary proofs.
    #[cfg(kani)]
    pub(crate) const fn unchecked_for_verification_with_evidence(
        kind: SecretComputeProviderKind,
        capability: SecretComputeProviderCapability,
        profile: SecretComputeProviderProfile,
        provider_id_digest: SecureAggregateCommitment,
        artifact_digest: SecureAggregateCommitment,
        evidence: SecretComputeProviderEvidence,
    ) -> Self {
        Self {
            kind,
            capability,
            profile,
            provider_id_digest,
            artifact_digest,
            parameter_set_digest: evidence.parameter_set_digest(),
            review_evidence_digest: evidence.review_evidence_digest(),
            external_review_completed: evidence.external_review_completed(),
            production_claim_requested: evidence.production_claim_requested(),
        }
    }

    /// Returns the provider family.
    #[must_use]
    pub const fn kind(self) -> SecretComputeProviderKind {
        self.kind
    }

    /// Returns the provider capability.
    #[must_use]
    pub const fn capability(self) -> SecretComputeProviderCapability {
        self.capability
    }

    /// Returns the provider profile.
    #[must_use]
    pub const fn profile(self) -> SecretComputeProviderProfile {
        self.profile
    }

    /// Returns the provider identifier digest.
    #[must_use]
    pub const fn provider_id_digest(self) -> SecureAggregateCommitment {
        self.provider_id_digest
    }

    /// Returns the provider artifact digest.
    #[must_use]
    pub const fn artifact_digest(self) -> SecureAggregateCommitment {
        self.artifact_digest
    }

    /// Returns the optional parameter-set digest.
    #[must_use]
    pub const fn parameter_set_digest(self) -> Option<SecureAggregateCommitment> {
        self.parameter_set_digest
    }

    /// Returns the optional external-review evidence digest.
    #[must_use]
    pub const fn review_evidence_digest(self) -> Option<SecureAggregateCommitment> {
        self.review_evidence_digest
    }

    /// Returns whether external review has been completed for the binding.
    #[must_use]
    pub const fn external_review_completed(self) -> bool {
        self.external_review_completed
    }

    /// Returns whether the binding is being used for a production claim.
    #[must_use]
    pub const fn production_claim_requested(self) -> bool {
        self.production_claim_requested
    }
}

/// Validates provider profile, review, and production-claim boundaries.
pub fn validate_secret_compute_provider_binding(
    binding: SecretComputeProviderBinding,
) -> Result<(), SecureAggregateError> {
    match binding.profile() {
        SecretComputeProviderProfile::KivaReferencePrimitivesV1 => {
            if binding.kind() != SecretComputeProviderKind::ReferencePrimitive {
                return Err(SecureAggregateError::ReferencePrimitiveProviderKindMismatch);
            }
            if binding.production_claim_requested() {
                return Err(
                    SecureAggregateError::ReferencePrimitiveProviderProductionClaimRejected,
                );
            }
        }
        SecretComputeProviderProfile::ExternalReviewedProviderV1 => {
            if secret_compute_provider_kind_requires_parameter_set(binding.kind())
                && binding.parameter_set_digest().is_none()
            {
                return Err(SecureAggregateError::SecretComputeProviderParameterSetRequired);
            }
            if binding.production_claim_requested()
                && (!binding.external_review_completed()
                    || binding.review_evidence_digest().is_none())
            {
                return Err(SecureAggregateError::ExternalProviderReviewRequired);
            }
        }
    }
    Ok(())
}

/// Returns the non-reference capability required by a concrete mechanism.
#[must_use]
pub const fn secret_compute_required_capability_for_mechanism(
    mechanism: ConfidentialMechanismKind,
) -> SecretComputeProviderCapability {
    descriptor_for_mechanism(mechanism).required_provider_capability()
}

/// Returns whether the provider kind needs parameter-set evidence.
#[must_use]
pub const fn secret_compute_provider_kind_requires_parameter_set(
    kind: SecretComputeProviderKind,
) -> bool {
    matches!(
        kind,
        SecretComputeProviderKind::FullyHomomorphicEncryption
            | SecretComputeProviderKind::PrivateSetIntersection
            | SecretComputeProviderKind::ThresholdCryptography
    )
}

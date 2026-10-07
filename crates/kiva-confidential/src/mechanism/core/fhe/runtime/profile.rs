#![allow(clippy::wildcard_imports)]

use super::*;

/// FHE scheme family admitted by the CC14 review-gated runtime boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FheRuntimeSchemeFamily {
    /// Exact BFV-style integer sums implemented by an external reviewed runtime.
    BfvIntegerSum,
}

impl FheRuntimeSchemeFamily {
    /// Returns the stable scheme-family label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::BfvIntegerSum => "bfv-integer-sum",
        }
    }
}

/// FHE operation admitted by the CC14 review-gated runtime boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FheRuntimeOperation {
    /// Exact integer sum over externally encrypted bounded inputs.
    Sum,
}

impl FheRuntimeOperation {
    /// Returns the stable operation label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sum => "sum",
        }
    }
}

/// Binding metadata for a reviewed external FHE runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FheRuntimeBinding {
    pub(in crate::mechanism::core) scheme_family: FheRuntimeSchemeFamily,
    pub(in crate::mechanism::core) operation: FheRuntimeOperation,
    pub(in crate::mechanism::core) runtime_artifact_digest: SecureAggregateCommitment,
    pub(in crate::mechanism::core) parameter_set_digest: SecureAggregateCommitment,
    pub(in crate::mechanism::core) review_evidence_digest: SecureAggregateCommitment,
    pub(in crate::mechanism::core) external_review_completed: bool,
}

impl FheRuntimeBinding {
    /// Creates an FHE runtime binding after checking mandatory digest metadata.
    pub fn new(
        scheme_family: FheRuntimeSchemeFamily,
        operation: FheRuntimeOperation,
        runtime_artifact_digest: SecureAggregateCommitment,
        parameter_set_digest: SecureAggregateCommitment,
        review_evidence_digest: SecureAggregateCommitment,
        external_review_completed: bool,
    ) -> Result<Self, SecureAggregateError> {
        let binding = Self {
            scheme_family,
            operation,
            runtime_artifact_digest,
            parameter_set_digest,
            review_evidence_digest,
            external_review_completed,
        };
        validate_fhe_runtime_binding(binding)?;
        Ok(binding)
    }

    /// Returns the selected FHE scheme family.
    #[must_use]
    pub const fn scheme_family(self) -> FheRuntimeSchemeFamily {
        self.scheme_family
    }

    /// Returns the selected FHE operation.
    #[must_use]
    pub const fn operation(self) -> FheRuntimeOperation {
        self.operation
    }

    /// Returns the digest of the isolated FHE runtime artifact.
    #[must_use]
    pub const fn runtime_artifact_digest(self) -> SecureAggregateCommitment {
        self.runtime_artifact_digest
    }

    /// Returns the digest of the selected runtime parameter set.
    #[must_use]
    pub const fn parameter_set_digest(self) -> SecureAggregateCommitment {
        self.parameter_set_digest
    }

    /// Returns the digest of the external cryptographic review evidence.
    #[must_use]
    pub const fn review_evidence_digest(self) -> SecureAggregateCommitment {
        self.review_evidence_digest
    }

    /// Returns whether external cryptographic review has completed.
    #[must_use]
    pub const fn external_review_completed(self) -> bool {
        self.external_review_completed
    }
}

/// Plan for one review-gated FHE sum runtime job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FheRuntimePlan {
    pub(in crate::mechanism::core) release_policy: ReleasePolicy,
    pub(in crate::mechanism::core) runtime_binding: FheRuntimeBinding,
    pub(in crate::mechanism::core) key_epoch: SecureAggregateEnvelopeSequence,
}

impl FheRuntimePlan {
    /// Creates an FHE runtime plan after enforcing n-of-n release semantics.
    pub fn new(
        release_policy: ReleasePolicy,
        runtime_binding: FheRuntimeBinding,
        key_epoch: SecureAggregateEnvelopeSequence,
    ) -> Result<Self, SecureAggregateError> {
        validate_fhe_release_policy(release_policy)?;
        validate_fhe_runtime_binding(runtime_binding)?;
        Ok(Self {
            release_policy,
            runtime_binding,
            key_epoch,
        })
    }

    /// Returns the release policy.
    #[must_use]
    pub const fn release_policy(self) -> ReleasePolicy {
        self.release_policy
    }

    /// Returns the configured participant count.
    #[must_use]
    pub const fn participant_count(self) -> NonZeroUsize {
        self.release_policy.participant_count()
    }

    /// Returns the runtime binding.
    #[must_use]
    pub const fn runtime_binding(self) -> FheRuntimeBinding {
        self.runtime_binding
    }

    /// Returns the key epoch bound to this runtime job.
    #[must_use]
    pub const fn key_epoch(self) -> SecureAggregateEnvelopeSequence {
        self.key_epoch
    }
}

/// Validates the selected external FHE runtime binding.
pub fn validate_fhe_runtime_binding(
    binding: FheRuntimeBinding,
) -> Result<(), SecureAggregateError> {
    match (binding.scheme_family(), binding.operation()) {
        (FheRuntimeSchemeFamily::BfvIntegerSum, FheRuntimeOperation::Sum) => {}
    }
    if artifact_is_empty(&binding.runtime_artifact_digest().bytes())
        || artifact_is_empty(&binding.parameter_set_digest().bytes())
        || artifact_is_empty(&binding.review_evidence_digest().bytes())
    {
        return Err(SecureAggregateError::FheRuntimeBindingMissing);
    }
    Ok(())
}

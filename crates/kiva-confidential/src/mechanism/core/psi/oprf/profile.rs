#![allow(clippy::wildcard_imports)]

use super::*;

/// OPRF suite admitted by the CC11 DH-OPRF PSI runtime boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PsiOprfSuite {
    /// Ristretto255/SHA-512 VOPRF suite, modeled as an external runtime boundary.
    Ristretto255Sha512Voprf,
}

impl PsiOprfSuite {
    /// Returns the stable suite label used in manifests and transcript hashes.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ristretto255Sha512Voprf => "ristretto255-sha512-voprf",
        }
    }
}

/// Binding metadata for the external DH-OPRF PSI runtime selected by CC11.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsiOprfRuntimeBinding {
    pub(in crate::mechanism::core) suite: PsiOprfSuite,
    pub(in crate::mechanism::core) runtime_artifact_digest: SecureAggregateCommitment,
    pub(in crate::mechanism::core) review_evidence_digest: SecureAggregateCommitment,
    pub(in crate::mechanism::core) external_review_completed: bool,
}

impl PsiOprfRuntimeBinding {
    /// Creates a runtime binding after checking that mandatory digests exist.
    pub fn new(
        suite: PsiOprfSuite,
        runtime_artifact_digest: SecureAggregateCommitment,
        review_evidence_digest: SecureAggregateCommitment,
        external_review_completed: bool,
    ) -> Result<Self, SecureAggregateError> {
        let binding = Self {
            suite,
            runtime_artifact_digest,
            review_evidence_digest,
            external_review_completed,
        };
        validate_psi_oprf_runtime_binding(binding)?;
        Ok(binding)
    }

    /// Returns the selected OPRF suite.
    #[must_use]
    pub const fn suite(self) -> PsiOprfSuite {
        self.suite
    }

    /// Returns the digest of the isolated runtime artifact.
    #[must_use]
    pub const fn runtime_artifact_digest(self) -> SecureAggregateCommitment {
        self.runtime_artifact_digest
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

/// Plan for one review-gated DH-OPRF PSI job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsiOprfPlan {
    pub(in crate::mechanism::core) release_policy: ReleasePolicy,
    pub(in crate::mechanism::core) runtime_binding: PsiOprfRuntimeBinding,
}

impl PsiOprfPlan {
    /// Creates a DH-OPRF PSI plan after enforcing n-of-n release semantics.
    pub fn new(
        release_policy: ReleasePolicy,
        runtime_binding: PsiOprfRuntimeBinding,
    ) -> Result<Self, SecureAggregateError> {
        validate_psi_release_policy(release_policy)?;
        validate_psi_oprf_runtime_binding(runtime_binding)?;
        Ok(Self {
            release_policy,
            runtime_binding,
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
    pub const fn runtime_binding(self) -> PsiOprfRuntimeBinding {
        self.runtime_binding
    }
}

/// Validates the selected external DH-OPRF PSI runtime binding.
pub fn validate_psi_oprf_runtime_binding(
    binding: PsiOprfRuntimeBinding,
) -> Result<(), SecureAggregateError> {
    match binding.suite() {
        PsiOprfSuite::Ristretto255Sha512Voprf => {}
    }
    if artifact_is_empty(&binding.runtime_artifact_digest().bytes())
        || artifact_is_empty(&binding.review_evidence_digest().bytes())
    {
        return Err(SecureAggregateError::PsiOprfRuntimeBindingMissing);
    }
    Ok(())
}

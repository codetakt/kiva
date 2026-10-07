#![allow(clippy::wildcard_imports)]

use super::*;

/// FROST suite admitted by the CC13 review-gated threshold-signing boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThresholdFrostSuite {
    /// Ed25519/SHA-512 FROST, implemented by an external reviewed runtime.
    Ed25519Sha512Frost,
}

impl ThresholdFrostSuite {
    /// Returns the stable suite label used in assurance manifests and hashes.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ed25519Sha512Frost => "ed25519-sha512-frost",
        }
    }
}

/// Binding metadata for the external FROST threshold-signing runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThresholdFrostRuntimeBinding {
    pub(in crate::mechanism::core) suite: ThresholdFrostSuite,
    pub(in crate::mechanism::core) runtime_artifact_digest: SecureAggregateCommitment,
    pub(in crate::mechanism::core) review_evidence_digest: SecureAggregateCommitment,
    pub(in crate::mechanism::core) external_review_completed: bool,
}

impl ThresholdFrostRuntimeBinding {
    /// Creates a runtime binding after checking mandatory digest metadata.
    pub fn new(
        suite: ThresholdFrostSuite,
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
        validate_threshold_frost_runtime_binding(binding)?;
        Ok(binding)
    }

    /// Returns the selected FROST suite.
    #[must_use]
    pub const fn suite(self) -> ThresholdFrostSuite {
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

/// Plan for one review-gated FROST threshold key ceremony.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThresholdFrostKeyCeremonyPlan {
    pub(in crate::mechanism::core) release_policy: ReleasePolicy,
    pub(in crate::mechanism::core) runtime_binding: ThresholdFrostRuntimeBinding,
    pub(in crate::mechanism::core) key_epoch: SecureAggregateEnvelopeSequence,
}

impl ThresholdFrostKeyCeremonyPlan {
    /// Creates a FROST key-ceremony plan after enforcing threshold semantics.
    pub fn new(
        release_policy: ReleasePolicy,
        runtime_binding: ThresholdFrostRuntimeBinding,
        key_epoch: SecureAggregateEnvelopeSequence,
    ) -> Result<Self, SecureAggregateError> {
        validate_shamir_release_policy(release_policy)?;
        validate_threshold_frost_runtime_binding(runtime_binding)?;
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

    /// Returns the signing threshold.
    #[must_use]
    pub const fn release_threshold(self) -> NonZeroUsize {
        self.release_policy.release_threshold()
    }

    /// Returns the configured runtime binding.
    #[must_use]
    pub const fn runtime_binding(self) -> ThresholdFrostRuntimeBinding {
        self.runtime_binding
    }

    /// Returns the key epoch produced by the ceremony.
    #[must_use]
    pub const fn key_epoch(self) -> SecureAggregateEnvelopeSequence {
        self.key_epoch
    }
}

/// Validates the selected external FROST threshold-signing runtime binding.
pub fn validate_threshold_frost_runtime_binding(
    binding: ThresholdFrostRuntimeBinding,
) -> Result<(), SecureAggregateError> {
    match binding.suite() {
        ThresholdFrostSuite::Ed25519Sha512Frost => {}
    }
    if artifact_is_empty(&binding.runtime_artifact_digest().bytes())
        || artifact_is_empty(&binding.review_evidence_digest().bytes())
    {
        return Err(SecureAggregateError::ThresholdFrostRuntimeBindingMissing);
    }
    Ok(())
}

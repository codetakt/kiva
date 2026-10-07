#![allow(clippy::wildcard_imports)]

use super::*;

/// Pure fact bitset evaluated before releasing an FHE runtime result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FheRuntimeReleaseFacts {
    bits: u8,
}

impl FheRuntimeReleaseFacts {
    /// Fact bit for an accepted ciphertext transcript.
    pub const CIPHERTEXT_TRANSCRIPT_ACCEPTED: u8 = 1 << 0;
    /// Fact bit for an accepted evaluation.
    pub const EVALUATION_COMPLETED: u8 = 1 << 1;
    /// Fact bit for completed external cryptographic review.
    pub const EXTERNAL_REVIEW_COMPLETED: u8 = 1 << 2;
    /// Fact bit for an active key epoch.
    pub const ACTIVE_KEY_EPOCH: u8 = 1 << 3;
    /// Fact bit for policy-authority approval.
    pub const POLICY_APPROVED: u8 = 1 << 4;
    /// Fact bit for governed release treatment.
    pub const GOVERNED_DATA_PRODUCT: u8 = 1 << 5;

    /// Creates a release fact set from raw bits.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    /// Returns the raw fact bits.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.bits
    }

    /// Returns whether a fact bit is present.
    #[must_use]
    pub const fn contains(self, bit: u8) -> bool {
        self.bits & bit == bit
    }
}

/// Runtime facts evaluated before releasing an FHE runtime result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FheRuntimeReleaseContext {
    facts: FheRuntimeReleaseFacts,
}

impl FheRuntimeReleaseContext {
    /// Creates a release context from normalized facts.
    #[must_use]
    pub const fn new(facts: FheRuntimeReleaseFacts) -> Self {
        Self { facts }
    }

    /// Returns the release fact set.
    #[must_use]
    pub const fn facts(self) -> FheRuntimeReleaseFacts {
        self.facts
    }

    /// Returns whether ciphertext metadata was accepted.
    #[must_use]
    pub const fn ciphertext_transcript_accepted(self) -> bool {
        self.facts()
            .contains(FheRuntimeReleaseFacts::CIPHERTEXT_TRANSCRIPT_ACCEPTED)
    }

    /// Returns whether evaluation completed before release.
    #[must_use]
    pub const fn evaluation_completed(self) -> bool {
        self.facts()
            .contains(FheRuntimeReleaseFacts::EVALUATION_COMPLETED)
    }

    /// Returns whether external cryptographic review has completed.
    #[must_use]
    pub const fn external_review_completed(self) -> bool {
        self.facts()
            .contains(FheRuntimeReleaseFacts::EXTERNAL_REVIEW_COMPLETED)
    }

    /// Returns whether the key epoch is active.
    #[must_use]
    pub const fn active_key_epoch(self) -> bool {
        self.facts()
            .contains(FheRuntimeReleaseFacts::ACTIVE_KEY_EPOCH)
    }

    /// Returns whether policy authority approved release.
    #[must_use]
    pub const fn policy_approved(self) -> bool {
        self.facts()
            .contains(FheRuntimeReleaseFacts::POLICY_APPROVED)
    }

    /// Returns whether the output has governed release treatment.
    #[must_use]
    pub const fn governed_data_product(self) -> bool {
        self.facts()
            .contains(FheRuntimeReleaseFacts::GOVERNED_DATA_PRODUCT)
    }
}

/// Governed release metadata produced by the FHE runtime boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FheRuntimeReleasedAggregateMetadata {
    operation: FheRuntimeOperation,
    participant_count: NonZeroUsize,
    evaluated_ciphertext_digest: SecureAggregateCommitment,
    evaluation_digest: SecureAggregateCommitment,
}

impl FheRuntimeReleasedAggregateMetadata {
    /// Returns the released operation.
    #[must_use]
    pub const fn operation(self) -> FheRuntimeOperation {
        self.operation
    }

    /// Returns the contributing participant count.
    #[must_use]
    pub const fn participant_count(self) -> NonZeroUsize {
        self.participant_count
    }

    /// Returns the evaluated ciphertext digest backing the release.
    #[must_use]
    pub const fn evaluated_ciphertext_digest(self) -> SecureAggregateCommitment {
        self.evaluated_ciphertext_digest
    }

    /// Returns the accepted evaluation digest backing the release.
    #[must_use]
    pub const fn evaluation_digest(self) -> SecureAggregateCommitment {
        self.evaluation_digest
    }
}

/// Validates the fail-closed release gate for an FHE runtime result.
pub const fn validate_fhe_runtime_release(
    context: FheRuntimeReleaseContext,
) -> Result<(), SecureAggregateError> {
    if !context.ciphertext_transcript_accepted() {
        return Err(SecureAggregateError::FheRuntimeReleaseRequiresCiphertextTranscript);
    }
    if !context.evaluation_completed() {
        return Err(SecureAggregateError::FheRuntimeReleaseRequiresEvaluation);
    }
    if !context.external_review_completed() {
        return Err(SecureAggregateError::FheRuntimeReleaseRequiresExternalReview);
    }
    if !context.active_key_epoch() {
        return Err(SecureAggregateError::FheRuntimeReleaseRequiresActiveKeyEpoch);
    }
    if !context.policy_approved() {
        return Err(SecureAggregateError::FheRuntimeReleaseRequiresPolicyApproval);
    }
    if !context.governed_data_product() {
        return Err(SecureAggregateError::FheRuntimeReleaseRequiresGovernedDataProduct);
    }
    Ok(())
}

/// Releases governed FHE aggregate metadata after all runtime gates pass.
pub const fn release_fhe_runtime_sum_metadata(
    evaluation: AcceptedFheRuntimeEvaluation,
    context: FheRuntimeReleaseContext,
) -> Result<FheRuntimeReleasedAggregateMetadata, SecureAggregateError> {
    match validate_fhe_runtime_release(context) {
        Ok(()) => Ok(FheRuntimeReleasedAggregateMetadata {
            operation: evaluation.operation(),
            participant_count: evaluation.participant_count(),
            evaluated_ciphertext_digest: evaluation.record().evaluated_ciphertext_digest(),
            evaluation_digest: evaluation.record().evaluation_digest(),
        }),
        Err(error) => Err(error),
    }
}

#![allow(clippy::wildcard_imports)]

use super::*;

/// Pure fact bitset evaluated by the FHE feasibility release gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FheFeasibilityReleaseFacts {
    bits: u8,
}

impl FheFeasibilityReleaseFacts {
    /// Fact bit for completed evaluation.
    pub const EVALUATION_COMPLETED: u8 = 1 << 0;
    /// Fact bit for policy-authority approval.
    pub const POLICY_APPROVED: u8 = 1 << 1;
    /// Fact bit for governed release treatment.
    pub const GOVERNED_DATA_PRODUCT: u8 = 1 << 2;
    /// Fact bit for clear plaintext/ciphertext boundary.
    pub const PLAINTEXT_BOUNDARY_CLEAR: u8 = 1 << 3;

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

/// Runtime facts required before releasing an FHE feasibility result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FheFeasibilityReleaseContext {
    facts: FheFeasibilityReleaseFacts,
}

impl FheFeasibilityReleaseContext {
    /// Creates an FHE feasibility release context from normalized facts.
    #[must_use]
    pub const fn new(facts: FheFeasibilityReleaseFacts) -> Self {
        Self { facts }
    }

    /// Returns the release fact set.
    #[must_use]
    pub const fn facts(self) -> FheFeasibilityReleaseFacts {
        self.facts
    }

    /// Returns whether evaluation completed before release.
    #[must_use]
    pub const fn evaluation_completed(self) -> bool {
        self.facts()
            .contains(FheFeasibilityReleaseFacts::EVALUATION_COMPLETED)
    }

    /// Returns whether the policy authority approved release.
    #[must_use]
    pub const fn policy_approved(self) -> bool {
        self.facts()
            .contains(FheFeasibilityReleaseFacts::POLICY_APPROVED)
    }

    /// Returns whether the output has governed release treatment.
    #[must_use]
    pub const fn governed_data_product(self) -> bool {
        self.facts()
            .contains(FheFeasibilityReleaseFacts::GOVERNED_DATA_PRODUCT)
    }

    /// Returns whether no plaintext/ciphertext boundary violation was observed.
    #[must_use]
    pub const fn plaintext_boundary_clear(self) -> bool {
        self.facts()
            .contains(FheFeasibilityReleaseFacts::PLAINTEXT_BOUNDARY_CLEAR)
    }
}

/// Governed release value produced by the deterministic FHE feasibility profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FheFeasibilityReleasedAggregate {
    job_id: SecureAggregateTranscriptId,
    operation: FheFeasibilityOperation,
    participant_count: NonZeroUsize,
    aggregate_value: u64,
    evaluated_ciphertext_digest: SecureAggregateCommitment,
}

impl FheFeasibilityReleasedAggregate {
    /// Returns the job identifier.
    #[must_use]
    pub const fn job_id(self) -> SecureAggregateTranscriptId {
        self.job_id
    }

    /// Returns the released operation.
    #[must_use]
    pub const fn operation(self) -> FheFeasibilityOperation {
        self.operation
    }

    /// Returns the contributing participant count.
    #[must_use]
    pub const fn participant_count(self) -> NonZeroUsize {
        self.participant_count
    }

    /// Returns the released aggregate value.
    #[must_use]
    pub const fn aggregate_value(self) -> u64 {
        self.aggregate_value
    }

    /// Returns the evaluated ciphertext digest backing the release.
    #[must_use]
    pub const fn evaluated_ciphertext_digest(self) -> SecureAggregateCommitment {
        self.evaluated_ciphertext_digest
    }
}

/// Validates the fail-closed release gate for an FHE feasibility result.
pub const fn validate_fhe_feasibility_release(
    context: FheFeasibilityReleaseContext,
) -> Result<(), SecureAggregateError> {
    if !context.evaluation_completed() {
        return Err(SecureAggregateError::FheReleaseRequiresEvaluation);
    }
    if !context.policy_approved() {
        return Err(SecureAggregateError::FheReleaseRequiresPolicyApproval);
    }
    if !context.governed_data_product() {
        return Err(SecureAggregateError::FheReleaseRequiresGovernedDataProduct);
    }
    if !context.plaintext_boundary_clear() {
        return Err(SecureAggregateError::FhePlaintextBoundaryViolation);
    }
    Ok(())
}

/// Decrypts the deterministic FHE feasibility sum fixture after release approval.
///
/// This is fixture decryption for feasibility tests only. Production FHE key
/// generation, encryption, evaluation, decryption, and bootstrapping remain
/// outside the CC6 claim.
pub const fn release_fhe_feasibility_sum_fixture(
    evaluation: FheFeasibilityEvaluation,
    context: FheFeasibilityReleaseContext,
) -> Result<FheFeasibilityReleasedAggregate, SecureAggregateError> {
    match validate_fhe_feasibility_release(context) {
        Ok(()) => Ok(FheFeasibilityReleasedAggregate {
            job_id: evaluation.job_id(),
            operation: evaluation.operation(),
            participant_count: evaluation.participant_count(),
            aggregate_value: evaluation.aggregate_value_fixture(),
            evaluated_ciphertext_digest: evaluation.evaluated_ciphertext_digest(),
        }),
        Err(error) => Err(error),
    }
}

#![allow(clippy::wildcard_imports)]

use super::*;

/// Pure fact bitset evaluated before releasing a DH-OPRF PSI result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsiOprfReleaseFacts {
    bits: u8,
}

impl PsiOprfReleaseFacts {
    /// Fact bit for a completed intersection transcript.
    pub const INTERSECTION_COMPUTED: u8 = 1 << 0;
    /// Fact bit for policy-authority approval.
    pub const POLICY_APPROVED: u8 = 1 << 1;
    /// Fact bit for governed release treatment.
    pub const GOVERNED_DATA_PRODUCT: u8 = 1 << 2;
    /// Fact bit for completed external cryptographic review.
    pub const EXTERNAL_REVIEW_COMPLETED: u8 = 1 << 3;
    /// Fact bit for active admitted participants.
    pub const PARTICIPANTS_ACTIVE: u8 = 1 << 4;

    /// Creates a fact set from raw bits.
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

/// Runtime facts evaluated before releasing a DH-OPRF PSI result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsiOprfReleaseContext {
    facts: PsiOprfReleaseFacts,
}

impl PsiOprfReleaseContext {
    /// Creates a release context from normalized facts.
    #[must_use]
    pub const fn new(facts: PsiOprfReleaseFacts) -> Self {
        Self { facts }
    }

    /// Returns the release fact set.
    #[must_use]
    pub const fn facts(self) -> PsiOprfReleaseFacts {
        self.facts
    }

    /// Returns whether the intersection was computed.
    #[must_use]
    pub const fn intersection_computed(self) -> bool {
        self.facts()
            .contains(PsiOprfReleaseFacts::INTERSECTION_COMPUTED)
    }

    /// Returns whether policy authority approved release.
    #[must_use]
    pub const fn policy_approved(self) -> bool {
        self.facts().contains(PsiOprfReleaseFacts::POLICY_APPROVED)
    }

    /// Returns whether output has governed release treatment.
    #[must_use]
    pub const fn governed_data_product(self) -> bool {
        self.facts()
            .contains(PsiOprfReleaseFacts::GOVERNED_DATA_PRODUCT)
    }

    /// Returns whether the external cryptographic review has completed.
    #[must_use]
    pub const fn external_review_completed(self) -> bool {
        self.facts()
            .contains(PsiOprfReleaseFacts::EXTERNAL_REVIEW_COMPLETED)
    }

    /// Returns whether participants are active and admitted.
    #[must_use]
    pub const fn participants_active(self) -> bool {
        self.facts()
            .contains(PsiOprfReleaseFacts::PARTICIPANTS_ACTIVE)
    }
}

/// Governed release value for a DH-OPRF PSI result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsiOprfReleasedIntersection {
    job_id: SecureAggregateTranscriptId,
    participant_count: NonZeroUsize,
    intersection_elements: Vec<PsiOprfElement>,
    transcript_digest: SecureAggregateCommitment,
}

impl PsiOprfReleasedIntersection {
    /// Returns the job identifier.
    #[must_use]
    pub const fn job_id(&self) -> SecureAggregateTranscriptId {
        self.job_id
    }

    /// Returns the participant count.
    #[must_use]
    pub const fn participant_count(&self) -> NonZeroUsize {
        self.participant_count
    }

    /// Returns released intersection elements only.
    #[must_use]
    pub fn intersection_elements(&self) -> &[PsiOprfElement] {
        &self.intersection_elements
    }

    /// Returns the accepted transcript digest.
    #[must_use]
    pub const fn transcript_digest(&self) -> SecureAggregateCommitment {
        self.transcript_digest
    }
}

/// Validates the fail-closed release gate for a DH-OPRF PSI result.
pub const fn validate_psi_oprf_release(
    context: PsiOprfReleaseContext,
) -> Result<(), SecureAggregateError> {
    if !context.intersection_computed() {
        return Err(SecureAggregateError::PsiOprfReleaseRequiresIntersection);
    }
    if !context.participants_active() {
        return Err(SecureAggregateError::PsiOprfReleaseRequiresActiveParticipants);
    }
    if !context.external_review_completed() {
        return Err(SecureAggregateError::PsiOprfExternalReviewRequired);
    }
    if !context.policy_approved() {
        return Err(SecureAggregateError::PsiOprfReleaseRequiresPolicyApproval);
    }
    if !context.governed_data_product() {
        return Err(SecureAggregateError::PsiOprfReleaseRequiresGovernedDataProduct);
    }
    Ok(())
}

/// Releases only governed DH-OPRF PSI intersection elements after all gates pass.
pub fn release_psi_oprf_intersection(
    result: &PsiOprfIntersectionResult,
    context: PsiOprfReleaseContext,
) -> Result<PsiOprfReleasedIntersection, SecureAggregateError> {
    validate_psi_oprf_release(context)?;
    Ok(PsiOprfReleasedIntersection {
        job_id: result.record().job_id(),
        participant_count: result.participant_count(),
        intersection_elements: result.intersection_elements().to_vec(),
        transcript_digest: result.record().transcript_digest(),
    })
}

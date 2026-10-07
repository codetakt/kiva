#![allow(clippy::wildcard_imports)]

use super::*;

/// One participant's Shamir share for threshold reconstruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShamirShare {
    participant_index: NonZeroUsize,
    value: SecureAggregateFieldElement,
}

impl ShamirShare {
    /// Creates a Shamir share for a one-based participant index.
    #[must_use]
    pub const fn new(participant_index: NonZeroUsize, value: SecureAggregateFieldElement) -> Self {
        Self {
            participant_index,
            value,
        }
    }

    /// Returns the one-based participant index used as the interpolation point.
    #[must_use]
    pub const fn participant_index(self) -> NonZeroUsize {
        self.participant_index
    }

    /// Returns the share value.
    #[must_use]
    pub const fn value(self) -> SecureAggregateFieldElement {
        self.value
    }
}

/// Nonce used to open one hash-committed Shamir share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "[u8; 32]", into = "[u8; 32]")]
pub struct ShamirShareOpeningNonce {
    pub(super) bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES],
}

impl ShamirShareOpeningNonce {
    /// Creates an opening nonce after rejecting the reserved empty value.
    pub fn new(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, SecureAggregateError> {
        if artifact_is_empty(&bytes) {
            return Err(SecureAggregateError::EmptyShamirShareOpeningNonce);
        }
        Ok(Self { bytes })
    }

    /// Returns the canonical bytes for this opening nonce.
    #[must_use]
    pub const fn bytes(self) -> [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
        self.bytes
    }
}

impl TryFrom<[u8; SECURE_AGGREGATE_ARTIFACT_BYTES]> for ShamirShareOpeningNonce {
    type Error = SecureAggregateError;

    fn try_from(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, Self::Error> {
        Self::new(bytes)
    }
}

impl From<ShamirShareOpeningNonce> for [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
    fn from(value: ShamirShareOpeningNonce) -> Self {
        value.bytes()
    }
}

/// One opened Shamir share with its published verification commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiableShamirShare {
    share: ShamirShare,
    commitment: SecureAggregateCommitment,
    opening_nonce: ShamirShareOpeningNonce,
}

impl VerifiableShamirShare {
    /// Creates a verifiable Shamir share opening.
    #[must_use]
    pub const fn new(
        share: ShamirShare,
        commitment: SecureAggregateCommitment,
        opening_nonce: ShamirShareOpeningNonce,
    ) -> Self {
        Self {
            share,
            commitment,
            opening_nonce,
        }
    }

    /// Returns the opened Shamir share.
    #[must_use]
    pub const fn share(self) -> ShamirShare {
        self.share
    }

    /// Returns the expected commitment for this share opening.
    #[must_use]
    pub const fn commitment(self) -> SecureAggregateCommitment {
        self.commitment
    }

    /// Returns the opening nonce.
    #[must_use]
    pub const fn opening_nonce(self) -> ShamirShareOpeningNonce {
        self.opening_nonce
    }
}

/// Shamir share that has passed commitment-opening verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedShamirShare {
    share: ShamirShare,
    commitment: SecureAggregateCommitment,
}

impl VerifiedShamirShare {
    /// Creates a verified Shamir share record.
    #[must_use]
    pub const fn new(share: ShamirShare, commitment: SecureAggregateCommitment) -> Self {
        Self { share, commitment }
    }

    /// Returns the verified Shamir share.
    #[must_use]
    pub const fn share(self) -> ShamirShare {
        self.share
    }

    /// Returns the commitment that verified this share.
    #[must_use]
    pub const fn commitment(self) -> SecureAggregateCommitment {
        self.commitment
    }
}

/// Outcome of adjudicating a complaint against one Shamir share opening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShamirShareComplaintOutcome {
    /// The complaint is sustained because the share opening failed verification.
    Sustained,
    /// The complaint is rejected because the share opening verified.
    Rejected,
}

/// Plan for Shamir threshold reconstruction over Kiva's fixed finite field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "ShamirThresholdPlanUnchecked",
    into = "ShamirThresholdPlanUnchecked"
)]
pub struct ShamirThresholdPlan {
    pub(super) release_policy: ReleasePolicy,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct ShamirThresholdPlanUnchecked {
    release_policy: ReleasePolicy,
}

impl ShamirThresholdPlan {
    /// Creates a Shamir threshold plan after enforcing threshold-specific rules.
    pub fn new(release_policy: ReleasePolicy) -> Result<Self, SecureAggregateError> {
        validate_shamir_release_policy(release_policy)?;
        Ok(Self { release_policy })
    }

    /// Returns the release policy.
    #[must_use]
    pub const fn release_policy(self) -> ReleasePolicy {
        self.release_policy
    }

    /// Returns the participant count.
    #[must_use]
    pub const fn participant_count(self) -> NonZeroUsize {
        self.release_policy.participant_count()
    }

    /// Returns the reconstruction threshold.
    #[must_use]
    pub const fn threshold(self) -> NonZeroUsize {
        self.release_policy.release_threshold()
    }
}

impl TryFrom<ShamirThresholdPlanUnchecked> for ShamirThresholdPlan {
    type Error = SecureAggregateError;

    fn try_from(value: ShamirThresholdPlanUnchecked) -> Result<Self, Self::Error> {
        Self::new(value.release_policy)
    }
}

impl From<ShamirThresholdPlan> for ShamirThresholdPlanUnchecked {
    fn from(value: ShamirThresholdPlan) -> Self {
        Self {
            release_policy: value.release_policy(),
        }
    }
}

/// Derives the hash commitment for one opened Shamir share.
///
/// This commitment binds the transcript, one-based participant index, field
/// value, and opening nonce. It is intentionally a share-opening commitment:
/// Kiva metadata may persist commitment identifiers, but it must not publish
/// share values or opening nonces.
pub fn derive_shamir_share_opening_commitment(
    transcript_id: SecureAggregateTranscriptId,
    share: ShamirShare,
    opening_nonce: ShamirShareOpeningNonce,
) -> Result<SecureAggregateCommitment, SecureAggregateError> {
    let mut hasher = Sha256::new();
    hasher.update(SHAMIR_SHARE_OPENING_COMMITMENT_DOMAIN);
    hasher.update(transcript_id.bytes());
    hasher.update((share.participant_index().get() as u128).to_be_bytes());
    hasher.update(share.value().value().to_be_bytes());
    hasher.update(opening_nonce.bytes());
    let digest = hasher.finalize();
    let mut commitment = [0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES];
    commitment.copy_from_slice(&digest);
    SecureAggregateCommitment::new(commitment)
}

/// Verifies one hash-committed Shamir share opening.
pub fn verify_shamir_share_opening(
    transcript_id: SecureAggregateTranscriptId,
    opening: VerifiableShamirShare,
) -> Result<VerifiedShamirShare, SecureAggregateError> {
    let actual = derive_shamir_share_opening_commitment(
        transcript_id,
        opening.share(),
        opening.opening_nonce(),
    )?;
    if actual != opening.commitment() {
        return Err(SecureAggregateError::ShamirShareCommitmentMismatch);
    }
    Ok(VerifiedShamirShare::new(
        opening.share(),
        opening.commitment(),
    ))
}

/// Verifies a batch of Shamir share openings.
pub fn verify_shamir_share_openings(
    transcript_id: SecureAggregateTranscriptId,
    openings: &[VerifiableShamirShare],
) -> Result<Vec<VerifiedShamirShare>, SecureAggregateError> {
    openings
        .iter()
        .map(|opening| verify_shamir_share_opening(transcript_id, *opening))
        .collect()
}

/// Resolves one Shamir share complaint from an already-computed verification fact.
#[must_use]
pub const fn resolve_shamir_share_complaint(opening_verified: bool) -> ShamirShareComplaintOutcome {
    if opening_verified {
        ShamirShareComplaintOutcome::Rejected
    } else {
        ShamirShareComplaintOutcome::Sustained
    }
}

/// Adjudicates a complaint against one Shamir share opening.
#[must_use]
pub fn adjudicate_shamir_share_complaint(
    transcript_id: SecureAggregateTranscriptId,
    opening: VerifiableShamirShare,
) -> ShamirShareComplaintOutcome {
    resolve_shamir_share_complaint(verify_shamir_share_opening(transcript_id, opening).is_ok())
}

/// Reconstructs a Shamir secret only after every supplied share opening verifies.
pub fn reconstruct_verified_shamir_secret(
    plan: ShamirThresholdPlan,
    transcript_id: SecureAggregateTranscriptId,
    openings: &[VerifiableShamirShare],
) -> Result<SecureAggregateFieldElement, SecureAggregateError> {
    let verified = verify_shamir_share_openings(transcript_id, openings)?;
    let shares = verified
        .iter()
        .map(|verified_share| verified_share.share())
        .collect::<Vec<_>>();
    reconstruct_shamir_secret(plan, &shares)
}

/// Reconstructs a Shamir secret from at least threshold unique shares.
pub fn reconstruct_shamir_secret(
    plan: ShamirThresholdPlan,
    shares: &[ShamirShare],
) -> Result<SecureAggregateFieldElement, SecureAggregateError> {
    validate_shamir_shares(plan, shares)?;

    let threshold = plan.threshold().get();
    let reconstruction_shares = &shares[..threshold];
    let mut secret = SecureAggregateFieldElement::ZERO;

    for (offset, share) in reconstruction_shares.iter().enumerate() {
        let xi = participant_index_field_element(share.participant_index())?;
        let mut numerator = SecureAggregateFieldElement::ONE;
        let mut denominator = SecureAggregateFieldElement::ONE;

        for (other_offset, other_share) in reconstruction_shares.iter().enumerate() {
            if offset == other_offset {
                continue;
            }
            let xj = participant_index_field_element(other_share.participant_index())?;
            numerator = numerator.mul_mod(SecureAggregateFieldElement::ZERO.sub_mod(xj));
            denominator = denominator.mul_mod(xi.sub_mod(xj));
        }

        let coefficient = numerator.mul_mod(denominator.inv_mod()?);
        secret = secret.add_mod(share.value().mul_mod(coefficient));
    }

    Ok(secret)
}

fn validate_shamir_shares(
    plan: ShamirThresholdPlan,
    shares: &[ShamirShare],
) -> Result<(), SecureAggregateError> {
    if shares.len() < plan.threshold().get() {
        return Err(SecureAggregateError::ShamirShareCountBelowThreshold);
    }

    for (offset, share) in shares.iter().enumerate() {
        if share.participant_index().get() > plan.participant_count().get() {
            return Err(SecureAggregateError::ParticipantIndexOutOfRange);
        }
        participant_index_field_element(share.participant_index())?;
        if shares[..offset]
            .iter()
            .any(|previous| previous.participant_index() == share.participant_index())
        {
            return Err(SecureAggregateError::DuplicateParticipantIndex);
        }
    }

    Ok(())
}

fn participant_index_field_element(
    participant_index: NonZeroUsize,
) -> Result<SecureAggregateFieldElement, SecureAggregateError> {
    let Ok(value) = u64::try_from(participant_index.get()) else {
        return Err(SecureAggregateError::ParticipantIndexOutOfRange);
    };
    SecureAggregateFieldElement::new(value)
        .map_err(|_error| SecureAggregateError::ParticipantIndexOutOfRange)
}

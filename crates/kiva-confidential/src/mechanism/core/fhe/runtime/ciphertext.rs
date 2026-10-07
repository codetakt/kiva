#![allow(clippy::wildcard_imports)]

use super::*;

/// Pure fact bitset evaluated before accepting FHE ciphertext metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FheRuntimeCiphertextTranscriptFacts {
    bits: u8,
}

impl FheRuntimeCiphertextTranscriptFacts {
    /// Fact bit for a configured runtime binding.
    pub const RUNTIME_BOUND: u8 = 1 << 0;
    /// Fact bit for completed external cryptographic review.
    pub const EXTERNAL_REVIEW_COMPLETED: u8 = 1 << 1;
    /// Fact bit for exactly one ciphertext record per participant.
    pub const PARTICIPANT_COUNT_MATCHES: u8 = 1 << 2;
    /// Fact bit for active admitted participants.
    pub const PARTICIPANTS_ACTIVE: u8 = 1 << 3;
    /// Fact bit for key-custody evidence.
    pub const KEY_CUSTODY_PRESENT: u8 = 1 << 4;
    /// Fact bit for parameter-set binding.
    pub const PARAMETER_SET_BOUND: u8 = 1 << 5;
    /// Fact bit for ciphertext transcript digest match.
    pub const DIGEST_MATCHES: u8 = 1 << 6;
    /// Fact bit for absence of replay.
    pub const NO_REPLAY: u8 = 1 << 7;

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

/// Normalized facts for accepting FHE ciphertext transcript metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FheRuntimeCiphertextTranscriptGateContext {
    facts: FheRuntimeCiphertextTranscriptFacts,
}

impl FheRuntimeCiphertextTranscriptGateContext {
    /// Creates a ciphertext-transcript gate context from normalized facts.
    #[must_use]
    pub const fn new(facts: FheRuntimeCiphertextTranscriptFacts) -> Self {
        Self { facts }
    }

    /// Returns the ciphertext-transcript fact set.
    #[must_use]
    pub const fn facts(self) -> FheRuntimeCiphertextTranscriptFacts {
        self.facts
    }

    /// Returns whether the runtime is configured.
    #[must_use]
    pub const fn runtime_bound(self) -> bool {
        self.facts()
            .contains(FheRuntimeCiphertextTranscriptFacts::RUNTIME_BOUND)
    }

    /// Returns whether external cryptographic review has completed.
    #[must_use]
    pub const fn external_review_completed(self) -> bool {
        self.facts()
            .contains(FheRuntimeCiphertextTranscriptFacts::EXTERNAL_REVIEW_COMPLETED)
    }

    /// Returns whether the participant ciphertext count matches the plan.
    #[must_use]
    pub const fn participant_count_matches(self) -> bool {
        self.facts()
            .contains(FheRuntimeCiphertextTranscriptFacts::PARTICIPANT_COUNT_MATCHES)
    }

    /// Returns whether all participants are active.
    #[must_use]
    pub const fn participants_active(self) -> bool {
        self.facts()
            .contains(FheRuntimeCiphertextTranscriptFacts::PARTICIPANTS_ACTIVE)
    }

    /// Returns whether key-custody evidence is present.
    #[must_use]
    pub const fn key_custody_present(self) -> bool {
        self.facts()
            .contains(FheRuntimeCiphertextTranscriptFacts::KEY_CUSTODY_PRESENT)
    }

    /// Returns whether every ciphertext is bound to the selected parameter set.
    #[must_use]
    pub const fn parameter_set_bound(self) -> bool {
        self.facts()
            .contains(FheRuntimeCiphertextTranscriptFacts::PARAMETER_SET_BOUND)
    }

    /// Returns whether the canonical transcript digest matched.
    #[must_use]
    pub const fn digest_matches(self) -> bool {
        self.facts()
            .contains(FheRuntimeCiphertextTranscriptFacts::DIGEST_MATCHES)
    }

    /// Returns whether no replay was detected.
    #[must_use]
    pub const fn no_replay(self) -> bool {
        self.facts()
            .contains(FheRuntimeCiphertextTranscriptFacts::NO_REPLAY)
    }
}

/// Participant-side state for one FHE runtime ciphertext record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FheRuntimeParticipantCiphertextStatus {
    /// Participant submitted complete ciphertext metadata.
    Submitted,
    /// Participant was removed or inactive for this runtime job.
    Removed,
    /// Participant aborted before complete ciphertext metadata existed.
    Aborted,
}

/// One participant's metadata-only FHE ciphertext record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FheRuntimeParticipantCiphertext {
    participant_index: NonZeroUsize,
    status: FheRuntimeParticipantCiphertextStatus,
    ciphertext_digest: SecureAggregateCommitment,
    parameter_set_digest: SecureAggregateCommitment,
    key_custody_digest: Option<SecureAggregateCommitment>,
}

impl FheRuntimeParticipantCiphertext {
    /// Creates one participant's FHE ciphertext metadata.
    #[must_use]
    pub const fn new(
        participant_index: NonZeroUsize,
        status: FheRuntimeParticipantCiphertextStatus,
        ciphertext_digest: SecureAggregateCommitment,
        parameter_set_digest: SecureAggregateCommitment,
        key_custody_digest: Option<SecureAggregateCommitment>,
    ) -> Self {
        Self {
            participant_index,
            status,
            ciphertext_digest,
            parameter_set_digest,
            key_custody_digest,
        }
    }

    /// Returns the one-based participant index.
    #[must_use]
    pub const fn participant_index(self) -> NonZeroUsize {
        self.participant_index
    }

    /// Returns the participant ciphertext status.
    #[must_use]
    pub const fn status(self) -> FheRuntimeParticipantCiphertextStatus {
        self.status
    }

    /// Returns the digest of the opaque ciphertext payload.
    #[must_use]
    pub const fn ciphertext_digest(self) -> SecureAggregateCommitment {
        self.ciphertext_digest
    }

    /// Returns the digest of the runtime parameter set used for encryption.
    #[must_use]
    pub const fn parameter_set_digest(self) -> SecureAggregateCommitment {
        self.parameter_set_digest
    }

    /// Returns metadata proving key custody for this ciphertext.
    #[must_use]
    pub const fn key_custody_digest(self) -> Option<SecureAggregateCommitment> {
        self.key_custody_digest
    }
}

/// Replay metadata for one accepted FHE ciphertext transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FheRuntimeCiphertextTranscriptRecord {
    job_id: SecureAggregateTranscriptId,
    transcript_digest: SecureAggregateCommitment,
    key_epoch: SecureAggregateEnvelopeSequence,
}

impl FheRuntimeCiphertextTranscriptRecord {
    /// Creates accepted FHE ciphertext transcript replay metadata.
    #[must_use]
    pub const fn new(
        job_id: SecureAggregateTranscriptId,
        transcript_digest: SecureAggregateCommitment,
        key_epoch: SecureAggregateEnvelopeSequence,
    ) -> Self {
        Self {
            job_id,
            transcript_digest,
            key_epoch,
        }
    }

    /// Returns the FHE runtime job identifier.
    #[must_use]
    pub const fn job_id(self) -> SecureAggregateTranscriptId {
        self.job_id
    }

    /// Returns the accepted ciphertext transcript digest.
    #[must_use]
    pub const fn transcript_digest(self) -> SecureAggregateCommitment {
        self.transcript_digest
    }

    /// Returns the key epoch bound to this transcript.
    #[must_use]
    pub const fn key_epoch(self) -> SecureAggregateEnvelopeSequence {
        self.key_epoch
    }

    /// Returns whether this record conflicts with a candidate transcript.
    #[must_use]
    pub fn conflicts_with(
        self,
        job_id: SecureAggregateTranscriptId,
        transcript_digest: SecureAggregateCommitment,
    ) -> bool {
        self.job_id == job_id || self.transcript_digest.bytes() == transcript_digest.bytes()
    }
}

/// Accepted metadata-only FHE ciphertext transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptedFheRuntimeCiphertextTranscript {
    record: FheRuntimeCiphertextTranscriptRecord,
    participant_count: NonZeroUsize,
    runtime_binding: FheRuntimeBinding,
}

impl AcceptedFheRuntimeCiphertextTranscript {
    /// Creates an accepted FHE ciphertext transcript.
    #[must_use]
    pub const fn new(
        record: FheRuntimeCiphertextTranscriptRecord,
        participant_count: NonZeroUsize,
        runtime_binding: FheRuntimeBinding,
    ) -> Self {
        Self {
            record,
            participant_count,
            runtime_binding,
        }
    }

    /// Returns the replay record that must be persisted.
    #[must_use]
    pub const fn record(self) -> FheRuntimeCiphertextTranscriptRecord {
        self.record
    }

    /// Returns the configured participant count.
    #[must_use]
    pub const fn participant_count(self) -> NonZeroUsize {
        self.participant_count
    }

    /// Returns the runtime binding that admitted this transcript.
    #[must_use]
    pub const fn runtime_binding(self) -> FheRuntimeBinding {
        self.runtime_binding
    }
}

/// Validates the pure fail-closed FHE ciphertext transcript admission gate.
pub const fn validate_fhe_runtime_ciphertext_transcript_gate_context(
    context: FheRuntimeCiphertextTranscriptGateContext,
) -> Result<(), SecureAggregateError> {
    if !context.runtime_bound() {
        return Err(SecureAggregateError::FheRuntimeBindingMissing);
    }
    if !context.external_review_completed() {
        return Err(SecureAggregateError::FheRuntimeExternalReviewRequired);
    }
    if !context.participant_count_matches() {
        return Err(SecureAggregateError::FheRuntimeCiphertextCountMismatch);
    }
    if !context.participants_active() {
        return Err(SecureAggregateError::FheRuntimeCiphertextParticipantInactive);
    }
    if !context.key_custody_present() {
        return Err(SecureAggregateError::FheRuntimeKeyCustodyMissing);
    }
    if !context.parameter_set_bound() {
        return Err(SecureAggregateError::FheRuntimeParameterSetRejected);
    }
    if !context.digest_matches() {
        return Err(SecureAggregateError::FheRuntimeCiphertextDigestMismatch);
    }
    if !context.no_replay() {
        return Err(SecureAggregateError::FheRuntimeCiphertextReplayDetected);
    }
    Ok(())
}

/// Accepts metadata for externally generated FHE ciphertexts after fail-closed checks.
pub fn accept_fhe_runtime_ciphertext_transcript(
    plan: FheRuntimePlan,
    job_id: SecureAggregateTranscriptId,
    expected_transcript_digest: SecureAggregateCommitment,
    existing_records: &[FheRuntimeCiphertextTranscriptRecord],
    ciphertexts: &[FheRuntimeParticipantCiphertext],
) -> Result<AcceptedFheRuntimeCiphertextTranscript, SecureAggregateError> {
    validate_fhe_runtime_ciphertext_transcript(
        plan,
        job_id,
        expected_transcript_digest,
        existing_records,
        ciphertexts,
    )?;
    let transcript_digest =
        derive_fhe_runtime_ciphertext_transcript_digest(plan, job_id, ciphertexts)?;
    let record =
        FheRuntimeCiphertextTranscriptRecord::new(job_id, transcript_digest, plan.key_epoch());
    Ok(AcceptedFheRuntimeCiphertextTranscript::new(
        record,
        plan.participant_count(),
        plan.runtime_binding(),
    ))
}

fn validate_fhe_runtime_ciphertext_transcript(
    plan: FheRuntimePlan,
    job_id: SecureAggregateTranscriptId,
    expected_transcript_digest: SecureAggregateCommitment,
    existing_records: &[FheRuntimeCiphertextTranscriptRecord],
    ciphertexts: &[FheRuntimeParticipantCiphertext],
) -> Result<(), SecureAggregateError> {
    validate_fhe_release_policy(plan.release_policy())?;
    validate_fhe_runtime_binding(plan.runtime_binding())?;
    if !plan.runtime_binding().external_review_completed() {
        return Err(SecureAggregateError::FheRuntimeExternalReviewRequired);
    }
    if ciphertexts.len() != plan.participant_count().get() {
        return Err(SecureAggregateError::FheRuntimeCiphertextCountMismatch);
    }

    for (offset, ciphertext) in ciphertexts.iter().enumerate() {
        let participant_index = ciphertext.participant_index().get();
        if participant_index > plan.participant_count().get() {
            return Err(SecureAggregateError::ParticipantIndexOutOfRange);
        }
        if ciphertexts[..offset]
            .iter()
            .any(|previous| previous.participant_index() == ciphertext.participant_index())
        {
            return Err(SecureAggregateError::DuplicateParticipantIndex);
        }
        match ciphertext.status() {
            FheRuntimeParticipantCiphertextStatus::Submitted => {}
            FheRuntimeParticipantCiphertextStatus::Removed => {
                return Err(SecureAggregateError::FheRuntimeCiphertextParticipantInactive);
            }
            FheRuntimeParticipantCiphertextStatus::Aborted => {
                return Err(SecureAggregateError::FheRuntimeCiphertextParticipantAborted);
            }
        }
        if ciphertext.key_custody_digest().is_none() {
            return Err(SecureAggregateError::FheRuntimeKeyCustodyMissing);
        }
        if ciphertext.parameter_set_digest() != plan.runtime_binding().parameter_set_digest() {
            return Err(SecureAggregateError::FheRuntimeCiphertextParameterMismatch);
        }
    }

    let transcript_digest =
        derive_fhe_runtime_ciphertext_transcript_digest(plan, job_id, ciphertexts)?;
    if transcript_digest != expected_transcript_digest {
        return Err(SecureAggregateError::FheRuntimeCiphertextDigestMismatch);
    }
    if existing_records
        .iter()
        .any(|record| record.conflicts_with(job_id, transcript_digest))
    {
        return Err(SecureAggregateError::FheRuntimeCiphertextReplayDetected);
    }
    Ok(())
}

/// Derives the canonical digest over accepted FHE ciphertext metadata.
pub fn derive_fhe_runtime_ciphertext_transcript_digest(
    plan: FheRuntimePlan,
    job_id: SecureAggregateTranscriptId,
    ciphertexts: &[FheRuntimeParticipantCiphertext],
) -> Result<SecureAggregateCommitment, SecureAggregateError> {
    validate_fhe_runtime_binding(plan.runtime_binding())?;
    let mut ordered = ciphertexts
        .iter()
        .map(|ciphertext| {
            (
                ciphertext.participant_index().get(),
                ciphertext.ciphertext_digest().bytes(),
                ciphertext.parameter_set_digest().bytes(),
                ciphertext
                    .key_custody_digest()
                    .map(SecureAggregateCommitment::bytes),
            )
        })
        .collect::<Vec<_>>();
    ordered.sort_unstable_by_key(|(participant_index, _ciphertext, _parameter, _custody)| {
        *participant_index
    });

    let mut hasher = Sha256::new();
    hasher.update(FHE_RUNTIME_CIPHERTEXT_TRANSCRIPT_DOMAIN);
    update_len_prefixed(&mut hasher, FHE_RUNTIME_PROFILE.as_bytes());
    update_len_prefixed(
        &mut hasher,
        plan.runtime_binding().scheme_family().label().as_bytes(),
    );
    update_len_prefixed(
        &mut hasher,
        plan.runtime_binding().operation().label().as_bytes(),
    );
    hasher.update(job_id.bytes());
    hasher.update((plan.participant_count().get() as u128).to_be_bytes());
    hasher.update(plan.key_epoch().value().get().to_be_bytes());
    hasher.update(plan.runtime_binding().runtime_artifact_digest().bytes());
    hasher.update(plan.runtime_binding().parameter_set_digest().bytes());
    hasher.update(plan.runtime_binding().review_evidence_digest().bytes());
    for (participant_index, ciphertext_digest, parameter_set_digest, custody_digest) in ordered {
        hasher.update((participant_index as u128).to_be_bytes());
        hasher.update(ciphertext_digest);
        hasher.update(parameter_set_digest);
        match custody_digest {
            Some(digest) => hasher.update(digest),
            None => hasher.update([0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES]),
        }
    }
    let digest = hasher.finalize();
    let mut bytes = [0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES];
    bytes.copy_from_slice(&digest);
    SecureAggregateCommitment::new(bytes)
}

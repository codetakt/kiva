#![allow(clippy::wildcard_imports)]

use super::*;

/// One accepted FROST signature-share metadata item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThresholdFrostSignatureShare {
    participant_index: NonZeroUsize,
    signature_share_digest: SecureAggregateCommitment,
}

impl ThresholdFrostSignatureShare {
    /// Creates FROST signature-share metadata.
    #[must_use]
    pub const fn new(
        participant_index: NonZeroUsize,
        signature_share_digest: SecureAggregateCommitment,
    ) -> Self {
        Self {
            participant_index,
            signature_share_digest,
        }
    }

    /// Returns the one-based participant index.
    #[must_use]
    pub const fn participant_index(self) -> NonZeroUsize {
        self.participant_index
    }

    /// Returns the signature-share digest.
    #[must_use]
    pub const fn signature_share_digest(self) -> SecureAggregateCommitment {
        self.signature_share_digest
    }
}

/// Replay metadata for one accepted FROST signing transcript.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThresholdFrostSignatureTranscriptRecord {
    key_ceremony_digest: SecureAggregateCommitment,
    message_digest: SecureAggregateCommitment,
    signature_digest: SecureAggregateCommitment,
    transcript_digest: SecureAggregateCommitment,
}

impl ThresholdFrostSignatureTranscriptRecord {
    /// Creates accepted FROST signing-transcript metadata.
    #[must_use]
    pub const fn new(
        key_ceremony_digest: SecureAggregateCommitment,
        message_digest: SecureAggregateCommitment,
        signature_digest: SecureAggregateCommitment,
        transcript_digest: SecureAggregateCommitment,
    ) -> Self {
        Self {
            key_ceremony_digest,
            message_digest,
            signature_digest,
            transcript_digest,
        }
    }

    /// Returns the accepted key-ceremony digest.
    #[must_use]
    pub const fn key_ceremony_digest(self) -> SecureAggregateCommitment {
        self.key_ceremony_digest
    }

    /// Returns the digest of the signed message.
    #[must_use]
    pub const fn message_digest(self) -> SecureAggregateCommitment {
        self.message_digest
    }

    /// Returns the accepted threshold signature digest.
    #[must_use]
    pub const fn signature_digest(self) -> SecureAggregateCommitment {
        self.signature_digest
    }

    /// Returns the accepted signing transcript digest.
    #[must_use]
    pub const fn transcript_digest(self) -> SecureAggregateCommitment {
        self.transcript_digest
    }

    /// Returns whether this record conflicts with a candidate signing transcript.
    #[must_use]
    pub fn conflicts_with(
        self,
        key_ceremony_digest: SecureAggregateCommitment,
        message_digest: SecureAggregateCommitment,
        transcript_digest: SecureAggregateCommitment,
    ) -> bool {
        self.key_ceremony_digest == key_ceremony_digest
            && (self.message_digest == message_digest
                || self.transcript_digest.bytes() == transcript_digest.bytes())
    }
}

/// Accepted metadata-only FROST signing transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptedThresholdFrostSignatureTranscript {
    record: ThresholdFrostSignatureTranscriptRecord,
    participant_count: NonZeroUsize,
    signature_share_count: NonZeroUsize,
    threshold: NonZeroUsize,
}

impl AcceptedThresholdFrostSignatureTranscript {
    /// Creates an accepted FROST signing transcript.
    #[must_use]
    pub const fn new(
        record: ThresholdFrostSignatureTranscriptRecord,
        participant_count: NonZeroUsize,
        signature_share_count: NonZeroUsize,
        threshold: NonZeroUsize,
    ) -> Self {
        Self {
            record,
            participant_count,
            signature_share_count,
            threshold,
        }
    }

    /// Returns the replay record that must be persisted.
    #[must_use]
    pub const fn record(self) -> ThresholdFrostSignatureTranscriptRecord {
        self.record
    }

    /// Returns the configured participant count.
    #[must_use]
    pub const fn participant_count(self) -> NonZeroUsize {
        self.participant_count
    }

    /// Returns the number of signature shares admitted.
    #[must_use]
    pub const fn signature_share_count(self) -> NonZeroUsize {
        self.signature_share_count
    }

    /// Returns the threshold required for signing.
    #[must_use]
    pub const fn threshold(self) -> NonZeroUsize {
        self.threshold
    }
}

/// Accepts metadata for a FROST threshold-signing transcript.
pub fn accept_threshold_frost_signature_transcript(
    ceremony: AcceptedThresholdFrostKeyCeremony,
    message_digest: SecureAggregateCommitment,
    signature_digest: SecureAggregateCommitment,
    expected_transcript_digest: SecureAggregateCommitment,
    existing_records: &[ThresholdFrostSignatureTranscriptRecord],
    signature_shares: &[ThresholdFrostSignatureShare],
) -> Result<AcceptedThresholdFrostSignatureTranscript, SecureAggregateError> {
    validate_threshold_frost_signature_transcript(
        ceremony,
        message_digest,
        expected_transcript_digest,
        existing_records,
        signature_shares,
    )?;
    let transcript_digest = derive_threshold_frost_signature_transcript_digest(
        ceremony.record().ceremony_digest(),
        message_digest,
        signature_digest,
        signature_shares,
    )?;
    if transcript_digest != expected_transcript_digest {
        return Err(SecureAggregateError::ThresholdFrostSignatureDigestMismatch);
    }
    let Some(signature_share_count) = NonZeroUsize::new(signature_shares.len()) else {
        return Err(SecureAggregateError::ThresholdFrostSignatureShareCountBelowThreshold);
    };
    let record = ThresholdFrostSignatureTranscriptRecord::new(
        ceremony.record().ceremony_digest(),
        message_digest,
        signature_digest,
        transcript_digest,
    );
    Ok(AcceptedThresholdFrostSignatureTranscript::new(
        record,
        ceremony.participant_count(),
        signature_share_count,
        ceremony.release_threshold(),
    ))
}

fn validate_threshold_frost_signature_transcript(
    ceremony: AcceptedThresholdFrostKeyCeremony,
    message_digest: SecureAggregateCommitment,
    expected_transcript_digest: SecureAggregateCommitment,
    existing_records: &[ThresholdFrostSignatureTranscriptRecord],
    signature_shares: &[ThresholdFrostSignatureShare],
) -> Result<(), SecureAggregateError> {
    validate_threshold_frost_runtime_binding(ceremony.runtime_binding())?;
    if !ceremony.runtime_binding().external_review_completed() {
        return Err(SecureAggregateError::ThresholdFrostExternalReviewRequired);
    }
    if signature_shares.len() < ceremony.release_threshold().get() {
        return Err(SecureAggregateError::ThresholdFrostSignatureShareCountBelowThreshold);
    }
    for (offset, share) in signature_shares.iter().enumerate() {
        let participant_index = share.participant_index().get();
        if participant_index > ceremony.participant_count().get() {
            return Err(SecureAggregateError::ParticipantIndexOutOfRange);
        }
        if signature_shares[..offset]
            .iter()
            .any(|previous| previous.participant_index() == share.participant_index())
        {
            return Err(SecureAggregateError::DuplicateParticipantIndex);
        }
    }
    if existing_records.iter().any(|record| {
        record.conflicts_with(
            ceremony.record().ceremony_digest(),
            message_digest,
            expected_transcript_digest,
        )
    }) {
        return Err(SecureAggregateError::ThresholdFrostReplayDetected);
    }
    Ok(())
}

/// Derives the canonical digest over accepted FROST signing metadata.
pub fn derive_threshold_frost_signature_transcript_digest(
    key_ceremony_digest: SecureAggregateCommitment,
    message_digest: SecureAggregateCommitment,
    signature_digest: SecureAggregateCommitment,
    signature_shares: &[ThresholdFrostSignatureShare],
) -> Result<SecureAggregateCommitment, SecureAggregateError> {
    let mut ordered = signature_shares
        .iter()
        .map(|share| {
            (
                share.participant_index().get(),
                share.signature_share_digest().bytes(),
            )
        })
        .collect::<Vec<_>>();
    ordered.sort_unstable_by_key(|(participant_index, _digest)| *participant_index);

    let mut hasher = Sha256::new();
    hasher.update(THRESHOLD_FROST_SIGNATURE_TRANSCRIPT_DOMAIN);
    update_len_prefixed(&mut hasher, THRESHOLD_FROST_PROFILE.as_bytes());
    update_len_prefixed(
        &mut hasher,
        ThresholdFrostSuite::Ed25519Sha512Frost.label().as_bytes(),
    );
    hasher.update(key_ceremony_digest.bytes());
    hasher.update(message_digest.bytes());
    hasher.update((ordered.len() as u128).to_be_bytes());
    for (participant_index, share_digest) in ordered {
        hasher.update((participant_index as u128).to_be_bytes());
        hasher.update(share_digest);
    }
    hasher.update(signature_digest.bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES];
    bytes.copy_from_slice(&digest);
    SecureAggregateCommitment::new(bytes)
}

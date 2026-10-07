#![allow(clippy::wildcard_imports)]

use super::*;

/// Replay metadata for one accepted FHE runtime evaluation.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FheRuntimeEvaluationRecord {
    ciphertext_transcript_digest: SecureAggregateCommitment,
    evaluated_ciphertext_digest: SecureAggregateCommitment,
    evaluation_proof_digest: SecureAggregateCommitment,
    evaluation_digest: SecureAggregateCommitment,
}

impl FheRuntimeEvaluationRecord {
    /// Creates accepted FHE evaluation replay metadata.
    #[must_use]
    pub const fn new(
        ciphertext_transcript_digest: SecureAggregateCommitment,
        evaluated_ciphertext_digest: SecureAggregateCommitment,
        evaluation_proof_digest: SecureAggregateCommitment,
        evaluation_digest: SecureAggregateCommitment,
    ) -> Self {
        Self {
            ciphertext_transcript_digest,
            evaluated_ciphertext_digest,
            evaluation_proof_digest,
            evaluation_digest,
        }
    }

    /// Returns the accepted ciphertext transcript digest.
    #[must_use]
    pub const fn ciphertext_transcript_digest(self) -> SecureAggregateCommitment {
        self.ciphertext_transcript_digest
    }

    /// Returns the evaluated ciphertext digest.
    #[must_use]
    pub const fn evaluated_ciphertext_digest(self) -> SecureAggregateCommitment {
        self.evaluated_ciphertext_digest
    }

    /// Returns the external runtime evaluation-proof digest.
    #[must_use]
    pub const fn evaluation_proof_digest(self) -> SecureAggregateCommitment {
        self.evaluation_proof_digest
    }

    /// Returns the accepted evaluation digest.
    #[must_use]
    pub const fn evaluation_digest(self) -> SecureAggregateCommitment {
        self.evaluation_digest
    }

    /// Returns whether this record conflicts with a candidate evaluation.
    #[must_use]
    pub fn conflicts_with(
        self,
        ciphertext_transcript_digest: SecureAggregateCommitment,
        evaluation_digest: SecureAggregateCommitment,
    ) -> bool {
        self.ciphertext_transcript_digest == ciphertext_transcript_digest
            || self.evaluation_digest.bytes() == evaluation_digest.bytes()
    }
}

/// Accepted metadata-only FHE runtime evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptedFheRuntimeEvaluation {
    record: FheRuntimeEvaluationRecord,
    participant_count: NonZeroUsize,
    operation: FheRuntimeOperation,
    runtime_binding: FheRuntimeBinding,
    key_epoch: SecureAggregateEnvelopeSequence,
}

impl AcceptedFheRuntimeEvaluation {
    /// Creates an accepted FHE runtime evaluation.
    #[must_use]
    pub const fn new(
        record: FheRuntimeEvaluationRecord,
        participant_count: NonZeroUsize,
        operation: FheRuntimeOperation,
        runtime_binding: FheRuntimeBinding,
        key_epoch: SecureAggregateEnvelopeSequence,
    ) -> Self {
        Self {
            record,
            participant_count,
            operation,
            runtime_binding,
            key_epoch,
        }
    }

    /// Returns the replay record that must be persisted.
    #[must_use]
    pub const fn record(self) -> FheRuntimeEvaluationRecord {
        self.record
    }

    /// Returns the configured participant count.
    #[must_use]
    pub const fn participant_count(self) -> NonZeroUsize {
        self.participant_count
    }

    /// Returns the evaluated operation.
    #[must_use]
    pub const fn operation(self) -> FheRuntimeOperation {
        self.operation
    }

    /// Returns the runtime binding that admitted this evaluation.
    #[must_use]
    pub const fn runtime_binding(self) -> FheRuntimeBinding {
        self.runtime_binding
    }

    /// Returns the key epoch bound to this evaluation.
    #[must_use]
    pub const fn key_epoch(self) -> SecureAggregateEnvelopeSequence {
        self.key_epoch
    }
}

/// Accepts metadata for an externally evaluated FHE ciphertext.
pub fn accept_fhe_runtime_evaluation(
    ciphertext_transcript: AcceptedFheRuntimeCiphertextTranscript,
    evaluated_ciphertext_digest: SecureAggregateCommitment,
    evaluation_proof_digest: SecureAggregateCommitment,
    expected_evaluation_digest: SecureAggregateCommitment,
    existing_records: &[FheRuntimeEvaluationRecord],
) -> Result<AcceptedFheRuntimeEvaluation, SecureAggregateError> {
    validate_fhe_runtime_binding(ciphertext_transcript.runtime_binding())?;
    if !ciphertext_transcript
        .runtime_binding()
        .external_review_completed()
    {
        return Err(SecureAggregateError::FheRuntimeExternalReviewRequired);
    }
    let evaluation_digest = derive_fhe_runtime_evaluation_digest(
        ciphertext_transcript,
        evaluated_ciphertext_digest,
        evaluation_proof_digest,
    )?;
    if evaluation_digest != expected_evaluation_digest {
        return Err(SecureAggregateError::FheRuntimeEvaluationDigestMismatch);
    }
    if existing_records.iter().any(|record| {
        record.conflicts_with(
            ciphertext_transcript.record().transcript_digest(),
            evaluation_digest,
        )
    }) {
        return Err(SecureAggregateError::FheRuntimeEvaluationReplayDetected);
    }
    let record = FheRuntimeEvaluationRecord::new(
        ciphertext_transcript.record().transcript_digest(),
        evaluated_ciphertext_digest,
        evaluation_proof_digest,
        evaluation_digest,
    );
    Ok(AcceptedFheRuntimeEvaluation::new(
        record,
        ciphertext_transcript.participant_count(),
        ciphertext_transcript.runtime_binding().operation(),
        ciphertext_transcript.runtime_binding(),
        ciphertext_transcript.record().key_epoch(),
    ))
}

/// Derives the canonical digest over accepted FHE evaluation metadata.
pub fn derive_fhe_runtime_evaluation_digest(
    ciphertext_transcript: AcceptedFheRuntimeCiphertextTranscript,
    evaluated_ciphertext_digest: SecureAggregateCommitment,
    evaluation_proof_digest: SecureAggregateCommitment,
) -> Result<SecureAggregateCommitment, SecureAggregateError> {
    validate_fhe_runtime_binding(ciphertext_transcript.runtime_binding())?;
    if ciphertext_transcript.participant_count().get() == 0 {
        return Err(SecureAggregateError::FheRuntimeEvaluationRequiresCiphertexts);
    }

    let mut hasher = Sha256::new();
    hasher.update(FHE_RUNTIME_EVALUATION_DOMAIN);
    update_len_prefixed(&mut hasher, FHE_RUNTIME_PROFILE.as_bytes());
    update_len_prefixed(
        &mut hasher,
        ciphertext_transcript
            .runtime_binding()
            .scheme_family()
            .label()
            .as_bytes(),
    );
    update_len_prefixed(
        &mut hasher,
        ciphertext_transcript
            .runtime_binding()
            .operation()
            .label()
            .as_bytes(),
    );
    hasher.update(ciphertext_transcript.record().transcript_digest().bytes());
    hasher.update((ciphertext_transcript.participant_count().get() as u128).to_be_bytes());
    hasher.update(
        ciphertext_transcript
            .record()
            .key_epoch()
            .value()
            .get()
            .to_be_bytes(),
    );
    hasher.update(
        ciphertext_transcript
            .runtime_binding()
            .runtime_artifact_digest()
            .bytes(),
    );
    hasher.update(
        ciphertext_transcript
            .runtime_binding()
            .parameter_set_digest()
            .bytes(),
    );
    hasher.update(evaluated_ciphertext_digest.bytes());
    hasher.update(evaluation_proof_digest.bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES];
    bytes.copy_from_slice(&digest);
    SecureAggregateCommitment::new(bytes)
}

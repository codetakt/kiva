#![allow(clippy::wildcard_imports)]

use super::*;

/// Accepts metadata for a reviewed FROST key ceremony after fail-closed checks.
pub fn accept_threshold_frost_key_ceremony(
    plan: ThresholdFrostKeyCeremonyPlan,
    job_id: SecureAggregateTranscriptId,
    existing_records: &[ThresholdFrostKeyCeremonyRecord],
    transcripts: &[ThresholdFrostParticipantTranscript],
) -> Result<AcceptedThresholdFrostKeyCeremony, SecureAggregateError> {
    validate_threshold_frost_key_ceremony(plan, job_id, existing_records, transcripts)?;
    let ceremony_digest = derive_threshold_frost_key_ceremony_digest(plan, job_id, transcripts)?;
    let record = ThresholdFrostKeyCeremonyRecord::new(job_id, ceremony_digest, plan.key_epoch());
    Ok(AcceptedThresholdFrostKeyCeremony::new(
        record,
        plan.participant_count(),
        plan.release_threshold(),
        plan.runtime_binding(),
    ))
}

fn validate_threshold_frost_key_ceremony(
    plan: ThresholdFrostKeyCeremonyPlan,
    job_id: SecureAggregateTranscriptId,
    existing_records: &[ThresholdFrostKeyCeremonyRecord],
    transcripts: &[ThresholdFrostParticipantTranscript],
) -> Result<(), SecureAggregateError> {
    validate_shamir_release_policy(plan.release_policy())?;
    validate_threshold_frost_runtime_binding(plan.runtime_binding())?;
    if !plan.runtime_binding().external_review_completed() {
        return Err(SecureAggregateError::ThresholdFrostExternalReviewRequired);
    }
    if transcripts.len() != plan.participant_count().get() {
        return Err(SecureAggregateError::ThresholdFrostCeremonyParticipantCountMismatch);
    }
    if existing_records
        .iter()
        .any(|record| record.job_id() == job_id)
    {
        return Err(SecureAggregateError::ThresholdFrostReplayDetected);
    }

    for (offset, transcript) in transcripts.iter().enumerate() {
        let participant_index = transcript.participant_index().get();
        if participant_index > plan.participant_count().get() {
            return Err(SecureAggregateError::ParticipantIndexOutOfRange);
        }
        if transcripts[..offset]
            .iter()
            .any(|previous| previous.participant_index() == transcript.participant_index())
        {
            return Err(SecureAggregateError::DuplicateParticipantIndex);
        }
        match transcript.status() {
            ThresholdFrostParticipantTranscriptStatus::Submitted => {}
            ThresholdFrostParticipantTranscriptStatus::Removed => {
                return Err(SecureAggregateError::ThresholdFrostParticipantInactive);
            }
            ThresholdFrostParticipantTranscriptStatus::Aborted => {
                return Err(SecureAggregateError::ThresholdFrostParticipantAborted);
            }
        }
        if transcript.share_custody_digest().is_none() {
            return Err(SecureAggregateError::ThresholdFrostShareCustodyMissing);
        }
        if !transcript.complaint_resolved() {
            return Err(SecureAggregateError::ThresholdFrostComplaintUnresolved);
        }
        if transcript.equivocation_detected() {
            return Err(SecureAggregateError::ThresholdFrostEquivocationDetected);
        }
    }

    let ceremony_digest = derive_threshold_frost_key_ceremony_digest(plan, job_id, transcripts)?;
    if existing_records
        .iter()
        .any(|record| record.conflicts_with(job_id, ceremony_digest))
    {
        return Err(SecureAggregateError::ThresholdFrostReplayDetected);
    }
    Ok(())
}

/// Derives the canonical digest over accepted FROST key-ceremony metadata.
pub fn derive_threshold_frost_key_ceremony_digest(
    plan: ThresholdFrostKeyCeremonyPlan,
    job_id: SecureAggregateTranscriptId,
    transcripts: &[ThresholdFrostParticipantTranscript],
) -> Result<SecureAggregateCommitment, SecureAggregateError> {
    let mut ordered = transcripts
        .iter()
        .map(|transcript| {
            (
                transcript.participant_index().get(),
                transcript.public_commitment_digest().bytes(),
                transcript
                    .share_custody_digest()
                    .map(SecureAggregateCommitment::bytes),
            )
        })
        .collect::<Vec<_>>();
    ordered.sort_unstable_by_key(|(participant_index, _commitment, _custody)| *participant_index);

    let mut hasher = Sha256::new();
    hasher.update(THRESHOLD_FROST_KEY_CEREMONY_DOMAIN);
    update_len_prefixed(&mut hasher, THRESHOLD_FROST_PROFILE.as_bytes());
    update_len_prefixed(
        &mut hasher,
        plan.runtime_binding().suite().label().as_bytes(),
    );
    hasher.update(job_id.bytes());
    hasher.update((plan.participant_count().get() as u128).to_be_bytes());
    hasher.update((plan.release_threshold().get() as u128).to_be_bytes());
    hasher.update(plan.key_epoch().value().get().to_be_bytes());
    hasher.update(plan.runtime_binding().runtime_artifact_digest().bytes());
    hasher.update(plan.runtime_binding().review_evidence_digest().bytes());
    for (participant_index, commitment, custody) in ordered {
        hasher.update((participant_index as u128).to_be_bytes());
        hasher.update(commitment);
        match custody {
            Some(custody) => hasher.update(custody),
            None => hasher.update([0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES]),
        }
    }
    let digest = hasher.finalize();
    let mut bytes = [0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES];
    bytes.copy_from_slice(&digest);
    SecureAggregateCommitment::new(bytes)
}

#![allow(clippy::wildcard_imports)]

use super::*;

pub(crate) fn combine_confidential_additive_aggregate(
    request: &ConfidentialAdditiveAggregateRequest,
    verification_keys: &[AdditiveShareVerificationKey],
    accepted_records: &[AcceptedAdditiveShareRecord],
    signed_envelopes: &[SignedAuthenticatedAdditiveShareEnvelope],
) -> Result<ReplayWindowedAdditiveAggregate, ConfidentialAggregateError> {
    combine_signed_authenticated_additive_share_envelopes(
        request.participant_count,
        request.transcript_id,
        &request.slots,
        verification_keys,
        request.replay_window,
        accepted_records,
        signed_envelopes,
    )
    .map_err(ConfidentialAggregateError::Secure)
}

pub(crate) fn ensure_matching_aggregate_result(
    result: Option<&ConfidentialAdditiveAggregateResult>,
    release: &ConfidentialThresholdReleaseState,
) -> Result<(), ConfidentialThresholdReleaseError> {
    let computed_aggregate_matches = matches!(
        result,
        Some(result)
            if result.transcript_id == release.transcript_id
                && result.aggregate == release.aggregate
    );
    validate_threshold_release_computed_aggregate(computed_aggregate_matches)
        .map_err(ConfidentialThresholdReleaseError::from)
}

pub(crate) fn ensure_matching_differential_privacy_aggregate_result(
    result: Option<&ConfidentialAdditiveAggregateResult>,
    request: &ConfidentialDifferentialPrivacyReleaseRequest,
) -> Result<(), ConfidentialDifferentialPrivacyReleaseError> {
    let computed_aggregate_matches = matches!(
        result,
        Some(result)
            if result.transcript_id == request.transcript_id
                && result.aggregate == request.aggregate
    );
    if computed_aggregate_matches {
        Ok(())
    } else {
        Err(ConfidentialDifferentialPrivacyReleaseError::MissingComputedAggregate)
    }
}

pub(crate) fn field_element_i64(
    value: SecureAggregateFieldElement,
) -> Result<i64, SecureAggregateError> {
    i64::try_from(value.value()).map_err(|_error| SecureAggregateError::ValueOutsideField)
}

pub(super) fn confidential_replay_record_from_row(
    row: ConfidentialReplayRecordRow,
) -> Result<AcceptedAdditiveShareRecord, ConfidentialAggregateStoreError> {
    let transcript_id = SecureAggregateTranscriptId::new(fixed_32_bytes(row.transcript_id)?)
        .map_err(ConfidentialAggregateStoreError::InvalidStoredReplayRecord)?;
    let participant_index = usize::try_from(row.participant_index)
        .ok()
        .and_then(std::num::NonZeroUsize::new)
        .ok_or(ConfidentialAggregateStoreError::InvalidStoredReplayRecord(
            SecureAggregateError::ParticipantIndexOutOfRange,
        ))?;
    let sequence = row
        .sequence
        .parse::<u64>()
        .ok()
        .and_then(|value| SecureAggregateEnvelopeSequence::try_from(value).ok())
        .ok_or(ConfidentialAggregateStoreError::InvalidStoredReplayRecord(
            SecureAggregateError::EnvelopeSequenceMustNotBeZero,
        ))?;
    let signing_key_id = SecureAggregateSigningKeyId::new(fixed_32_bytes(row.signing_key_id)?)
        .map_err(ConfidentialAggregateStoreError::InvalidStoredReplayRecord)?;

    Ok(AcceptedAdditiveShareRecord::new(
        transcript_id,
        participant_index,
        sequence,
        signing_key_id,
    ))
}

pub(super) fn active_confidential_signing_key_from_row(
    row: ConfidentialSigningKeyRow,
) -> Result<ConfidentialShareSigningKey, ConfidentialAggregateStoreError> {
    let signing_key = confidential_signing_key_from_row(row)?;
    if signing_key.revoked_at.is_some() {
        return Err(ConfidentialAggregateStoreError::InvalidStoredSigningKey(
            SecureAggregateError::VerificationKeyMissing,
        ));
    }
    Ok(signing_key)
}

pub(super) fn confidential_signing_key_from_row(
    row: ConfidentialSigningKeyRow,
) -> Result<ConfidentialShareSigningKey, ConfidentialAggregateStoreError> {
    let signing_key_id = SecureAggregateSigningKeyId::new(fixed_32_bytes(row.signing_key_id)?)
        .map_err(ConfidentialAggregateStoreError::InvalidStoredSigningKey)?;
    let participant_index = usize::try_from(row.participant_index)
        .ok()
        .and_then(std::num::NonZeroUsize::new)
        .ok_or(ConfidentialAggregateStoreError::InvalidStoredSigningKey(
            SecureAggregateError::ParticipantIndexOutOfRange,
        ))?;
    let public_key = SecureAggregateSigningPublicKey::new(fixed_32_bytes(row.public_key)?)
        .map_err(ConfidentialAggregateStoreError::InvalidStoredSigningKey)?;

    Ok(ConfidentialShareSigningKey {
        oauth_subject: row.oauth_subject,
        signing_key_id,
        participant_index,
        public_key,
        revoked_at: row.revoked_at,
    })
}

pub(crate) fn active_signing_key_for_envelope(
    signing_keys: &BTreeMap<ConfidentialSigningKeyStorageKey, ConfidentialShareSigningKey>,
    signed_envelope: SignedAuthenticatedAdditiveShareEnvelope,
) -> Result<&ConfidentialShareSigningKey, SecureAggregateError> {
    let key = ConfidentialSigningKeyStorageKey::from_signing_key_id(
        signed_envelope.envelope().signing_key_id(),
    );
    let Some(signing_key) = signing_keys.get(&key) else {
        return Err(SecureAggregateError::VerificationKeyMissing);
    };
    if signing_key.revoked_at.is_some() {
        return Err(SecureAggregateError::VerificationKeyMissing);
    }
    Ok(signing_key)
}

pub(crate) fn validate_share_submission(
    principal: &AuthenticatedPrincipal,
    signing_key: &ConfidentialShareSigningKey,
    signed_envelope: SignedAuthenticatedAdditiveShareEnvelope,
) -> Result<(), ConfidentialShareSubmissionError> {
    if signing_key.oauth_subject != principal.subject {
        return Err(ConfidentialShareSubmissionError::Forbidden);
    }
    if signed_envelope.envelope().share().participant_index() != signing_key.participant_index {
        return Err(ConfidentialShareSubmissionError::Secure(
            SecureAggregateError::EnvelopeShareParticipantMismatch,
        ));
    }
    verify_signed_authenticated_additive_share_envelope(
        signing_key.verification_key(),
        signed_envelope,
    )
    .map_err(ConfidentialShareSubmissionError::Secure)?;
    Ok(())
}

pub(crate) fn verification_keys_for_slots(
    signing_keys: &BTreeMap<ConfidentialSigningKeyStorageKey, ConfidentialShareSigningKey>,
    slots: &[AdditiveShareCommitmentSlot],
) -> Result<Vec<AdditiveShareVerificationKey>, SecureAggregateError> {
    let mut verification_keys = Vec::with_capacity(slots.len());
    for slot in slots {
        let key = ConfidentialSigningKeyStorageKey::from_signing_key_id(slot.signing_key_id());
        let Some(signing_key) = signing_keys.get(&key) else {
            return Err(SecureAggregateError::VerificationKeyMissing);
        };
        if signing_key.revoked_at.is_some() {
            return Err(SecureAggregateError::VerificationKeyMissing);
        }
        if signing_key.participant_index != slot.participant_index() {
            return Err(SecureAggregateError::EnvelopeSigningKeyMismatch);
        }
        verification_keys.push(signing_key.verification_key());
    }
    Ok(verification_keys)
}

pub(crate) fn verification_keys_from_signing_key_rows(
    slots: &[AdditiveShareCommitmentSlot],
    signing_keys: Vec<ConfidentialShareSigningKey>,
) -> Result<Vec<AdditiveShareVerificationKey>, SecureAggregateError> {
    let signing_keys = signing_keys
        .into_iter()
        .map(|signing_key| {
            (
                ConfidentialSigningKeyStorageKey::from_signing_key_id(signing_key.signing_key_id),
                signing_key,
            )
        })
        .collect::<BTreeMap<_, _>>();
    verification_keys_for_slots(&signing_keys, slots)
}

pub(crate) fn fixed_32_bytes(bytes: Vec<u8>) -> Result<[u8; 32], ConfidentialAggregateStoreError> {
    bytes
        .try_into()
        .map_err(|_bytes| ConfidentialAggregateStoreError::InvalidStoredArtifactLength)
}

pub(crate) fn participant_index_i64(
    record: AcceptedAdditiveShareRecord,
) -> Result<i64, ConfidentialAggregateStoreError> {
    nonzero_usize_i64(record.participant_index())
}

pub(crate) fn signing_key_participant_index_i64(
    signing_key: &ConfidentialShareSigningKey,
) -> Result<i64, ConfidentialAggregateStoreError> {
    nonzero_usize_i64(signing_key.participant_index)
}

pub(crate) fn nonzero_usize_i64(
    value: std::num::NonZeroUsize,
) -> Result<i64, ConfidentialAggregateStoreError> {
    i64::try_from(value.get())
        .map_err(|_error| ConfidentialAggregateStoreError::ParticipantIndexTooLarge)
}

pub(crate) fn usize_i64(value: usize) -> Result<i64, ConfidentialAggregateStoreError> {
    i64::try_from(value).map_err(|_error| ConfidentialAggregateStoreError::ParticipantIndexTooLarge)
}

pub(crate) const fn confidential_store(source: sqlx::Error) -> ConfidentialAggregateStoreError {
    ConfidentialAggregateStoreError::Store { source }
}

#![allow(clippy::wildcard_imports)]

use super::super::*;
use super::envelope::*;
use super::replay::*;

/// Combines one additive share per participant into the released aggregate.
pub fn combine_additive_shares(
    participant_count: NonZeroUsize,
    shares: &[SecureAggregateShare],
) -> Result<SecureAggregateFieldElement, SecureAggregateError> {
    if shares.len() != participant_count.get() {
        return Err(SecureAggregateError::ShareCountMismatch);
    }

    let mut total = SecureAggregateFieldElement::ZERO;
    for (offset, share) in shares.iter().enumerate() {
        let participant_index = share.participant_index().get();
        if participant_index > participant_count.get() {
            return Err(SecureAggregateError::ParticipantIndexOutOfRange);
        }
        if shares[..offset]
            .iter()
            .any(|previous| previous.participant_index() == share.participant_index())
        {
            return Err(SecureAggregateError::DuplicateParticipantIndex);
        }
        total = total.add_mod(share.value());
    }

    Ok(total)
}

/// Validates the commitment slots for an additive secure aggregate transcript.
pub fn validate_additive_share_commitment_slots(
    participant_count: NonZeroUsize,
    transcript_id: SecureAggregateTranscriptId,
    slots: &[AdditiveShareCommitmentSlot],
) -> Result<(), SecureAggregateError> {
    if slots.len() != participant_count.get() {
        return Err(SecureAggregateError::CommitmentSlotCountMismatch);
    }

    for (offset, slot) in slots.iter().enumerate() {
        let participant_index = slot.participant_index().get();
        if participant_index > participant_count.get() {
            return Err(SecureAggregateError::ParticipantIndexOutOfRange);
        }
        if slot.transcript_id() != transcript_id {
            return Err(SecureAggregateError::CommitmentSlotTranscriptMismatch);
        }
        if slots[..offset]
            .iter()
            .any(|previous| previous.participant_index() == slot.participant_index())
        {
            return Err(SecureAggregateError::DuplicateCommitmentSlotParticipantIndex);
        }
    }

    Ok(())
}

/// Validates the verification keys registered for an additive transcript.
pub fn validate_additive_share_verification_keys(
    participant_count: NonZeroUsize,
    slots: &[AdditiveShareCommitmentSlot],
    verification_keys: &[AdditiveShareVerificationKey],
) -> Result<(), SecureAggregateError> {
    if verification_keys.len() != participant_count.get() {
        return Err(SecureAggregateError::VerificationKeyCountMismatch);
    }

    for (offset, verification_key) in verification_keys.iter().enumerate() {
        if verification_keys[..offset]
            .iter()
            .any(|previous| previous.signing_key_id() == verification_key.signing_key_id())
        {
            return Err(SecureAggregateError::DuplicateVerificationKeyIdentifier);
        }
        if slots
            .iter()
            .all(|slot| slot.signing_key_id() != verification_key.signing_key_id())
        {
            return Err(SecureAggregateError::VerificationKeyMissing);
        }
    }

    for slot in slots {
        if verification_keys
            .iter()
            .all(|verification_key| verification_key.signing_key_id() != slot.signing_key_id())
        {
            return Err(SecureAggregateError::VerificationKeyMissing);
        }
    }

    Ok(())
}

/// Accepts an additive share envelope when it matches the expected commitment slot.
pub fn accept_authenticated_additive_share(
    slot: AdditiveShareCommitmentSlot,
    envelope: AuthenticatedAdditiveShareEnvelope,
) -> Result<SecureAggregateShare, SecureAggregateError> {
    if envelope.transcript_id() != slot.transcript_id() {
        return Err(SecureAggregateError::EnvelopeTranscriptMismatch);
    }
    if envelope.share().participant_index() != slot.participant_index() {
        return Err(SecureAggregateError::EnvelopeShareParticipantMismatch);
    }
    if envelope.commitment() != slot.commitment() {
        return Err(SecureAggregateError::EnvelopeCommitmentMismatch);
    }
    if envelope.signing_key_id() != slot.signing_key_id() {
        return Err(SecureAggregateError::EnvelopeSigningKeyMismatch);
    }

    Ok(envelope.share())
}

/// Verifies the Ed25519 signature on an authenticated additive-share envelope.
pub fn verify_signed_authenticated_additive_share_envelope(
    verification_key: AdditiveShareVerificationKey,
    signed_envelope: SignedAuthenticatedAdditiveShareEnvelope,
) -> Result<AuthenticatedAdditiveShareEnvelope, SecureAggregateError> {
    let envelope = signed_envelope.envelope();
    if envelope.signing_key_id() != verification_key.signing_key_id() {
        return Err(SecureAggregateError::EnvelopeSigningKeyMismatch);
    }

    let public_key = VerifyingKey::from_bytes(&verification_key.public_key().bytes())
        .map_err(|_error| SecureAggregateError::InvalidSigningPublicKey)?;
    let signature = Signature::from_bytes(&signed_envelope.signature().bytes());
    let payload = additive_share_envelope_signature_payload(envelope);
    public_key
        .verify_strict(&payload, &signature)
        .map_err(|_error| SecureAggregateError::EnvelopeSignatureMismatch)?;

    Ok(envelope)
}

/// Accepts an additive share envelope after slot and replay-window validation.
pub fn accept_replay_windowed_authenticated_additive_share(
    slot: AdditiveShareCommitmentSlot,
    replay_window: AdditiveShareReplayWindow,
    accepted_records: &[AcceptedAdditiveShareRecord],
    envelope: AuthenticatedAdditiveShareEnvelope,
) -> Result<AcceptedAdditiveShare, SecureAggregateError> {
    let accepted_share = accept_authenticated_additive_share(slot, envelope)?;
    if !replay_window.contains(envelope.sequence()) {
        return Err(SecureAggregateError::EnvelopeSequenceOutsideReplayWindow);
    }

    let replay_record = AcceptedAdditiveShareRecord::new(
        envelope.transcript_id(),
        envelope.share().participant_index(),
        envelope.sequence(),
        envelope.signing_key_id(),
    );
    if accepted_records
        .iter()
        .any(|record| record.conflicts_with(replay_record))
    {
        return Err(SecureAggregateError::EnvelopeReplayDetected);
    }

    Ok(AcceptedAdditiveShare::new(accepted_share, replay_record))
}

/// Verifies and accepts an additive share envelope after slot and replay validation.
pub fn accept_signed_replay_windowed_authenticated_additive_share(
    slot: AdditiveShareCommitmentSlot,
    replay_window: AdditiveShareReplayWindow,
    accepted_records: &[AcceptedAdditiveShareRecord],
    verification_key: AdditiveShareVerificationKey,
    signed_envelope: SignedAuthenticatedAdditiveShareEnvelope,
) -> Result<AcceptedAdditiveShare, SecureAggregateError> {
    let envelope =
        verify_signed_authenticated_additive_share_envelope(verification_key, signed_envelope)?;
    accept_replay_windowed_authenticated_additive_share(
        slot,
        replay_window,
        accepted_records,
        envelope,
    )
}

/// Combines authenticated additive-share envelopes after slot and replay validation.
pub fn combine_authenticated_additive_share_envelopes(
    participant_count: NonZeroUsize,
    transcript_id: SecureAggregateTranscriptId,
    slots: &[AdditiveShareCommitmentSlot],
    replay_window: AdditiveShareReplayWindow,
    accepted_records: &[AcceptedAdditiveShareRecord],
    envelopes: &[AuthenticatedAdditiveShareEnvelope],
) -> Result<ReplayWindowedAdditiveAggregate, SecureAggregateError> {
    validate_additive_share_commitment_slots(participant_count, transcript_id, slots)?;

    if envelopes.len() != participant_count.get() {
        return Err(SecureAggregateError::EnvelopeCountMismatch);
    }

    let mut total = SecureAggregateFieldElement::ZERO;
    let mut replay_records: Vec<AcceptedAdditiveShareRecord> = Vec::with_capacity(envelopes.len());
    for (offset, envelope) in envelopes.iter().enumerate() {
        let participant_index = envelope.share().participant_index();
        if participant_index.get() > participant_count.get() {
            return Err(SecureAggregateError::ParticipantIndexOutOfRange);
        }
        if envelopes[..offset]
            .iter()
            .any(|previous| previous.share().participant_index() == participant_index)
        {
            return Err(SecureAggregateError::DuplicateParticipantIndex);
        }

        let Some(slot) = find_commitment_slot(slots, participant_index) else {
            return Err(SecureAggregateError::EnvelopeShareParticipantMismatch);
        };
        let accepted = accept_replay_windowed_authenticated_additive_share(
            slot,
            replay_window,
            accepted_records,
            *envelope,
        )?;
        if replay_records
            .iter()
            .any(|record| record.conflicts_with(accepted.replay_record()))
        {
            return Err(SecureAggregateError::EnvelopeReplayDetected);
        }
        total = total.add_mod(accepted.share().value());
        replay_records.push(accepted.replay_record());
    }

    Ok(ReplayWindowedAdditiveAggregate::new(total, replay_records))
}

/// Combines signed additive-share envelopes after signature, slot, and replay validation.
pub fn combine_signed_authenticated_additive_share_envelopes(
    participant_count: NonZeroUsize,
    transcript_id: SecureAggregateTranscriptId,
    slots: &[AdditiveShareCommitmentSlot],
    verification_keys: &[AdditiveShareVerificationKey],
    replay_window: AdditiveShareReplayWindow,
    accepted_records: &[AcceptedAdditiveShareRecord],
    signed_envelopes: &[SignedAuthenticatedAdditiveShareEnvelope],
) -> Result<ReplayWindowedAdditiveAggregate, SecureAggregateError> {
    validate_additive_share_commitment_slots(participant_count, transcript_id, slots)?;
    validate_additive_share_verification_keys(participant_count, slots, verification_keys)?;

    if signed_envelopes.len() != participant_count.get() {
        return Err(SecureAggregateError::EnvelopeCountMismatch);
    }

    let mut total = SecureAggregateFieldElement::ZERO;
    let mut replay_records: Vec<AcceptedAdditiveShareRecord> =
        Vec::with_capacity(signed_envelopes.len());
    for (offset, signed_envelope) in signed_envelopes.iter().enumerate() {
        let envelope = signed_envelope.envelope();
        let participant_index = envelope.share().participant_index();
        if participant_index.get() > participant_count.get() {
            return Err(SecureAggregateError::ParticipantIndexOutOfRange);
        }
        if signed_envelopes[..offset]
            .iter()
            .any(|previous| previous.envelope().share().participant_index() == participant_index)
        {
            return Err(SecureAggregateError::DuplicateParticipantIndex);
        }

        let Some(slot) = find_commitment_slot(slots, participant_index) else {
            return Err(SecureAggregateError::EnvelopeShareParticipantMismatch);
        };
        let Some(verification_key) =
            find_verification_key(verification_keys, envelope.signing_key_id())
        else {
            return Err(SecureAggregateError::VerificationKeyMissing);
        };
        let accepted = accept_signed_replay_windowed_authenticated_additive_share(
            slot,
            replay_window,
            accepted_records,
            verification_key,
            *signed_envelope,
        )?;
        if replay_records
            .iter()
            .any(|record| record.conflicts_with(accepted.replay_record()))
        {
            return Err(SecureAggregateError::EnvelopeReplayDetected);
        }
        total = total.add_mod(accepted.share().value());
        replay_records.push(accepted.replay_record());
    }

    Ok(ReplayWindowedAdditiveAggregate::new(total, replay_records))
}

fn find_commitment_slot(
    slots: &[AdditiveShareCommitmentSlot],
    participant_index: NonZeroUsize,
) -> Option<AdditiveShareCommitmentSlot> {
    slots
        .iter()
        .copied()
        .find(|slot| slot.participant_index() == participant_index)
}

fn find_verification_key(
    verification_keys: &[AdditiveShareVerificationKey],
    signing_key_id: SecureAggregateSigningKeyId,
) -> Option<AdditiveShareVerificationKey> {
    verification_keys
        .iter()
        .copied()
        .find(|verification_key| verification_key.signing_key_id() == signing_key_id)
}

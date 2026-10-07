#![allow(clippy::wildcard_imports)]

use super::*;

#[test]
fn rejects_duplicate_secure_aggregate_participant() {
    let participant_count = nonzero(2);
    let shares = [share(1, 10), share(1, 20)];

    assert_eq!(
        combine_additive_shares(participant_count, &shares),
        Err(SecureAggregateError::DuplicateParticipantIndex)
    );
}

#[test]
fn rejects_empty_share_transport_artifacts() {
    let empty = [0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES];

    assert_eq!(
        SecureAggregateTranscriptId::new(empty),
        Err(SecureAggregateError::EmptyTranscriptIdentifier)
    );
    assert_eq!(
        SecureAggregateCommitment::new(empty),
        Err(SecureAggregateError::EmptyShareCommitment)
    );
    assert_eq!(
        SecureAggregateSigningKeyId::new(empty),
        Err(SecureAggregateError::EmptySigningKeyIdentifier)
    );
    assert_eq!(
        SecureAggregateSigningPublicKey::new(empty),
        Err(SecureAggregateError::EmptySigningPublicKey)
    );
    assert_eq!(
        SecureAggregateTransportProof::new(empty),
        Err(SecureAggregateError::EmptyTransportProof)
    );
    assert_eq!(
        SecureAggregateSignature::new([0_u8; SECURE_AGGREGATE_SIGNATURE_BYTES]),
        Err(SecureAggregateError::EmptySignature)
    );
}

#[test]
fn serde_rejects_empty_share_transport_artifacts() {
    let payload = serde_yaml::to_string(&[0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES]);
    assert!(payload.is_ok());
    let Ok(payload) = payload else {
        return;
    };

    let parsed: Result<SecureAggregateTranscriptId, serde_yaml::Error> =
        serde_yaml::from_str(&payload);

    assert!(parsed.is_err());
}

#[test]
fn serde_rejects_zero_envelope_sequence() {
    let payload = serde_yaml::to_string(&0_u64);
    assert!(payload.is_ok());
    let Ok(payload) = payload else {
        return;
    };

    let parsed: Result<SecureAggregateEnvelopeSequence, serde_yaml::Error> =
        serde_yaml::from_str(&payload);

    assert!(parsed.is_err());
}

#[test]
fn rejects_reversed_replay_window() {
    assert_eq!(
        AdditiveShareReplayWindow::new(sequence(3), sequence(2)),
        Err(SecureAggregateError::ReplayWindowLowerBoundExceedsUpperBound)
    );
}

#[test]
fn accepts_authenticated_share_envelope_when_slot_matches() {
    let transcript_id = transcript(1);
    let slot = commitment_slot(transcript_id, 1, 2, 3);
    let envelope = authenticated_envelope(transcript_id, 1, 10, 2, 3, 1, 4);

    assert_eq!(
        accept_authenticated_additive_share(slot, envelope),
        Ok(share(1, 10))
    );
}

#[test]
fn rejects_authenticated_share_envelope_transcript_mismatch() {
    let slot = commitment_slot(transcript(1), 1, 2, 3);
    let envelope = authenticated_envelope(transcript(9), 1, 10, 2, 3, 1, 4);

    assert_eq!(
        accept_authenticated_additive_share(slot, envelope),
        Err(SecureAggregateError::EnvelopeTranscriptMismatch)
    );
}

#[test]
fn rejects_authenticated_share_envelope_commitment_mismatch() {
    let transcript_id = transcript(1);
    let slot = commitment_slot(transcript_id, 1, 2, 3);
    let envelope = authenticated_envelope(transcript_id, 1, 10, 8, 3, 1, 4);

    assert_eq!(
        accept_authenticated_additive_share(slot, envelope),
        Err(SecureAggregateError::EnvelopeCommitmentMismatch)
    );
}

#[test]
fn verifies_signed_authenticated_share_envelope_when_signature_matches() {
    let envelope = authenticated_envelope(transcript(1), 1, 10, 2, 21, 1, 4);
    let signed_envelope = signed_envelope(21, envelope);

    assert_eq!(
        verify_signed_authenticated_additive_share_envelope(verification_key(21), signed_envelope,),
        Ok(envelope)
    );
}

#[test]
fn rejects_signed_authenticated_share_envelope_when_payload_is_tampered() {
    let original_envelope = authenticated_envelope(transcript(1), 1, 10, 2, 21, 1, 4);
    let tampered_envelope = authenticated_envelope(transcript(1), 1, 11, 2, 21, 1, 4);
    let signed_envelope = SignedAuthenticatedAdditiveShareEnvelope::new(
        tampered_envelope,
        signature_for(21, original_envelope),
    );

    assert_eq!(
        verify_signed_authenticated_additive_share_envelope(verification_key(21), signed_envelope,),
        Err(SecureAggregateError::EnvelopeSignatureMismatch)
    );
}

#[test]
fn accepts_replay_windowed_envelope_when_sequence_is_fresh() {
    let transcript_id = transcript(1);
    let slot = commitment_slot(transcript_id, 1, 2, 3);
    let envelope = authenticated_envelope(transcript_id, 1, 10, 2, 3, 2, 4);
    let accepted = accept_replay_windowed_authenticated_additive_share(
        slot,
        replay_window(1, 3),
        &[],
        envelope,
    );

    assert!(accepted.is_ok());
    let Ok(accepted) = accepted else {
        return;
    };
    assert_eq!(accepted.share(), share(1, 10));
    assert_eq!(
        accepted.replay_record(),
        AcceptedAdditiveShareRecord::new(transcript_id, nonzero(1), sequence(2), signing_key_id(3))
    );
}

#[test]
fn rejects_replay_windowed_envelope_outside_window() {
    let transcript_id = transcript(1);
    let slot = commitment_slot(transcript_id, 1, 2, 3);
    let envelope = authenticated_envelope(transcript_id, 1, 10, 2, 3, 9, 4);

    assert_eq!(
        accept_replay_windowed_authenticated_additive_share(
            slot,
            replay_window(1, 3),
            &[],
            envelope,
        ),
        Err(SecureAggregateError::EnvelopeSequenceOutsideReplayWindow)
    );
}

#[test]
fn rejects_replay_windowed_envelope_when_record_already_exists() {
    let transcript_id = transcript(1);
    let slot = commitment_slot(transcript_id, 1, 2, 3);
    let envelope = authenticated_envelope(transcript_id, 1, 10, 2, 3, 2, 4);
    let existing = [AcceptedAdditiveShareRecord::new(
        transcript_id,
        nonzero(1),
        sequence(2),
        signing_key_id(3),
    )];

    assert_eq!(
        accept_replay_windowed_authenticated_additive_share(
            slot,
            replay_window(1, 3),
            &existing,
            envelope,
        ),
        Err(SecureAggregateError::EnvelopeReplayDetected)
    );
}

#[test]
fn rejects_replay_windowed_envelope_when_participant_record_already_exists() {
    let transcript_id = transcript(1);
    let slot = commitment_slot(transcript_id, 1, 2, 3);
    let envelope = authenticated_envelope(transcript_id, 1, 10, 2, 3, 3, 4);
    let existing = [AcceptedAdditiveShareRecord::new(
        transcript_id,
        nonzero(1),
        sequence(2),
        signing_key_id(3),
    )];

    assert_eq!(
        accept_replay_windowed_authenticated_additive_share(
            slot,
            replay_window(1, 3),
            &existing,
            envelope,
        ),
        Err(SecureAggregateError::EnvelopeReplayDetected)
    );
}

#[test]
fn validates_commitment_slots_for_one_slot_per_participant() {
    let transcript_id = transcript(1);
    let participant_count = nonzero(2);
    let slots = [
        commitment_slot(transcript_id, 1, 2, 3),
        commitment_slot(transcript_id, 1, 4, 5),
    ];

    assert_eq!(
        validate_additive_share_commitment_slots(participant_count, transcript_id, &slots),
        Err(SecureAggregateError::DuplicateCommitmentSlotParticipantIndex)
    );
}

#[test]
fn rejects_duplicate_or_missing_verification_keys() {
    let transcript_id = transcript(1);
    let participant_count = nonzero(2);
    let slots = [
        commitment_slot(transcript_id, 1, 11, 21),
        commitment_slot(transcript_id, 2, 12, 22),
    ];
    let duplicate_keys = [verification_key(21), verification_key(21)];
    let missing_keys = [verification_key(21), verification_key(23)];

    assert_eq!(
        validate_additive_share_verification_keys(participant_count, &slots, &duplicate_keys),
        Err(SecureAggregateError::DuplicateVerificationKeyIdentifier)
    );
    assert_eq!(
        validate_additive_share_verification_keys(participant_count, &slots, &missing_keys),
        Err(SecureAggregateError::VerificationKeyMissing)
    );
}

#[test]
fn combines_authenticated_share_envelopes_for_matching_slots() {
    let transcript_id = transcript(1);
    let participant_count = nonzero(3);
    let slots = [
        commitment_slot(transcript_id, 1, 11, 21),
        commitment_slot(transcript_id, 2, 12, 22),
        commitment_slot(transcript_id, 3, 13, 23),
    ];
    let envelopes = [
        authenticated_envelope(transcript_id, 2, 2_000_000_000, 12, 22, 2, 32),
        authenticated_envelope(transcript_id, 3, 1_294_967_333, 13, 23, 3, 33),
        authenticated_envelope(transcript_id, 1, 1_000_000_000, 11, 21, 1, 31),
    ];

    let result = combine_authenticated_additive_share_envelopes(
        participant_count,
        transcript_id,
        &slots,
        replay_window(1, 3),
        &[],
        &envelopes,
    );
    assert!(result.is_ok());
    let Ok(result) = result else {
        return;
    };
    assert_eq!(result.aggregate(), field(42));
    assert_eq!(result.accepted_records().len(), 3);
}

#[test]
fn combines_signed_authenticated_share_envelopes_for_matching_slots() {
    let transcript_id = transcript(1);
    let participant_count = nonzero(3);
    let slots = [
        commitment_slot(transcript_id, 1, 11, 21),
        commitment_slot(transcript_id, 2, 12, 22),
        commitment_slot(transcript_id, 3, 13, 23),
    ];
    let verification_keys = [
        verification_key(21),
        verification_key(22),
        verification_key(23),
    ];
    let signed_envelopes = [
        signed_envelope(
            22,
            authenticated_envelope(transcript_id, 2, 2_000_000_000, 12, 22, 2, 32),
        ),
        signed_envelope(
            23,
            authenticated_envelope(transcript_id, 3, 1_294_967_333, 13, 23, 3, 33),
        ),
        signed_envelope(
            21,
            authenticated_envelope(transcript_id, 1, 1_000_000_000, 11, 21, 1, 31),
        ),
    ];

    let result = combine_signed_authenticated_additive_share_envelopes(
        participant_count,
        transcript_id,
        &slots,
        &verification_keys,
        replay_window(1, 3),
        &[],
        &signed_envelopes,
    );
    assert!(result.is_ok());
    let Ok(result) = result else {
        return;
    };
    assert_eq!(result.aggregate(), field(42));
    assert_eq!(result.accepted_records().len(), 3);
}

#[test]
fn rejects_duplicate_authenticated_envelope_participant() {
    let transcript_id = transcript(1);
    let participant_count = nonzero(2);
    let slots = [
        commitment_slot(transcript_id, 1, 11, 21),
        commitment_slot(transcript_id, 2, 12, 22),
    ];
    let envelopes = [
        authenticated_envelope(transcript_id, 1, 10, 11, 21, 1, 31),
        authenticated_envelope(transcript_id, 1, 20, 11, 21, 2, 32),
    ];

    assert_eq!(
        combine_authenticated_additive_share_envelopes(
            participant_count,
            transcript_id,
            &slots,
            replay_window(1, 3),
            &[],
            &envelopes,
        ),
        Err(SecureAggregateError::DuplicateParticipantIndex)
    );
}

#[test]
fn validates_bundled_secure_aggregate_vectors() {
    let parsed: Result<VectorFile, serde_yaml::Error> =
        serde_yaml::from_str(ADDITIVE_SECURE_AGGREGATE_VECTORS);
    assert!(parsed.is_ok());
    let Ok(parsed) = parsed else {
        return;
    };

    for vector in parsed.vectors {
        assert_eq!(vector.field_modulus, SECURE_AGGREGATE_FIELD_MODULUS);
        let participant_count = optional_nonzero(vector.participant_count);
        let release_threshold = optional_nonzero(vector.release_threshold);
        let release_policy = ReleasePolicy::new(participant_count, release_threshold);
        assert!(release_policy.is_ok());
        let Ok(release_policy) = release_policy else {
            continue;
        };
        assert!(AdditiveSecureAggregatePlan::new(release_policy).is_ok());

        let shares: Vec<SecureAggregateShare> = vector
            .shares
            .iter()
            .map(|share| {
                SecureAggregateShare::new(
                    optional_nonzero(share.participant_index),
                    field(share.value),
                )
            })
            .collect();

        assert_eq!(
            combine_additive_shares(participant_count, &shares),
            Ok(field(vector.expected_aggregate))
        );
    }
}

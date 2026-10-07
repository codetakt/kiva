#![allow(clippy::wildcard_imports)]

use super::*;

#[kani::proof]
pub(super) fn confidential_participant_lifecycle_matches_strict_transition_table() {
    let current = any_confidential_participant_status();
    let requested = any_confidential_participant_status();
    let result = transition_confidential_participant_status(current, requested);

    if current == requested || confidential_participant_transition_is_allowed(current, requested) {
        assert_eq!(result, Ok(requested));
    } else {
        assert!(matches!(
            result,
            Err(ConfidentialParticipantTransitionError::InvalidTransition)
        ));
    }
}

#[kani::proof]
pub(super) fn confidential_participant_admission_is_active_only() {
    let status = any_confidential_participant_status();

    assert_eq!(
        confidential_participant_accepts_jobs(status),
        status == ConfidentialParticipantStatus::Active
    );
    assert_eq!(
        confidential_participant_accepts_new_keys(status),
        status == ConfidentialParticipantStatus::Active
    );
}

#[kani::proof]
pub(super) fn signing_key_rotation_requires_owner_participant_and_valid_states() {
    let old_state = any_signing_key_lifecycle_state();
    let new_state = any_signing_key_lifecycle_state();
    let owner_matches: bool = kani::any();
    let participant_matches: bool = kani::any();
    let precondition = ShareSigningKeyRotationPrecondition::new(
        old_state,
        new_state,
        owner_matches,
        participant_matches,
    );

    let result = rotate_share_signing_key(precondition);

    if !owner_matches {
        assert!(matches!(
            result,
            Err(ShareSigningKeyLifecycleError::OwnerMismatch)
        ));
    } else if !participant_matches {
        assert!(matches!(
            result,
            Err(ShareSigningKeyLifecycleError::ParticipantMismatch)
        ));
    } else if old_state != ShareSigningKeyLifecycleState::Active {
        assert!(matches!(
            result,
            Err(ShareSigningKeyLifecycleError::OldKeyNotActive)
        ));
    } else if new_state != ShareSigningKeyLifecycleState::Missing {
        assert!(matches!(
            result,
            Err(ShareSigningKeyLifecycleError::NewKeyAlreadyRegistered)
        ));
    } else {
        assert!(result.is_ok());
    }
}

#[kani::proof]
pub(super) fn signing_key_rotation_revokes_old_and_activates_new_key() {
    let precondition = ShareSigningKeyRotationPrecondition::new(
        ShareSigningKeyLifecycleState::Active,
        ShareSigningKeyLifecycleState::Missing,
        true,
        true,
    );

    let Ok(outcome) = rotate_share_signing_key(precondition) else {
        assert!(false);
        return;
    };

    assert_eq!(outcome.old_state(), ShareSigningKeyLifecycleState::Revoked);
    assert_eq!(outcome.new_state(), ShareSigningKeyLifecycleState::Active);
    assert!(!share_signing_key_accepts_new_submissions(
        outcome.old_state()
    ));
    assert!(share_signing_key_accepts_new_submissions(
        outcome.new_state()
    ));
}

#[kani::proof]
pub(super) fn signing_key_revocation_is_idempotent_and_fail_closed() {
    let state = any_signing_key_lifecycle_state();
    let once = revoke_share_signing_key(state);
    let twice = revoke_share_signing_key(once);

    assert_eq!(once, twice);
    assert!(!share_signing_key_accepts_new_submissions(once));
}

#[kani::proof]
pub(super) fn secure_aggregate_field_addition_stays_inside_field() {
    let left = bounded_field_element(kani::any());
    let right = bounded_field_element(kani::any());
    let total = left.add_mod(right);

    assert!(total.value() < SECURE_AGGREGATE_FIELD_MODULUS);
}

#[kani::proof]
pub(super) fn additive_secure_aggregate_requires_all_participants() {
    let participant_count = bounded_participant_count(kani::any());
    let release_threshold = bounded_threshold(kani::any(), participant_count);
    let Ok(policy) = ReleasePolicy::new(participant_count, release_threshold) else {
        assert!(false);
        return;
    };

    let result = AdditiveSecureAggregatePlan::new(policy);
    if participant_count == release_threshold {
        assert!(result.is_ok());
    } else {
        assert!(matches!(
            result,
            Err(SecureAggregateError::ThresholdMustRequireAllParticipants)
        ));
    }
}

#[kani::proof]
pub(super) fn authenticated_share_acceptance_matches_commitment_slot() {
    let first_participant = NonZeroUsize::MIN;
    let Some(second_participant) = NonZeroUsize::new(2) else {
        assert!(false);
        return;
    };
    let transcript = fixed_transcript_id(1);
    let alternate_transcript = fixed_transcript_id(2);
    let commitment = fixed_commitment(3);
    let alternate_commitment = fixed_commitment(4);
    let signing_key_id = fixed_signing_key_id(5);
    let alternate_signing_key_id = fixed_signing_key_id(6);

    let transcript_matches: bool = kani::any();
    let participant_matches: bool = kani::any();
    let commitment_matches: bool = kani::any();
    let signing_key_matches: bool = kani::any();

    let slot =
        AdditiveShareCommitmentSlot::new(transcript, first_participant, commitment, signing_key_id);
    let envelope = AuthenticatedAdditiveShareEnvelope::new(
        if transcript_matches {
            transcript
        } else {
            alternate_transcript
        },
        SecureAggregateShare::new(
            if participant_matches {
                first_participant
            } else {
                second_participant
            },
            SecureAggregateFieldElement::ZERO,
        ),
        if commitment_matches {
            commitment
        } else {
            alternate_commitment
        },
        if signing_key_matches {
            signing_key_id
        } else {
            alternate_signing_key_id
        },
        fixed_sequence(1),
        fixed_transport_proof(7),
    );

    let result = accept_authenticated_additive_share(slot, envelope);
    if transcript_matches && participant_matches && commitment_matches && signing_key_matches {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
    }
}

#[kani::proof]
pub(super) fn verification_key_validation_requires_one_registered_key_per_slot() {
    let first_participant = NonZeroUsize::MIN;
    let Some(second_participant) = NonZeroUsize::new(2) else {
        assert!(false);
        return;
    };
    let participant_count = second_participant;
    let transcript = fixed_transcript_id(1);
    let first_signing_key_id = fixed_signing_key_id(21);
    let second_signing_key_id = fixed_signing_key_id(22);
    let unrelated_signing_key_id = fixed_signing_key_id(23);
    let slots = [
        AdditiveShareCommitmentSlot::new(
            transcript,
            first_participant,
            fixed_commitment(11),
            first_signing_key_id,
        ),
        AdditiveShareCommitmentSlot::new(
            transcript,
            second_participant,
            fixed_commitment(12),
            second_signing_key_id,
        ),
    ];
    let duplicate_identifier: bool = kani::any();
    let missing_slot_key: bool = kani::any();
    let verification_keys = [
        fixed_verification_key(first_signing_key_id, 31),
        if duplicate_identifier {
            fixed_verification_key(first_signing_key_id, 32)
        } else if missing_slot_key {
            fixed_verification_key(unrelated_signing_key_id, 33)
        } else {
            fixed_verification_key(second_signing_key_id, 34)
        },
    ];

    let result =
        validate_additive_share_verification_keys(participant_count, &slots, &verification_keys);
    if duplicate_identifier {
        assert!(matches!(
            result,
            Err(SecureAggregateError::DuplicateVerificationKeyIdentifier)
        ));
    } else if missing_slot_key {
        assert!(matches!(
            result,
            Err(SecureAggregateError::VerificationKeyMissing)
        ));
    } else {
        assert!(result.is_ok());
    }
}

#[kani::proof]
pub(super) fn replay_windowed_acceptance_requires_sequence_inside_window() {
    let first_participant = NonZeroUsize::MIN;
    let transcript = fixed_transcript_id(1);
    let commitment = fixed_commitment(2);
    let signing_key_id = fixed_signing_key_id(3);
    let slot =
        AdditiveShareCommitmentSlot::new(transcript, first_participant, commitment, signing_key_id);
    let inside_window: bool = kani::any();
    let envelope = AuthenticatedAdditiveShareEnvelope::new(
        transcript,
        SecureAggregateShare::new(first_participant, SecureAggregateFieldElement::ZERO),
        commitment,
        signing_key_id,
        if inside_window {
            fixed_sequence(2)
        } else {
            fixed_sequence(4)
        },
        fixed_transport_proof(7),
    );
    let Ok(replay_window) = AdditiveShareReplayWindow::new(fixed_sequence(1), fixed_sequence(3))
    else {
        assert!(false);
        return;
    };

    let result =
        accept_replay_windowed_authenticated_additive_share(slot, replay_window, &[], envelope);
    if inside_window {
        assert!(result.is_ok());
    } else {
        assert!(matches!(
            result,
            Err(SecureAggregateError::EnvelopeSequenceOutsideReplayWindow)
        ));
    }
}

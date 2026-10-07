#![allow(clippy::wildcard_imports)]

use super::*;

#[test]
fn threshold_frost_key_ceremony_accepts_reviewed_metadata() {
    let job_id = transcript(121);
    let plan = threshold_frost_plan(3, 2, true);
    let participants = [
        threshold_frost_participant(1, 101, 111),
        threshold_frost_participant(2, 102, 112),
        threshold_frost_participant(3, 103, 113),
    ];

    let accepted = accept_threshold_frost_key_ceremony(plan, job_id, &[], &participants);

    assert!(accepted.is_ok());
    let Ok(accepted) = accepted else {
        return;
    };
    let expected_digest = derive_threshold_frost_key_ceremony_digest(plan, job_id, &participants);
    assert!(expected_digest.is_ok());
    let Ok(expected_digest) = expected_digest else {
        return;
    };
    assert_eq!(accepted.participant_count(), nonzero(3));
    assert_eq!(accepted.release_threshold(), nonzero(2));
    assert_eq!(
        accepted.record(),
        ThresholdFrostKeyCeremonyRecord::new(job_id, expected_digest, sequence(7))
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn threshold_frost_key_ceremony_rejects_unreviewed_or_invalid_metadata() {
    let job_id = transcript(122);
    let plan = threshold_frost_plan(3, 2, true);
    let review_pending = threshold_frost_plan(3, 2, false);
    let valid = [
        threshold_frost_participant(1, 101, 111),
        threshold_frost_participant(2, 102, 112),
        threshold_frost_participant(3, 103, 113),
    ];

    assert_eq!(
        accept_threshold_frost_key_ceremony(review_pending, job_id, &[], &valid),
        Err(SecureAggregateError::ThresholdFrostExternalReviewRequired)
    );
    assert_eq!(
        accept_threshold_frost_key_ceremony(plan, job_id, &[], &valid[..2]),
        Err(SecureAggregateError::ThresholdFrostCeremonyParticipantCountMismatch)
    );

    let duplicate = [
        threshold_frost_participant(1, 101, 111),
        threshold_frost_participant(1, 102, 112),
        threshold_frost_participant(3, 103, 113),
    ];
    assert_eq!(
        accept_threshold_frost_key_ceremony(plan, job_id, &[], &duplicate),
        Err(SecureAggregateError::DuplicateParticipantIndex)
    );

    let removed = [
        threshold_frost_participant_with_status(
            1,
            ThresholdFrostParticipantTranscriptStatus::Removed,
            101,
            Some(111),
            true,
            false,
        ),
        threshold_frost_participant(2, 102, 112),
        threshold_frost_participant(3, 103, 113),
    ];
    assert_eq!(
        accept_threshold_frost_key_ceremony(plan, job_id, &[], &removed),
        Err(SecureAggregateError::ThresholdFrostParticipantInactive)
    );

    let aborted = [
        threshold_frost_participant_with_status(
            1,
            ThresholdFrostParticipantTranscriptStatus::Aborted,
            101,
            Some(111),
            true,
            false,
        ),
        threshold_frost_participant(2, 102, 112),
        threshold_frost_participant(3, 103, 113),
    ];
    assert_eq!(
        accept_threshold_frost_key_ceremony(plan, job_id, &[], &aborted),
        Err(SecureAggregateError::ThresholdFrostParticipantAborted)
    );

    let missing_custody = [
        threshold_frost_participant_with_status(
            1,
            ThresholdFrostParticipantTranscriptStatus::Submitted,
            101,
            None,
            true,
            false,
        ),
        threshold_frost_participant(2, 102, 112),
        threshold_frost_participant(3, 103, 113),
    ];
    assert_eq!(
        accept_threshold_frost_key_ceremony(plan, job_id, &[], &missing_custody),
        Err(SecureAggregateError::ThresholdFrostShareCustodyMissing)
    );

    let unresolved = [
        threshold_frost_participant_with_status(
            1,
            ThresholdFrostParticipantTranscriptStatus::Submitted,
            101,
            Some(111),
            false,
            false,
        ),
        threshold_frost_participant(2, 102, 112),
        threshold_frost_participant(3, 103, 113),
    ];
    assert_eq!(
        accept_threshold_frost_key_ceremony(plan, job_id, &[], &unresolved),
        Err(SecureAggregateError::ThresholdFrostComplaintUnresolved)
    );

    let equivocation = [
        threshold_frost_participant_with_status(
            1,
            ThresholdFrostParticipantTranscriptStatus::Submitted,
            101,
            Some(111),
            true,
            true,
        ),
        threshold_frost_participant(2, 102, 112),
        threshold_frost_participant(3, 103, 113),
    ];
    assert_eq!(
        accept_threshold_frost_key_ceremony(plan, job_id, &[], &equivocation),
        Err(SecureAggregateError::ThresholdFrostEquivocationDetected)
    );

    let accepted = accept_threshold_frost_key_ceremony(plan, job_id, &[], &valid);
    assert!(accepted.is_ok());
    let Ok(accepted) = accepted else {
        return;
    };
    assert_eq!(
        accept_threshold_frost_key_ceremony(plan, job_id, &[accepted.record()], &valid),
        Err(SecureAggregateError::ThresholdFrostReplayDetected)
    );
}

#[test]
fn threshold_frost_signature_transcript_enforces_threshold_digest_and_replay() {
    let job_id = transcript(123);
    let plan = threshold_frost_plan(3, 2, true);
    let participants = [
        threshold_frost_participant(1, 101, 111),
        threshold_frost_participant(2, 102, 112),
        threshold_frost_participant(3, 103, 113),
    ];
    let accepted = accept_threshold_frost_key_ceremony(plan, job_id, &[], &participants);
    assert!(accepted.is_ok());
    let Ok(accepted) = accepted else {
        return;
    };
    let message_digest = commitment(151);
    let signature_digest = commitment(152);
    let shares = [
        threshold_frost_signature_share(1, 161),
        threshold_frost_signature_share(2, 162),
    ];
    let transcript_digest = derive_threshold_frost_signature_transcript_digest(
        accepted.record().ceremony_digest(),
        message_digest,
        signature_digest,
        &shares,
    );
    assert!(transcript_digest.is_ok());
    let Ok(transcript_digest) = transcript_digest else {
        return;
    };

    let signed = accept_threshold_frost_signature_transcript(
        accepted,
        message_digest,
        signature_digest,
        transcript_digest,
        &[],
        &shares,
    );
    assert!(signed.is_ok());
    let Ok(signed) = signed else {
        return;
    };
    assert_eq!(signed.signature_share_count(), nonzero(2));
    assert_eq!(signed.threshold(), nonzero(2));

    assert_eq!(
        accept_threshold_frost_signature_transcript(
            accepted,
            message_digest,
            signature_digest,
            transcript_digest,
            &[],
            &shares[..1],
        ),
        Err(SecureAggregateError::ThresholdFrostSignatureShareCountBelowThreshold)
    );
    let duplicate = [
        threshold_frost_signature_share(1, 161),
        threshold_frost_signature_share(1, 162),
    ];
    assert_eq!(
        accept_threshold_frost_signature_transcript(
            accepted,
            message_digest,
            signature_digest,
            transcript_digest,
            &[],
            &duplicate,
        ),
        Err(SecureAggregateError::DuplicateParticipantIndex)
    );
    assert_eq!(
        accept_threshold_frost_signature_transcript(
            accepted,
            message_digest,
            signature_digest,
            transcript_digest,
            &[signed.record()],
            &shares,
        ),
        Err(SecureAggregateError::ThresholdFrostReplayDetected)
    );
    assert_eq!(
        accept_threshold_frost_signature_transcript(
            accepted,
            message_digest,
            signature_digest,
            commitment(199),
            &[],
            &shares,
        ),
        Err(SecureAggregateError::ThresholdFrostSignatureDigestMismatch)
    );
}

#[test]
fn threshold_frost_key_ceremony_gate_fails_closed() {
    let missing_runtime =
        ThresholdFrostKeyCeremonyGateContext::new(ThresholdFrostKeyCeremonyFacts::from_bits(
            ThresholdFrostKeyCeremonyFacts::EXTERNAL_REVIEW_COMPLETED
                | ThresholdFrostKeyCeremonyFacts::PARTICIPANT_COUNT_MATCHES
                | ThresholdFrostKeyCeremonyFacts::PARTICIPANTS_ACTIVE
                | ThresholdFrostKeyCeremonyFacts::SHARE_CUSTODY_PRESENT
                | ThresholdFrostKeyCeremonyFacts::COMPLAINTS_RESOLVED
                | ThresholdFrostKeyCeremonyFacts::NO_EQUIVOCATION
                | ThresholdFrostKeyCeremonyFacts::NO_REPLAY,
        ));
    assert_eq!(
        validate_threshold_frost_key_ceremony_gate_context(missing_runtime),
        Err(SecureAggregateError::ThresholdFrostRuntimeBindingMissing)
    );

    let complaint_unresolved =
        ThresholdFrostKeyCeremonyGateContext::new(ThresholdFrostKeyCeremonyFacts::from_bits(
            ThresholdFrostKeyCeremonyFacts::RUNTIME_BOUND
                | ThresholdFrostKeyCeremonyFacts::EXTERNAL_REVIEW_COMPLETED
                | ThresholdFrostKeyCeremonyFacts::PARTICIPANT_COUNT_MATCHES
                | ThresholdFrostKeyCeremonyFacts::PARTICIPANTS_ACTIVE
                | ThresholdFrostKeyCeremonyFacts::SHARE_CUSTODY_PRESENT
                | ThresholdFrostKeyCeremonyFacts::NO_EQUIVOCATION
                | ThresholdFrostKeyCeremonyFacts::NO_REPLAY,
        ));
    assert_eq!(
        validate_threshold_frost_key_ceremony_gate_context(complaint_unresolved),
        Err(SecureAggregateError::ThresholdFrostComplaintUnresolved)
    );

    let ready =
        ThresholdFrostKeyCeremonyGateContext::new(ThresholdFrostKeyCeremonyFacts::from_bits(
            ThresholdFrostKeyCeremonyFacts::RUNTIME_BOUND
                | ThresholdFrostKeyCeremonyFacts::EXTERNAL_REVIEW_COMPLETED
                | ThresholdFrostKeyCeremonyFacts::PARTICIPANT_COUNT_MATCHES
                | ThresholdFrostKeyCeremonyFacts::PARTICIPANTS_ACTIVE
                | ThresholdFrostKeyCeremonyFacts::SHARE_CUSTODY_PRESENT
                | ThresholdFrostKeyCeremonyFacts::COMPLAINTS_RESOLVED
                | ThresholdFrostKeyCeremonyFacts::NO_EQUIVOCATION
                | ThresholdFrostKeyCeremonyFacts::NO_REPLAY,
        ));
    assert_eq!(
        validate_threshold_frost_key_ceremony_gate_context(ready),
        Ok(())
    );
}

#[test]
fn threshold_frost_release_gate_fails_closed() {
    let missing_ceremony =
        ThresholdFrostReleaseContext::new(ThresholdFrostReleaseFacts::from_bits(
            ThresholdFrostReleaseFacts::SIGNATURE_COMPUTED
                | ThresholdFrostReleaseFacts::EXTERNAL_REVIEW_COMPLETED
                | ThresholdFrostReleaseFacts::ACTIVE_KEY_EPOCH
                | ThresholdFrostReleaseFacts::POLICY_APPROVED
                | ThresholdFrostReleaseFacts::GOVERNED_KEY_USE,
        ));
    assert_eq!(
        validate_threshold_frost_release(missing_ceremony),
        Err(SecureAggregateError::ThresholdFrostReleaseRequiresKeyCeremony)
    );

    let review_pending = ThresholdFrostReleaseContext::new(ThresholdFrostReleaseFacts::from_bits(
        ThresholdFrostReleaseFacts::KEY_CEREMONY_COMPLETED
            | ThresholdFrostReleaseFacts::SIGNATURE_COMPUTED
            | ThresholdFrostReleaseFacts::ACTIVE_KEY_EPOCH
            | ThresholdFrostReleaseFacts::POLICY_APPROVED
            | ThresholdFrostReleaseFacts::GOVERNED_KEY_USE,
    ));
    assert_eq!(
        validate_threshold_frost_release(review_pending),
        Err(SecureAggregateError::ThresholdFrostExternalReviewRequired)
    );

    let ready = ThresholdFrostReleaseContext::new(ThresholdFrostReleaseFacts::from_bits(
        ThresholdFrostReleaseFacts::KEY_CEREMONY_COMPLETED
            | ThresholdFrostReleaseFacts::SIGNATURE_COMPUTED
            | ThresholdFrostReleaseFacts::EXTERNAL_REVIEW_COMPLETED
            | ThresholdFrostReleaseFacts::ACTIVE_KEY_EPOCH
            | ThresholdFrostReleaseFacts::POLICY_APPROVED
            | ThresholdFrostReleaseFacts::GOVERNED_KEY_USE,
    ));
    assert_eq!(validate_threshold_frost_release(ready), Ok(()));
}

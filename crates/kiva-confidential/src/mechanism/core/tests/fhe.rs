#![allow(clippy::wildcard_imports)]

use super::*;

#[test]
fn fhe_feasibility_parameters_are_strict() {
    let parameter_set = FheFeasibilityParameterSet::sum_profile_v1();
    assert_eq!(validate_fhe_feasibility_parameters(parameter_set), Ok(()));
    let Some(max_participant_count) = NonZeroUsize::new(16) else {
        return;
    };
    assert_eq!(
        FheFeasibilityParameterSet::new(
            FheFeasibilitySchemeFamily::BfvStyleIntegerSum,
            super::FheFeasibilityOperation::Sum,
            2_048,
            parameter_set.plaintext_modulus(),
            parameter_set.ciphertext_modulus_bits(),
            parameter_set.security_level_bits(),
            max_participant_count,
            parameter_set.max_plaintext_value(),
        ),
        Err(SecureAggregateError::FheParameterSetRejected)
    );

    let invalid_policy = policy(3, 2);
    assert_eq!(
        FheFeasibilityPlan::new(invalid_policy, parameter_set),
        Err(SecureAggregateError::FheRequiresAllParticipants)
    );
    assert_eq!(
        ConfidentialMechanismPlan::new(
            ConfidentialMechanismKind::FheSumFeasibility,
            invalid_policy,
        ),
        Err(SecureAggregateError::FheRequiresAllParticipants)
    );
}

#[test]
fn fhe_feasibility_fixture_evaluates_sum_and_releases_only_after_policy() {
    let plan = fhe_plan(3);
    let job_id = transcript(91);
    let ciphertexts = [
        fhe_ciphertext(plan, job_id, 1, 42, 11),
        fhe_ciphertext(plan, job_id, 2, 57, 12),
        fhe_ciphertext(plan, job_id, 3, 100, 13),
    ];

    let evaluation = evaluate_fhe_feasibility_sum(plan, job_id, &ciphertexts);
    assert!(evaluation.is_ok());
    let Ok(evaluation) = evaluation else {
        return;
    };
    assert_eq!(evaluation.participant_count(), nonzero(3));
    assert_eq!(evaluation.operation(), super::FheFeasibilityOperation::Sum);
    assert_eq!(
        evaluation.noise_budget_bits(),
        super::FHE_FEASIBILITY_INITIAL_NOISE_BUDGET_BITS
            - super::FHE_FEASIBILITY_SUM_NOISE_COST_BITS
    );

    let context = FheFeasibilityReleaseContext::new(FheFeasibilityReleaseFacts::from_bits(
        FheFeasibilityReleaseFacts::EVALUATION_COMPLETED
            | FheFeasibilityReleaseFacts::POLICY_APPROVED
            | FheFeasibilityReleaseFacts::GOVERNED_DATA_PRODUCT
            | FheFeasibilityReleaseFacts::PLAINTEXT_BOUNDARY_CLEAR,
    ));
    let released = release_fhe_feasibility_sum_fixture(evaluation, context);
    assert!(released.is_ok());
    let Ok(released) = released else {
        return;
    };
    assert_eq!(released.aggregate_value(), 199);
    assert_eq!(
        released.evaluated_ciphertext_digest(),
        evaluation.evaluated_ciphertext_digest()
    );
}

#[test]
fn fhe_feasibility_fixture_rejects_plaintext_digest_and_duplicate_failures() {
    let plan = fhe_plan(2);
    let parameter_set = plan.parameter_set();
    let job_id = transcript(92);
    assert_eq!(
        FheFeasibilityPlaintext::new(
            parameter_set,
            nonzero(1),
            parameter_set.max_plaintext_value() + 1,
        ),
        Err(SecureAggregateError::FhePlaintextOutsideRange)
    );

    let plaintext = fhe_plaintext(parameter_set, 1, 42);
    let nonce = fhe_nonce(21);
    let ciphertext = encrypt_fhe_feasibility_plaintext_fixture(plan, job_id, plaintext, nonce);
    assert!(ciphertext.is_ok());
    let Ok(ciphertext) = ciphertext else {
        return;
    };
    let tampered = FheFeasibilityCiphertextFixture::from_fixture_parts(
        ciphertext.job_id(),
        ciphertext.parameter_set(),
        ciphertext.participant_index(),
        commitment(99),
        ciphertext.slot_count(),
        ciphertext.noise_budget_bits(),
        42,
    );
    assert_eq!(
        verify_fhe_feasibility_ciphertext_fixture(
            job_id,
            parameter_set,
            plaintext,
            nonce,
            tampered,
        ),
        Err(SecureAggregateError::FheCiphertextDigestMismatch)
    );

    let duplicate = [ciphertext, ciphertext];
    assert_eq!(
        evaluate_fhe_feasibility_sum(plan, job_id, &duplicate),
        Err(SecureAggregateError::DuplicateParticipantIndex)
    );
}

#[test]
fn fhe_feasibility_release_gate_fails_closed() {
    assert_eq!(
        validate_fhe_feasibility_release(FheFeasibilityReleaseContext::new(
            FheFeasibilityReleaseFacts::from_bits(
                FheFeasibilityReleaseFacts::POLICY_APPROVED
                    | FheFeasibilityReleaseFacts::GOVERNED_DATA_PRODUCT
                    | FheFeasibilityReleaseFacts::PLAINTEXT_BOUNDARY_CLEAR,
            ),
        )),
        Err(SecureAggregateError::FheReleaseRequiresEvaluation)
    );
    assert_eq!(
        validate_fhe_feasibility_release(FheFeasibilityReleaseContext::new(
            FheFeasibilityReleaseFacts::from_bits(
                FheFeasibilityReleaseFacts::EVALUATION_COMPLETED
                    | FheFeasibilityReleaseFacts::GOVERNED_DATA_PRODUCT
                    | FheFeasibilityReleaseFacts::PLAINTEXT_BOUNDARY_CLEAR,
            ),
        )),
        Err(SecureAggregateError::FheReleaseRequiresPolicyApproval)
    );
    assert_eq!(
        validate_fhe_feasibility_release(FheFeasibilityReleaseContext::new(
            FheFeasibilityReleaseFacts::from_bits(
                FheFeasibilityReleaseFacts::EVALUATION_COMPLETED
                    | FheFeasibilityReleaseFacts::POLICY_APPROVED
                    | FheFeasibilityReleaseFacts::PLAINTEXT_BOUNDARY_CLEAR,
            ),
        )),
        Err(SecureAggregateError::FheReleaseRequiresGovernedDataProduct)
    );
    assert_eq!(
        validate_fhe_feasibility_release(FheFeasibilityReleaseContext::new(
            FheFeasibilityReleaseFacts::from_bits(
                FheFeasibilityReleaseFacts::EVALUATION_COMPLETED
                    | FheFeasibilityReleaseFacts::POLICY_APPROVED
                    | FheFeasibilityReleaseFacts::GOVERNED_DATA_PRODUCT,
            ),
        )),
        Err(SecureAggregateError::FhePlaintextBoundaryViolation)
    );
    assert_eq!(
        validate_fhe_feasibility_release(FheFeasibilityReleaseContext::new(
            FheFeasibilityReleaseFacts::from_bits(
                FheFeasibilityReleaseFacts::EVALUATION_COMPLETED
                    | FheFeasibilityReleaseFacts::POLICY_APPROVED
                    | FheFeasibilityReleaseFacts::GOVERNED_DATA_PRODUCT
                    | FheFeasibilityReleaseFacts::PLAINTEXT_BOUNDARY_CLEAR,
            ),
        )),
        Ok(())
    );
}

#[test]
fn fhe_runtime_accepts_reviewed_ciphertexts_evaluation_and_release_metadata() {
    let job_id = transcript(131);
    let plan = fhe_runtime_plan(3, true);
    let ciphertexts = [
        fhe_runtime_ciphertext(1, 181, 172, Some(191)),
        fhe_runtime_ciphertext(2, 182, 172, Some(192)),
        fhe_runtime_ciphertext(3, 183, 172, Some(193)),
    ];
    let transcript_digest =
        derive_fhe_runtime_ciphertext_transcript_digest(plan, job_id, &ciphertexts);
    assert!(transcript_digest.is_ok());
    let Ok(transcript_digest) = transcript_digest else {
        return;
    };
    let accepted = accept_fhe_runtime_ciphertext_transcript(
        plan,
        job_id,
        transcript_digest,
        &[],
        &ciphertexts,
    );
    assert!(accepted.is_ok());
    let Ok(accepted) = accepted else {
        return;
    };
    assert_eq!(accepted.participant_count(), nonzero(3));
    assert_eq!(accepted.record().transcript_digest(), transcript_digest);

    let evaluated_ciphertext_digest = commitment(202);
    let evaluation_proof_digest = commitment(201);
    let evaluation_digest = derive_fhe_runtime_evaluation_digest(
        accepted,
        evaluated_ciphertext_digest,
        evaluation_proof_digest,
    );
    assert!(evaluation_digest.is_ok());
    let Ok(evaluation_digest) = evaluation_digest else {
        return;
    };
    let evaluation = accept_fhe_runtime_evaluation(
        accepted,
        evaluated_ciphertext_digest,
        evaluation_proof_digest,
        evaluation_digest,
        &[],
    );
    assert!(evaluation.is_ok());
    let Ok(evaluation) = evaluation else {
        return;
    };
    assert_eq!(evaluation.operation(), FheRuntimeOperation::Sum);
    assert_eq!(evaluation.record().evaluation_digest(), evaluation_digest);

    let context = FheRuntimeReleaseContext::new(FheRuntimeReleaseFacts::from_bits(
        FheRuntimeReleaseFacts::CIPHERTEXT_TRANSCRIPT_ACCEPTED
            | FheRuntimeReleaseFacts::EVALUATION_COMPLETED
            | FheRuntimeReleaseFacts::EXTERNAL_REVIEW_COMPLETED
            | FheRuntimeReleaseFacts::ACTIVE_KEY_EPOCH
            | FheRuntimeReleaseFacts::POLICY_APPROVED
            | FheRuntimeReleaseFacts::GOVERNED_DATA_PRODUCT,
    ));
    let released = release_fhe_runtime_sum_metadata(evaluation, context);
    assert!(released.is_ok());
    let Ok(released) = released else {
        return;
    };
    assert_eq!(released.operation(), FheRuntimeOperation::Sum);
    assert_eq!(released.participant_count(), nonzero(3));
    assert_eq!(released.evaluation_digest(), evaluation_digest);
}

#[test]
#[allow(clippy::too_many_lines)]
fn fhe_runtime_rejects_unreviewed_invalid_and_replayed_ciphertexts() {
    let job_id = transcript(132);
    let plan = fhe_runtime_plan(3, true);
    let review_pending = fhe_runtime_plan(3, false);
    let valid = [
        fhe_runtime_ciphertext(1, 181, 172, Some(191)),
        fhe_runtime_ciphertext(2, 182, 172, Some(192)),
        fhe_runtime_ciphertext(3, 183, 172, Some(193)),
    ];
    let transcript_digest = derive_fhe_runtime_ciphertext_transcript_digest(plan, job_id, &valid);
    assert!(transcript_digest.is_ok());
    let Ok(transcript_digest) = transcript_digest else {
        return;
    };

    assert_eq!(
        accept_fhe_runtime_ciphertext_transcript(
            review_pending,
            job_id,
            transcript_digest,
            &[],
            &valid,
        ),
        Err(SecureAggregateError::FheRuntimeExternalReviewRequired)
    );
    assert_eq!(
        accept_fhe_runtime_ciphertext_transcript(plan, job_id, transcript_digest, &[], &valid[..2],),
        Err(SecureAggregateError::FheRuntimeCiphertextCountMismatch)
    );

    let duplicate = [
        fhe_runtime_ciphertext(1, 181, 172, Some(191)),
        fhe_runtime_ciphertext(1, 182, 172, Some(192)),
        fhe_runtime_ciphertext(3, 183, 172, Some(193)),
    ];
    assert_eq!(
        accept_fhe_runtime_ciphertext_transcript(plan, job_id, transcript_digest, &[], &duplicate,),
        Err(SecureAggregateError::DuplicateParticipantIndex)
    );

    let removed = [
        fhe_runtime_ciphertext_with_status(
            1,
            FheRuntimeParticipantCiphertextStatus::Removed,
            181,
            172,
            Some(191),
        ),
        fhe_runtime_ciphertext(2, 182, 172, Some(192)),
        fhe_runtime_ciphertext(3, 183, 172, Some(193)),
    ];
    assert_eq!(
        accept_fhe_runtime_ciphertext_transcript(plan, job_id, transcript_digest, &[], &removed,),
        Err(SecureAggregateError::FheRuntimeCiphertextParticipantInactive)
    );

    let aborted = [
        fhe_runtime_ciphertext_with_status(
            1,
            FheRuntimeParticipantCiphertextStatus::Aborted,
            181,
            172,
            Some(191),
        ),
        fhe_runtime_ciphertext(2, 182, 172, Some(192)),
        fhe_runtime_ciphertext(3, 183, 172, Some(193)),
    ];
    assert_eq!(
        accept_fhe_runtime_ciphertext_transcript(plan, job_id, transcript_digest, &[], &aborted,),
        Err(SecureAggregateError::FheRuntimeCiphertextParticipantAborted)
    );

    let missing_custody = [
        fhe_runtime_ciphertext(1, 181, 172, None),
        fhe_runtime_ciphertext(2, 182, 172, Some(192)),
        fhe_runtime_ciphertext(3, 183, 172, Some(193)),
    ];
    assert_eq!(
        accept_fhe_runtime_ciphertext_transcript(
            plan,
            job_id,
            transcript_digest,
            &[],
            &missing_custody,
        ),
        Err(SecureAggregateError::FheRuntimeKeyCustodyMissing)
    );

    let parameter_mismatch = [
        fhe_runtime_ciphertext(1, 181, 199, Some(191)),
        fhe_runtime_ciphertext(2, 182, 172, Some(192)),
        fhe_runtime_ciphertext(3, 183, 172, Some(193)),
    ];
    assert_eq!(
        accept_fhe_runtime_ciphertext_transcript(
            plan,
            job_id,
            transcript_digest,
            &[],
            &parameter_mismatch,
        ),
        Err(SecureAggregateError::FheRuntimeCiphertextParameterMismatch)
    );
    assert_eq!(
        accept_fhe_runtime_ciphertext_transcript(plan, job_id, commitment(199), &[], &valid,),
        Err(SecureAggregateError::FheRuntimeCiphertextDigestMismatch)
    );

    let accepted =
        accept_fhe_runtime_ciphertext_transcript(plan, job_id, transcript_digest, &[], &valid);
    assert!(accepted.is_ok());
    let Ok(accepted) = accepted else {
        return;
    };
    assert_eq!(
        accept_fhe_runtime_ciphertext_transcript(
            plan,
            job_id,
            transcript_digest,
            &[accepted.record()],
            &valid,
        ),
        Err(SecureAggregateError::FheRuntimeCiphertextReplayDetected)
    );
}

#[test]
fn fhe_runtime_evaluation_and_release_gates_fail_closed() {
    let job_id = transcript(133);
    let plan = fhe_runtime_plan(2, true);
    let ciphertexts = [
        fhe_runtime_ciphertext(1, 181, 172, Some(191)),
        fhe_runtime_ciphertext(2, 182, 172, Some(192)),
    ];
    let transcript_digest =
        derive_fhe_runtime_ciphertext_transcript_digest(plan, job_id, &ciphertexts);
    assert!(transcript_digest.is_ok());
    let Ok(transcript_digest) = transcript_digest else {
        return;
    };
    let accepted = accept_fhe_runtime_ciphertext_transcript(
        plan,
        job_id,
        transcript_digest,
        &[],
        &ciphertexts,
    );
    assert!(accepted.is_ok());
    let Ok(accepted) = accepted else {
        return;
    };
    let evaluated_ciphertext_digest = commitment(202);
    let evaluation_proof_digest = commitment(201);
    let evaluation_digest = derive_fhe_runtime_evaluation_digest(
        accepted,
        evaluated_ciphertext_digest,
        evaluation_proof_digest,
    );
    assert!(evaluation_digest.is_ok());
    let Ok(evaluation_digest) = evaluation_digest else {
        return;
    };

    assert_eq!(
        accept_fhe_runtime_evaluation(
            accepted,
            evaluated_ciphertext_digest,
            evaluation_proof_digest,
            commitment(199),
            &[],
        ),
        Err(SecureAggregateError::FheRuntimeEvaluationDigestMismatch)
    );
    let evaluation = accept_fhe_runtime_evaluation(
        accepted,
        evaluated_ciphertext_digest,
        evaluation_proof_digest,
        evaluation_digest,
        &[],
    );
    assert!(evaluation.is_ok());
    let Ok(evaluation) = evaluation else {
        return;
    };
    assert_eq!(
        accept_fhe_runtime_evaluation(
            accepted,
            evaluated_ciphertext_digest,
            evaluation_proof_digest,
            evaluation_digest,
            &[evaluation.record()],
        ),
        Err(SecureAggregateError::FheRuntimeEvaluationReplayDetected)
    );

    let missing_evaluation = FheRuntimeReleaseContext::new(FheRuntimeReleaseFacts::from_bits(
        FheRuntimeReleaseFacts::CIPHERTEXT_TRANSCRIPT_ACCEPTED
            | FheRuntimeReleaseFacts::EXTERNAL_REVIEW_COMPLETED
            | FheRuntimeReleaseFacts::ACTIVE_KEY_EPOCH
            | FheRuntimeReleaseFacts::POLICY_APPROVED
            | FheRuntimeReleaseFacts::GOVERNED_DATA_PRODUCT,
    ));
    assert_eq!(
        validate_fhe_runtime_release(missing_evaluation),
        Err(SecureAggregateError::FheRuntimeReleaseRequiresEvaluation)
    );
    let review_pending = FheRuntimeReleaseContext::new(FheRuntimeReleaseFacts::from_bits(
        FheRuntimeReleaseFacts::CIPHERTEXT_TRANSCRIPT_ACCEPTED
            | FheRuntimeReleaseFacts::EVALUATION_COMPLETED
            | FheRuntimeReleaseFacts::ACTIVE_KEY_EPOCH
            | FheRuntimeReleaseFacts::POLICY_APPROVED
            | FheRuntimeReleaseFacts::GOVERNED_DATA_PRODUCT,
    ));
    assert_eq!(
        validate_fhe_runtime_release(review_pending),
        Err(SecureAggregateError::FheRuntimeReleaseRequiresExternalReview)
    );
    let ready = FheRuntimeReleaseContext::new(FheRuntimeReleaseFacts::from_bits(
        FheRuntimeReleaseFacts::CIPHERTEXT_TRANSCRIPT_ACCEPTED
            | FheRuntimeReleaseFacts::EVALUATION_COMPLETED
            | FheRuntimeReleaseFacts::EXTERNAL_REVIEW_COMPLETED
            | FheRuntimeReleaseFacts::ACTIVE_KEY_EPOCH
            | FheRuntimeReleaseFacts::POLICY_APPROVED
            | FheRuntimeReleaseFacts::GOVERNED_DATA_PRODUCT,
    ));
    assert_eq!(validate_fhe_runtime_release(ready), Ok(()));
    assert!(release_fhe_runtime_sum_metadata(evaluation, ready).is_ok());
}

#[test]
fn fhe_runtime_ciphertext_transcript_gate_fails_closed() {
    let missing_runtime = FheRuntimeCiphertextTranscriptGateContext::new(
        FheRuntimeCiphertextTranscriptFacts::from_bits(
            FheRuntimeCiphertextTranscriptFacts::EXTERNAL_REVIEW_COMPLETED
                | FheRuntimeCiphertextTranscriptFacts::PARTICIPANT_COUNT_MATCHES
                | FheRuntimeCiphertextTranscriptFacts::PARTICIPANTS_ACTIVE
                | FheRuntimeCiphertextTranscriptFacts::KEY_CUSTODY_PRESENT
                | FheRuntimeCiphertextTranscriptFacts::PARAMETER_SET_BOUND
                | FheRuntimeCiphertextTranscriptFacts::DIGEST_MATCHES
                | FheRuntimeCiphertextTranscriptFacts::NO_REPLAY,
        ),
    );
    assert_eq!(
        validate_fhe_runtime_ciphertext_transcript_gate_context(missing_runtime),
        Err(SecureAggregateError::FheRuntimeBindingMissing)
    );

    let missing_digest = FheRuntimeCiphertextTranscriptGateContext::new(
        FheRuntimeCiphertextTranscriptFacts::from_bits(
            FheRuntimeCiphertextTranscriptFacts::RUNTIME_BOUND
                | FheRuntimeCiphertextTranscriptFacts::EXTERNAL_REVIEW_COMPLETED
                | FheRuntimeCiphertextTranscriptFacts::PARTICIPANT_COUNT_MATCHES
                | FheRuntimeCiphertextTranscriptFacts::PARTICIPANTS_ACTIVE
                | FheRuntimeCiphertextTranscriptFacts::KEY_CUSTODY_PRESENT
                | FheRuntimeCiphertextTranscriptFacts::PARAMETER_SET_BOUND
                | FheRuntimeCiphertextTranscriptFacts::NO_REPLAY,
        ),
    );
    assert_eq!(
        validate_fhe_runtime_ciphertext_transcript_gate_context(missing_digest),
        Err(SecureAggregateError::FheRuntimeCiphertextDigestMismatch)
    );

    let ready = FheRuntimeCiphertextTranscriptGateContext::new(
        FheRuntimeCiphertextTranscriptFacts::from_bits(
            FheRuntimeCiphertextTranscriptFacts::RUNTIME_BOUND
                | FheRuntimeCiphertextTranscriptFacts::EXTERNAL_REVIEW_COMPLETED
                | FheRuntimeCiphertextTranscriptFacts::PARTICIPANT_COUNT_MATCHES
                | FheRuntimeCiphertextTranscriptFacts::PARTICIPANTS_ACTIVE
                | FheRuntimeCiphertextTranscriptFacts::KEY_CUSTODY_PRESENT
                | FheRuntimeCiphertextTranscriptFacts::PARAMETER_SET_BOUND
                | FheRuntimeCiphertextTranscriptFacts::DIGEST_MATCHES
                | FheRuntimeCiphertextTranscriptFacts::NO_REPLAY,
        ),
    );
    assert_eq!(
        validate_fhe_runtime_ciphertext_transcript_gate_context(ready),
        Ok(())
    );
}

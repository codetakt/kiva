#![allow(clippy::wildcard_imports)]

use super::*;

#[test]
fn tee_attestation_accepts_valid_evidence_and_records_metadata() {
    let policy = tee_policy(true, 1, 100);
    let claims = tee_claims(1, 10, 20, 30, 90, 110, 2);
    let evidence = TeeAttestationEvidence::new(claims);

    assert!(evidence.is_ok());
    let Ok(evidence) = evidence else {
        return;
    };
    let accepted = super::verify_tee_attestation_evidence(policy, &[], evidence);

    assert!(accepted.is_ok());
    let Ok(accepted) = accepted else {
        return;
    };
    assert_eq!(
        accepted.family(),
        TeeAttestationFamily::DeploymentBoundPortable
    );
    assert_eq!(accepted.format(), TeeAttestationFormat::PortableEvidenceV1);
    assert_eq!(accepted.measurement(), commitment(10));
    assert_eq!(accepted.image_digest(), commitment(20));
    assert_eq!(accepted.policy_binding(), commitment(30));
    assert_eq!(accepted.accepted_at_epoch_seconds(), 100);
    assert_eq!(
        accepted.record(),
        TeeAttestationRecord::new(transcript(1), sequence(2), evidence.evidence_digest())
    );
}

#[test]
fn tee_attestation_rejects_binding_mismatches() {
    let policy = tee_policy(true, 1, 100);

    assert_eq!(
        super::verify_tee_attestation_evidence(
            policy,
            &[],
            tee_evidence(2, 10, 20, 30, 90, 110, 2),
        ),
        Err(SecureAggregateError::TeeJobBindingMismatch)
    );
    assert_eq!(
        super::verify_tee_attestation_evidence(
            policy,
            &[],
            tee_evidence(1, 11, 20, 30, 90, 110, 2),
        ),
        Err(SecureAggregateError::TeeMeasurementMismatch)
    );
    assert_eq!(
        super::verify_tee_attestation_evidence(
            policy,
            &[],
            tee_evidence(1, 10, 21, 30, 90, 110, 2),
        ),
        Err(SecureAggregateError::TeeImageDigestMismatch)
    );
    assert_eq!(
        super::verify_tee_attestation_evidence(
            policy,
            &[],
            tee_evidence(1, 10, 20, 31, 90, 110, 2),
        ),
        Err(SecureAggregateError::TeePolicyBindingMismatch)
    );
}

#[test]
fn tee_attestation_rejects_policy_stale_rollback_replay_and_digest_mismatch() {
    let policy = tee_policy(true, 1, 100);
    let valid = tee_evidence(1, 10, 20, 30, 90, 110, 2);
    let replay_record =
        TeeAttestationRecord::new(transcript(1), sequence(2), valid.evidence_digest());
    let tampered =
        TeeAttestationEvidence::from_parts(tee_claims(1, 10, 20, 30, 90, 110, 2), commitment(99));

    assert_eq!(
        super::verify_tee_attestation_evidence(tee_policy(false, 1, 100), &[], valid),
        Err(SecureAggregateError::TeePolicyApprovalMissing)
    );
    assert_eq!(
        super::verify_tee_attestation_evidence(
            policy,
            &[],
            tee_evidence(1, 10, 20, 30, 101, 110, 2),
        ),
        Err(SecureAggregateError::TeeEvidenceNotYetValid)
    );
    assert_eq!(
        super::verify_tee_attestation_evidence(policy, &[], tee_evidence(1, 10, 20, 30, 90, 99, 2),),
        Err(SecureAggregateError::TeeEvidenceExpired)
    );
    assert_eq!(
        super::verify_tee_attestation_evidence(
            tee_policy_with_min_sequence(true, 1, 100, 3),
            &[],
            valid,
        ),
        Err(SecureAggregateError::TeeRollbackDetected)
    );
    assert_eq!(
        super::verify_tee_attestation_evidence(policy, &[], tampered),
        Err(SecureAggregateError::TeeEvidenceDigestMismatch)
    );
    assert_eq!(
        super::verify_tee_attestation_evidence(policy, &[replay_record], valid),
        Err(SecureAggregateError::TeeReplayDetected)
    );
}

#[test]
fn tee_secret_and_result_release_gates_fail_closed() {
    assert_eq!(
        validate_tee_secret_provisioning(false, true),
        Err(SecureAggregateError::TeeAttestationRequired)
    );
    assert_eq!(
        validate_tee_secret_provisioning(true, false),
        Err(SecureAggregateError::TeePolicyApprovalMissing)
    );
    assert_eq!(validate_tee_secret_provisioning(true, true), Ok(()));

    assert_eq!(
        validate_tee_result_release(false, true, true),
        Err(SecureAggregateError::TeeAttestationRequired)
    );
    assert_eq!(
        validate_tee_result_release(true, true, false),
        Err(SecureAggregateError::TeePolicyApprovalMissing)
    );
    assert_eq!(
        validate_tee_result_release(true, false, true),
        Err(SecureAggregateError::TeeJobBindingMismatch)
    );
    assert_eq!(validate_tee_result_release(true, true, true), Ok(()));
}

#[test]
fn tee_aws_nitro_attestation_accepts_valid_reviewed_evidence() {
    let policy = nitro_policy(true, 1, 100);
    let claims = nitro_claims(1, 90, 110, 2, false);
    let evidence =
        TeeAwsNitroAttestationEvidence::new(claims, nitro_verifier_binding(true, 2), true, true);

    assert!(evidence.is_ok());
    let Ok(evidence) = evidence else {
        return;
    };
    let accepted = super::verify_tee_aws_nitro_attestation_evidence(policy, &[], evidence);

    assert!(accepted.is_ok());
    let Ok(accepted) = accepted else {
        return;
    };
    assert_eq!(accepted.pcr0(), nitro_pcr(10));
    assert_eq!(accepted.pcr1(), nitro_pcr(11));
    assert_eq!(accepted.pcr2(), nitro_pcr(12));
    assert_eq!(accepted.pcr3(), nitro_pcr(13));
    assert_eq!(accepted.pcr8(), nitro_pcr(18));
    assert_eq!(accepted.image_digest(), commitment(20));
    assert_eq!(accepted.policy_binding(), commitment(30));
    assert_eq!(accepted.nonce_digest(), commitment(31));
    assert_eq!(accepted.trust_anchor_digest(), commitment(40));
    assert_eq!(accepted.verifier_artifact_digest(), commitment(41));
    assert_eq!(accepted.verifier_key_epoch(), sequence(2));
    assert_eq!(accepted.accepted_at_epoch_seconds(), 100);
    assert_eq!(
        accepted.record(),
        TeeAwsNitroAttestationRecord::new(transcript(1), sequence(2), evidence.document_digest(),)
    );
}

#[test]
fn tee_aws_nitro_attestation_rejects_policy_and_measurement_mismatches() {
    let policy = nitro_policy(true, 1, 100);

    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(
            nitro_policy(false, 1, 100),
            &[],
            nitro_evidence(1, 90, 110, 2),
        ),
        Err(SecureAggregateError::TeePolicyApprovalMissing)
    );
    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(
            policy,
            &[],
            nitro_evidence(2, 90, 110, 2),
        ),
        Err(SecureAggregateError::TeeJobBindingMismatch)
    );

    let pcr_mismatch = TeeAwsNitroAttestationEvidence::new(
        nitro_claims_with_seeds(1, 99, 11, 12, 13, 18, 20, 30, 31, 50, 51, 90, 110, 2, false),
        nitro_verifier_binding(true, 2),
        true,
        true,
    );
    assert!(pcr_mismatch.is_ok());
    let Ok(pcr_mismatch) = pcr_mismatch else {
        return;
    };
    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(policy, &[], pcr_mismatch),
        Err(SecureAggregateError::TeeNitroPcrMismatch)
    );

    let image_mismatch = TeeAwsNitroAttestationEvidence::new(
        nitro_claims_with_seeds(1, 10, 11, 12, 13, 18, 21, 30, 31, 50, 51, 90, 110, 2, false),
        nitro_verifier_binding(true, 2),
        true,
        true,
    );
    assert!(image_mismatch.is_ok());
    let Ok(image_mismatch) = image_mismatch else {
        return;
    };
    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(policy, &[], image_mismatch),
        Err(SecureAggregateError::TeeImageDigestMismatch)
    );

    let nonce_mismatch = TeeAwsNitroAttestationEvidence::new(
        nitro_claims_with_seeds(1, 10, 11, 12, 13, 18, 20, 30, 32, 50, 51, 90, 110, 2, false),
        nitro_verifier_binding(true, 2),
        true,
        true,
    );
    assert!(nonce_mismatch.is_ok());
    let Ok(nonce_mismatch) = nonce_mismatch else {
        return;
    };
    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(policy, &[], nonce_mismatch),
        Err(SecureAggregateError::TeeNitroNonceMismatch)
    );
}

#[test]
fn tee_aws_nitro_attestation_rejects_debug_unverified_and_stale_evidence() {
    let policy = nitro_policy(true, 1, 100);

    let debug = TeeAwsNitroAttestationEvidence::new(
        nitro_claims(1, 90, 110, 2, true),
        nitro_verifier_binding(true, 2),
        true,
        true,
    );
    assert!(debug.is_ok());
    let Ok(debug) = debug else {
        return;
    };
    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(policy, &[], debug),
        Err(SecureAggregateError::TeeNitroDebugModeRejected)
    );

    let zero_pcr = TeeAwsNitroAttestationEvidence::new(
        nitro_claims_with_pcr0(nitro_zero_pcr(), false),
        nitro_verifier_binding(true, 2),
        true,
        true,
    );
    assert!(zero_pcr.is_ok());
    let Ok(zero_pcr) = zero_pcr else {
        return;
    };
    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(policy, &[], zero_pcr),
        Err(SecureAggregateError::TeeNitroPcrAllZeroRejected)
    );

    let cert_invalid = TeeAwsNitroAttestationEvidence::new(
        nitro_claims(1, 90, 110, 2, false),
        nitro_verifier_binding(true, 2),
        false,
        true,
    );
    assert!(cert_invalid.is_ok());
    let Ok(cert_invalid) = cert_invalid else {
        return;
    };
    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(policy, &[], cert_invalid),
        Err(SecureAggregateError::TeeNitroCertificateChainInvalid)
    );

    let cose_invalid = TeeAwsNitroAttestationEvidence::new(
        nitro_claims(1, 90, 110, 2, false),
        nitro_verifier_binding(true, 2),
        true,
        false,
    );
    assert!(cose_invalid.is_ok());
    let Ok(cose_invalid) = cose_invalid else {
        return;
    };
    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(policy, &[], cose_invalid),
        Err(SecureAggregateError::TeeNitroCoseSignatureInvalid)
    );

    let trust_mismatch = TeeAwsNitroAttestationEvidence::new(
        nitro_claims(1, 90, 110, 2, false),
        TeeAwsNitroVerifierBinding::new(
            commitment(41),
            commitment(43),
            commitment(42),
            sequence(2),
            true,
        ),
        true,
        true,
    );
    assert!(trust_mismatch.is_ok());
    let Ok(trust_mismatch) = trust_mismatch else {
        return;
    };
    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(policy, &[], trust_mismatch),
        Err(SecureAggregateError::TeeNitroTrustAnchorMismatch)
    );

    let stale_verifier = TeeAwsNitroAttestationEvidence::new(
        nitro_claims(1, 90, 110, 2, false),
        nitro_verifier_binding(true, 1),
        true,
        true,
    );
    assert!(stale_verifier.is_ok());
    let Ok(stale_verifier) = stale_verifier else {
        return;
    };
    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(
            nitro_policy_with_minimums(true, 1, 100, 1, 2),
            &[],
            stale_verifier,
        ),
        Err(SecureAggregateError::TeeNitroVerifierKeyRotationRequired)
    );
}

#[test]
fn tee_aws_nitro_attestation_rejects_review_rollback_replay_and_digest_mismatch() {
    let policy = nitro_policy(true, 1, 100);
    let review_pending = TeeAwsNitroAttestationEvidence::new(
        nitro_claims(1, 90, 110, 2, false),
        nitro_verifier_binding(false, 2),
        true,
        true,
    );
    assert!(review_pending.is_ok());
    let Ok(review_pending) = review_pending else {
        return;
    };
    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(policy, &[], review_pending),
        Err(SecureAggregateError::TeeNitroExternalReviewRequired)
    );

    let verifier_mismatch = TeeAwsNitroAttestationEvidence::new(
        nitro_claims(1, 90, 110, 2, false),
        TeeAwsNitroVerifierBinding::new(
            commitment(44),
            commitment(40),
            commitment(42),
            sequence(2),
            true,
        ),
        true,
        true,
    );
    assert!(verifier_mismatch.is_ok());
    let Ok(verifier_mismatch) = verifier_mismatch else {
        return;
    };
    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(policy, &[], verifier_mismatch),
        Err(SecureAggregateError::TeeNitroVerifierBindingMissing)
    );

    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(
            policy,
            &[],
            nitro_evidence(1, 101, 110, 2),
        ),
        Err(SecureAggregateError::TeeEvidenceNotYetValid)
    );
    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(policy, &[], nitro_evidence(1, 90, 99, 2),),
        Err(SecureAggregateError::TeeEvidenceExpired)
    );
    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(
            nitro_policy_with_minimums(true, 1, 100, 3, 1),
            &[],
            nitro_evidence(1, 90, 110, 2),
        ),
        Err(SecureAggregateError::TeeRollbackDetected)
    );

    let valid = nitro_evidence(1, 90, 110, 2);
    let replay_record =
        TeeAwsNitroAttestationRecord::new(transcript(1), sequence(2), valid.document_digest());
    let tampered = TeeAwsNitroAttestationEvidence::from_parts(
        nitro_claims(1, 90, 110, 2, false),
        nitro_verifier_binding(true, 2),
        commitment(99),
        true,
        true,
    );

    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(policy, &[], tampered),
        Err(SecureAggregateError::TeeEvidenceDigestMismatch)
    );
    assert_eq!(
        super::verify_tee_aws_nitro_attestation_evidence(policy, &[replay_record], valid),
        Err(SecureAggregateError::TeeReplayDetected)
    );
}

#[test]
fn tee_aws_nitro_key_and_result_release_gates_fail_closed() {
    let missing_attestation = TeeAwsNitroReleaseContext::new(TeeAwsNitroReleaseFacts::from_bits(
        TeeAwsNitroReleaseFacts::POLICY_APPROVED
            | TeeAwsNitroReleaseFacts::EXTERNAL_REVIEW_COMPLETED
            | TeeAwsNitroReleaseFacts::VERIFIER_KEY_CURRENT,
    ));
    assert_eq!(
        validate_tee_aws_nitro_key_release(missing_attestation),
        Err(SecureAggregateError::TeeAttestationRequired)
    );

    let review_pending = TeeAwsNitroReleaseContext::new(TeeAwsNitroReleaseFacts::from_bits(
        TeeAwsNitroReleaseFacts::ATTESTATION_ACCEPTED
            | TeeAwsNitroReleaseFacts::POLICY_APPROVED
            | TeeAwsNitroReleaseFacts::VERIFIER_KEY_CURRENT,
    ));
    assert_eq!(
        validate_tee_aws_nitro_key_release(review_pending),
        Err(SecureAggregateError::TeeNitroExternalReviewRequired)
    );

    let key_ready = TeeAwsNitroReleaseContext::new(TeeAwsNitroReleaseFacts::from_bits(
        TeeAwsNitroReleaseFacts::ATTESTATION_ACCEPTED
            | TeeAwsNitroReleaseFacts::POLICY_APPROVED
            | TeeAwsNitroReleaseFacts::EXTERNAL_REVIEW_COMPLETED
            | TeeAwsNitroReleaseFacts::VERIFIER_KEY_CURRENT,
    ));
    assert_eq!(validate_tee_aws_nitro_key_release(key_ready), Ok(()));
    assert_eq!(
        validate_tee_aws_nitro_result_release(key_ready),
        Err(SecureAggregateError::TeeNitroSecretProvisioningRequired)
    );

    let release_ready = TeeAwsNitroReleaseContext::new(TeeAwsNitroReleaseFacts::from_bits(
        TeeAwsNitroReleaseFacts::ATTESTATION_ACCEPTED
            | TeeAwsNitroReleaseFacts::POLICY_APPROVED
            | TeeAwsNitroReleaseFacts::EXTERNAL_REVIEW_COMPLETED
            | TeeAwsNitroReleaseFacts::VERIFIER_KEY_CURRENT
            | TeeAwsNitroReleaseFacts::SECRETS_PROVISIONED
            | TeeAwsNitroReleaseFacts::JOB_MATCHES_RELEASE_POLICY,
    ));
    assert_eq!(validate_tee_aws_nitro_result_release(release_ready), Ok(()));
}

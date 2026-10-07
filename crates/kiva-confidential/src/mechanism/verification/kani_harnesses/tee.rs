#![allow(clippy::wildcard_imports)]

use super::*;

#[kani::proof]
pub(super) fn tee_attestation_gate_fails_closed_for_policy_freshness_and_replay() {
    let mut bits = 0_u16;
    if kani::any::<bool>() {
        bits |= TeeAttestationGateFacts::POLICY_APPROVED;
    }
    if kani::any::<bool>() {
        bits |= TeeAttestationGateFacts::FORMAT_SUPPORTED;
    }
    if kani::any::<bool>() {
        bits |= TeeAttestationGateFacts::JOB_MATCHES;
    }
    if kani::any::<bool>() {
        bits |= TeeAttestationGateFacts::MEASUREMENT_MATCHES;
    }
    if kani::any::<bool>() {
        bits |= TeeAttestationGateFacts::IMAGE_DIGEST_MATCHES;
    }
    if kani::any::<bool>() {
        bits |= TeeAttestationGateFacts::POLICY_BINDING_MATCHES;
    }
    if kani::any::<bool>() {
        bits |= TeeAttestationGateFacts::SEQUENCE_AT_OR_ABOVE_MINIMUM;
    }
    if kani::any::<bool>() {
        bits |= TeeAttestationGateFacts::EVIDENCE_DIGEST_MATCHES;
    }
    if kani::any::<bool>() {
        bits |= TeeAttestationGateFacts::REPLAY_DETECTED;
    }
    let facts = TeeAttestationGateFacts::from_bits(bits);
    let context = TeeAttestationGateContext {
        facts,
        current_epoch_seconds: u64::from(kani::any::<u8>() % 16),
        not_before_epoch_seconds: u64::from(kani::any::<u8>() % 16),
        not_after_epoch_seconds: u64::from(kani::any::<u8>() % 16),
    };

    let result = validate_tee_attestation_gate_context(context);
    let admissible = facts.contains(TeeAttestationGateFacts::POLICY_APPROVED)
        && facts.contains(TeeAttestationGateFacts::FORMAT_SUPPORTED)
        && facts.contains(TeeAttestationGateFacts::JOB_MATCHES)
        && facts.contains(TeeAttestationGateFacts::MEASUREMENT_MATCHES)
        && facts.contains(TeeAttestationGateFacts::IMAGE_DIGEST_MATCHES)
        && facts.contains(TeeAttestationGateFacts::POLICY_BINDING_MATCHES)
        && context.current_epoch_seconds >= context.not_before_epoch_seconds
        && context.current_epoch_seconds <= context.not_after_epoch_seconds
        && facts.contains(TeeAttestationGateFacts::SEQUENCE_AT_OR_ABOVE_MINIMUM)
        && facts.contains(TeeAttestationGateFacts::EVIDENCE_DIGEST_MATCHES)
        && !facts.contains(TeeAttestationGateFacts::REPLAY_DETECTED);

    if admissible {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
    }
}

#[kani::proof]
pub(super) fn tee_secret_and_result_release_gates_fail_closed() {
    let attestation_accepted: bool = kani::any();
    let policy_approved: bool = kani::any();
    let job_matches_release_policy: bool = kani::any();

    let secret = validate_tee_secret_provisioning(attestation_accepted, policy_approved);
    if attestation_accepted && policy_approved {
        assert!(secret.is_ok());
    } else {
        assert!(secret.is_err());
    }

    let release = validate_tee_result_release(
        attestation_accepted,
        job_matches_release_policy,
        policy_approved,
    );
    if attestation_accepted && policy_approved && job_matches_release_policy {
        assert!(release.is_ok());
    } else {
        assert!(release.is_err());
    }
}

#[kani::proof]
pub(super) fn tee_aws_nitro_attestation_gate_fails_closed() {
    let mut bits = 0_u32;
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::POLICY_APPROVED;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::FORMAT_SUPPORTED;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::JOB_MATCHES;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::PCRS_MATCH;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::PCRS_NON_ZERO;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::DEBUG_MODE_REPORTED;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::IMAGE_DIGEST_MATCHES;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::POLICY_BINDING_MATCHES;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::NONCE_MATCHES;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::CERTIFICATE_CHAIN_VALID;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::COSE_SIGNATURE_VALID;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::TRUST_ANCHOR_MATCHES;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::VERIFIER_ARTIFACT_MATCHES;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::EXTERNAL_REVIEW_COMPLETED;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::VERIFIER_KEY_CURRENT;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::SEQUENCE_AT_OR_ABOVE_MINIMUM;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::DOCUMENT_DIGEST_MATCHES;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroAttestationGateFacts::REPLAY_DETECTED;
    }
    let facts = TeeAwsNitroAttestationGateFacts::from_bits(bits);
    let context = TeeAwsNitroAttestationGateContext {
        facts,
        current_epoch_seconds: u64::from(kani::any::<u8>() % 16),
        not_before_epoch_seconds: u64::from(kani::any::<u8>() % 16),
        not_after_epoch_seconds: u64::from(kani::any::<u8>() % 16),
    };

    let result = validate_tee_aws_nitro_attestation_gate_context(context);
    let admissible = facts.contains(TeeAwsNitroAttestationGateFacts::POLICY_APPROVED)
        && facts.contains(TeeAwsNitroAttestationGateFacts::FORMAT_SUPPORTED)
        && facts.contains(TeeAwsNitroAttestationGateFacts::JOB_MATCHES)
        && !facts.contains(TeeAwsNitroAttestationGateFacts::DEBUG_MODE_REPORTED)
        && facts.contains(TeeAwsNitroAttestationGateFacts::PCRS_NON_ZERO)
        && facts.contains(TeeAwsNitroAttestationGateFacts::PCRS_MATCH)
        && facts.contains(TeeAwsNitroAttestationGateFacts::IMAGE_DIGEST_MATCHES)
        && facts.contains(TeeAwsNitroAttestationGateFacts::POLICY_BINDING_MATCHES)
        && facts.contains(TeeAwsNitroAttestationGateFacts::NONCE_MATCHES)
        && context.current_epoch_seconds >= context.not_before_epoch_seconds
        && context.current_epoch_seconds <= context.not_after_epoch_seconds
        && facts.contains(TeeAwsNitroAttestationGateFacts::CERTIFICATE_CHAIN_VALID)
        && facts.contains(TeeAwsNitroAttestationGateFacts::COSE_SIGNATURE_VALID)
        && facts.contains(TeeAwsNitroAttestationGateFacts::TRUST_ANCHOR_MATCHES)
        && facts.contains(TeeAwsNitroAttestationGateFacts::VERIFIER_ARTIFACT_MATCHES)
        && facts.contains(TeeAwsNitroAttestationGateFacts::EXTERNAL_REVIEW_COMPLETED)
        && facts.contains(TeeAwsNitroAttestationGateFacts::VERIFIER_KEY_CURRENT)
        && facts.contains(TeeAwsNitroAttestationGateFacts::SEQUENCE_AT_OR_ABOVE_MINIMUM)
        && facts.contains(TeeAwsNitroAttestationGateFacts::DOCUMENT_DIGEST_MATCHES)
        && !facts.contains(TeeAwsNitroAttestationGateFacts::REPLAY_DETECTED);

    if admissible {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
    }
}

#[kani::proof]
pub(super) fn tee_aws_nitro_key_and_result_release_gates_fail_closed() {
    let mut bits = 0_u8;
    if kani::any::<bool>() {
        bits |= TeeAwsNitroReleaseFacts::ATTESTATION_ACCEPTED;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroReleaseFacts::POLICY_APPROVED;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroReleaseFacts::EXTERNAL_REVIEW_COMPLETED;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroReleaseFacts::VERIFIER_KEY_CURRENT;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroReleaseFacts::JOB_MATCHES_RELEASE_POLICY;
    }
    if kani::any::<bool>() {
        bits |= TeeAwsNitroReleaseFacts::SECRETS_PROVISIONED;
    }
    let facts = TeeAwsNitroReleaseFacts::from_bits(bits);
    let context = TeeAwsNitroReleaseContext::new(facts);

    let key_release = validate_tee_aws_nitro_key_release(context);
    let key_admissible = context.attestation_accepted()
        && context.policy_approved()
        && context.external_review_completed()
        && context.verifier_key_current();
    if key_admissible {
        assert!(key_release.is_ok());
    } else {
        assert!(key_release.is_err());
    }

    let result_release = validate_tee_aws_nitro_result_release(context);
    if key_admissible && context.secrets_provisioned() && context.job_matches_release_policy() {
        assert!(result_release.is_ok());
    } else {
        assert!(result_release.is_err());
    }
}

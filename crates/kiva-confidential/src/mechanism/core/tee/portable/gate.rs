#![allow(clippy::wildcard_imports)]

use super::*;

/// Evaluates the pure fail-closed TEE attestation gate facts.
pub const fn validate_tee_attestation_gate_context(
    context: TeeAttestationGateContext,
) -> Result<(), SecureAggregateError> {
    let facts = context.facts;
    if !facts.contains(TeeAttestationGateFacts::POLICY_APPROVED) {
        return Err(SecureAggregateError::TeePolicyApprovalMissing);
    }
    if !facts.contains(TeeAttestationGateFacts::FORMAT_SUPPORTED) {
        return Err(SecureAggregateError::TeeUnsupportedAttestationFormat);
    }
    if !facts.contains(TeeAttestationGateFacts::JOB_MATCHES) {
        return Err(SecureAggregateError::TeeJobBindingMismatch);
    }
    if !facts.contains(TeeAttestationGateFacts::MEASUREMENT_MATCHES) {
        return Err(SecureAggregateError::TeeMeasurementMismatch);
    }
    if !facts.contains(TeeAttestationGateFacts::IMAGE_DIGEST_MATCHES) {
        return Err(SecureAggregateError::TeeImageDigestMismatch);
    }
    if !facts.contains(TeeAttestationGateFacts::POLICY_BINDING_MATCHES) {
        return Err(SecureAggregateError::TeePolicyBindingMismatch);
    }
    if context.current_epoch_seconds < context.not_before_epoch_seconds {
        return Err(SecureAggregateError::TeeEvidenceNotYetValid);
    }
    if context.current_epoch_seconds > context.not_after_epoch_seconds {
        return Err(SecureAggregateError::TeeEvidenceExpired);
    }
    if !facts.contains(TeeAttestationGateFacts::SEQUENCE_AT_OR_ABOVE_MINIMUM) {
        return Err(SecureAggregateError::TeeRollbackDetected);
    }
    if !facts.contains(TeeAttestationGateFacts::EVIDENCE_DIGEST_MATCHES) {
        return Err(SecureAggregateError::TeeEvidenceDigestMismatch);
    }
    if facts.contains(TeeAttestationGateFacts::REPLAY_DETECTED) {
        return Err(SecureAggregateError::TeeReplayDetected);
    }
    Ok(())
}

/// Verifies portable TEE attestation evidence against a deployment policy.
///
/// This is a deployment-bound evidence gate. It validates Kiva's canonical
/// envelope fields, freshness, rollback/replay metadata, and policy approval.
/// It does not verify Nitro, SGX, SEV-SNP, or other hardware-vendor quotes.
pub fn verify_tee_attestation_evidence(
    policy: TeeAttestationPolicy,
    existing_records: &[TeeAttestationRecord],
    evidence: TeeAttestationEvidence,
) -> Result<AcceptedTeeAttestation, SecureAggregateError> {
    if !policy.policy_approved() {
        return Err(SecureAggregateError::TeePolicyApprovalMissing);
    }

    let claims = evidence.claims();
    if claims.family() != policy.family() || claims.format() != policy.format() {
        return Err(SecureAggregateError::TeeUnsupportedAttestationFormat);
    }
    if claims.job_id() != policy.expected_job_id() {
        return Err(SecureAggregateError::TeeJobBindingMismatch);
    }
    if claims.measurement() != policy.expected_measurement() {
        return Err(SecureAggregateError::TeeMeasurementMismatch);
    }
    if claims.image_digest() != policy.expected_image_digest() {
        return Err(SecureAggregateError::TeeImageDigestMismatch);
    }
    if claims.policy_binding() != policy.expected_policy_binding() {
        return Err(SecureAggregateError::TeePolicyBindingMismatch);
    }
    if policy.current_epoch_seconds() < claims.not_before_epoch_seconds() {
        return Err(SecureAggregateError::TeeEvidenceNotYetValid);
    }
    if policy.current_epoch_seconds() > claims.not_after_epoch_seconds() {
        return Err(SecureAggregateError::TeeEvidenceExpired);
    }
    if claims.evidence_sequence() < policy.minimum_evidence_sequence() {
        return Err(SecureAggregateError::TeeRollbackDetected);
    }
    if derive_tee_attestation_evidence_digest(claims)? != evidence.evidence_digest() {
        return Err(SecureAggregateError::TeeEvidenceDigestMismatch);
    }
    if existing_records.iter().any(|record| {
        record.conflicts_with(
            claims.job_id(),
            claims.evidence_sequence(),
            evidence.evidence_digest(),
        )
    }) {
        return Err(SecureAggregateError::TeeReplayDetected);
    }

    let record = TeeAttestationRecord::new(
        claims.job_id(),
        claims.evidence_sequence(),
        evidence.evidence_digest(),
    );
    Ok(AcceptedTeeAttestation::new(
        record,
        claims.family(),
        claims.format(),
        claims.measurement(),
        claims.image_digest(),
        claims.policy_binding(),
        policy.current_epoch_seconds(),
    ))
}

/// Validates that secret provisioning is gated by accepted TEE evidence.
pub const fn validate_tee_secret_provisioning(
    attestation_accepted: bool,
    policy_approved: bool,
) -> Result<(), SecureAggregateError> {
    if !attestation_accepted {
        return Err(SecureAggregateError::TeeAttestationRequired);
    }
    if !policy_approved {
        return Err(SecureAggregateError::TeePolicyApprovalMissing);
    }
    Ok(())
}

/// Validates that result release is bound to accepted TEE evidence and job policy.
pub const fn validate_tee_result_release(
    attestation_accepted: bool,
    job_matches_release_policy: bool,
    policy_approved: bool,
) -> Result<(), SecureAggregateError> {
    if !attestation_accepted {
        return Err(SecureAggregateError::TeeAttestationRequired);
    }
    if !policy_approved {
        return Err(SecureAggregateError::TeePolicyApprovalMissing);
    }
    if !job_matches_release_policy {
        return Err(SecureAggregateError::TeeJobBindingMismatch);
    }
    Ok(())
}

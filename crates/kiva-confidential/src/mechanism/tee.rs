//! TEE attestation and release-gate boundary facade.

pub use super::core::{
    AcceptedTeeAttestation, AcceptedTeeAwsNitroAttestation, TEE_ATTESTATION_EVIDENCE_DOMAIN,
    TEE_AWS_NITRO_ATTESTATION_DOCUMENT_DOMAIN, TEE_AWS_NITRO_ATTESTATION_PROFILE,
    TEE_PORTABLE_ATTESTATION_PROFILE, TeeAttestationClaims, TeeAttestationEvidence,
    TeeAttestationFamily, TeeAttestationFormat, TeeAttestationGateContext, TeeAttestationGateFacts,
    TeeAttestationPolicy, TeeAttestationRecord, TeeAwsNitroAttestationClaims,
    TeeAwsNitroAttestationEvidence, TeeAwsNitroAttestationFamily, TeeAwsNitroAttestationFormat,
    TeeAwsNitroAttestationGateContext, TeeAwsNitroAttestationGateFacts,
    TeeAwsNitroAttestationPolicy, TeeAwsNitroAttestationRecord, TeeAwsNitroPcrDigest,
    TeeAwsNitroReleaseContext, TeeAwsNitroReleaseFacts, TeeAwsNitroVerifierBinding,
    derive_tee_attestation_evidence_digest, derive_tee_aws_nitro_attestation_document_digest,
    validate_tee_attestation_gate_context, validate_tee_aws_nitro_attestation_gate_context,
    validate_tee_aws_nitro_key_release, validate_tee_aws_nitro_result_release,
    validate_tee_result_release, validate_tee_secret_provisioning, verify_tee_attestation_evidence,
    verify_tee_aws_nitro_attestation_evidence,
};

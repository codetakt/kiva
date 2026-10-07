#![allow(unused_imports)]

pub(super) use std::num::{NonZeroU64, NonZeroUsize};

pub(super) use ed25519_dalek::{Signer, SigningKey};
pub(super) use serde::Deserialize;

pub(super) use super::{
    AcceptedAdditiveShareRecord, AdditiveSecureAggregatePlan, AdditiveShareCommitmentSlot,
    AdditiveShareReplayWindow, AdditiveShareVerificationKey, AuthenticatedAdditiveShareEnvelope,
    ConfidentialJobAdmissionError, ConfidentialJobAdmissionFacts, ConfidentialJobStatus,
    ConfidentialJobTransitionError, ConfidentialMechanismKind, ConfidentialMechanismPlan,
    ConfidentialParticipantStatus, ConfidentialParticipantTransitionError,
    DifferentialPrivacyBudgetError, DifferentialPrivacyBudgetLedger,
    DifferentialPrivacyBudgetPolicyLifecycleError, DifferentialPrivacyBudgetPolicyStatus,
    DifferentialPrivacyCompositionError, DifferentialPrivacyNoiseError,
    DifferentialPrivacyNoiseMechanism, DifferentialPrivacyNoiseParameters,
    DifferentialPrivacyPolicyError, DifferentialPrivacyReleaseContext,
    DifferentialPrivacyReleaseError, DifferentialPrivacyReleasePolicy,
    FHE_FEASIBILITY_CIPHERTEXT_DOMAIN, FHE_FEASIBILITY_EVALUATION_DOMAIN, FHE_FEASIBILITY_PROFILE,
    FHE_RUNTIME_CIPHERTEXT_TRANSCRIPT_DOMAIN, FHE_RUNTIME_EVALUATION_DOMAIN, FHE_RUNTIME_PROFILE,
    FheFeasibilityCiphertextFixture, FheFeasibilityNonce, FheFeasibilityParameterSet,
    FheFeasibilityPlaintext, FheFeasibilityPlan, FheFeasibilityReleaseContext,
    FheFeasibilityReleaseFacts, FheFeasibilitySchemeFamily, FheRuntimeBinding,
    FheRuntimeCiphertextTranscriptFacts, FheRuntimeCiphertextTranscriptGateContext,
    FheRuntimeOperation, FheRuntimeParticipantCiphertext, FheRuntimeParticipantCiphertextStatus,
    FheRuntimePlan, FheRuntimeReleaseContext, FheRuntimeReleaseFacts, FheRuntimeSchemeFamily,
    PSI_COHORT_IDENTIFIER_DOMAIN, PSI_OPRF_DH_VOPRF_ELEMENT_DOMAIN, PSI_OPRF_DH_VOPRF_PROFILE,
    PSI_OPRF_DH_VOPRF_TRANSCRIPT_DOMAIN, PsiIdentifierDigest, PsiJobRecord, PsiOprfElement,
    PsiOprfParticipantTranscript, PsiOprfParticipantTranscriptStatus, PsiOprfPlan,
    PsiOprfReleaseContext, PsiOprfReleaseFacts, PsiOprfRuntimeBinding, PsiOprfSuite,
    PsiOprfTranscriptRecord, PsiParticipantDataset, ReleasePolicy, SECURE_AGGREGATE_ARTIFACT_BYTES,
    SECURE_AGGREGATE_FIELD_MODULUS, SECURE_AGGREGATE_SIGNATURE_BYTES,
    SecretComputeProviderAdmission, SecretComputeProviderBinding, SecretComputeProviderCapability,
    SecretComputeProviderEvidence, SecretComputeProviderEvidenceGate, SecretComputeProviderKind,
    SecretComputeProviderProfile, SecretComputeProviderReleaseGate,
    SecretComputeProviderReviewEvidence, SecretComputeProviderUsageClaim,
    SecureAggregateCommitment, SecureAggregateEnvelopeSequence, SecureAggregateError,
    SecureAggregateFieldElement, SecureAggregateShare, SecureAggregateSignature,
    SecureAggregateSigningKeyId, SecureAggregateSigningPublicKey, SecureAggregateTranscriptId,
    SecureAggregateTransportProof, ShamirShare, ShamirShareComplaintOutcome,
    ShamirShareOpeningNonce, ShamirThresholdPlan, ShareSigningKeyLifecycleError,
    ShareSigningKeyLifecycleState, ShareSigningKeyRotationPrecondition,
    SignedAuthenticatedAdditiveShareEnvelope, TEE_ATTESTATION_EVIDENCE_DOMAIN,
    TEE_AWS_NITRO_ATTESTATION_DOCUMENT_DOMAIN, TEE_AWS_NITRO_ATTESTATION_PROFILE,
    TEE_PORTABLE_ATTESTATION_PROFILE, THRESHOLD_FROST_KEY_CEREMONY_DOMAIN, THRESHOLD_FROST_PROFILE,
    THRESHOLD_FROST_SIGNATURE_TRANSCRIPT_DOMAIN, TeeAttestationClaims, TeeAttestationEvidence,
    TeeAttestationFamily, TeeAttestationFormat, TeeAttestationPolicy, TeeAttestationRecord,
    TeeAwsNitroAttestationClaims, TeeAwsNitroAttestationEvidence, TeeAwsNitroAttestationFamily,
    TeeAwsNitroAttestationFormat, TeeAwsNitroAttestationPolicy, TeeAwsNitroAttestationRecord,
    TeeAwsNitroPcrDigest, TeeAwsNitroReleaseContext, TeeAwsNitroReleaseFacts,
    TeeAwsNitroVerifierBinding, ThresholdFrostKeyCeremonyFacts,
    ThresholdFrostKeyCeremonyGateContext, ThresholdFrostKeyCeremonyPlan,
    ThresholdFrostKeyCeremonyRecord, ThresholdFrostParticipantTranscript,
    ThresholdFrostParticipantTranscriptStatus, ThresholdFrostReleaseContext,
    ThresholdFrostReleaseFacts, ThresholdFrostRuntimeBinding, ThresholdFrostSignatureShare,
    ThresholdFrostSuite, ThresholdReleaseApprovalSet, ThresholdReleaseAuthoritySide,
    ThresholdReleaseTransitionError, VerifiableShamirShare, WorkflowError,
    accept_authenticated_additive_share, accept_fhe_runtime_ciphertext_transcript,
    accept_fhe_runtime_evaluation, accept_replay_windowed_authenticated_additive_share,
    accept_threshold_frost_key_ceremony, accept_threshold_frost_signature_transcript,
    additive_share_envelope_signature_payload, adjudicate_shamir_share_complaint,
    admit_secret_compute_provider, apply_differential_privacy_integer_noise,
    bound_differential_privacy_advanced_composition, canonicalize_psi_cohort_identifier,
    classify_threshold_release_authority, combine_additive_shares,
    combine_authenticated_additive_share_envelopes,
    combine_signed_authenticated_additive_share_envelopes, compute_psi_cohort_intersection,
    compute_psi_oprf_intersection, confidential_job_transition_is_allowed,
    confidential_participant_accepts_jobs, confidential_participant_accepts_new_keys,
    confidential_participant_transition_is_allowed, derive_fhe_feasibility_ciphertext_digest,
    derive_fhe_runtime_ciphertext_transcript_digest, derive_fhe_runtime_evaluation_digest,
    derive_psi_cohort_identifier_digest, derive_psi_oprf_fixture_element,
    derive_secret_compute_provider_binding_digest, derive_shamir_share_opening_commitment,
    derive_tee_attestation_evidence_digest, derive_tee_aws_nitro_attestation_document_digest,
    derive_threshold_frost_key_ceremony_digest, derive_threshold_frost_signature_transcript_digest,
    encrypt_fhe_feasibility_plaintext_fixture, evaluate_fhe_feasibility_sum,
    reconstruct_shamir_secret, reconstruct_verified_shamir_secret,
    record_threshold_release_approval, release_fhe_feasibility_sum_fixture,
    release_fhe_runtime_sum_metadata, release_psi_oprf_intersection, revoke_share_signing_key,
    rotate_share_signing_key, sample_differential_privacy_discrete_laplace_noise_from_uniforms,
    secret_compute_provider_kind_requires_parameter_set,
    secret_compute_required_capability_for_mechanism, share_signing_key_accepts_new_submissions,
    spend_differential_privacy_budget, summarize_differential_privacy_composition,
    transition_confidential_job_status, transition_confidential_participant_status,
    transition_differential_privacy_budget_policy, validate_additive_share_commitment_slots,
    validate_additive_share_verification_keys, validate_confidential_job_admission,
    validate_differential_privacy_release, validate_fhe_feasibility_parameters,
    validate_fhe_feasibility_release, validate_fhe_runtime_ciphertext_transcript_gate_context,
    validate_fhe_runtime_release, validate_psi_oprf_release,
    validate_secret_compute_provider_for_mechanism, validate_tee_aws_nitro_key_release,
    validate_tee_aws_nitro_result_release, validate_tee_result_release,
    validate_tee_secret_provisioning, validate_threshold_frost_key_ceremony_gate_context,
    validate_threshold_frost_release, validate_threshold_release_approval_threshold,
    validate_threshold_release_computed_aggregate, verify_fhe_feasibility_ciphertext_fixture,
    verify_signed_authenticated_additive_share_envelope,
};

pub(super) const ADDITIVE_SECURE_AGGREGATE_VECTORS: &str =
    include_str!("../../../../../../spec/confidential/additive-secure-aggregate-vectors.yaml");
pub(super) const SHAMIR_THRESHOLD_VECTORS: &str =
    include_str!("../../../../../../spec/confidential/shamir-threshold-vectors.yaml");
pub(super) const PSI_COHORT_VECTORS: &str =
    include_str!("../../../../../../spec/confidential/psi-cohort-vectors.yaml");
pub(super) const PSI_OPRF_VECTORS: &str =
    include_str!("../../../../../../spec/confidential/psi-oprf-dh-voprf-vectors.yaml");
pub(super) const THRESHOLD_FROST_VECTORS: &str =
    include_str!("../../../../../../spec/confidential/threshold-frost-vectors.yaml");
pub(super) const TEE_ATTESTATION_VECTORS: &str =
    include_str!("../../../../../../spec/confidential/tee-attestation-vectors.yaml");
pub(super) const TEE_AWS_NITRO_ATTESTATION_VECTORS: &str =
    include_str!("../../../../../../spec/confidential/tee-nitro-attestation-vectors.yaml");
pub(super) const FHE_FEASIBILITY_VECTORS: &str =
    include_str!("../../../../../../spec/confidential/fhe-sum-feasibility-vectors.yaml");
pub(super) const FHE_RUNTIME_VECTORS: &str =
    include_str!("../../../../../../spec/confidential/fhe-runtime-vectors.yaml");

#[derive(Debug, Deserialize)]
pub(super) struct VectorFile {
    pub(super) vectors: Vec<AggregateVector>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ShamirVectorFile {
    pub(super) vectors: Vec<ShamirVector>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PsiVectorFile {
    pub(super) vectors: Vec<PsiVector>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PsiOprfVectorFile {
    pub(super) vectors: Vec<PsiOprfVector>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TeeVectorFile {
    pub(super) vectors: Vec<TeeVector>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FheVectorFile {
    pub(super) vectors: Vec<FheVector>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FheRuntimeVectorFile {
    pub(super) vectors: Vec<FheRuntimeVector>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ThresholdFrostVectorFile {
    pub(super) vectors: Vec<ThresholdFrostVector>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AggregateVector {
    pub(super) field_modulus: u64,
    pub(super) participant_count: usize,
    pub(super) release_threshold: usize,
    pub(super) shares: Vec<VectorShare>,
    pub(super) expected_aggregate: u64,
}

#[derive(Debug, Deserialize)]
pub(super) struct VectorShare {
    pub(super) participant_index: usize,
    pub(super) value: u64,
}

#[derive(Debug, Deserialize)]
pub(super) struct ShamirVector {
    pub(super) field_modulus: u64,
    pub(super) verification_profile: String,
    pub(super) transcript_seed: u8,
    pub(super) participant_count: usize,
    pub(super) threshold: usize,
    pub(super) secret: u64,
    pub(super) shares: Vec<VectorShare>,
    pub(super) share_openings: Vec<ShamirOpeningVector>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ShamirOpeningVector {
    pub(super) participant_index: usize,
    pub(super) value: u64,
    pub(super) opening_nonce_seed: u8,
    pub(super) commitment_hex: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct PsiVector {
    pub(super) profile: String,
    pub(super) domain: String,
    pub(super) namespace: String,
    pub(super) job_seed: u8,
    pub(super) participant_count: usize,
    pub(super) participants: Vec<PsiParticipantVector>,
    pub(super) expected_intersection_hex: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PsiParticipantVector {
    pub(super) participant_index: usize,
    pub(super) identifiers: Vec<String>,
    pub(super) digest_hex: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PsiOprfVector {
    pub(super) profile: String,
    pub(super) element_domain: String,
    pub(super) transcript_domain: String,
    pub(super) suite: String,
    pub(super) namespace: String,
    pub(super) job_seed: u8,
    pub(super) runtime_artifact_seed: u8,
    pub(super) review_evidence_seed: u8,
    pub(super) external_review_completed: bool,
    pub(super) participant_count: usize,
    pub(super) participants: Vec<PsiOprfParticipantVector>,
    pub(super) expected_intersection_element_hex: Vec<String>,
    pub(super) expected_transcript_digest_hex: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct PsiOprfParticipantVector {
    pub(super) participant_index: usize,
    pub(super) identifiers: Vec<String>,
    pub(super) element_hex: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TeeVector {
    pub(super) profile: String,
    pub(super) domain: String,
    pub(super) format: String,
    pub(super) family: String,
    pub(super) job_seed: u8,
    pub(super) measurement_seed: u8,
    pub(super) image_digest_seed: u8,
    pub(super) policy_binding_seed: u8,
    pub(super) not_before_epoch_seconds: u64,
    pub(super) not_after_epoch_seconds: u64,
    pub(super) evidence_sequence: u64,
    pub(super) expected_evidence_digest_hex: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct TeeAwsNitroVectorFile {
    pub(super) vectors: Vec<TeeAwsNitroVector>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TeeAwsNitroVector {
    pub(super) profile: String,
    pub(super) document_domain: String,
    pub(super) family: String,
    pub(super) format: String,
    pub(super) job_seed: u8,
    pub(super) pcr0_seed: u8,
    pub(super) pcr1_seed: u8,
    pub(super) pcr2_seed: u8,
    pub(super) pcr3_seed: u8,
    pub(super) pcr8_seed: u8,
    pub(super) image_digest_seed: u8,
    pub(super) policy_binding_seed: u8,
    pub(super) nonce_digest_seed: u8,
    pub(super) certificate_chain_digest_seed: u8,
    pub(super) cose_signing_key_digest_seed: u8,
    pub(super) not_before_epoch_seconds: u64,
    pub(super) not_after_epoch_seconds: u64,
    pub(super) evidence_sequence: u64,
    pub(super) debug_mode: bool,
    pub(super) expected_document_digest_hex: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct FheVector {
    pub(super) profile: String,
    pub(super) ciphertext_domain: String,
    pub(super) evaluation_domain: String,
    pub(super) scheme_family: String,
    pub(super) operation: String,
    pub(super) polynomial_modulus_degree: u32,
    pub(super) plaintext_modulus: u64,
    pub(super) ciphertext_modulus_bits: u16,
    pub(super) security_level_bits: u16,
    pub(super) participant_count: usize,
    pub(super) max_participant_count: usize,
    pub(super) max_plaintext_value: u64,
    pub(super) job_seed: u8,
    pub(super) participants: Vec<FheParticipantVector>,
    pub(super) expected_sum: u64,
    pub(super) expected_evaluated_ciphertext_digest_hex: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct FheParticipantVector {
    pub(super) participant_index: usize,
    pub(super) plaintext_value: u64,
    pub(super) nonce_seed: u8,
    pub(super) expected_ciphertext_digest_hex: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct FheRuntimeVector {
    pub(super) profile: String,
    pub(super) ciphertext_transcript_domain: String,
    pub(super) evaluation_domain: String,
    pub(super) scheme_family: String,
    pub(super) operation: String,
    pub(super) job_seed: u8,
    pub(super) participant_count: usize,
    pub(super) key_epoch: u64,
    pub(super) runtime_artifact_seed: u8,
    pub(super) parameter_set_seed: u8,
    pub(super) review_evidence_seed: u8,
    pub(super) external_review_completed: bool,
    pub(super) participants: Vec<FheRuntimeParticipantVector>,
    pub(super) evaluated_ciphertext_seed: u8,
    pub(super) evaluation_proof_seed: u8,
    pub(super) expected_ciphertext_transcript_digest_hex: String,
    pub(super) expected_evaluation_digest_hex: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct FheRuntimeParticipantVector {
    pub(super) participant_index: usize,
    pub(super) ciphertext_seed: u8,
    pub(super) parameter_set_seed: u8,
    pub(super) key_custody_seed: u8,
}

#[derive(Debug, Deserialize)]
pub(super) struct ThresholdFrostVector {
    pub(super) profile: String,
    pub(super) key_ceremony_domain: String,
    pub(super) signature_transcript_domain: String,
    pub(super) suite: String,
    pub(super) job_seed: u8,
    pub(super) participant_count: usize,
    pub(super) threshold: usize,
    pub(super) key_epoch: u64,
    pub(super) runtime_artifact_seed: u8,
    pub(super) review_evidence_seed: u8,
    pub(super) external_review_completed: bool,
    pub(super) participants: Vec<ThresholdFrostParticipantVector>,
    pub(super) message_digest_seed: u8,
    pub(super) signature_digest_seed: u8,
    pub(super) signature_shares: Vec<ThresholdFrostSignatureShareVector>,
    pub(super) expected_key_ceremony_digest_hex: String,
    pub(super) expected_signature_transcript_digest_hex: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct ThresholdFrostParticipantVector {
    pub(super) participant_index: usize,
    pub(super) public_commitment_seed: u8,
    pub(super) share_custody_seed: u8,
}

#[derive(Debug, Deserialize)]
pub(super) struct ThresholdFrostSignatureShareVector {
    pub(super) participant_index: usize,
    pub(super) signature_share_seed: u8,
}

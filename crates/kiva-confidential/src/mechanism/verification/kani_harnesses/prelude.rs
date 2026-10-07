#![allow(unused_imports)]

pub(super) use std::num::{NonZeroU64, NonZeroUsize};

pub(super) use super::{
    AdditiveSecureAggregatePlan, AdditiveShareCommitmentSlot, AdditiveShareReplayWindow,
    AdditiveShareVerificationKey, AuthenticatedAdditiveShareEnvelope,
    ConfidentialJobAdmissionError, ConfidentialJobAdmissionFacts, ConfidentialJobStatus,
    ConfidentialJobTransitionError, ConfidentialMechanismKind, ConfidentialMechanismPlan,
    ConfidentialParticipantStatus, ConfidentialParticipantTransitionError,
    DifferentialPrivacyBudgetError, DifferentialPrivacyBudgetLedger,
    DifferentialPrivacyBudgetPolicyLifecycleError, DifferentialPrivacyBudgetPolicyStatus,
    DifferentialPrivacyCompositionError, DifferentialPrivacyNoiseError,
    DifferentialPrivacyNoiseMechanism, DifferentialPrivacyNoiseParameters,
    DifferentialPrivacyPolicyError, DifferentialPrivacyReleaseContext,
    DifferentialPrivacyReleaseError, DifferentialPrivacyReleasePolicy, FheFeasibilityParameterSet,
    FheFeasibilityReleaseContext, FheFeasibilityReleaseFacts, FheFeasibilitySchemeFamily,
    FheRuntimeCiphertextTranscriptFacts, FheRuntimeCiphertextTranscriptGateContext,
    FheRuntimeReleaseContext, FheRuntimeReleaseFacts, PsiIdentifierDigest, PsiJobRecord,
    PsiOprfElement, PsiOprfParticipantTranscript, PsiOprfParticipantTranscriptStatus, PsiOprfPlan,
    PsiOprfReleaseContext, PsiOprfReleaseFacts, PsiOprfRuntimeBinding, PsiOprfSuite,
    PsiParticipantDataset, ReleasePolicy, SECURE_AGGREGATE_ARTIFACT_BYTES,
    SECURE_AGGREGATE_FIELD_MODULUS, SecretComputeProviderBinding, SecretComputeProviderCapability,
    SecretComputeProviderEvidence, SecretComputeProviderKind, SecretComputeProviderProfile,
    SecretComputeProviderReviewEvidence, SecretComputeProviderUsageClaim,
    SecureAggregateCommitment, SecureAggregateEnvelopeSequence, SecureAggregateError,
    SecureAggregateFieldElement, SecureAggregateShare, SecureAggregateSigningKeyId,
    SecureAggregateSigningPublicKey, SecureAggregateTranscriptId, SecureAggregateTransportProof,
    ShamirShare, ShamirShareComplaintOutcome, ShamirThresholdPlan, ShareSigningKeyLifecycleError,
    ShareSigningKeyLifecycleState, ShareSigningKeyRotationPrecondition, TeeAttestationGateContext,
    TeeAttestationGateFacts, TeeAwsNitroAttestationGateContext, TeeAwsNitroAttestationGateFacts,
    TeeAwsNitroReleaseContext, TeeAwsNitroReleaseFacts, ThresholdFrostKeyCeremonyFacts,
    ThresholdFrostKeyCeremonyGateContext, ThresholdFrostReleaseContext, ThresholdFrostReleaseFacts,
    ThresholdReleaseApprovalSet, ThresholdReleaseAuthoritySide, ThresholdReleaseTransitionError,
    WorkflowError, accept_authenticated_additive_share,
    accept_replay_windowed_authenticated_additive_share, apply_differential_privacy_integer_noise,
    bound_differential_privacy_advanced_composition, classify_threshold_release_authority,
    compute_psi_cohort_intersection, compute_psi_oprf_intersection,
    confidential_job_transition_is_allowed, confidential_participant_accepts_jobs,
    confidential_participant_accepts_new_keys, confidential_participant_transition_is_allowed,
    reconstruct_shamir_secret, record_threshold_release_approval, resolve_shamir_share_complaint,
    revoke_share_signing_key, rotate_share_signing_key, share_signing_key_accepts_new_submissions,
    spend_differential_privacy_budget, summarize_differential_privacy_composition,
    transition_confidential_job_status, transition_confidential_participant_status,
    transition_differential_privacy_budget_policy, validate_additive_share_verification_keys,
    validate_confidential_job_admission, validate_differential_privacy_release,
    validate_fhe_feasibility_parameters, validate_fhe_feasibility_release,
    validate_fhe_runtime_ciphertext_transcript_gate_context, validate_fhe_runtime_release,
    validate_psi_oprf_release, validate_secret_compute_provider_binding,
    validate_secret_compute_provider_for_mechanism, validate_tee_attestation_gate_context,
    validate_tee_aws_nitro_attestation_gate_context, validate_tee_aws_nitro_key_release,
    validate_tee_aws_nitro_result_release, validate_tee_result_release,
    validate_tee_secret_provisioning, validate_threshold_frost_key_ceremony_gate_context,
    validate_threshold_frost_release, validate_threshold_release_approval_threshold,
    validate_threshold_release_authority_set, validate_threshold_release_completion,
    validate_threshold_release_computed_aggregate,
};

pub(super) fn bounded_nonzero(raw: u8) -> NonZeroUsize {
    let value = usize::from(raw % 8) + 1;
    let Some(nonzero) = NonZeroUsize::new(value) else {
        return NonZeroUsize::MIN;
    };
    nonzero
}

pub(super) fn bounded_participant_count(raw: u8) -> NonZeroUsize {
    let value = usize::from(raw % 7) + 2;
    let Some(nonzero) = NonZeroUsize::new(value) else {
        return NonZeroUsize::MIN;
    };
    nonzero
}

pub(super) fn bounded_threshold(raw: u8, participant_count: NonZeroUsize) -> NonZeroUsize {
    let value = usize::from(raw) % participant_count.get() + 1;
    let Some(nonzero) = NonZeroUsize::new(value) else {
        return NonZeroUsize::MIN;
    };
    nonzero
}

pub(super) fn bounded_nonzero_u64(raw: u8) -> NonZeroU64 {
    let value = u64::from(raw % 16) + 1;
    let Some(nonzero) = NonZeroU64::new(value) else {
        return NonZeroU64::MIN;
    };
    nonzero
}

pub(super) fn any_differential_privacy_budget_policy_status()
-> DifferentialPrivacyBudgetPolicyStatus {
    match kani::any::<u8>() % 4 {
        0 => DifferentialPrivacyBudgetPolicyStatus::Draft,
        1 => DifferentialPrivacyBudgetPolicyStatus::Active,
        2 => DifferentialPrivacyBudgetPolicyStatus::Superseded,
        _ => DifferentialPrivacyBudgetPolicyStatus::Retired,
    }
}

pub(super) fn any_confidential_job_status() -> ConfidentialJobStatus {
    match kani::any::<u8>() % 9 {
        0 => ConfidentialJobStatus::Draft,
        1 => ConfidentialJobStatus::Admitted,
        2 => ConfidentialJobStatus::Running,
        3 => ConfidentialJobStatus::AwaitingRelease,
        4 => ConfidentialJobStatus::Released,
        5 => ConfidentialJobStatus::Failed,
        6 => ConfidentialJobStatus::Cancelled,
        7 => ConfidentialJobStatus::Expired,
        _ => ConfidentialJobStatus::Retired,
    }
}

pub(super) fn bounded_field_element(raw: u32) -> SecureAggregateFieldElement {
    let value = u64::from(raw) % SECURE_AGGREGATE_FIELD_MODULUS;
    let Ok(element) = SecureAggregateFieldElement::new(value) else {
        return SecureAggregateFieldElement::ZERO;
    };
    element
}

pub(super) fn fixed_shamir_plan() -> ShamirThresholdPlan {
    let Some(participant_count) = NonZeroUsize::new(3) else {
        return ShamirThresholdPlan {
            release_policy: ReleasePolicy {
                participant_count: NonZeroUsize::MIN,
                release_threshold: NonZeroUsize::MIN,
            },
        };
    };
    let Some(threshold) = NonZeroUsize::new(2) else {
        return ShamirThresholdPlan {
            release_policy: ReleasePolicy {
                participant_count: NonZeroUsize::MIN,
                release_threshold: NonZeroUsize::MIN,
            },
        };
    };
    let Ok(policy) = ReleasePolicy::new(participant_count, threshold) else {
        return ShamirThresholdPlan {
            release_policy: ReleasePolicy {
                participant_count: NonZeroUsize::MIN,
                release_threshold: NonZeroUsize::MIN,
            },
        };
    };
    let Ok(plan) = ShamirThresholdPlan::new(policy) else {
        return ShamirThresholdPlan {
            release_policy: policy,
        };
    };
    plan
}

pub(super) fn bounded_shamir_share(participant_index: NonZeroUsize, raw: u32) -> ShamirShare {
    ShamirShare::new(participant_index, bounded_field_element(raw))
}

pub(super) fn fixed_transcript_id(seed: u8) -> SecureAggregateTranscriptId {
    let bytes = [seed; SECURE_AGGREGATE_ARTIFACT_BYTES];
    let Ok(identifier) = SecureAggregateTranscriptId::new(bytes) else {
        return SecureAggregateTranscriptId {
            bytes: [1; SECURE_AGGREGATE_ARTIFACT_BYTES],
        };
    };
    identifier
}

pub(super) fn fixed_commitment(seed: u8) -> SecureAggregateCommitment {
    let bytes = [seed; SECURE_AGGREGATE_ARTIFACT_BYTES];
    let Ok(commitment) = SecureAggregateCommitment::new(bytes) else {
        return SecureAggregateCommitment {
            bytes: [1; SECURE_AGGREGATE_ARTIFACT_BYTES],
        };
    };
    commitment
}

pub(super) fn fixed_reference_provider_binding(
    production_claim_requested: bool,
) -> SecretComputeProviderBinding {
    let evidence = if production_claim_requested {
        SecretComputeProviderEvidence::new(
            None,
            SecretComputeProviderReviewEvidence::Pending,
            SecretComputeProviderUsageClaim::Production,
        )
    } else {
        SecretComputeProviderEvidence::non_production(None)
    };
    SecretComputeProviderBinding::unchecked_for_verification_with_evidence(
        SecretComputeProviderKind::ReferencePrimitive,
        SecretComputeProviderCapability::ReferenceFixtures,
        SecretComputeProviderProfile::KivaReferencePrimitivesV1,
        fixed_commitment(201),
        fixed_commitment(202),
        evidence,
    )
}

pub(super) fn fixed_secret_custody_provider_binding() -> SecretComputeProviderBinding {
    SecretComputeProviderBinding::unchecked_for_verification_with_evidence(
        SecretComputeProviderKind::SecretCustody,
        SecretComputeProviderCapability::SecretCustody,
        SecretComputeProviderProfile::ExternalReviewedProviderV1,
        fixed_commitment(211),
        fixed_commitment(212),
        SecretComputeProviderEvidence::reviewed_production(None, fixed_commitment(213)),
    )
}

pub(super) fn fixed_psi_digest(seed: u8) -> PsiIdentifierDigest {
    let bytes = [seed; SECURE_AGGREGATE_ARTIFACT_BYTES];
    let Ok(digest) = PsiIdentifierDigest::new(bytes) else {
        return PsiIdentifierDigest {
            bytes: [1; SECURE_AGGREGATE_ARTIFACT_BYTES],
        };
    };
    digest
}

pub(super) fn fixed_psi_oprf_element(seed: u8) -> PsiOprfElement {
    let bytes = [seed; SECURE_AGGREGATE_ARTIFACT_BYTES];
    let Ok(element) = PsiOprfElement::new(bytes) else {
        return PsiOprfElement {
            bytes: [1; SECURE_AGGREGATE_ARTIFACT_BYTES],
        };
    };
    element
}

pub(super) fn fixed_psi_oprf_plan() -> PsiOprfPlan {
    let Some(participant_count) = NonZeroUsize::new(2) else {
        return PsiOprfPlan {
            release_policy: ReleasePolicy {
                participant_count: NonZeroUsize::MIN,
                release_threshold: NonZeroUsize::MIN,
            },
            runtime_binding: fixed_psi_oprf_runtime_binding(),
        };
    };
    let Ok(policy) = ReleasePolicy::new(participant_count, participant_count) else {
        return PsiOprfPlan {
            release_policy: ReleasePolicy {
                participant_count: NonZeroUsize::MIN,
                release_threshold: NonZeroUsize::MIN,
            },
            runtime_binding: fixed_psi_oprf_runtime_binding(),
        };
    };
    let Ok(plan) = PsiOprfPlan::new(policy, fixed_psi_oprf_runtime_binding()) else {
        return PsiOprfPlan {
            release_policy: policy,
            runtime_binding: fixed_psi_oprf_runtime_binding(),
        };
    };
    plan
}

pub(super) fn fixed_psi_oprf_runtime_binding() -> PsiOprfRuntimeBinding {
    let Ok(binding) = PsiOprfRuntimeBinding::new(
        PsiOprfSuite::Ristretto255Sha512Voprf,
        fixed_commitment(91),
        fixed_commitment(92),
        true,
    ) else {
        return PsiOprfRuntimeBinding {
            suite: PsiOprfSuite::Ristretto255Sha512Voprf,
            runtime_artifact_digest: fixed_commitment(91),
            review_evidence_digest: fixed_commitment(92),
            external_review_completed: true,
        };
    };
    binding
}

pub(super) fn fixed_signing_key_id(seed: u8) -> SecureAggregateSigningKeyId {
    let bytes = [seed; SECURE_AGGREGATE_ARTIFACT_BYTES];
    let Ok(identifier) = SecureAggregateSigningKeyId::new(bytes) else {
        return SecureAggregateSigningKeyId {
            bytes: [1; SECURE_AGGREGATE_ARTIFACT_BYTES],
        };
    };
    identifier
}

pub(super) fn fixed_signing_public_key(seed: u8) -> SecureAggregateSigningPublicKey {
    let bytes = [seed; SECURE_AGGREGATE_ARTIFACT_BYTES];
    let Ok(public_key) = SecureAggregateSigningPublicKey::new(bytes) else {
        return SecureAggregateSigningPublicKey {
            bytes: [1; SECURE_AGGREGATE_ARTIFACT_BYTES],
        };
    };
    public_key
}

pub(super) fn fixed_verification_key(
    signing_key_id: SecureAggregateSigningKeyId,
    public_key_seed: u8,
) -> AdditiveShareVerificationKey {
    AdditiveShareVerificationKey::new(signing_key_id, fixed_signing_public_key(public_key_seed))
}

pub(super) fn fixed_transport_proof(seed: u8) -> SecureAggregateTransportProof {
    let bytes = [seed; SECURE_AGGREGATE_ARTIFACT_BYTES];
    let Ok(proof) = SecureAggregateTransportProof::new(bytes) else {
        return SecureAggregateTransportProof {
            bytes: [1; SECURE_AGGREGATE_ARTIFACT_BYTES],
        };
    };
    proof
}

pub(super) fn fixed_sequence(raw: u8) -> SecureAggregateEnvelopeSequence {
    let value = u64::from(raw % 8) + 1;
    let Some(nonzero) = NonZeroU64::new(value) else {
        return SecureAggregateEnvelopeSequence::new(NonZeroU64::MIN);
    };
    SecureAggregateEnvelopeSequence::new(nonzero)
}

pub(super) fn any_signing_key_lifecycle_state() -> ShareSigningKeyLifecycleState {
    match kani::any::<u8>() % 3 {
        0 => ShareSigningKeyLifecycleState::Missing,
        1 => ShareSigningKeyLifecycleState::Active,
        _ => ShareSigningKeyLifecycleState::Revoked,
    }
}

pub(super) fn any_confidential_participant_status() -> ConfidentialParticipantStatus {
    match kani::any::<u8>() % 5 {
        0 => ConfidentialParticipantStatus::Invited,
        1 => ConfidentialParticipantStatus::Active,
        2 => ConfidentialParticipantStatus::Suspended,
        3 => ConfidentialParticipantStatus::Frozen,
        _ => ConfidentialParticipantStatus::Removed,
    }
}

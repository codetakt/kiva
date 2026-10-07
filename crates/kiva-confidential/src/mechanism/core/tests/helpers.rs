#![allow(clippy::wildcard_imports)]

use super::*;

pub(super) fn field(value: u64) -> SecureAggregateFieldElement {
    let element = SecureAggregateFieldElement::new(value);
    assert!(element.is_ok());
    let Ok(element) = element else {
        return SecureAggregateFieldElement::ZERO;
    };
    element
}

pub(super) fn nonzero(value: usize) -> NonZeroUsize {
    let nonzero = NonZeroUsize::new(value);
    assert!(nonzero.is_some());
    let Some(nonzero) = nonzero else {
        return NonZeroUsize::MIN;
    };
    nonzero
}

pub(super) fn nonzero_u64(value: u64) -> NonZeroU64 {
    let nonzero = NonZeroU64::new(value);
    assert!(nonzero.is_some());
    let Some(nonzero) = nonzero else {
        return NonZeroU64::MIN;
    };
    nonzero
}

pub(super) fn splitmix64(seed: u64) -> u64 {
    let mut value = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

pub(super) fn optional_nonzero(value: usize) -> NonZeroUsize {
    nonzero(value)
}

pub(super) fn policy(participant_count: usize, release_threshold: usize) -> ReleasePolicy {
    let policy = ReleasePolicy::new(nonzero(participant_count), nonzero(release_threshold));
    assert!(policy.is_ok());
    let Ok(policy) = policy else {
        return ReleasePolicy {
            participant_count: nonzero(2),
            release_threshold: nonzero(2),
        };
    };
    policy
}

pub(super) fn shamir_plan(participant_count: usize, threshold: usize) -> ShamirThresholdPlan {
    let plan = ShamirThresholdPlan::new(policy(participant_count, threshold));
    assert!(plan.is_ok());
    let Ok(plan) = plan else {
        return ShamirThresholdPlan {
            release_policy: policy(2, 2),
        };
    };
    plan
}

pub(super) fn differential_privacy_policy(
    minimum_population: usize,
    max_epsilon_microunits: u64,
) -> DifferentialPrivacyReleasePolicy {
    let policy = DifferentialPrivacyReleasePolicy::new(
        nonzero(minimum_population),
        nonzero_u64(max_epsilon_microunits),
    );
    assert!(policy.is_ok());
    let Ok(policy) = policy else {
        return DifferentialPrivacyReleasePolicy {
            minimum_population: nonzero(2),
            max_epsilon_microunits: NonZeroU64::MIN,
        };
    };
    policy
}

pub(super) fn differential_privacy_budget_ledger(
    total_epsilon_microunits: u64,
    spent_epsilon_microunits: u64,
) -> DifferentialPrivacyBudgetLedger {
    let ledger = DifferentialPrivacyBudgetLedger::new(
        nonzero_u64(total_epsilon_microunits),
        spent_epsilon_microunits,
    );
    assert!(ledger.is_ok());
    let Ok(ledger) = ledger else {
        return DifferentialPrivacyBudgetLedger {
            total_epsilon_microunits: NonZeroU64::MIN,
            spent_epsilon_microunits: 0,
        };
    };
    ledger
}

pub(super) fn differential_privacy_noise_parameters(
    epsilon_microunits: u64,
    l1_sensitivity: u64,
) -> DifferentialPrivacyNoiseParameters {
    DifferentialPrivacyNoiseParameters::new(
        DifferentialPrivacyNoiseMechanism::DiscreteLaplace,
        super::DifferentialPrivacyAccountingScope::SubjectBudget,
        nonzero_u64(epsilon_microunits),
        nonzero_u64(l1_sensitivity),
    )
}

pub(super) fn fhe_plan(participant_count: usize) -> FheFeasibilityPlan {
    let plan = FheFeasibilityPlan::new(
        policy(participant_count, participant_count),
        FheFeasibilityParameterSet::sum_profile_v1(),
    );
    assert!(plan.is_ok());
    let Ok(plan) = plan else {
        return FheFeasibilityPlan {
            release_policy: policy(2, 2),
            parameter_set: FheFeasibilityParameterSet::sum_profile_v1(),
        };
    };
    plan
}

pub(super) fn fhe_plaintext(
    parameter_set: FheFeasibilityParameterSet,
    participant_index: usize,
    value: u64,
) -> FheFeasibilityPlaintext {
    let plaintext = FheFeasibilityPlaintext::new(parameter_set, nonzero(participant_index), value);
    assert!(plaintext.is_ok());
    let Ok(plaintext) = plaintext else {
        return FheFeasibilityPlaintext {
            participant_index: NonZeroUsize::MIN,
            value: 0,
        };
    };
    plaintext
}

pub(super) fn fhe_nonce(seed: u8) -> FheFeasibilityNonce {
    let bytes = artifact(seed);
    assert!(FheFeasibilityNonce::new(bytes).is_ok());
    FheFeasibilityNonce { bytes }
}

pub(super) fn fhe_ciphertext(
    plan: FheFeasibilityPlan,
    job_id: SecureAggregateTranscriptId,
    participant_index: usize,
    value: u64,
    nonce_seed: u8,
) -> FheFeasibilityCiphertextFixture {
    let plaintext = fhe_plaintext(plan.parameter_set(), participant_index, value);
    let nonce = fhe_nonce(nonce_seed);
    let ciphertext = encrypt_fhe_feasibility_plaintext_fixture(plan, job_id, plaintext, nonce);
    assert!(ciphertext.is_ok());
    let Ok(ciphertext) = ciphertext else {
        return FheFeasibilityCiphertextFixture::from_fixture_parts(
            job_id,
            plan.parameter_set(),
            nonzero(participant_index),
            commitment(1),
            NonZeroUsize::MIN,
            0,
            0,
        );
    };
    assert_eq!(
        verify_fhe_feasibility_ciphertext_fixture(
            job_id,
            plan.parameter_set(),
            plaintext,
            nonce,
            ciphertext,
        ),
        Ok(())
    );
    ciphertext
}

pub(super) fn share(participant_index: usize, value: u64) -> SecureAggregateShare {
    SecureAggregateShare::new(nonzero(participant_index), field(value))
}

pub(super) fn shamir_share(participant_index: usize, value: u64) -> ShamirShare {
    ShamirShare::new(nonzero(participant_index), field(value))
}

pub(super) fn psi_dataset(
    participant_index: usize,
    identifier_digests: Vec<PsiIdentifierDigest>,
) -> PsiParticipantDataset {
    PsiParticipantDataset::new(nonzero(participant_index), identifier_digests)
}

pub(super) fn psi_digest(
    job_id: SecureAggregateTranscriptId,
    namespace: &str,
    identifier: &str,
) -> PsiIdentifierDigest {
    let digest = derive_psi_cohort_identifier_digest(job_id, namespace, identifier);
    assert!(digest.is_ok());
    let Ok(digest) = digest else {
        return PsiIdentifierDigest { bytes: artifact(1) };
    };
    digest
}

pub(super) fn psi_oprf_runtime_binding(external_review_completed: bool) -> PsiOprfRuntimeBinding {
    let binding = PsiOprfRuntimeBinding::new(
        PsiOprfSuite::Ristretto255Sha512Voprf,
        commitment(91),
        commitment(92),
        external_review_completed,
    );
    assert!(binding.is_ok());
    let Ok(binding) = binding else {
        return PsiOprfRuntimeBinding {
            suite: PsiOprfSuite::Ristretto255Sha512Voprf,
            runtime_artifact_digest: commitment(91),
            review_evidence_digest: commitment(92),
            external_review_completed,
        };
    };
    binding
}

pub(super) fn psi_oprf_plan(
    participant_count: usize,
    external_review_completed: bool,
) -> PsiOprfPlan {
    let plan = PsiOprfPlan::new(
        policy(participant_count, participant_count),
        psi_oprf_runtime_binding(external_review_completed),
    );
    assert!(plan.is_ok());
    let Ok(plan) = plan else {
        return PsiOprfPlan {
            release_policy: policy(2, 2),
            runtime_binding: psi_oprf_runtime_binding(external_review_completed),
        };
    };
    plan
}

pub(super) fn psi_oprf_element(
    job_id: SecureAggregateTranscriptId,
    namespace: &str,
    identifier: &str,
) -> PsiOprfElement {
    let element = derive_psi_oprf_fixture_element(job_id, namespace, identifier);
    assert!(element.is_ok());
    let Ok(element) = element else {
        return PsiOprfElement { bytes: artifact(1) };
    };
    element
}

pub(super) fn psi_oprf_transcript(
    participant_index: usize,
    evaluated_elements: Vec<PsiOprfElement>,
) -> PsiOprfParticipantTranscript {
    PsiOprfParticipantTranscript::new(
        nonzero(participant_index),
        PsiOprfParticipantTranscriptStatus::Submitted,
        evaluated_elements,
    )
}

pub(super) fn threshold_frost_runtime_binding(
    external_review_completed: bool,
) -> ThresholdFrostRuntimeBinding {
    let binding = ThresholdFrostRuntimeBinding::new(
        ThresholdFrostSuite::Ed25519Sha512Frost,
        commitment(141),
        commitment(142),
        external_review_completed,
    );
    assert!(binding.is_ok());
    let Ok(binding) = binding else {
        return ThresholdFrostRuntimeBinding {
            suite: ThresholdFrostSuite::Ed25519Sha512Frost,
            runtime_artifact_digest: commitment(141),
            review_evidence_digest: commitment(142),
            external_review_completed,
        };
    };
    binding
}

pub(super) fn threshold_frost_plan(
    participant_count: usize,
    threshold: usize,
    external_review_completed: bool,
) -> ThresholdFrostKeyCeremonyPlan {
    let plan = ThresholdFrostKeyCeremonyPlan::new(
        policy(participant_count, threshold),
        threshold_frost_runtime_binding(external_review_completed),
        sequence(7),
    );
    assert!(plan.is_ok());
    let Ok(plan) = plan else {
        return ThresholdFrostKeyCeremonyPlan {
            release_policy: policy(2, 2),
            runtime_binding: threshold_frost_runtime_binding(external_review_completed),
            key_epoch: sequence(1),
        };
    };
    plan
}

pub(super) fn threshold_frost_participant(
    participant_index: usize,
    public_commitment_seed: u8,
    share_custody_seed: u8,
) -> ThresholdFrostParticipantTranscript {
    threshold_frost_participant_with_status(
        participant_index,
        ThresholdFrostParticipantTranscriptStatus::Submitted,
        public_commitment_seed,
        Some(share_custody_seed),
        true,
        false,
    )
}

pub(super) fn threshold_frost_participant_with_status(
    participant_index: usize,
    status: ThresholdFrostParticipantTranscriptStatus,
    public_commitment_seed: u8,
    share_custody_seed: Option<u8>,
    complaint_resolved: bool,
    equivocation_detected: bool,
) -> ThresholdFrostParticipantTranscript {
    ThresholdFrostParticipantTranscript::new(
        nonzero(participant_index),
        status,
        commitment(public_commitment_seed),
        share_custody_seed.map(commitment),
        complaint_resolved,
        equivocation_detected,
    )
}

pub(super) fn threshold_frost_signature_share(
    participant_index: usize,
    signature_share_seed: u8,
) -> ThresholdFrostSignatureShare {
    ThresholdFrostSignatureShare::new(nonzero(participant_index), commitment(signature_share_seed))
}

pub(super) fn fhe_runtime_binding(external_review_completed: bool) -> FheRuntimeBinding {
    let binding = FheRuntimeBinding::new(
        FheRuntimeSchemeFamily::BfvIntegerSum,
        FheRuntimeOperation::Sum,
        commitment(171),
        commitment(172),
        commitment(173),
        external_review_completed,
    );
    assert!(binding.is_ok());
    let Ok(binding) = binding else {
        return FheRuntimeBinding {
            scheme_family: FheRuntimeSchemeFamily::BfvIntegerSum,
            operation: FheRuntimeOperation::Sum,
            runtime_artifact_digest: commitment(171),
            parameter_set_digest: commitment(172),
            review_evidence_digest: commitment(173),
            external_review_completed,
        };
    };
    binding
}

pub(super) fn fhe_runtime_plan(
    participant_count: usize,
    external_review_completed: bool,
) -> FheRuntimePlan {
    let plan = FheRuntimePlan::new(
        policy(participant_count, participant_count),
        fhe_runtime_binding(external_review_completed),
        sequence(9),
    );
    assert!(plan.is_ok());
    let Ok(plan) = plan else {
        return FheRuntimePlan {
            release_policy: policy(2, 2),
            runtime_binding: fhe_runtime_binding(external_review_completed),
            key_epoch: sequence(1),
        };
    };
    plan
}

pub(super) fn fhe_runtime_ciphertext(
    participant_index: usize,
    ciphertext_seed: u8,
    parameter_set_seed: u8,
    key_custody_seed: Option<u8>,
) -> FheRuntimeParticipantCiphertext {
    fhe_runtime_ciphertext_with_status(
        participant_index,
        FheRuntimeParticipantCiphertextStatus::Submitted,
        ciphertext_seed,
        parameter_set_seed,
        key_custody_seed,
    )
}

pub(super) fn fhe_runtime_ciphertext_with_status(
    participant_index: usize,
    status: FheRuntimeParticipantCiphertextStatus,
    ciphertext_seed: u8,
    parameter_set_seed: u8,
    key_custody_seed: Option<u8>,
) -> FheRuntimeParticipantCiphertext {
    FheRuntimeParticipantCiphertext::new(
        nonzero(participant_index),
        status,
        commitment(ciphertext_seed),
        commitment(parameter_set_seed),
        key_custody_seed.map(commitment),
    )
}

pub(super) fn tee_policy(
    policy_approved: bool,
    job_seed: u8,
    current_epoch_seconds: u64,
) -> TeeAttestationPolicy {
    tee_policy_with_min_sequence(policy_approved, job_seed, current_epoch_seconds, 1)
}

pub(super) fn tee_policy_with_min_sequence(
    policy_approved: bool,
    job_seed: u8,
    current_epoch_seconds: u64,
    minimum_sequence: u64,
) -> TeeAttestationPolicy {
    TeeAttestationPolicy::new(
        TeeAttestationFamily::DeploymentBoundPortable,
        TeeAttestationFormat::PortableEvidenceV1,
        transcript(job_seed),
        commitment(10),
        commitment(20),
        commitment(30),
        sequence(minimum_sequence),
        current_epoch_seconds,
        policy_approved,
    )
}

pub(super) fn tee_claims(
    job_seed: u8,
    measurement_seed: u8,
    image_digest_seed: u8,
    policy_binding_seed: u8,
    not_before_epoch_seconds: u64,
    not_after_epoch_seconds: u64,
    evidence_sequence: u64,
) -> TeeAttestationClaims {
    let claims = TeeAttestationClaims::new(
        TeeAttestationFamily::DeploymentBoundPortable,
        TeeAttestationFormat::PortableEvidenceV1,
        transcript(job_seed),
        commitment(measurement_seed),
        commitment(image_digest_seed),
        commitment(policy_binding_seed),
        not_before_epoch_seconds,
        not_after_epoch_seconds,
        sequence(evidence_sequence),
    );
    assert!(claims.is_ok());
    let Ok(claims) = claims else {
        return TeeAttestationClaims {
            family: TeeAttestationFamily::DeploymentBoundPortable,
            format: TeeAttestationFormat::PortableEvidenceV1,
            job_id: transcript(1),
            measurement: commitment(10),
            image_digest: commitment(20),
            policy_binding: commitment(30),
            not_before_epoch_seconds: 0,
            not_after_epoch_seconds: 0,
            evidence_sequence: sequence(1),
        };
    };
    claims
}

pub(super) fn tee_evidence(
    job_seed: u8,
    measurement_seed: u8,
    image_digest_seed: u8,
    policy_binding_seed: u8,
    not_before_epoch_seconds: u64,
    not_after_epoch_seconds: u64,
    evidence_sequence: u64,
) -> TeeAttestationEvidence {
    let evidence = TeeAttestationEvidence::new(tee_claims(
        job_seed,
        measurement_seed,
        image_digest_seed,
        policy_binding_seed,
        not_before_epoch_seconds,
        not_after_epoch_seconds,
        evidence_sequence,
    ));
    assert!(evidence.is_ok());
    let Ok(evidence) = evidence else {
        return TeeAttestationEvidence::from_parts(
            tee_claims(1, 10, 20, 30, 0, 0, 1),
            commitment(1),
        );
    };
    evidence
}

pub(super) fn nitro_pcr(seed: u8) -> TeeAwsNitroPcrDigest {
    TeeAwsNitroPcrDigest::new(artifact(seed))
}

pub(super) fn nitro_zero_pcr() -> TeeAwsNitroPcrDigest {
    TeeAwsNitroPcrDigest::new([0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES])
}

pub(super) fn nitro_verifier_binding(
    external_security_review_completed: bool,
    verifier_key_epoch: u64,
) -> TeeAwsNitroVerifierBinding {
    TeeAwsNitroVerifierBinding::new(
        commitment(41),
        commitment(40),
        commitment(42),
        sequence(verifier_key_epoch),
        external_security_review_completed,
    )
}

pub(super) fn nitro_policy(
    policy_approved: bool,
    job_seed: u8,
    current_epoch_seconds: u64,
) -> TeeAwsNitroAttestationPolicy {
    nitro_policy_with_minimums(policy_approved, job_seed, current_epoch_seconds, 1, 1)
}

pub(super) fn nitro_policy_with_minimums(
    policy_approved: bool,
    job_seed: u8,
    current_epoch_seconds: u64,
    minimum_evidence_sequence: u64,
    minimum_verifier_key_epoch: u64,
) -> TeeAwsNitroAttestationPolicy {
    TeeAwsNitroAttestationPolicy::new(
        TeeAwsNitroAttestationFamily::AwsNitroEnclaves,
        TeeAwsNitroAttestationFormat::VerifiedNitroAttestationDocumentV1,
        transcript(job_seed),
        nitro_pcr(10),
        nitro_pcr(11),
        nitro_pcr(12),
        nitro_pcr(13),
        nitro_pcr(18),
        commitment(20),
        commitment(30),
        commitment(31),
        commitment(40),
        commitment(41),
        sequence(minimum_evidence_sequence),
        sequence(minimum_verifier_key_epoch),
        current_epoch_seconds,
        policy_approved,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn nitro_claims_with_seeds(
    job_seed: u8,
    pcr0_seed: u8,
    pcr1_seed: u8,
    pcr2_seed: u8,
    pcr3_seed: u8,
    pcr8_seed: u8,
    image_digest_seed: u8,
    policy_binding_seed: u8,
    nonce_digest_seed: u8,
    certificate_chain_digest_seed: u8,
    cose_signing_key_digest_seed: u8,
    not_before_epoch_seconds: u64,
    not_after_epoch_seconds: u64,
    evidence_sequence: u64,
    debug_mode: bool,
) -> TeeAwsNitroAttestationClaims {
    let claims = TeeAwsNitroAttestationClaims::new(
        TeeAwsNitroAttestationFamily::AwsNitroEnclaves,
        TeeAwsNitroAttestationFormat::VerifiedNitroAttestationDocumentV1,
        transcript(job_seed),
        nitro_pcr(pcr0_seed),
        nitro_pcr(pcr1_seed),
        nitro_pcr(pcr2_seed),
        nitro_pcr(pcr3_seed),
        nitro_pcr(pcr8_seed),
        commitment(image_digest_seed),
        commitment(policy_binding_seed),
        commitment(nonce_digest_seed),
        commitment(certificate_chain_digest_seed),
        commitment(cose_signing_key_digest_seed),
        not_before_epoch_seconds,
        not_after_epoch_seconds,
        sequence(evidence_sequence),
        debug_mode,
    );
    assert!(claims.is_ok());
    let Ok(claims) = claims else {
        return TeeAwsNitroAttestationClaims {
            family: TeeAwsNitroAttestationFamily::AwsNitroEnclaves,
            format: TeeAwsNitroAttestationFormat::VerifiedNitroAttestationDocumentV1,
            job_id: transcript(1),
            pcr0: nitro_pcr(10),
            pcr1: nitro_pcr(11),
            pcr2: nitro_pcr(12),
            pcr3: nitro_pcr(13),
            pcr8: nitro_pcr(18),
            image_digest: commitment(20),
            policy_binding: commitment(30),
            nonce_digest: commitment(31),
            certificate_chain_digest: commitment(50),
            cose_signing_key_digest: commitment(51),
            not_before_epoch_seconds: 0,
            not_after_epoch_seconds: 0,
            evidence_sequence: sequence(1),
            debug_mode: false,
        };
    };
    claims
}

pub(super) fn nitro_claims(
    job_seed: u8,
    not_before_epoch_seconds: u64,
    not_after_epoch_seconds: u64,
    evidence_sequence: u64,
    debug_mode: bool,
) -> TeeAwsNitroAttestationClaims {
    nitro_claims_with_seeds(
        job_seed,
        10,
        11,
        12,
        13,
        18,
        20,
        30,
        31,
        50,
        51,
        not_before_epoch_seconds,
        not_after_epoch_seconds,
        evidence_sequence,
        debug_mode,
    )
}

pub(super) fn nitro_claims_with_pcr0(
    pcr0: TeeAwsNitroPcrDigest,
    debug_mode: bool,
) -> TeeAwsNitroAttestationClaims {
    TeeAwsNitroAttestationClaims {
        pcr0,
        debug_mode,
        ..nitro_claims(1, 90, 110, 2, false)
    }
}

pub(super) fn nitro_evidence(
    job_seed: u8,
    not_before_epoch_seconds: u64,
    not_after_epoch_seconds: u64,
    evidence_sequence: u64,
) -> TeeAwsNitroAttestationEvidence {
    let evidence = TeeAwsNitroAttestationEvidence::new(
        nitro_claims(
            job_seed,
            not_before_epoch_seconds,
            not_after_epoch_seconds,
            evidence_sequence,
            false,
        ),
        nitro_verifier_binding(true, 2),
        true,
        true,
    );
    assert!(evidence.is_ok());
    let Ok(evidence) = evidence else {
        return TeeAwsNitroAttestationEvidence::from_parts(
            nitro_claims(1, 0, 0, 1, false),
            nitro_verifier_binding(true, 1),
            commitment(1),
            true,
            true,
        );
    };
    evidence
}

pub(super) fn shamir_opening_nonce(seed: u8) -> ShamirShareOpeningNonce {
    let bytes = artifact(seed);
    assert!(ShamirShareOpeningNonce::new(bytes).is_ok());
    ShamirShareOpeningNonce { bytes }
}

pub(super) fn verifiable_shamir_share(
    transcript_id: SecureAggregateTranscriptId,
    participant_index: usize,
    value: u64,
    nonce_seed: u8,
) -> VerifiableShamirShare {
    let share = shamir_share(participant_index, value);
    let opening_nonce = shamir_opening_nonce(nonce_seed);
    let commitment = derive_shamir_share_opening_commitment(transcript_id, share, opening_nonce);
    assert!(commitment.is_ok());
    let Ok(commitment) = commitment else {
        return VerifiableShamirShare::new(
            share,
            SecureAggregateCommitment { bytes: artifact(1) },
            opening_nonce,
        );
    };
    VerifiableShamirShare::new(share, commitment, opening_nonce)
}

pub(super) fn hex_32(value: &str) -> [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
    assert_eq!(value.len(), SECURE_AGGREGATE_ARTIFACT_BYTES * 2);
    let mut bytes = [0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let start = index * 2;
        let end = start + 2;
        let parsed = u8::from_str_radix(&value[start..end], 16);
        assert!(parsed.is_ok());
        let Ok(parsed) = parsed else {
            continue;
        };
        *byte = parsed;
    }
    bytes
}

pub(super) fn commitment_from_hex(value: &str) -> SecureAggregateCommitment {
    let commitment = SecureAggregateCommitment::new(hex_32(value));
    assert!(commitment.is_ok());
    let Ok(commitment) = commitment else {
        return SecureAggregateCommitment { bytes: artifact(1) };
    };
    commitment
}

pub(super) fn artifact(seed: u8) -> [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
    [seed; SECURE_AGGREGATE_ARTIFACT_BYTES]
}

pub(super) fn transcript(seed: u8) -> SecureAggregateTranscriptId {
    let bytes = artifact(seed);
    assert!(SecureAggregateTranscriptId::new(bytes).is_ok());
    SecureAggregateTranscriptId { bytes }
}

pub(super) fn commitment(seed: u8) -> SecureAggregateCommitment {
    let bytes = artifact(seed);
    assert!(SecureAggregateCommitment::new(bytes).is_ok());
    SecureAggregateCommitment { bytes }
}

pub(super) fn signing_key_id(seed: u8) -> SecureAggregateSigningKeyId {
    let bytes = artifact(seed);
    assert!(SecureAggregateSigningKeyId::new(bytes).is_ok());
    SecureAggregateSigningKeyId { bytes }
}

pub(super) fn transport_proof(seed: u8) -> SecureAggregateTransportProof {
    let bytes = artifact(seed);
    assert!(SecureAggregateTransportProof::new(bytes).is_ok());
    SecureAggregateTransportProof { bytes }
}

pub(super) fn sequence(value: u64) -> SecureAggregateEnvelopeSequence {
    let sequence = SecureAggregateEnvelopeSequence::try_from(value);
    assert!(sequence.is_ok());
    let Ok(sequence) = sequence else {
        return SecureAggregateEnvelopeSequence::new(NonZeroU64::MIN);
    };
    sequence
}

pub(super) fn replay_window(first: u64, last: u64) -> AdditiveShareReplayWindow {
    let replay_window = AdditiveShareReplayWindow::new(sequence(first), sequence(last));
    assert!(replay_window.is_ok());
    let Ok(replay_window) = replay_window else {
        return AdditiveShareReplayWindow {
            first_sequence: sequence(1),
            last_sequence: sequence(1),
        };
    };
    replay_window
}

pub(super) fn commitment_slot(
    transcript_id: SecureAggregateTranscriptId,
    participant_index: usize,
    commitment_seed: u8,
    signing_key_seed: u8,
) -> AdditiveShareCommitmentSlot {
    AdditiveShareCommitmentSlot::new(
        transcript_id,
        nonzero(participant_index),
        commitment(commitment_seed),
        signing_key_id(signing_key_seed),
    )
}

pub(super) fn authenticated_envelope(
    transcript_id: SecureAggregateTranscriptId,
    participant_index: usize,
    value: u64,
    commitment_seed: u8,
    signing_key_seed: u8,
    sequence_value: u64,
    proof_seed: u8,
) -> AuthenticatedAdditiveShareEnvelope {
    AuthenticatedAdditiveShareEnvelope::new(
        transcript_id,
        share(participant_index, value),
        commitment(commitment_seed),
        signing_key_id(signing_key_seed),
        sequence(sequence_value),
        transport_proof(proof_seed),
    )
}

pub(super) fn signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&artifact(seed))
}

pub(super) fn signing_public_key(seed: u8) -> SecureAggregateSigningPublicKey {
    let bytes = signing_key(seed).verifying_key().to_bytes();
    assert!(SecureAggregateSigningPublicKey::new(bytes).is_ok());
    SecureAggregateSigningPublicKey { bytes }
}

pub(super) fn verification_key(seed: u8) -> AdditiveShareVerificationKey {
    AdditiveShareVerificationKey::new(signing_key_id(seed), signing_public_key(seed))
}

pub(super) fn signature_for(
    signing_key_seed: u8,
    envelope: AuthenticatedAdditiveShareEnvelope,
) -> SecureAggregateSignature {
    let payload = additive_share_envelope_signature_payload(envelope);
    let signature = signing_key(signing_key_seed).sign(&payload);
    let bytes = signature.to_bytes();
    assert!(SecureAggregateSignature::new(bytes).is_ok());
    SecureAggregateSignature { bytes }
}

pub(super) fn signed_envelope(
    signing_key_seed: u8,
    envelope: AuthenticatedAdditiveShareEnvelope,
) -> SignedAuthenticatedAdditiveShareEnvelope {
    SignedAuthenticatedAdditiveShareEnvelope::new(
        envelope,
        signature_for(signing_key_seed, envelope),
    )
}

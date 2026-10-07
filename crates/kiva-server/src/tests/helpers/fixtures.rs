#![allow(clippy::wildcard_imports)]

use super::*;

pub(in crate::tests) fn assert_no_pds_raw_payload_fields(value: &Value) {
    for field in [
        "statement",
        "result",
        "aggregate",
        "aggregateValue",
        "noiseValue",
        "reconstructedValue",
        "share",
        "shares",
        "signature",
        "signedEnvelope",
    ] {
        assert!(
            !json_contains_key(value, field),
            "PDS metadata response must not expose raw field {field}"
        );
    }
}

pub(in crate::tests) fn json_contains_key(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(object) => {
            object.contains_key(key) || object.values().any(|value| json_contains_key(value, key))
        }
        Value::Array(values) => values.iter().any(|value| json_contains_key(value, key)),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => false,
    }
}

pub(in crate::tests) fn json_body<T: serde::Serialize>(
    value: &T,
    label: &str,
) -> Result<Bytes, String> {
    serde_json::to_vec(value)
        .map(Bytes::from)
        .map_err(|error| format!("{label} should serialize: {error:?}"))
}

pub(in crate::tests) fn confidential_headers() -> HeaderMap {
    confidential_headers_for("confidential-coordinator")
}

pub(in crate::tests) fn confidential_headers_for(token: &'static str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let authorization = format!("Bearer {token}");
    let value = HeaderValue::from_str(&authorization)
        .unwrap_or_else(|error| panic!("authorization header should be valid: {error:?}"));
    headers.insert(AUTHORIZATION, value);
    headers
}

pub(in crate::tests) fn fixed_time() -> OffsetDateTime {
    let date = match Date::from_calendar_date(2025, Month::January, 1) {
        Ok(date) => date,
        Err(error) => panic!("fixed date should be valid: {error:?}"),
    };
    date.with_time(Time::MIDNIGHT).assume_utc()
}

pub(in crate::tests) async fn response_json(
    response: axum::response::Response,
) -> Result<Value, String> {
    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .map_err(|error| format!("response body should be readable: {error:?}"))?;
    serde_json::from_slice(&body)
        .map_err(|error| format!("response body should be JSON: {error:?}"))
}

pub(in crate::tests) fn confidential_aggregate_request()
-> super::ConfidentialAdditiveAggregateRequest {
    let transcript_id = transcript(1);
    super::ConfidentialAdditiveAggregateRequest {
        participant_count: nonzero(3),
        transcript_id,
        slots: vec![
            commitment_slot(transcript_id, 1, 11, 21),
            commitment_slot(transcript_id, 2, 12, 22),
            commitment_slot(transcript_id, 3, 13, 23),
        ],
        replay_window: replay_window(1, 3),
        signed_envelopes: vec![
            signed_envelope(
                22,
                authenticated_envelope(transcript_id, 2, 2_000_000_000, 12, 22, 2, 32),
            ),
            signed_envelope(
                23,
                authenticated_envelope(transcript_id, 3, 1_294_967_333, 13, 23, 3, 33),
            ),
            signed_envelope(
                21,
                authenticated_envelope(transcript_id, 1, 1_000_000_000, 11, 21, 1, 31),
            ),
        ],
    }
}

pub(in crate::tests) fn confidential_aggregate_request_with_rotated_key()
-> super::ConfidentialAdditiveAggregateRequest {
    let transcript_id = transcript(1);
    super::ConfidentialAdditiveAggregateRequest {
        participant_count: nonzero(3),
        transcript_id,
        slots: vec![
            commitment_slot(transcript_id, 1, 11, 21),
            commitment_slot(transcript_id, 2, 12, 24),
            commitment_slot(transcript_id, 3, 13, 23),
        ],
        replay_window: replay_window(1, 3),
        signed_envelopes: vec![
            signed_envelope(
                24,
                authenticated_envelope(transcript_id, 2, 2_000_000_000, 12, 24, 2, 32),
            ),
            signed_envelope(
                23,
                authenticated_envelope(transcript_id, 3, 1_294_967_333, 13, 23, 3, 33),
            ),
            signed_envelope(
                21,
                authenticated_envelope(transcript_id, 1, 1_000_000_000, 11, 21, 1, 31),
            ),
        ],
    }
}

pub(in crate::tests) fn confidential_aggregate_request_without_envelopes()
-> super::ConfidentialAdditiveAggregateRequest {
    let mut request = confidential_aggregate_request();
    request.signed_envelopes = Vec::new();
    request
}

pub(in crate::tests) fn confidential_signing_key_request(
    seed: u8,
) -> super::ConfidentialShareSigningKeyRequest {
    super::ConfidentialShareSigningKeyRequest {
        signing_key_id: signing_key_id(seed),
        participant_index: nonzero(usize::from(seed - 20)),
        public_key: signing_public_key(seed),
    }
}

pub(in crate::tests) fn confidential_signing_key_rotation_request(
    old_seed: u8,
    new_seed: u8,
) -> super::ConfidentialShareSigningKeyRotationRequest {
    super::ConfidentialShareSigningKeyRotationRequest {
        old_signing_key_id: signing_key_id(old_seed),
        new_signing_key_id: signing_key_id(new_seed),
        participant_index: nonzero(usize::from(old_seed - 20)),
        public_key: signing_public_key(new_seed),
    }
}

pub(in crate::tests) fn confidential_signing_key_revocation_request(
    seed: u8,
) -> super::ConfidentialShareSigningKeyRevocationRequest {
    super::ConfidentialShareSigningKeyRevocationRequest {
        signing_key_id: signing_key_id(seed),
    }
}

pub(in crate::tests) fn threshold_release_request() -> super::ConfidentialThresholdReleaseRequest {
    super::ConfidentialThresholdReleaseRequest {
        release_id: transcript(41),
        transcript_id: transcript(1),
        aggregate: field(42),
        left_authority_subject: "policy-authority-left".to_owned(),
        right_authority_subject: "policy-authority-right".to_owned(),
    }
}

pub(in crate::tests) fn threshold_release_approval_request()
-> super::ConfidentialThresholdReleaseApprovalRequest {
    super::ConfidentialThresholdReleaseApprovalRequest {
        release_id: transcript(41),
    }
}

pub(in crate::tests) fn shamir_reconstruction_request()
-> super::ConfidentialShamirThresholdReconstructionRequest {
    let transcript_id = transcript(72);
    super::ConfidentialShamirThresholdReconstructionRequest {
        reconstruction_id: transcript(71),
        transcript_id,
        participant_count: nonzero(5),
        threshold: nonzero(3),
        share_openings: vec![
            verifiable_shamir_share(transcript_id, 1, 1_567, 91),
            verifiable_shamir_share(transcript_id, 2, 2_344, 92),
            verifiable_shamir_share(transcript_id, 3, 3_565, 93),
        ],
    }
}

pub(in crate::tests) fn differential_privacy_budget_request()
-> super::ConfidentialDifferentialPrivacyBudgetRequest {
    super::ConfidentialDifferentialPrivacyBudgetRequest {
        budget_id: transcript(51),
        total_epsilon_microunits: nonzero_u64(1_000_000),
    }
}

pub(in crate::tests) fn differential_privacy_budget_spend_request(
    epsilon_microunits: u64,
) -> super::ConfidentialDifferentialPrivacyBudgetSpendRequest {
    super::ConfidentialDifferentialPrivacyBudgetSpendRequest {
        budget_id: transcript(51),
        epsilon_microunits: nonzero_u64(epsilon_microunits),
    }
}

pub(in crate::tests) fn differential_privacy_release_request()
-> super::ConfidentialDifferentialPrivacyReleaseRequest {
    super::ConfidentialDifferentialPrivacyReleaseRequest {
        release_id: transcript(61),
        budget_id: transcript(51),
        transcript_id: transcript(1),
        aggregate: field(42),
        data_subject_count: nonzero(3),
        minimum_population: nonzero(3),
        epsilon_microunits: nonzero_u64(250_000),
        l1_sensitivity: nonzero_u64(1),
        noise_value: -5,
    }
}

pub(in crate::tests) fn differential_privacy_release_request_with_noise(
    noise_value: i64,
) -> super::ConfidentialDifferentialPrivacyReleaseRequest {
    let mut request = differential_privacy_release_request();
    request.noise_value = noise_value;
    request
}

pub(in crate::tests) fn differential_privacy_release_request_with_minimum_population(
    minimum_population: usize,
) -> super::ConfidentialDifferentialPrivacyReleaseRequest {
    let mut request = differential_privacy_release_request();
    request.release_id = transcript(62);
    request.minimum_population = nonzero(minimum_population);
    request
}

pub(in crate::tests) fn differential_privacy_sampled_release_request()
-> super::ConfidentialDifferentialPrivacySampledReleaseRequest {
    super::ConfidentialDifferentialPrivacySampledReleaseRequest {
        release_id: transcript(63),
        budget_id: transcript(51),
        transcript_id: transcript(1),
        data_subject_count: nonzero(3),
        minimum_population: nonzero(3),
        epsilon_microunits: nonzero_u64(250_000),
        l1_sensitivity: nonzero_u64(1),
    }
}

pub(in crate::tests) fn pds_governance_profile_request(
    profile_id: Uuid,
) -> super::PdsGovernanceProfileRequest {
    super::PdsGovernanceProfileRequest {
        profile_id,
        profile_kind: super::PdsGovernanceProfileKind::SchoolDistrict,
        tenant_partition: "tenant:district-alpha".to_owned(),
        subject_partition: "subject-partition:district-alpha:learners".to_owned(),
        data_custodian_subject: "school-data-custodian".to_owned(),
        platform_operator_subject: "regional-platform-operator".to_owned(),
        policy_authority_subject: "district-policy-authority".to_owned(),
        analyst_subject: "district-analyst".to_owned(),
        auditor_subject: "public-auditor".to_owned(),
        identity_authority_subject: Some("identity-authority".to_owned()),
        emergency_access_policy:
            "emergency-access requires policy-authority approval and auditor review".to_owned(),
        runbook_refs: vec![
            "runbook:key-rotation".to_owned(),
            "runbook:policy-change".to_owned(),
            "runbook:consent-incident".to_owned(),
            "runbook:audit-export".to_owned(),
            "runbook:emergency-access".to_owned(),
        ],
    }
}

pub(in crate::tests) fn pds_uuid(seed: u8) -> Result<Uuid, String> {
    Uuid::parse_str(&format!("00000000-0000-4000-8000-{seed:012x}"))
        .map_err(|error| format!("PDS UUID should parse: {error:?}"))
}

pub(in crate::tests) fn field(value: u64) -> SecureAggregateFieldElement {
    match SecureAggregateFieldElement::new(value) {
        Ok(value) => value,
        Err(error) => panic!("field element should be valid: {error:?}"),
    }
}

pub(in crate::tests) fn nonzero(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).map_or_else(
        || panic!("test nonzero value should be positive"),
        |value| value,
    )
}

pub(in crate::tests) fn nonzero_u64(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).map_or_else(
        || panic!("test nonzero value should be positive"),
        |value| value,
    )
}

pub(in crate::tests) fn sequence(value: u64) -> SecureAggregateEnvelopeSequence {
    match SecureAggregateEnvelopeSequence::try_from(value) {
        Ok(value) => value,
        Err(error) => panic!("sequence should be valid: {error:?}"),
    }
}

pub(in crate::tests) fn replay_window(first: u64, last: u64) -> AdditiveShareReplayWindow {
    match AdditiveShareReplayWindow::new(sequence(first), sequence(last)) {
        Ok(value) => value,
        Err(error) => panic!("replay window should be valid: {error:?}"),
    }
}

pub(in crate::tests) fn artifact(seed: u8) -> [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
    [seed; SECURE_AGGREGATE_ARTIFACT_BYTES]
}

pub(in crate::tests) fn transcript(seed: u8) -> SecureAggregateTranscriptId {
    match SecureAggregateTranscriptId::new(artifact(seed)) {
        Ok(value) => value,
        Err(error) => panic!("transcript id should be valid: {error:?}"),
    }
}

pub(in crate::tests) fn commitment(seed: u8) -> SecureAggregateCommitment {
    match SecureAggregateCommitment::new(artifact(seed)) {
        Ok(value) => value,
        Err(error) => panic!("commitment should be valid: {error:?}"),
    }
}

pub(in crate::tests) fn signing_key_id(seed: u8) -> SecureAggregateSigningKeyId {
    match SecureAggregateSigningKeyId::new(artifact(seed)) {
        Ok(value) => value,
        Err(error) => panic!("signing key id should be valid: {error:?}"),
    }
}

pub(in crate::tests) fn signing_public_key(seed: u8) -> SecureAggregateSigningPublicKey {
    let bytes = signing_key(seed).verifying_key().to_bytes();
    match SecureAggregateSigningPublicKey::new(bytes) {
        Ok(value) => value,
        Err(error) => panic!("signing public key should be valid: {error:?}"),
    }
}

pub(in crate::tests) fn transport_proof(seed: u8) -> SecureAggregateTransportProof {
    match SecureAggregateTransportProof::new(artifact(seed)) {
        Ok(value) => value,
        Err(error) => panic!("transport proof should be valid: {error:?}"),
    }
}

pub(in crate::tests) fn signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&artifact(seed))
}

pub(in crate::tests) fn commitment_slot(
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

pub(in crate::tests) fn authenticated_envelope(
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
        SecureAggregateShare::new(nonzero(participant_index), field(value)),
        commitment(commitment_seed),
        signing_key_id(signing_key_seed),
        sequence(sequence_value),
        transport_proof(proof_seed),
    )
}

pub(in crate::tests) fn shamir_share(participant_index: usize, value: u64) -> ShamirShare {
    ShamirShare::new(nonzero(participant_index), field(value))
}

pub(in crate::tests) fn shamir_opening_nonce(seed: u8) -> ShamirShareOpeningNonce {
    match ShamirShareOpeningNonce::new(artifact(seed)) {
        Ok(value) => value,
        Err(error) => panic!("Shamir opening nonce should be valid: {error:?}"),
    }
}

pub(in crate::tests) fn verifiable_shamir_share(
    transcript_id: SecureAggregateTranscriptId,
    participant_index: usize,
    value: u64,
    nonce_seed: u8,
) -> VerifiableShamirShare {
    let share = shamir_share(participant_index, value);
    let opening_nonce = shamir_opening_nonce(nonce_seed);
    let commitment = derive_shamir_share_opening_commitment(transcript_id, share, opening_nonce)
        .unwrap_or_else(|error| panic!("Shamir share commitment should derive: {error:?}"));
    VerifiableShamirShare::new(share, commitment, opening_nonce)
}

pub(in crate::tests) fn signed_envelope(
    signing_key_seed: u8,
    envelope: AuthenticatedAdditiveShareEnvelope,
) -> SignedAuthenticatedAdditiveShareEnvelope {
    let payload = additive_share_envelope_signature_payload(envelope);
    let signature = signing_key(signing_key_seed).sign(&payload);
    let signature = match SecureAggregateSignature::new(signature.to_bytes()) {
        Ok(value) => value,
        Err(error) => panic!("signature should be valid: {error:?}"),
    };
    SignedAuthenticatedAdditiveShareEnvelope::new(envelope, signature)
}

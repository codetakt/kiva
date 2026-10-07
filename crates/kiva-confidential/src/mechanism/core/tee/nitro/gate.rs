#![allow(clippy::wildcard_imports)]

use super::*;

/// Policy used to admit review-gated AWS Nitro attestation evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeeAwsNitroAttestationPolicy {
    family: TeeAwsNitroAttestationFamily,
    format: TeeAwsNitroAttestationFormat,
    expected_job_id: SecureAggregateTranscriptId,
    expected_pcr0: TeeAwsNitroPcrDigest,
    expected_pcr1: TeeAwsNitroPcrDigest,
    expected_pcr2: TeeAwsNitroPcrDigest,
    expected_pcr3: TeeAwsNitroPcrDigest,
    expected_pcr8: TeeAwsNitroPcrDigest,
    expected_image_digest: SecureAggregateCommitment,
    expected_policy_binding: SecureAggregateCommitment,
    expected_nonce_digest: SecureAggregateCommitment,
    expected_trust_anchor_digest: SecureAggregateCommitment,
    expected_verifier_artifact_digest: SecureAggregateCommitment,
    minimum_evidence_sequence: SecureAggregateEnvelopeSequence,
    minimum_verifier_key_epoch: SecureAggregateEnvelopeSequence,
    current_epoch_seconds: u64,
    policy_approved: bool,
}

impl TeeAwsNitroAttestationPolicy {
    /// Creates an AWS Nitro attestation admission policy.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        family: TeeAwsNitroAttestationFamily,
        format: TeeAwsNitroAttestationFormat,
        expected_job_id: SecureAggregateTranscriptId,
        expected_pcr0: TeeAwsNitroPcrDigest,
        expected_pcr1: TeeAwsNitroPcrDigest,
        expected_pcr2: TeeAwsNitroPcrDigest,
        expected_pcr3: TeeAwsNitroPcrDigest,
        expected_pcr8: TeeAwsNitroPcrDigest,
        expected_image_digest: SecureAggregateCommitment,
        expected_policy_binding: SecureAggregateCommitment,
        expected_nonce_digest: SecureAggregateCommitment,
        expected_trust_anchor_digest: SecureAggregateCommitment,
        expected_verifier_artifact_digest: SecureAggregateCommitment,
        minimum_evidence_sequence: SecureAggregateEnvelopeSequence,
        minimum_verifier_key_epoch: SecureAggregateEnvelopeSequence,
        current_epoch_seconds: u64,
        policy_approved: bool,
    ) -> Self {
        Self {
            family,
            format,
            expected_job_id,
            expected_pcr0,
            expected_pcr1,
            expected_pcr2,
            expected_pcr3,
            expected_pcr8,
            expected_image_digest,
            expected_policy_binding,
            expected_nonce_digest,
            expected_trust_anchor_digest,
            expected_verifier_artifact_digest,
            minimum_evidence_sequence,
            minimum_verifier_key_epoch,
            current_epoch_seconds,
            policy_approved,
        }
    }

    /// Returns the expected family.
    #[must_use]
    pub const fn family(self) -> TeeAwsNitroAttestationFamily {
        self.family
    }

    /// Returns the expected format.
    #[must_use]
    pub const fn format(self) -> TeeAwsNitroAttestationFormat {
        self.format
    }

    /// Returns the expected job identifier.
    #[must_use]
    pub const fn expected_job_id(self) -> SecureAggregateTranscriptId {
        self.expected_job_id
    }

    /// Returns the expected Nitro PCR0 value.
    #[must_use]
    pub const fn expected_pcr0(self) -> TeeAwsNitroPcrDigest {
        self.expected_pcr0
    }

    /// Returns the expected Nitro PCR1 value.
    #[must_use]
    pub const fn expected_pcr1(self) -> TeeAwsNitroPcrDigest {
        self.expected_pcr1
    }

    /// Returns the expected Nitro PCR2 value.
    #[must_use]
    pub const fn expected_pcr2(self) -> TeeAwsNitroPcrDigest {
        self.expected_pcr2
    }

    /// Returns the expected Nitro PCR3 value.
    #[must_use]
    pub const fn expected_pcr3(self) -> TeeAwsNitroPcrDigest {
        self.expected_pcr3
    }

    /// Returns the expected Nitro PCR8 value.
    #[must_use]
    pub const fn expected_pcr8(self) -> TeeAwsNitroPcrDigest {
        self.expected_pcr8
    }

    /// Returns the expected image digest.
    #[must_use]
    pub const fn expected_image_digest(self) -> SecureAggregateCommitment {
        self.expected_image_digest
    }

    /// Returns the expected policy binding.
    #[must_use]
    pub const fn expected_policy_binding(self) -> SecureAggregateCommitment {
        self.expected_policy_binding
    }

    /// Returns the expected nonce or challenge digest.
    #[must_use]
    pub const fn expected_nonce_digest(self) -> SecureAggregateCommitment {
        self.expected_nonce_digest
    }

    /// Returns the expected trust-anchor digest.
    #[must_use]
    pub const fn expected_trust_anchor_digest(self) -> SecureAggregateCommitment {
        self.expected_trust_anchor_digest
    }

    /// Returns the expected verifier adapter artifact digest.
    #[must_use]
    pub const fn expected_verifier_artifact_digest(self) -> SecureAggregateCommitment {
        self.expected_verifier_artifact_digest
    }

    /// Returns the minimum accepted evidence sequence.
    #[must_use]
    pub const fn minimum_evidence_sequence(self) -> SecureAggregateEnvelopeSequence {
        self.minimum_evidence_sequence
    }

    /// Returns the minimum accepted verifier key epoch.
    #[must_use]
    pub const fn minimum_verifier_key_epoch(self) -> SecureAggregateEnvelopeSequence {
        self.minimum_verifier_key_epoch
    }

    /// Returns the policy clock in epoch seconds.
    #[must_use]
    pub const fn current_epoch_seconds(self) -> u64 {
        self.current_epoch_seconds
    }

    /// Returns whether policy authority approved this attested execution.
    #[must_use]
    pub const fn policy_approved(self) -> bool {
        self.policy_approved
    }
}

/// Pure fact bitset evaluated by the AWS Nitro attestation gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeeAwsNitroAttestationGateFacts {
    bits: u32,
}

impl TeeAwsNitroAttestationGateFacts {
    /// Fact bit for policy-authority approval.
    pub const POLICY_APPROVED: u32 = 1 << 0;
    /// Fact bit for supported Nitro family and format.
    pub const FORMAT_SUPPORTED: u32 = 1 << 1;
    /// Fact bit for matching job identity.
    pub const JOB_MATCHES: u32 = 1 << 2;
    /// Fact bit for matching PCR policy.
    pub const PCRS_MATCH: u32 = 1 << 3;
    /// Fact bit for non-debug, non-zero PCR values.
    pub const PCRS_NON_ZERO: u32 = 1 << 4;
    /// Fact bit set when debug mode is reported.
    pub const DEBUG_MODE_REPORTED: u32 = 1 << 5;
    /// Fact bit for matching image provenance digest.
    pub const IMAGE_DIGEST_MATCHES: u32 = 1 << 6;
    /// Fact bit for matching policy binding.
    pub const POLICY_BINDING_MATCHES: u32 = 1 << 7;
    /// Fact bit for matching nonce or challenge digest.
    pub const NONCE_MATCHES: u32 = 1 << 8;
    /// Fact bit for verifier-accepted certificate chain.
    pub const CERTIFICATE_CHAIN_VALID: u32 = 1 << 9;
    /// Fact bit for verifier-accepted COSE signature.
    pub const COSE_SIGNATURE_VALID: u32 = 1 << 10;
    /// Fact bit for matching trust anchor.
    pub const TRUST_ANCHOR_MATCHES: u32 = 1 << 11;
    /// Fact bit for matching verifier adapter artifact.
    pub const VERIFIER_ARTIFACT_MATCHES: u32 = 1 << 12;
    /// Fact bit for completed external security review.
    pub const EXTERNAL_REVIEW_COMPLETED: u32 = 1 << 13;
    /// Fact bit for verifier key epoch at or above the configured minimum.
    pub const VERIFIER_KEY_CURRENT: u32 = 1 << 14;
    /// Fact bit for evidence sequence at or above the configured minimum.
    pub const SEQUENCE_AT_OR_ABOVE_MINIMUM: u32 = 1 << 15;
    /// Fact bit for matching canonical document digest.
    pub const DOCUMENT_DIGEST_MATCHES: u32 = 1 << 16;
    /// Fact bit set when replay is detected.
    pub const REPLAY_DETECTED: u32 = 1 << 17;

    /// Creates a gate fact set from raw bits.
    #[must_use]
    pub const fn from_bits(bits: u32) -> Self {
        Self { bits }
    }

    /// Returns the raw fact bits.
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.bits
    }

    /// Returns whether a fact bit is present.
    #[must_use]
    pub const fn contains(self, bit: u32) -> bool {
        self.bits & bit == bit
    }
}

/// Pure facts and time window evaluated by the AWS Nitro attestation gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeeAwsNitroAttestationGateContext {
    /// Boolean facts encoded as a bitset.
    pub facts: TeeAwsNitroAttestationGateFacts,
    /// Policy clock in epoch seconds.
    pub current_epoch_seconds: u64,
    /// First valid evidence epoch second.
    pub not_before_epoch_seconds: u64,
    /// Last valid evidence epoch second.
    pub not_after_epoch_seconds: u64,
}

/// Replay metadata for one accepted AWS Nitro attestation document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeeAwsNitroAttestationRecord {
    job_id: SecureAggregateTranscriptId,
    evidence_sequence: SecureAggregateEnvelopeSequence,
    document_digest: SecureAggregateCommitment,
}

impl TeeAwsNitroAttestationRecord {
    /// Creates AWS Nitro attestation replay metadata.
    #[must_use]
    pub const fn new(
        job_id: SecureAggregateTranscriptId,
        evidence_sequence: SecureAggregateEnvelopeSequence,
        document_digest: SecureAggregateCommitment,
    ) -> Self {
        Self {
            job_id,
            evidence_sequence,
            document_digest,
        }
    }

    /// Returns the accepted job identifier.
    #[must_use]
    pub const fn job_id(self) -> SecureAggregateTranscriptId {
        self.job_id
    }

    /// Returns the accepted evidence sequence.
    #[must_use]
    pub const fn evidence_sequence(self) -> SecureAggregateEnvelopeSequence {
        self.evidence_sequence
    }

    /// Returns the accepted document digest.
    #[must_use]
    pub const fn document_digest(self) -> SecureAggregateCommitment {
        self.document_digest
    }

    /// Returns whether this record conflicts with a candidate attestation document.
    #[must_use]
    pub fn conflicts_with(
        self,
        job_id: SecureAggregateTranscriptId,
        evidence_sequence: SecureAggregateEnvelopeSequence,
        document_digest: SecureAggregateCommitment,
    ) -> bool {
        (self.job_id == job_id && self.evidence_sequence == evidence_sequence)
            || self.document_digest == document_digest
    }
}

/// Metadata-only result of accepting AWS Nitro attestation evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptedTeeAwsNitroAttestation {
    record: TeeAwsNitroAttestationRecord,
    pcr0: TeeAwsNitroPcrDigest,
    pcr1: TeeAwsNitroPcrDigest,
    pcr2: TeeAwsNitroPcrDigest,
    pcr3: TeeAwsNitroPcrDigest,
    pcr8: TeeAwsNitroPcrDigest,
    image_digest: SecureAggregateCommitment,
    policy_binding: SecureAggregateCommitment,
    nonce_digest: SecureAggregateCommitment,
    trust_anchor_digest: SecureAggregateCommitment,
    verifier_artifact_digest: SecureAggregateCommitment,
    verifier_key_epoch: SecureAggregateEnvelopeSequence,
    accepted_at_epoch_seconds: u64,
}

impl AcceptedTeeAwsNitroAttestation {
    /// Creates accepted AWS Nitro attestation metadata.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        record: TeeAwsNitroAttestationRecord,
        pcr0: TeeAwsNitroPcrDigest,
        pcr1: TeeAwsNitroPcrDigest,
        pcr2: TeeAwsNitroPcrDigest,
        pcr3: TeeAwsNitroPcrDigest,
        pcr8: TeeAwsNitroPcrDigest,
        image_digest: SecureAggregateCommitment,
        policy_binding: SecureAggregateCommitment,
        nonce_digest: SecureAggregateCommitment,
        trust_anchor_digest: SecureAggregateCommitment,
        verifier_artifact_digest: SecureAggregateCommitment,
        verifier_key_epoch: SecureAggregateEnvelopeSequence,
        accepted_at_epoch_seconds: u64,
    ) -> Self {
        Self {
            record,
            pcr0,
            pcr1,
            pcr2,
            pcr3,
            pcr8,
            image_digest,
            policy_binding,
            nonce_digest,
            trust_anchor_digest,
            verifier_artifact_digest,
            verifier_key_epoch,
            accepted_at_epoch_seconds,
        }
    }

    /// Returns the replay record that must be persisted.
    #[must_use]
    pub const fn record(self) -> TeeAwsNitroAttestationRecord {
        self.record
    }

    /// Returns accepted PCR0.
    #[must_use]
    pub const fn pcr0(self) -> TeeAwsNitroPcrDigest {
        self.pcr0
    }

    /// Returns accepted PCR1.
    #[must_use]
    pub const fn pcr1(self) -> TeeAwsNitroPcrDigest {
        self.pcr1
    }

    /// Returns accepted PCR2.
    #[must_use]
    pub const fn pcr2(self) -> TeeAwsNitroPcrDigest {
        self.pcr2
    }

    /// Returns accepted PCR3.
    #[must_use]
    pub const fn pcr3(self) -> TeeAwsNitroPcrDigest {
        self.pcr3
    }

    /// Returns accepted PCR8.
    #[must_use]
    pub const fn pcr8(self) -> TeeAwsNitroPcrDigest {
        self.pcr8
    }

    /// Returns the accepted image provenance digest.
    #[must_use]
    pub const fn image_digest(self) -> SecureAggregateCommitment {
        self.image_digest
    }

    /// Returns the accepted policy binding.
    #[must_use]
    pub const fn policy_binding(self) -> SecureAggregateCommitment {
        self.policy_binding
    }

    /// Returns the accepted nonce digest.
    #[must_use]
    pub const fn nonce_digest(self) -> SecureAggregateCommitment {
        self.nonce_digest
    }

    /// Returns the accepted trust-anchor digest.
    #[must_use]
    pub const fn trust_anchor_digest(self) -> SecureAggregateCommitment {
        self.trust_anchor_digest
    }

    /// Returns the accepted verifier adapter artifact digest.
    #[must_use]
    pub const fn verifier_artifact_digest(self) -> SecureAggregateCommitment {
        self.verifier_artifact_digest
    }

    /// Returns the accepted verifier signing-key epoch.
    #[must_use]
    pub const fn verifier_key_epoch(self) -> SecureAggregateEnvelopeSequence {
        self.verifier_key_epoch
    }

    /// Returns the policy-clock timestamp at acceptance.
    #[must_use]
    pub const fn accepted_at_epoch_seconds(self) -> u64 {
        self.accepted_at_epoch_seconds
    }
}

/// Evaluates the pure fail-closed AWS Nitro attestation gate facts.
pub const fn validate_tee_aws_nitro_attestation_gate_context(
    context: TeeAwsNitroAttestationGateContext,
) -> Result<(), SecureAggregateError> {
    let facts = context.facts;
    if !facts.contains(TeeAwsNitroAttestationGateFacts::POLICY_APPROVED) {
        return Err(SecureAggregateError::TeePolicyApprovalMissing);
    }
    if !facts.contains(TeeAwsNitroAttestationGateFacts::FORMAT_SUPPORTED) {
        return Err(SecureAggregateError::TeeUnsupportedAttestationFormat);
    }
    if !facts.contains(TeeAwsNitroAttestationGateFacts::JOB_MATCHES) {
        return Err(SecureAggregateError::TeeJobBindingMismatch);
    }
    if facts.contains(TeeAwsNitroAttestationGateFacts::DEBUG_MODE_REPORTED) {
        return Err(SecureAggregateError::TeeNitroDebugModeRejected);
    }
    if !facts.contains(TeeAwsNitroAttestationGateFacts::PCRS_NON_ZERO) {
        return Err(SecureAggregateError::TeeNitroPcrAllZeroRejected);
    }
    if !facts.contains(TeeAwsNitroAttestationGateFacts::PCRS_MATCH) {
        return Err(SecureAggregateError::TeeNitroPcrMismatch);
    }
    if !facts.contains(TeeAwsNitroAttestationGateFacts::IMAGE_DIGEST_MATCHES) {
        return Err(SecureAggregateError::TeeImageDigestMismatch);
    }
    if !facts.contains(TeeAwsNitroAttestationGateFacts::POLICY_BINDING_MATCHES) {
        return Err(SecureAggregateError::TeePolicyBindingMismatch);
    }
    if !facts.contains(TeeAwsNitroAttestationGateFacts::NONCE_MATCHES) {
        return Err(SecureAggregateError::TeeNitroNonceMismatch);
    }
    if context.current_epoch_seconds < context.not_before_epoch_seconds {
        return Err(SecureAggregateError::TeeEvidenceNotYetValid);
    }
    if context.current_epoch_seconds > context.not_after_epoch_seconds {
        return Err(SecureAggregateError::TeeEvidenceExpired);
    }
    if !facts.contains(TeeAwsNitroAttestationGateFacts::CERTIFICATE_CHAIN_VALID) {
        return Err(SecureAggregateError::TeeNitroCertificateChainInvalid);
    }
    if !facts.contains(TeeAwsNitroAttestationGateFacts::COSE_SIGNATURE_VALID) {
        return Err(SecureAggregateError::TeeNitroCoseSignatureInvalid);
    }
    if !facts.contains(TeeAwsNitroAttestationGateFacts::TRUST_ANCHOR_MATCHES) {
        return Err(SecureAggregateError::TeeNitroTrustAnchorMismatch);
    }
    if !facts.contains(TeeAwsNitroAttestationGateFacts::VERIFIER_ARTIFACT_MATCHES) {
        return Err(SecureAggregateError::TeeNitroVerifierBindingMissing);
    }
    if !facts.contains(TeeAwsNitroAttestationGateFacts::EXTERNAL_REVIEW_COMPLETED) {
        return Err(SecureAggregateError::TeeNitroExternalReviewRequired);
    }
    if !facts.contains(TeeAwsNitroAttestationGateFacts::VERIFIER_KEY_CURRENT) {
        return Err(SecureAggregateError::TeeNitroVerifierKeyRotationRequired);
    }
    if !facts.contains(TeeAwsNitroAttestationGateFacts::SEQUENCE_AT_OR_ABOVE_MINIMUM) {
        return Err(SecureAggregateError::TeeRollbackDetected);
    }
    if !facts.contains(TeeAwsNitroAttestationGateFacts::DOCUMENT_DIGEST_MATCHES) {
        return Err(SecureAggregateError::TeeEvidenceDigestMismatch);
    }
    if facts.contains(TeeAwsNitroAttestationGateFacts::REPLAY_DETECTED) {
        return Err(SecureAggregateError::TeeReplayDetected);
    }
    Ok(())
}

/// Verifies review-gated AWS Nitro evidence against a deployment policy.
///
/// Kiva accepts only claims that an external Nitro verifier adapter has already
/// normalized after certificate-chain, COSE-signature, trust-anchor, and PCR
/// checks. This function binds those verified claims to Kiva policy, replay
/// storage, verifier provenance, key rotation, and release-gating metadata.
pub fn verify_tee_aws_nitro_attestation_evidence(
    policy: TeeAwsNitroAttestationPolicy,
    existing_records: &[TeeAwsNitroAttestationRecord],
    evidence: TeeAwsNitroAttestationEvidence,
) -> Result<AcceptedTeeAwsNitroAttestation, SecureAggregateError> {
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
    if claims.debug_mode() {
        return Err(SecureAggregateError::TeeNitroDebugModeRejected);
    }
    if claims.has_all_zero_pcr() {
        return Err(SecureAggregateError::TeeNitroPcrAllZeroRejected);
    }
    if claims.pcr0() != policy.expected_pcr0()
        || claims.pcr1() != policy.expected_pcr1()
        || claims.pcr2() != policy.expected_pcr2()
        || claims.pcr3() != policy.expected_pcr3()
        || claims.pcr8() != policy.expected_pcr8()
    {
        return Err(SecureAggregateError::TeeNitroPcrMismatch);
    }
    if claims.image_digest() != policy.expected_image_digest() {
        return Err(SecureAggregateError::TeeImageDigestMismatch);
    }
    if claims.policy_binding() != policy.expected_policy_binding() {
        return Err(SecureAggregateError::TeePolicyBindingMismatch);
    }
    if claims.nonce_digest() != policy.expected_nonce_digest() {
        return Err(SecureAggregateError::TeeNitroNonceMismatch);
    }
    if policy.current_epoch_seconds() < claims.not_before_epoch_seconds() {
        return Err(SecureAggregateError::TeeEvidenceNotYetValid);
    }
    if policy.current_epoch_seconds() > claims.not_after_epoch_seconds() {
        return Err(SecureAggregateError::TeeEvidenceExpired);
    }
    if !evidence.certificate_chain_valid() {
        return Err(SecureAggregateError::TeeNitroCertificateChainInvalid);
    }
    if !evidence.cose_signature_valid() {
        return Err(SecureAggregateError::TeeNitroCoseSignatureInvalid);
    }

    let binding = evidence.verifier_binding();
    if binding.trust_anchor_digest() != policy.expected_trust_anchor_digest() {
        return Err(SecureAggregateError::TeeNitroTrustAnchorMismatch);
    }
    if binding.verifier_artifact_digest() != policy.expected_verifier_artifact_digest() {
        return Err(SecureAggregateError::TeeNitroVerifierBindingMissing);
    }
    if !binding.external_security_review_completed() {
        return Err(SecureAggregateError::TeeNitroExternalReviewRequired);
    }
    if binding.verifier_key_epoch() < policy.minimum_verifier_key_epoch() {
        return Err(SecureAggregateError::TeeNitroVerifierKeyRotationRequired);
    }
    if claims.evidence_sequence() < policy.minimum_evidence_sequence() {
        return Err(SecureAggregateError::TeeRollbackDetected);
    }
    if derive_tee_aws_nitro_attestation_document_digest(claims)? != evidence.document_digest() {
        return Err(SecureAggregateError::TeeEvidenceDigestMismatch);
    }
    if existing_records.iter().any(|record| {
        record.conflicts_with(
            claims.job_id(),
            claims.evidence_sequence(),
            evidence.document_digest(),
        )
    }) {
        return Err(SecureAggregateError::TeeReplayDetected);
    }

    let record = TeeAwsNitroAttestationRecord::new(
        claims.job_id(),
        claims.evidence_sequence(),
        evidence.document_digest(),
    );
    Ok(AcceptedTeeAwsNitroAttestation::new(
        record,
        claims.pcr0(),
        claims.pcr1(),
        claims.pcr2(),
        claims.pcr3(),
        claims.pcr8(),
        claims.image_digest(),
        claims.policy_binding(),
        claims.nonce_digest(),
        binding.trust_anchor_digest(),
        binding.verifier_artifact_digest(),
        binding.verifier_key_epoch(),
        policy.current_epoch_seconds(),
    ))
}

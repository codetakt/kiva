#![allow(clippy::wildcard_imports)]

use super::*;

/// Policy used to admit portable TEE attestation evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeeAttestationPolicy {
    family: TeeAttestationFamily,
    format: TeeAttestationFormat,
    expected_job_id: SecureAggregateTranscriptId,
    expected_measurement: SecureAggregateCommitment,
    expected_image_digest: SecureAggregateCommitment,
    expected_policy_binding: SecureAggregateCommitment,
    minimum_evidence_sequence: SecureAggregateEnvelopeSequence,
    current_epoch_seconds: u64,
    policy_approved: bool,
}

impl TeeAttestationPolicy {
    /// Creates a TEE attestation admission policy.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        family: TeeAttestationFamily,
        format: TeeAttestationFormat,
        expected_job_id: SecureAggregateTranscriptId,
        expected_measurement: SecureAggregateCommitment,
        expected_image_digest: SecureAggregateCommitment,
        expected_policy_binding: SecureAggregateCommitment,
        minimum_evidence_sequence: SecureAggregateEnvelopeSequence,
        current_epoch_seconds: u64,
        policy_approved: bool,
    ) -> Self {
        Self {
            family,
            format,
            expected_job_id,
            expected_measurement,
            expected_image_digest,
            expected_policy_binding,
            minimum_evidence_sequence,
            current_epoch_seconds,
            policy_approved,
        }
    }

    /// Returns the expected attestation family.
    #[must_use]
    pub const fn family(self) -> TeeAttestationFamily {
        self.family
    }

    /// Returns the expected attestation format.
    #[must_use]
    pub const fn format(self) -> TeeAttestationFormat {
        self.format
    }

    /// Returns the expected job identifier.
    #[must_use]
    pub const fn expected_job_id(self) -> SecureAggregateTranscriptId {
        self.expected_job_id
    }

    /// Returns the expected measurement.
    #[must_use]
    pub const fn expected_measurement(self) -> SecureAggregateCommitment {
        self.expected_measurement
    }

    /// Returns the expected image provenance digest.
    #[must_use]
    pub const fn expected_image_digest(self) -> SecureAggregateCommitment {
        self.expected_image_digest
    }

    /// Returns the expected policy binding.
    #[must_use]
    pub const fn expected_policy_binding(self) -> SecureAggregateCommitment {
        self.expected_policy_binding
    }

    /// Returns the minimum accepted evidence sequence.
    #[must_use]
    pub const fn minimum_evidence_sequence(self) -> SecureAggregateEnvelopeSequence {
        self.minimum_evidence_sequence
    }

    /// Returns the policy clock in epoch seconds.
    #[must_use]
    pub const fn current_epoch_seconds(self) -> u64 {
        self.current_epoch_seconds
    }

    /// Returns whether the policy authority approved this attested execution.
    #[must_use]
    pub const fn policy_approved(self) -> bool {
        self.policy_approved
    }
}

/// Pure fact bitset evaluated by the TEE attestation gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeeAttestationGateFacts {
    bits: u16,
}

impl TeeAttestationGateFacts {
    /// Fact bit for policy-authority approval.
    pub const POLICY_APPROVED: u16 = 1 << 0;
    /// Fact bit for supported evidence family and format.
    pub const FORMAT_SUPPORTED: u16 = 1 << 1;
    /// Fact bit for matching job identity.
    pub const JOB_MATCHES: u16 = 1 << 2;
    /// Fact bit for matching measurement.
    pub const MEASUREMENT_MATCHES: u16 = 1 << 3;
    /// Fact bit for matching image provenance digest.
    pub const IMAGE_DIGEST_MATCHES: u16 = 1 << 4;
    /// Fact bit for matching policy binding.
    pub const POLICY_BINDING_MATCHES: u16 = 1 << 5;
    /// Fact bit for evidence sequence at or above the configured minimum.
    pub const SEQUENCE_AT_OR_ABOVE_MINIMUM: u16 = 1 << 6;
    /// Fact bit for matching canonical evidence digest.
    pub const EVIDENCE_DIGEST_MATCHES: u16 = 1 << 7;
    /// Fact bit set when replay is detected.
    pub const REPLAY_DETECTED: u16 = 1 << 8;

    /// Creates a gate fact set from raw bits.
    #[must_use]
    pub const fn from_bits(bits: u16) -> Self {
        Self { bits }
    }

    /// Returns the raw fact bits.
    #[must_use]
    pub const fn bits(self) -> u16 {
        self.bits
    }

    /// Returns whether a fact bit is present.
    #[must_use]
    pub const fn contains(self, bit: u16) -> bool {
        self.bits & bit == bit
    }
}

/// Pure facts and time window evaluated by the TEE attestation gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeeAttestationGateContext {
    /// Boolean facts encoded as a bitset.
    pub facts: TeeAttestationGateFacts,
    /// Policy clock in epoch seconds.
    pub current_epoch_seconds: u64,
    /// First valid evidence epoch second.
    pub not_before_epoch_seconds: u64,
    /// Last valid evidence epoch second.
    pub not_after_epoch_seconds: u64,
}

/// Replay metadata for one accepted TEE attestation evidence envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeeAttestationRecord {
    job_id: SecureAggregateTranscriptId,
    evidence_sequence: SecureAggregateEnvelopeSequence,
    evidence_digest: SecureAggregateCommitment,
}

impl TeeAttestationRecord {
    /// Creates a TEE attestation replay record.
    #[must_use]
    pub const fn new(
        job_id: SecureAggregateTranscriptId,
        evidence_sequence: SecureAggregateEnvelopeSequence,
        evidence_digest: SecureAggregateCommitment,
    ) -> Self {
        Self {
            job_id,
            evidence_sequence,
            evidence_digest,
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

    /// Returns the accepted evidence digest.
    #[must_use]
    pub const fn evidence_digest(self) -> SecureAggregateCommitment {
        self.evidence_digest
    }

    /// Returns whether this record conflicts with a new attestation evidence.
    #[must_use]
    pub fn conflicts_with(
        self,
        job_id: SecureAggregateTranscriptId,
        evidence_sequence: SecureAggregateEnvelopeSequence,
        evidence_digest: SecureAggregateCommitment,
    ) -> bool {
        (self.job_id == job_id && self.evidence_sequence == evidence_sequence)
            || self.evidence_digest == evidence_digest
    }
}

/// Metadata-only result of accepting TEE attestation evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptedTeeAttestation {
    record: TeeAttestationRecord,
    family: TeeAttestationFamily,
    format: TeeAttestationFormat,
    measurement: SecureAggregateCommitment,
    image_digest: SecureAggregateCommitment,
    policy_binding: SecureAggregateCommitment,
    accepted_at_epoch_seconds: u64,
}

impl AcceptedTeeAttestation {
    /// Creates accepted TEE attestation metadata.
    #[must_use]
    pub const fn new(
        record: TeeAttestationRecord,
        family: TeeAttestationFamily,
        format: TeeAttestationFormat,
        measurement: SecureAggregateCommitment,
        image_digest: SecureAggregateCommitment,
        policy_binding: SecureAggregateCommitment,
        accepted_at_epoch_seconds: u64,
    ) -> Self {
        Self {
            record,
            family,
            format,
            measurement,
            image_digest,
            policy_binding,
            accepted_at_epoch_seconds,
        }
    }

    /// Returns the replay record that must be persisted.
    #[must_use]
    pub const fn record(self) -> TeeAttestationRecord {
        self.record
    }

    /// Returns the accepted attestation family.
    #[must_use]
    pub const fn family(self) -> TeeAttestationFamily {
        self.family
    }

    /// Returns the accepted attestation format.
    #[must_use]
    pub const fn format(self) -> TeeAttestationFormat {
        self.format
    }

    /// Returns the accepted measurement for audit metadata.
    #[must_use]
    pub const fn measurement(self) -> SecureAggregateCommitment {
        self.measurement
    }

    /// Returns the accepted image provenance digest for audit metadata.
    #[must_use]
    pub const fn image_digest(self) -> SecureAggregateCommitment {
        self.image_digest
    }

    /// Returns the accepted policy binding for audit metadata.
    #[must_use]
    pub const fn policy_binding(self) -> SecureAggregateCommitment {
        self.policy_binding
    }

    /// Returns the policy-clock timestamp at acceptance.
    #[must_use]
    pub const fn accepted_at_epoch_seconds(self) -> u64 {
        self.accepted_at_epoch_seconds
    }
}

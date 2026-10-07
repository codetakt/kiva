#![allow(clippy::wildcard_imports)]

use super::*;

/// Supported TEE attestation family for the current CC5 boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeeAttestationFamily {
    /// Deployment-bound portable evidence envelope, not a vendor quote format.
    DeploymentBoundPortable,
}

impl TeeAttestationFamily {
    /// Returns the stable profile family label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::DeploymentBoundPortable => "deployment-bound-portable",
        }
    }
}

/// Supported TEE attestation evidence format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeeAttestationFormat {
    /// Kiva portable attestation evidence envelope version 1.
    PortableEvidenceV1,
}

impl TeeAttestationFormat {
    /// Returns the stable evidence format label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::PortableEvidenceV1 => "portable-evidence-v1",
        }
    }
}

/// Canonical claims carried by portable TEE attestation evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "TeeAttestationClaimsUnchecked",
    into = "TeeAttestationClaimsUnchecked"
)]
pub struct TeeAttestationClaims {
    pub(in crate::mechanism::core) family: TeeAttestationFamily,
    pub(in crate::mechanism::core) format: TeeAttestationFormat,
    pub(in crate::mechanism::core) job_id: SecureAggregateTranscriptId,
    pub(in crate::mechanism::core) measurement: SecureAggregateCommitment,
    pub(in crate::mechanism::core) image_digest: SecureAggregateCommitment,
    pub(in crate::mechanism::core) policy_binding: SecureAggregateCommitment,
    pub(in crate::mechanism::core) not_before_epoch_seconds: u64,
    pub(in crate::mechanism::core) not_after_epoch_seconds: u64,
    pub(in crate::mechanism::core) evidence_sequence: SecureAggregateEnvelopeSequence,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct TeeAttestationClaimsUnchecked {
    family: TeeAttestationFamily,
    format: TeeAttestationFormat,
    job_id: SecureAggregateTranscriptId,
    measurement: SecureAggregateCommitment,
    image_digest: SecureAggregateCommitment,
    policy_binding: SecureAggregateCommitment,
    not_before_epoch_seconds: u64,
    not_after_epoch_seconds: u64,
    evidence_sequence: SecureAggregateEnvelopeSequence,
}

impl TeeAttestationClaims {
    /// Creates TEE attestation claims after validating the time window.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        family: TeeAttestationFamily,
        format: TeeAttestationFormat,
        job_id: SecureAggregateTranscriptId,
        measurement: SecureAggregateCommitment,
        image_digest: SecureAggregateCommitment,
        policy_binding: SecureAggregateCommitment,
        not_before_epoch_seconds: u64,
        not_after_epoch_seconds: u64,
        evidence_sequence: SecureAggregateEnvelopeSequence,
    ) -> Result<Self, SecureAggregateError> {
        if not_after_epoch_seconds < not_before_epoch_seconds {
            return Err(SecureAggregateError::TeeEvidenceWindowInvalid);
        }
        Ok(Self {
            family,
            format,
            job_id,
            measurement,
            image_digest,
            policy_binding,
            not_before_epoch_seconds,
            not_after_epoch_seconds,
            evidence_sequence,
        })
    }

    /// Returns the attestation family.
    #[must_use]
    pub const fn family(self) -> TeeAttestationFamily {
        self.family
    }

    /// Returns the evidence format.
    #[must_use]
    pub const fn format(self) -> TeeAttestationFormat {
        self.format
    }

    /// Returns the attested job identifier.
    #[must_use]
    pub const fn job_id(self) -> SecureAggregateTranscriptId {
        self.job_id
    }

    /// Returns the attested execution measurement.
    #[must_use]
    pub const fn measurement(self) -> SecureAggregateCommitment {
        self.measurement
    }

    /// Returns the attested image provenance digest.
    #[must_use]
    pub const fn image_digest(self) -> SecureAggregateCommitment {
        self.image_digest
    }

    /// Returns the attested policy binding.
    #[must_use]
    pub const fn policy_binding(self) -> SecureAggregateCommitment {
        self.policy_binding
    }

    /// Returns the first valid epoch-second timestamp.
    #[must_use]
    pub const fn not_before_epoch_seconds(self) -> u64 {
        self.not_before_epoch_seconds
    }

    /// Returns the last valid epoch-second timestamp.
    #[must_use]
    pub const fn not_after_epoch_seconds(self) -> u64 {
        self.not_after_epoch_seconds
    }

    /// Returns the monotonic evidence sequence.
    #[must_use]
    pub const fn evidence_sequence(self) -> SecureAggregateEnvelopeSequence {
        self.evidence_sequence
    }
}

impl TryFrom<TeeAttestationClaimsUnchecked> for TeeAttestationClaims {
    type Error = SecureAggregateError;

    fn try_from(value: TeeAttestationClaimsUnchecked) -> Result<Self, Self::Error> {
        Self::new(
            value.family,
            value.format,
            value.job_id,
            value.measurement,
            value.image_digest,
            value.policy_binding,
            value.not_before_epoch_seconds,
            value.not_after_epoch_seconds,
            value.evidence_sequence,
        )
    }
}

impl From<TeeAttestationClaims> for TeeAttestationClaimsUnchecked {
    fn from(value: TeeAttestationClaims) -> Self {
        Self {
            family: value.family(),
            format: value.format(),
            job_id: value.job_id(),
            measurement: value.measurement(),
            image_digest: value.image_digest(),
            policy_binding: value.policy_binding(),
            not_before_epoch_seconds: value.not_before_epoch_seconds(),
            not_after_epoch_seconds: value.not_after_epoch_seconds(),
            evidence_sequence: value.evidence_sequence(),
        }
    }
}

#![allow(clippy::wildcard_imports)]

use super::*;

/// AWS Nitro TEE family admitted by the CC12 review-gated boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeeAwsNitroAttestationFamily {
    /// AWS Nitro Enclaves attestation documents verified by an external adapter.
    AwsNitroEnclaves,
}

impl TeeAwsNitroAttestationFamily {
    /// Returns the stable family label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::AwsNitroEnclaves => "aws-nitro-enclaves",
        }
    }
}

/// Verified AWS Nitro attestation document format admitted by Kiva.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeeAwsNitroAttestationFormat {
    /// Claims emitted after an external adapter verifies a Nitro attestation document.
    VerifiedNitroAttestationDocumentV1,
}

impl TeeAwsNitroAttestationFormat {
    /// Returns the stable format label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::VerifiedNitroAttestationDocumentV1 => "verified-nitro-attestation-document-v1",
        }
    }
}

/// Raw 32-byte AWS Nitro PCR digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeeAwsNitroPcrDigest {
    bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES],
}

impl TeeAwsNitroPcrDigest {
    /// Creates a PCR digest. All-zero values are representable so debug mode can be rejected.
    #[must_use]
    pub const fn new(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Self {
        Self { bytes }
    }

    /// Returns the canonical PCR bytes.
    #[must_use]
    pub const fn bytes(self) -> [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
        self.bytes
    }

    /// Returns whether the PCR is all zero.
    #[must_use]
    pub const fn is_all_zero(self) -> bool {
        let mut index = 0;
        while index < SECURE_AGGREGATE_ARTIFACT_BYTES {
            if self.bytes[index] != 0 {
                return false;
            }
            index += 1;
        }
        true
    }
}

impl From<[u8; SECURE_AGGREGATE_ARTIFACT_BYTES]> for TeeAwsNitroPcrDigest {
    fn from(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Self {
        Self::new(bytes)
    }
}

impl From<TeeAwsNitroPcrDigest> for [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
    fn from(value: TeeAwsNitroPcrDigest) -> Self {
        value.bytes()
    }
}

/// Binding metadata for the external AWS Nitro verifier adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeeAwsNitroVerifierBinding {
    verifier_artifact_digest: SecureAggregateCommitment,
    trust_anchor_digest: SecureAggregateCommitment,
    external_review_evidence_digest: SecureAggregateCommitment,
    verifier_key_epoch: SecureAggregateEnvelopeSequence,
    external_security_review_completed: bool,
}

impl TeeAwsNitroVerifierBinding {
    /// Creates verifier binding metadata for an external Nitro adapter.
    #[must_use]
    pub const fn new(
        verifier_artifact_digest: SecureAggregateCommitment,
        trust_anchor_digest: SecureAggregateCommitment,
        external_review_evidence_digest: SecureAggregateCommitment,
        verifier_key_epoch: SecureAggregateEnvelopeSequence,
        external_security_review_completed: bool,
    ) -> Self {
        Self {
            verifier_artifact_digest,
            trust_anchor_digest,
            external_review_evidence_digest,
            verifier_key_epoch,
            external_security_review_completed,
        }
    }

    /// Returns the verifier adapter artifact digest.
    #[must_use]
    pub const fn verifier_artifact_digest(self) -> SecureAggregateCommitment {
        self.verifier_artifact_digest
    }

    /// Returns the configured AWS Nitro trust-anchor digest.
    #[must_use]
    pub const fn trust_anchor_digest(self) -> SecureAggregateCommitment {
        self.trust_anchor_digest
    }

    /// Returns the external security-review evidence digest.
    #[must_use]
    pub const fn external_review_evidence_digest(self) -> SecureAggregateCommitment {
        self.external_review_evidence_digest
    }

    /// Returns the verifier signing-key epoch.
    #[must_use]
    pub const fn verifier_key_epoch(self) -> SecureAggregateEnvelopeSequence {
        self.verifier_key_epoch
    }

    /// Returns whether the external security review has completed.
    #[must_use]
    pub const fn external_security_review_completed(self) -> bool {
        self.external_security_review_completed
    }
}

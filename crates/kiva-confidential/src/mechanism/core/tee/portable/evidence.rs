#![allow(clippy::wildcard_imports)]

use super::*;

/// Portable TEE attestation evidence with a canonical claims digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeeAttestationEvidence {
    claims: TeeAttestationClaims,
    evidence_digest: SecureAggregateCommitment,
}

impl TeeAttestationEvidence {
    /// Creates portable TEE evidence and derives its canonical digest.
    pub fn new(claims: TeeAttestationClaims) -> Result<Self, SecureAggregateError> {
        Ok(Self {
            claims,
            evidence_digest: derive_tee_attestation_evidence_digest(claims)?,
        })
    }

    /// Creates evidence from explicit parts for repository boundary validation.
    #[must_use]
    pub const fn from_parts(
        claims: TeeAttestationClaims,
        evidence_digest: SecureAggregateCommitment,
    ) -> Self {
        Self {
            claims,
            evidence_digest,
        }
    }

    /// Returns the attestation claims.
    #[must_use]
    pub const fn claims(self) -> TeeAttestationClaims {
        self.claims
    }

    /// Returns the canonical evidence digest.
    #[must_use]
    pub const fn evidence_digest(self) -> SecureAggregateCommitment {
        self.evidence_digest
    }
}

/// Derives the canonical digest over portable TEE attestation claims.
pub fn derive_tee_attestation_evidence_digest(
    claims: TeeAttestationClaims,
) -> Result<SecureAggregateCommitment, SecureAggregateError> {
    let mut hasher = Sha256::new();
    hasher.update(TEE_ATTESTATION_EVIDENCE_DOMAIN);
    update_len_prefixed(&mut hasher, TEE_PORTABLE_ATTESTATION_PROFILE.as_bytes());
    update_len_prefixed(&mut hasher, claims.family().label().as_bytes());
    update_len_prefixed(&mut hasher, claims.format().label().as_bytes());
    hasher.update(claims.job_id().bytes());
    hasher.update(claims.measurement().bytes());
    hasher.update(claims.image_digest().bytes());
    hasher.update(claims.policy_binding().bytes());
    hasher.update(claims.not_before_epoch_seconds().to_be_bytes());
    hasher.update(claims.not_after_epoch_seconds().to_be_bytes());
    hasher.update(claims.evidence_sequence().value().get().to_be_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES];
    bytes.copy_from_slice(&digest);
    SecureAggregateCommitment::new(bytes)
}

#![allow(clippy::wildcard_imports)]

use super::*;

/// Canonical claims emitted by a reviewed AWS Nitro verifier adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "TeeAwsNitroAttestationClaimsUnchecked",
    into = "TeeAwsNitroAttestationClaimsUnchecked"
)]
pub struct TeeAwsNitroAttestationClaims {
    pub(in crate::mechanism::core) family: TeeAwsNitroAttestationFamily,
    pub(in crate::mechanism::core) format: TeeAwsNitroAttestationFormat,
    pub(in crate::mechanism::core) job_id: SecureAggregateTranscriptId,
    pub(in crate::mechanism::core) pcr0: TeeAwsNitroPcrDigest,
    pub(in crate::mechanism::core) pcr1: TeeAwsNitroPcrDigest,
    pub(in crate::mechanism::core) pcr2: TeeAwsNitroPcrDigest,
    pub(in crate::mechanism::core) pcr3: TeeAwsNitroPcrDigest,
    pub(in crate::mechanism::core) pcr8: TeeAwsNitroPcrDigest,
    pub(in crate::mechanism::core) image_digest: SecureAggregateCommitment,
    pub(in crate::mechanism::core) policy_binding: SecureAggregateCommitment,
    pub(in crate::mechanism::core) nonce_digest: SecureAggregateCommitment,
    pub(in crate::mechanism::core) certificate_chain_digest: SecureAggregateCommitment,
    pub(in crate::mechanism::core) cose_signing_key_digest: SecureAggregateCommitment,
    pub(in crate::mechanism::core) not_before_epoch_seconds: u64,
    pub(in crate::mechanism::core) not_after_epoch_seconds: u64,
    pub(in crate::mechanism::core) evidence_sequence: SecureAggregateEnvelopeSequence,
    pub(in crate::mechanism::core) debug_mode: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct TeeAwsNitroAttestationClaimsUnchecked {
    family: TeeAwsNitroAttestationFamily,
    format: TeeAwsNitroAttestationFormat,
    job_id: SecureAggregateTranscriptId,
    pcr0: TeeAwsNitroPcrDigest,
    pcr1: TeeAwsNitroPcrDigest,
    pcr2: TeeAwsNitroPcrDigest,
    pcr3: TeeAwsNitroPcrDigest,
    pcr8: TeeAwsNitroPcrDigest,
    image_digest: SecureAggregateCommitment,
    policy_binding: SecureAggregateCommitment,
    nonce_digest: SecureAggregateCommitment,
    certificate_chain_digest: SecureAggregateCommitment,
    cose_signing_key_digest: SecureAggregateCommitment,
    not_before_epoch_seconds: u64,
    not_after_epoch_seconds: u64,
    evidence_sequence: SecureAggregateEnvelopeSequence,
    debug_mode: bool,
}

impl TeeAwsNitroAttestationClaims {
    /// Creates verified Nitro claims after validating the time window.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        family: TeeAwsNitroAttestationFamily,
        format: TeeAwsNitroAttestationFormat,
        job_id: SecureAggregateTranscriptId,
        pcr0: TeeAwsNitroPcrDigest,
        pcr1: TeeAwsNitroPcrDigest,
        pcr2: TeeAwsNitroPcrDigest,
        pcr3: TeeAwsNitroPcrDigest,
        pcr8: TeeAwsNitroPcrDigest,
        image_digest: SecureAggregateCommitment,
        policy_binding: SecureAggregateCommitment,
        nonce_digest: SecureAggregateCommitment,
        certificate_chain_digest: SecureAggregateCommitment,
        cose_signing_key_digest: SecureAggregateCommitment,
        not_before_epoch_seconds: u64,
        not_after_epoch_seconds: u64,
        evidence_sequence: SecureAggregateEnvelopeSequence,
        debug_mode: bool,
    ) -> Result<Self, SecureAggregateError> {
        if not_after_epoch_seconds < not_before_epoch_seconds {
            return Err(SecureAggregateError::TeeEvidenceWindowInvalid);
        }
        Ok(Self {
            family,
            format,
            job_id,
            pcr0,
            pcr1,
            pcr2,
            pcr3,
            pcr8,
            image_digest,
            policy_binding,
            nonce_digest,
            certificate_chain_digest,
            cose_signing_key_digest,
            not_before_epoch_seconds,
            not_after_epoch_seconds,
            evidence_sequence,
            debug_mode,
        })
    }

    /// Returns the attestation family.
    #[must_use]
    pub const fn family(self) -> TeeAwsNitroAttestationFamily {
        self.family
    }

    /// Returns the attestation format.
    #[must_use]
    pub const fn format(self) -> TeeAwsNitroAttestationFormat {
        self.format
    }

    /// Returns the attested job identifier.
    #[must_use]
    pub const fn job_id(self) -> SecureAggregateTranscriptId {
        self.job_id
    }

    /// Returns Nitro PCR0, the enclave image measurement.
    #[must_use]
    pub const fn pcr0(self) -> TeeAwsNitroPcrDigest {
        self.pcr0
    }

    /// Returns Nitro PCR1, the kernel/bootstrap measurement.
    #[must_use]
    pub const fn pcr1(self) -> TeeAwsNitroPcrDigest {
        self.pcr1
    }

    /// Returns Nitro PCR2, the application measurement.
    #[must_use]
    pub const fn pcr2(self) -> TeeAwsNitroPcrDigest {
        self.pcr2
    }

    /// Returns Nitro PCR3, the parent IAM-role measurement.
    #[must_use]
    pub const fn pcr3(self) -> TeeAwsNitroPcrDigest {
        self.pcr3
    }

    /// Returns Nitro PCR8, the EIF signing-certificate measurement.
    #[must_use]
    pub const fn pcr8(self) -> TeeAwsNitroPcrDigest {
        self.pcr8
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

    /// Returns the verifier-checked nonce or challenge digest.
    #[must_use]
    pub const fn nonce_digest(self) -> SecureAggregateCommitment {
        self.nonce_digest
    }

    /// Returns the verifier-checked certificate-chain digest.
    #[must_use]
    pub const fn certificate_chain_digest(self) -> SecureAggregateCommitment {
        self.certificate_chain_digest
    }

    /// Returns the verifier-checked COSE signing-key digest.
    #[must_use]
    pub const fn cose_signing_key_digest(self) -> SecureAggregateCommitment {
        self.cose_signing_key_digest
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

    /// Returns the monotonic attestation evidence sequence.
    #[must_use]
    pub const fn evidence_sequence(self) -> SecureAggregateEnvelopeSequence {
        self.evidence_sequence
    }

    /// Returns whether the verifier reported Nitro debug mode.
    #[must_use]
    pub const fn debug_mode(self) -> bool {
        self.debug_mode
    }

    /// Returns whether any admitted PCR is the all-zero debug-mode value.
    #[must_use]
    pub const fn has_all_zero_pcr(self) -> bool {
        self.pcr0.is_all_zero()
            || self.pcr1.is_all_zero()
            || self.pcr2.is_all_zero()
            || self.pcr3.is_all_zero()
            || self.pcr8.is_all_zero()
    }
}

impl TryFrom<TeeAwsNitroAttestationClaimsUnchecked> for TeeAwsNitroAttestationClaims {
    type Error = SecureAggregateError;

    fn try_from(value: TeeAwsNitroAttestationClaimsUnchecked) -> Result<Self, Self::Error> {
        Self::new(
            value.family,
            value.format,
            value.job_id,
            value.pcr0,
            value.pcr1,
            value.pcr2,
            value.pcr3,
            value.pcr8,
            value.image_digest,
            value.policy_binding,
            value.nonce_digest,
            value.certificate_chain_digest,
            value.cose_signing_key_digest,
            value.not_before_epoch_seconds,
            value.not_after_epoch_seconds,
            value.evidence_sequence,
            value.debug_mode,
        )
    }
}

impl From<TeeAwsNitroAttestationClaims> for TeeAwsNitroAttestationClaimsUnchecked {
    fn from(value: TeeAwsNitroAttestationClaims) -> Self {
        Self {
            family: value.family(),
            format: value.format(),
            job_id: value.job_id(),
            pcr0: value.pcr0(),
            pcr1: value.pcr1(),
            pcr2: value.pcr2(),
            pcr3: value.pcr3(),
            pcr8: value.pcr8(),
            image_digest: value.image_digest(),
            policy_binding: value.policy_binding(),
            nonce_digest: value.nonce_digest(),
            certificate_chain_digest: value.certificate_chain_digest(),
            cose_signing_key_digest: value.cose_signing_key_digest(),
            not_before_epoch_seconds: value.not_before_epoch_seconds(),
            not_after_epoch_seconds: value.not_after_epoch_seconds(),
            evidence_sequence: value.evidence_sequence(),
            debug_mode: value.debug_mode(),
        }
    }
}

/// AWS Nitro attestation evidence after external verifier normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeeAwsNitroAttestationEvidence {
    claims: TeeAwsNitroAttestationClaims,
    verifier_binding: TeeAwsNitroVerifierBinding,
    document_digest: SecureAggregateCommitment,
    certificate_chain_valid: bool,
    cose_signature_valid: bool,
}

impl TeeAwsNitroAttestationEvidence {
    /// Creates verified Nitro evidence and derives Kiva's canonical document digest.
    pub fn new(
        claims: TeeAwsNitroAttestationClaims,
        verifier_binding: TeeAwsNitroVerifierBinding,
        certificate_chain_valid: bool,
        cose_signature_valid: bool,
    ) -> Result<Self, SecureAggregateError> {
        Ok(Self {
            claims,
            verifier_binding,
            document_digest: derive_tee_aws_nitro_attestation_document_digest(claims)?,
            certificate_chain_valid,
            cose_signature_valid,
        })
    }

    /// Creates evidence from explicit parts for repository boundary validation.
    #[must_use]
    pub const fn from_parts(
        claims: TeeAwsNitroAttestationClaims,
        verifier_binding: TeeAwsNitroVerifierBinding,
        document_digest: SecureAggregateCommitment,
        certificate_chain_valid: bool,
        cose_signature_valid: bool,
    ) -> Self {
        Self {
            claims,
            verifier_binding,
            document_digest,
            certificate_chain_valid,
            cose_signature_valid,
        }
    }

    /// Returns the normalized attestation claims.
    #[must_use]
    pub const fn claims(self) -> TeeAwsNitroAttestationClaims {
        self.claims
    }

    /// Returns the verifier binding metadata.
    #[must_use]
    pub const fn verifier_binding(self) -> TeeAwsNitroVerifierBinding {
        self.verifier_binding
    }

    /// Returns Kiva's canonical digest of the verified document claims.
    #[must_use]
    pub const fn document_digest(self) -> SecureAggregateCommitment {
        self.document_digest
    }

    /// Returns whether the external verifier accepted the certificate chain.
    #[must_use]
    pub const fn certificate_chain_valid(self) -> bool {
        self.certificate_chain_valid
    }

    /// Returns whether the external verifier accepted the COSE signature.
    #[must_use]
    pub const fn cose_signature_valid(self) -> bool {
        self.cose_signature_valid
    }
}

/// Derives the canonical digest over externally verified AWS Nitro claims.
pub fn derive_tee_aws_nitro_attestation_document_digest(
    claims: TeeAwsNitroAttestationClaims,
) -> Result<SecureAggregateCommitment, SecureAggregateError> {
    let mut hasher = Sha256::new();
    hasher.update(TEE_AWS_NITRO_ATTESTATION_DOCUMENT_DOMAIN);
    update_len_prefixed(&mut hasher, TEE_AWS_NITRO_ATTESTATION_PROFILE.as_bytes());
    update_len_prefixed(&mut hasher, claims.family().label().as_bytes());
    update_len_prefixed(&mut hasher, claims.format().label().as_bytes());
    hasher.update(claims.job_id().bytes());
    hasher.update(claims.pcr0().bytes());
    hasher.update(claims.pcr1().bytes());
    hasher.update(claims.pcr2().bytes());
    hasher.update(claims.pcr3().bytes());
    hasher.update(claims.pcr8().bytes());
    hasher.update(claims.image_digest().bytes());
    hasher.update(claims.policy_binding().bytes());
    hasher.update(claims.nonce_digest().bytes());
    hasher.update(claims.certificate_chain_digest().bytes());
    hasher.update(claims.cose_signing_key_digest().bytes());
    hasher.update(claims.not_before_epoch_seconds().to_be_bytes());
    hasher.update(claims.not_after_epoch_seconds().to_be_bytes());
    hasher.update(claims.evidence_sequence().value().get().to_be_bytes());
    hasher.update([u8::from(claims.debug_mode())]);
    let digest = hasher.finalize();
    let mut bytes = [0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES];
    bytes.copy_from_slice(&digest);
    SecureAggregateCommitment::new(bytes)
}

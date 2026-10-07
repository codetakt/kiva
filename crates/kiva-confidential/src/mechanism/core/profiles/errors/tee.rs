use super::SecureAggregateError;
use thiserror::Error;

/// TEE attestation and release-gate boundary errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TeeBoundaryError {
    /// The TEE boundary requires all configured participants.
    #[error("TEE attestation gate requires an n-of-n participant threshold")]
    RequiresAllParticipants,
    /// The attestation evidence format is unsupported.
    #[error("TEE attestation evidence format is unsupported")]
    UnsupportedAttestationFormat,
    /// Policy approval is missing for attested execution.
    #[error("TEE attestation policy approval is missing")]
    PolicyApprovalMissing,
    /// The attestation evidence is bound to a different job.
    #[error("TEE attestation job binding does not match")]
    JobBindingMismatch,
    /// The attested measurement does not match policy.
    #[error("TEE attestation measurement does not match")]
    MeasurementMismatch,
    /// The attested image digest does not match policy.
    #[error("TEE attestation image digest does not match")]
    ImageDigestMismatch,
    /// The attested policy binding does not match policy.
    #[error("TEE attestation policy binding does not match")]
    PolicyBindingMismatch,
    /// The attestation validity window is structurally invalid.
    #[error("TEE attestation validity window is invalid")]
    EvidenceWindowInvalid,
    /// The attestation evidence is not yet valid.
    #[error("TEE attestation evidence is not yet valid")]
    EvidenceNotYetValid,
    /// The attestation evidence is expired.
    #[error("TEE attestation evidence is expired")]
    EvidenceExpired,
    /// TEE rollback evidence was detected.
    #[error("TEE attestation rollback was detected")]
    RollbackDetected,
    /// TEE attestation evidence was replayed.
    #[error("TEE attestation replay was detected")]
    ReplayDetected,
    /// The attestation evidence digest does not match.
    #[error("TEE attestation evidence digest does not match")]
    EvidenceDigestMismatch,
    /// Attestation is required before secret provisioning or release.
    #[error("TEE attestation is required before secret provisioning or release")]
    AttestationRequired,
    /// Nitro verifier binding metadata is missing or stale.
    #[error("AWS Nitro TEE verifier binding is missing or stale")]
    NitroVerifierBindingMissing,
    /// External security review is required before Nitro release.
    #[error("AWS Nitro TEE external security review is required")]
    NitroExternalReviewRequired,
    /// The Nitro certificate chain failed validation.
    #[error("AWS Nitro attestation certificate chain is invalid")]
    NitroCertificateChainInvalid,
    /// The Nitro COSE signature failed validation.
    #[error("AWS Nitro attestation COSE signature is invalid")]
    NitroCoseSignatureInvalid,
    /// The Nitro trust anchor does not match policy.
    #[error("AWS Nitro attestation trust anchor does not match")]
    NitroTrustAnchorMismatch,
    /// Nitro debug-mode attestation is rejected.
    #[error("AWS Nitro debug-mode attestation is rejected")]
    NitroDebugModeRejected,
    /// Nitro PCR values do not match policy.
    #[error("AWS Nitro attestation PCR policy does not match")]
    NitroPcrMismatch,
    /// Nitro attestation nonce does not match the job challenge.
    #[error("AWS Nitro attestation nonce does not match")]
    NitroNonceMismatch,
    /// Nitro PCR values are all zero and therefore rejected.
    #[error("AWS Nitro attestation PCR values must not be all zero")]
    NitroPcrAllZeroRejected,
    /// Nitro verifier key rotation is required.
    #[error("AWS Nitro verifier key rotation is required")]
    NitroVerifierKeyRotationRequired,
    /// Nitro secret provisioning evidence is missing.
    #[error("AWS Nitro TEE secret provisioning is required before result release")]
    NitroSecretProvisioningRequired,
}

impl From<TeeBoundaryError> for SecureAggregateError {
    fn from(error: TeeBoundaryError) -> Self {
        match error {
            TeeBoundaryError::RequiresAllParticipants => Self::TeeRequiresAllParticipants,
            TeeBoundaryError::UnsupportedAttestationFormat => Self::TeeUnsupportedAttestationFormat,
            TeeBoundaryError::PolicyApprovalMissing => Self::TeePolicyApprovalMissing,
            TeeBoundaryError::JobBindingMismatch => Self::TeeJobBindingMismatch,
            TeeBoundaryError::MeasurementMismatch => Self::TeeMeasurementMismatch,
            TeeBoundaryError::ImageDigestMismatch => Self::TeeImageDigestMismatch,
            TeeBoundaryError::PolicyBindingMismatch => Self::TeePolicyBindingMismatch,
            TeeBoundaryError::EvidenceWindowInvalid => Self::TeeEvidenceWindowInvalid,
            TeeBoundaryError::EvidenceNotYetValid => Self::TeeEvidenceNotYetValid,
            TeeBoundaryError::EvidenceExpired => Self::TeeEvidenceExpired,
            TeeBoundaryError::RollbackDetected => Self::TeeRollbackDetected,
            TeeBoundaryError::ReplayDetected => Self::TeeReplayDetected,
            TeeBoundaryError::EvidenceDigestMismatch => Self::TeeEvidenceDigestMismatch,
            TeeBoundaryError::AttestationRequired => Self::TeeAttestationRequired,
            TeeBoundaryError::NitroVerifierBindingMissing => Self::TeeNitroVerifierBindingMissing,
            TeeBoundaryError::NitroExternalReviewRequired => Self::TeeNitroExternalReviewRequired,
            TeeBoundaryError::NitroCertificateChainInvalid => Self::TeeNitroCertificateChainInvalid,
            TeeBoundaryError::NitroCoseSignatureInvalid => Self::TeeNitroCoseSignatureInvalid,
            TeeBoundaryError::NitroTrustAnchorMismatch => Self::TeeNitroTrustAnchorMismatch,
            TeeBoundaryError::NitroDebugModeRejected => Self::TeeNitroDebugModeRejected,
            TeeBoundaryError::NitroPcrMismatch => Self::TeeNitroPcrMismatch,
            TeeBoundaryError::NitroNonceMismatch => Self::TeeNitroNonceMismatch,
            TeeBoundaryError::NitroPcrAllZeroRejected => Self::TeeNitroPcrAllZeroRejected,
            TeeBoundaryError::NitroVerifierKeyRotationRequired => {
                Self::TeeNitroVerifierKeyRotationRequired
            }
            TeeBoundaryError::NitroSecretProvisioningRequired => {
                Self::TeeNitroSecretProvisioningRequired
            }
        }
    }
}

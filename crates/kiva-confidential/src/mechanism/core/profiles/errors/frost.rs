use super::SecureAggregateError;
use thiserror::Error;

/// FROST threshold-signing boundary errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum FrostBoundaryError {
    /// Runtime binding metadata for the FROST provider is missing.
    #[error("FROST threshold runtime binding is missing")]
    RuntimeBindingMissing,
    /// External cryptographic review is required before this FROST boundary may release.
    #[error("FROST threshold external cryptographic review is required")]
    ExternalReviewRequired,
    /// The key ceremony does not contain one record per participant.
    #[error("FROST threshold key ceremony participant count does not match")]
    CeremonyParticipantCountMismatch,
    /// A required participant is inactive.
    #[error("FROST threshold key ceremony participant is inactive")]
    ParticipantInactive,
    /// A required participant has aborted.
    #[error("FROST threshold participant aborted")]
    ParticipantAborted,
    /// A complaint raised during the ceremony is unresolved.
    #[error("FROST threshold complaint is unresolved")]
    ComplaintUnresolved,
    /// Participant equivocation was detected.
    #[error("FROST threshold equivocation was detected")]
    EquivocationDetected,
    /// Share-custody evidence is missing.
    #[error("FROST threshold share custody evidence is missing")]
    ShareCustodyMissing,
    /// A FROST transcript was replayed.
    #[error("FROST threshold transcript replay was detected")]
    ReplayDetected,
    /// The available signature shares do not meet the threshold.
    #[error("FROST threshold signing requires threshold signature shares")]
    SignatureShareCountBelowThreshold,
    /// The threshold signature digest does not match.
    #[error("FROST threshold signature digest does not match")]
    SignatureDigestMismatch,
    /// Release is blocked until key ceremony evidence exists.
    #[error("FROST threshold release requires a completed key ceremony")]
    ReleaseRequiresKeyCeremony,
    /// Release is blocked until computed signature evidence exists.
    #[error("FROST threshold release requires a computed signature")]
    ReleaseRequiresSignature,
    /// Release is blocked until policy approval is present.
    #[error("FROST threshold release requires policy approval")]
    ReleaseRequiresPolicyApproval,
    /// Release is blocked until an active key epoch exists.
    #[error("FROST threshold release requires an active key epoch")]
    ReleaseRequiresActiveKeyEpoch,
    /// Release is blocked until governed key-use approval exists.
    #[error("FROST threshold release requires governed key use")]
    ReleaseRequiresGovernedKeyUse,
}

impl From<FrostBoundaryError> for SecureAggregateError {
    fn from(error: FrostBoundaryError) -> Self {
        match error {
            FrostBoundaryError::RuntimeBindingMissing => Self::ThresholdFrostRuntimeBindingMissing,
            FrostBoundaryError::ExternalReviewRequired => {
                Self::ThresholdFrostExternalReviewRequired
            }
            FrostBoundaryError::CeremonyParticipantCountMismatch => {
                Self::ThresholdFrostCeremonyParticipantCountMismatch
            }
            FrostBoundaryError::ParticipantInactive => Self::ThresholdFrostParticipantInactive,
            FrostBoundaryError::ParticipantAborted => Self::ThresholdFrostParticipantAborted,
            FrostBoundaryError::ComplaintUnresolved => Self::ThresholdFrostComplaintUnresolved,
            FrostBoundaryError::EquivocationDetected => Self::ThresholdFrostEquivocationDetected,
            FrostBoundaryError::ShareCustodyMissing => Self::ThresholdFrostShareCustodyMissing,
            FrostBoundaryError::ReplayDetected => Self::ThresholdFrostReplayDetected,
            FrostBoundaryError::SignatureShareCountBelowThreshold => {
                Self::ThresholdFrostSignatureShareCountBelowThreshold
            }
            FrostBoundaryError::SignatureDigestMismatch => {
                Self::ThresholdFrostSignatureDigestMismatch
            }
            FrostBoundaryError::ReleaseRequiresKeyCeremony => {
                Self::ThresholdFrostReleaseRequiresKeyCeremony
            }
            FrostBoundaryError::ReleaseRequiresSignature => {
                Self::ThresholdFrostReleaseRequiresSignature
            }
            FrostBoundaryError::ReleaseRequiresPolicyApproval => {
                Self::ThresholdFrostReleaseRequiresPolicyApproval
            }
            FrostBoundaryError::ReleaseRequiresActiveKeyEpoch => {
                Self::ThresholdFrostReleaseRequiresActiveKeyEpoch
            }
            FrostBoundaryError::ReleaseRequiresGovernedKeyUse => {
                Self::ThresholdFrostReleaseRequiresGovernedKeyUse
            }
        }
    }
}

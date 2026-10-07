use super::SecureAggregateError;
use thiserror::Error;

/// Shamir threshold-reconstruction boundary errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ShamirBoundaryError {
    /// The Shamir threshold is below the minimum admitted value.
    #[error("Shamir threshold reconstruction requires a threshold of at least two")]
    ThresholdTooSmall,
    /// The participant count does not fit in the configured field.
    #[error("Shamir participant count is outside the configured finite field")]
    ParticipantCountOutsideField,
    /// The available unique shares do not meet the threshold.
    #[error("Shamir reconstruction requires at least threshold unique shares")]
    ShareCountBelowThreshold,
    /// A field element has no multiplicative inverse.
    #[error("secure aggregate field element has no multiplicative inverse")]
    FieldElementHasNoInverse,
    /// A share names a participant index outside the configured participant set.
    #[error("secure aggregate participant index is outside the participant set")]
    ParticipantIndexOutOfRange,
    /// A participant index appears more than once.
    #[error("secure aggregate transcript contains a duplicate participant index")]
    DuplicateParticipantIndex,
    /// A share-opening nonce is the reserved all-zero value.
    #[error("Shamir share-opening nonce must not be empty")]
    EmptyShareOpeningNonce,
    /// A share opening does not match its commitment.
    #[error("Shamir share opening does not match its commitment")]
    ShareCommitmentMismatch,
}

impl From<ShamirBoundaryError> for SecureAggregateError {
    fn from(error: ShamirBoundaryError) -> Self {
        match error {
            ShamirBoundaryError::ThresholdTooSmall => Self::ShamirThresholdTooSmall,
            ShamirBoundaryError::ParticipantCountOutsideField => {
                Self::ShamirParticipantCountOutsideField
            }
            ShamirBoundaryError::ShareCountBelowThreshold => Self::ShamirShareCountBelowThreshold,
            ShamirBoundaryError::FieldElementHasNoInverse => Self::FieldElementHasNoInverse,
            ShamirBoundaryError::ParticipantIndexOutOfRange => Self::ParticipantIndexOutOfRange,
            ShamirBoundaryError::DuplicateParticipantIndex => Self::DuplicateParticipantIndex,
            ShamirBoundaryError::EmptyShareOpeningNonce => Self::EmptyShamirShareOpeningNonce,
            ShamirBoundaryError::ShareCommitmentMismatch => Self::ShamirShareCommitmentMismatch,
        }
    }
}

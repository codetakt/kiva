use super::SecureAggregateError;
use thiserror::Error;

/// Additive secure-aggregation boundary errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AdditiveSecureAggregateError {
    /// The value is not a member of the configured finite field.
    #[error("secure aggregate field element must be below the field modulus")]
    ValueOutsideField,
    /// The additive mechanism requires all participants for release.
    #[error("additive secure aggregation requires an n-of-n release threshold")]
    ThresholdMustRequireAllParticipants,
    /// The transcript does not contain exactly one share per participant.
    #[error("secure aggregate share count does not match participant count")]
    ShareCountMismatch,
    /// A share names a participant index outside the configured participant set.
    #[error("secure aggregate participant index is outside the participant set")]
    ParticipantIndexOutOfRange,
    /// A participant contributed more than one share to the transcript.
    #[error("secure aggregate transcript contains a duplicate participant index")]
    DuplicateParticipantIndex,
    /// The transcript identifier is the reserved all-zero value.
    #[error("secure aggregate transcript identifier must not be empty")]
    EmptyTranscriptIdentifier,
    /// The share commitment is the reserved all-zero value.
    #[error("secure aggregate share commitment must not be empty")]
    EmptyShareCommitment,
    /// The share signing key identifier is the reserved all-zero value.
    #[error("secure aggregate signing key identifier must not be empty")]
    EmptySigningKeyIdentifier,
    /// The transport proof tag is the reserved all-zero value.
    #[error("secure aggregate transport proof must not be empty")]
    EmptyTransportProof,
    /// The transcript does not contain exactly one commitment slot per participant.
    #[error("secure aggregate commitment slot count does not match participant count")]
    CommitmentSlotCountMismatch,
    /// A commitment slot belongs to a different transcript.
    #[error("secure aggregate commitment slot transcript does not match")]
    CommitmentSlotTranscriptMismatch,
    /// A commitment slot duplicates a participant index.
    #[error("secure aggregate commitment slots contain a duplicate participant index")]
    DuplicateCommitmentSlotParticipantIndex,
    /// The transcript does not contain exactly one authenticated envelope per participant.
    #[error("secure aggregate authenticated envelope count does not match participant count")]
    EnvelopeCountMismatch,
    /// An authenticated envelope belongs to a different transcript.
    #[error("secure aggregate authenticated envelope transcript does not match")]
    EnvelopeTranscriptMismatch,
    /// An authenticated envelope does not match the expected participant slot.
    #[error("secure aggregate authenticated envelope participant does not match")]
    EnvelopeShareParticipantMismatch,
    /// An authenticated envelope does not match the expected commitment.
    #[error("secure aggregate authenticated envelope commitment does not match")]
    EnvelopeCommitmentMismatch,
    /// An authenticated envelope does not match the expected signing key.
    #[error("secure aggregate authenticated envelope signing key does not match")]
    EnvelopeSigningKeyMismatch,
    /// The signing public key is the reserved all-zero value.
    #[error("secure aggregate signing public key must not be empty")]
    EmptySigningPublicKey,
    /// The signing public key cannot be decoded as an Ed25519 verification key.
    #[error("secure aggregate signing public key is not a valid Ed25519 key")]
    InvalidSigningPublicKey,
    /// The signature is the reserved all-zero value.
    #[error("secure aggregate signature must not be empty")]
    EmptySignature,
    /// The signature does not have the required Ed25519 byte length.
    #[error("secure aggregate signature must be exactly 64 bytes")]
    SignatureLengthMismatch,
    /// The envelope signature does not verify against the configured key.
    #[error("secure aggregate authenticated envelope signature does not verify")]
    EnvelopeSignatureMismatch,
    /// The transcript does not contain exactly one verification key per participant.
    #[error("secure aggregate verification key count does not match participant count")]
    VerificationKeyCountMismatch,
    /// The verification key set contains a duplicate signing-key identifier.
    #[error("secure aggregate verification key set contains a duplicate signing-key identifier")]
    DuplicateVerificationKeyIdentifier,
    /// A commitment slot has no matching verification key.
    #[error("secure aggregate commitment slot has no matching verification key")]
    VerificationKeyMissing,
    /// The envelope sequence is the reserved zero value.
    #[error("secure aggregate envelope sequence must not be zero")]
    EnvelopeSequenceMustNotBeZero,
    /// The replay window lower bound exceeds the upper bound.
    #[error("secure aggregate replay window lower bound exceeds upper bound")]
    ReplayWindowLowerBoundExceedsUpperBound,
    /// The envelope sequence is outside the configured replay window.
    #[error("secure aggregate envelope sequence is outside the replay window")]
    EnvelopeSequenceOutsideReplayWindow,
    /// The envelope matches an already accepted replay record.
    #[error("secure aggregate envelope was already accepted")]
    EnvelopeReplayDetected,
}

impl From<AdditiveSecureAggregateError> for SecureAggregateError {
    fn from(error: AdditiveSecureAggregateError) -> Self {
        match error {
            AdditiveSecureAggregateError::ValueOutsideField => Self::ValueOutsideField,
            AdditiveSecureAggregateError::ThresholdMustRequireAllParticipants => {
                Self::ThresholdMustRequireAllParticipants
            }
            AdditiveSecureAggregateError::ShareCountMismatch => Self::ShareCountMismatch,
            AdditiveSecureAggregateError::ParticipantIndexOutOfRange => {
                Self::ParticipantIndexOutOfRange
            }
            AdditiveSecureAggregateError::DuplicateParticipantIndex => {
                Self::DuplicateParticipantIndex
            }
            AdditiveSecureAggregateError::EmptyTranscriptIdentifier => {
                Self::EmptyTranscriptIdentifier
            }
            AdditiveSecureAggregateError::EmptyShareCommitment => Self::EmptyShareCommitment,
            AdditiveSecureAggregateError::EmptySigningKeyIdentifier => {
                Self::EmptySigningKeyIdentifier
            }
            AdditiveSecureAggregateError::EmptyTransportProof => Self::EmptyTransportProof,
            AdditiveSecureAggregateError::CommitmentSlotCountMismatch => {
                Self::CommitmentSlotCountMismatch
            }
            AdditiveSecureAggregateError::CommitmentSlotTranscriptMismatch => {
                Self::CommitmentSlotTranscriptMismatch
            }
            AdditiveSecureAggregateError::DuplicateCommitmentSlotParticipantIndex => {
                Self::DuplicateCommitmentSlotParticipantIndex
            }
            AdditiveSecureAggregateError::EnvelopeCountMismatch => Self::EnvelopeCountMismatch,
            AdditiveSecureAggregateError::EnvelopeTranscriptMismatch => {
                Self::EnvelopeTranscriptMismatch
            }
            AdditiveSecureAggregateError::EnvelopeShareParticipantMismatch => {
                Self::EnvelopeShareParticipantMismatch
            }
            AdditiveSecureAggregateError::EnvelopeCommitmentMismatch => {
                Self::EnvelopeCommitmentMismatch
            }
            AdditiveSecureAggregateError::EnvelopeSigningKeyMismatch => {
                Self::EnvelopeSigningKeyMismatch
            }
            AdditiveSecureAggregateError::EmptySigningPublicKey => Self::EmptySigningPublicKey,
            AdditiveSecureAggregateError::InvalidSigningPublicKey => Self::InvalidSigningPublicKey,
            AdditiveSecureAggregateError::EmptySignature => Self::EmptySignature,
            AdditiveSecureAggregateError::SignatureLengthMismatch => Self::SignatureLengthMismatch,
            AdditiveSecureAggregateError::EnvelopeSignatureMismatch => {
                Self::EnvelopeSignatureMismatch
            }
            AdditiveSecureAggregateError::VerificationKeyCountMismatch => {
                Self::VerificationKeyCountMismatch
            }
            AdditiveSecureAggregateError::DuplicateVerificationKeyIdentifier => {
                Self::DuplicateVerificationKeyIdentifier
            }
            AdditiveSecureAggregateError::VerificationKeyMissing => Self::VerificationKeyMissing,
            AdditiveSecureAggregateError::EnvelopeSequenceMustNotBeZero => {
                Self::EnvelopeSequenceMustNotBeZero
            }
            AdditiveSecureAggregateError::ReplayWindowLowerBoundExceedsUpperBound => {
                Self::ReplayWindowLowerBoundExceedsUpperBound
            }
            AdditiveSecureAggregateError::EnvelopeSequenceOutsideReplayWindow => {
                Self::EnvelopeSequenceOutsideReplayWindow
            }
            AdditiveSecureAggregateError::EnvelopeReplayDetected => Self::EnvelopeReplayDetected,
        }
    }
}

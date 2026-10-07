use super::SecureAggregateError;
use thiserror::Error;

/// FHE feasibility and runtime boundary errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum FheBoundaryError {
    /// The FHE boundary requires all configured participants.
    #[error("FHE feasibility profile requires an n-of-n participant threshold")]
    RequiresAllParticipants,
    /// The requested FHE profile or operation is outside the admitted set.
    #[error("FHE feasibility profile or operation is unsupported")]
    UnsupportedProfile,
    /// The declared FHE parameter set is not admitted.
    #[error("FHE feasibility parameter set is rejected")]
    ParameterSetRejected,
    /// A plaintext lies outside the admitted input range.
    #[error("FHE feasibility plaintext is outside the admitted range")]
    PlaintextOutsideRange,
    /// The FHE nonce is the reserved all-zero value.
    #[error("FHE feasibility nonce must not be empty")]
    EmptyNonce,
    /// The transcript does not contain one ciphertext per participant.
    #[error("FHE feasibility ciphertext count does not match participant count")]
    CiphertextCountMismatch,
    /// A ciphertext is bound to a different job.
    #[error("FHE feasibility ciphertext job does not match")]
    CiphertextJobMismatch,
    /// A ciphertext is bound to a different parameter set.
    #[error("FHE feasibility ciphertext parameter set does not match")]
    CiphertextParameterMismatch,
    /// A ciphertext digest does not match its declared value.
    #[error("FHE feasibility ciphertext digest does not match")]
    CiphertextDigestMismatch,
    /// The feasibility evaluation overflowed the admitted range.
    #[error("FHE feasibility sum evaluation overflowed")]
    EvaluationOverflow,
    /// Release is blocked until feasibility evaluation completes.
    #[error("FHE feasibility release requires completed evaluation")]
    ReleaseRequiresEvaluation,
    /// Release is blocked until policy approval is present.
    #[error("FHE feasibility release requires policy approval")]
    ReleaseRequiresPolicyApproval,
    /// Release is blocked until the governed data product is approved.
    #[error("FHE feasibility release requires governed release treatment")]
    ReleaseRequiresGovernedDataProduct,
    /// The plaintext boundary proof failed.
    #[error("FHE feasibility plaintext boundary was violated")]
    PlaintextBoundaryViolation,
    /// Runtime provider binding metadata is missing.
    #[error("FHE runtime binding metadata is missing")]
    RuntimeBindingMissing,
    /// Runtime provider use requires external cryptographic review.
    #[error("FHE runtime external cryptographic review is required")]
    RuntimeExternalReviewRequired,
    /// The runtime provider rejected the parameter set.
    #[error("FHE runtime parameter set is rejected")]
    RuntimeParameterSetRejected,
    /// Runtime key-custody evidence is missing.
    #[error("FHE runtime key custody metadata is missing")]
    RuntimeKeyCustodyMissing,
    /// The runtime transcript does not contain one ciphertext per participant.
    #[error("FHE runtime ciphertext count does not match participant count")]
    RuntimeCiphertextCountMismatch,
    /// A runtime ciphertext was submitted by an inactive participant.
    #[error("FHE runtime ciphertext participant is inactive")]
    RuntimeCiphertextParticipantInactive,
    /// A runtime ciphertext was submitted by an aborted participant.
    #[error("FHE runtime ciphertext participant aborted")]
    RuntimeCiphertextParticipantAborted,
    /// A runtime ciphertext is bound to a different parameter set.
    #[error("FHE runtime ciphertext parameter set does not match")]
    RuntimeCiphertextParameterMismatch,
    /// A runtime ciphertext transcript digest does not match.
    #[error("FHE runtime ciphertext transcript digest does not match")]
    RuntimeCiphertextDigestMismatch,
    /// A runtime ciphertext transcript was replayed.
    #[error("FHE runtime ciphertext transcript replay detected")]
    RuntimeCiphertextReplayDetected,
    /// Runtime evaluation was requested without accepted ciphertexts.
    #[error("FHE runtime evaluation requires accepted ciphertexts")]
    RuntimeEvaluationRequiresCiphertexts,
    /// The runtime evaluation digest does not match.
    #[error("FHE runtime evaluation digest does not match")]
    RuntimeEvaluationDigestMismatch,
    /// A runtime evaluation transcript was replayed.
    #[error("FHE runtime evaluation replay detected")]
    RuntimeEvaluationReplayDetected,
    /// Runtime release is blocked until ciphertext transcript evidence exists.
    #[error("FHE runtime release requires accepted ciphertext transcript")]
    RuntimeReleaseRequiresCiphertextTranscript,
    /// Runtime release is blocked until evaluation completes.
    #[error("FHE runtime release requires completed evaluation")]
    RuntimeReleaseRequiresEvaluation,
    /// Runtime release is blocked until policy approval is present.
    #[error("FHE runtime release requires policy approval")]
    RuntimeReleaseRequiresPolicyApproval,
    /// Runtime release is blocked until governed data-product approval exists.
    #[error("FHE runtime release requires governed release treatment")]
    RuntimeReleaseRequiresGovernedDataProduct,
    /// Runtime release is blocked until external review evidence exists.
    #[error("FHE runtime release requires external review")]
    RuntimeReleaseRequiresExternalReview,
    /// Runtime release is blocked until an active key epoch exists.
    #[error("FHE runtime release requires an active key epoch")]
    RuntimeReleaseRequiresActiveKeyEpoch,
}

impl From<FheBoundaryError> for SecureAggregateError {
    fn from(error: FheBoundaryError) -> Self {
        match error {
            FheBoundaryError::RequiresAllParticipants => Self::FheRequiresAllParticipants,
            FheBoundaryError::UnsupportedProfile => Self::FheUnsupportedProfile,
            FheBoundaryError::ParameterSetRejected => Self::FheParameterSetRejected,
            FheBoundaryError::PlaintextOutsideRange => Self::FhePlaintextOutsideRange,
            FheBoundaryError::EmptyNonce => Self::EmptyFheNonce,
            FheBoundaryError::CiphertextCountMismatch => Self::FheCiphertextCountMismatch,
            FheBoundaryError::CiphertextJobMismatch => Self::FheCiphertextJobMismatch,
            FheBoundaryError::CiphertextParameterMismatch => Self::FheCiphertextParameterMismatch,
            FheBoundaryError::CiphertextDigestMismatch => Self::FheCiphertextDigestMismatch,
            FheBoundaryError::EvaluationOverflow => Self::FheEvaluationOverflow,
            FheBoundaryError::ReleaseRequiresEvaluation => Self::FheReleaseRequiresEvaluation,
            FheBoundaryError::ReleaseRequiresPolicyApproval => {
                Self::FheReleaseRequiresPolicyApproval
            }
            FheBoundaryError::ReleaseRequiresGovernedDataProduct => {
                Self::FheReleaseRequiresGovernedDataProduct
            }
            FheBoundaryError::PlaintextBoundaryViolation => Self::FhePlaintextBoundaryViolation,
            FheBoundaryError::RuntimeBindingMissing => Self::FheRuntimeBindingMissing,
            FheBoundaryError::RuntimeExternalReviewRequired => {
                Self::FheRuntimeExternalReviewRequired
            }
            FheBoundaryError::RuntimeParameterSetRejected => Self::FheRuntimeParameterSetRejected,
            FheBoundaryError::RuntimeKeyCustodyMissing => Self::FheRuntimeKeyCustodyMissing,
            FheBoundaryError::RuntimeCiphertextCountMismatch => {
                Self::FheRuntimeCiphertextCountMismatch
            }
            FheBoundaryError::RuntimeCiphertextParticipantInactive => {
                Self::FheRuntimeCiphertextParticipantInactive
            }
            FheBoundaryError::RuntimeCiphertextParticipantAborted => {
                Self::FheRuntimeCiphertextParticipantAborted
            }
            FheBoundaryError::RuntimeCiphertextParameterMismatch => {
                Self::FheRuntimeCiphertextParameterMismatch
            }
            FheBoundaryError::RuntimeCiphertextDigestMismatch => {
                Self::FheRuntimeCiphertextDigestMismatch
            }
            FheBoundaryError::RuntimeCiphertextReplayDetected => {
                Self::FheRuntimeCiphertextReplayDetected
            }
            FheBoundaryError::RuntimeEvaluationRequiresCiphertexts => {
                Self::FheRuntimeEvaluationRequiresCiphertexts
            }
            FheBoundaryError::RuntimeEvaluationDigestMismatch => {
                Self::FheRuntimeEvaluationDigestMismatch
            }
            FheBoundaryError::RuntimeEvaluationReplayDetected => {
                Self::FheRuntimeEvaluationReplayDetected
            }
            FheBoundaryError::RuntimeReleaseRequiresCiphertextTranscript => {
                Self::FheRuntimeReleaseRequiresCiphertextTranscript
            }
            FheBoundaryError::RuntimeReleaseRequiresEvaluation => {
                Self::FheRuntimeReleaseRequiresEvaluation
            }
            FheBoundaryError::RuntimeReleaseRequiresPolicyApproval => {
                Self::FheRuntimeReleaseRequiresPolicyApproval
            }
            FheBoundaryError::RuntimeReleaseRequiresGovernedDataProduct => {
                Self::FheRuntimeReleaseRequiresGovernedDataProduct
            }
            FheBoundaryError::RuntimeReleaseRequiresExternalReview => {
                Self::FheRuntimeReleaseRequiresExternalReview
            }
            FheBoundaryError::RuntimeReleaseRequiresActiveKeyEpoch => {
                Self::FheRuntimeReleaseRequiresActiveKeyEpoch
            }
        }
    }
}

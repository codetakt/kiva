use super::SecureAggregateError;
use thiserror::Error;

/// PSI and DH-OPRF PSI boundary errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PsiBoundaryError {
    /// PSI requires all configured participants.
    #[error("private set intersection requires an n-of-n participant threshold")]
    RequiresAllParticipants,
    /// The PSI namespace is empty after canonicalization.
    #[error("private set intersection namespace must not be empty")]
    NamespaceEmpty,
    /// The PSI namespace contains a control character.
    #[error("private set intersection namespace contains a control character")]
    NamespaceContainsControl,
    /// A PSI identifier is empty after canonicalization.
    #[error("private set intersection identifier must not be empty")]
    IdentifierEmpty,
    /// A PSI identifier contains a control character.
    #[error("private set intersection identifier contains a control character")]
    IdentifierContainsControl,
    /// A PSI identifier digest is the reserved all-zero value.
    #[error("private set intersection identifier digest must not be empty")]
    EmptyIdentifierDigest,
    /// A participant PSI dataset contains no identifiers.
    #[error("private set intersection participant dataset must not be empty")]
    ParticipantDatasetEmpty,
    /// The job does not contain exactly one dataset per participant.
    #[error("private set intersection participant dataset count does not match participant count")]
    ParticipantDatasetCountMismatch,
    /// A participant PSI dataset contains duplicate identifier digests.
    #[error("private set intersection participant dataset contains duplicate identifier digests")]
    DuplicateIdentifierDigest,
    /// A PSI job identifier was replayed.
    #[error("private set intersection job was already accepted")]
    ReplayDetected,
    /// The DH-OPRF PSI suite is unsupported.
    #[error("DH-OPRF PSI runtime suite is unsupported")]
    OprfUnsupportedSuite,
    /// Runtime binding metadata for DH-OPRF PSI is missing.
    #[error("DH-OPRF PSI runtime binding is missing")]
    OprfRuntimeBindingMissing,
    /// External cryptographic review is required before DH-OPRF PSI release.
    #[error("DH-OPRF PSI external cryptographic review is required")]
    OprfExternalReviewRequired,
    /// A DH-OPRF PSI element is the reserved all-zero value.
    #[error("DH-OPRF PSI element must not be empty")]
    EmptyOprfElement,
    /// A participant DH-OPRF transcript contains no elements.
    #[error("DH-OPRF PSI participant transcript must not be empty")]
    OprfParticipantTranscriptEmpty,
    /// The job does not contain exactly one DH-OPRF transcript per participant.
    #[error("DH-OPRF PSI participant transcript count does not match participant count")]
    OprfParticipantTranscriptCountMismatch,
    /// A participant DH-OPRF transcript contains duplicate elements.
    #[error("DH-OPRF PSI participant transcript contains duplicate elements")]
    DuplicateOprfElement,
    /// A DH-OPRF PSI transcript was replayed.
    #[error("DH-OPRF PSI transcript replay was detected")]
    OprfTranscriptReplayDetected,
    /// A participant aborted the DH-OPRF PSI transcript.
    #[error("DH-OPRF PSI transcript was aborted")]
    OprfTranscriptAborted,
    /// Release is blocked until intersection evidence exists.
    #[error("DH-OPRF PSI release requires a computed intersection")]
    OprfReleaseRequiresIntersection,
    /// Release is blocked until policy approval is present.
    #[error("DH-OPRF PSI release requires policy approval")]
    OprfReleaseRequiresPolicyApproval,
    /// Release is blocked until governed data-product approval exists.
    #[error("DH-OPRF PSI release requires governed release treatment")]
    OprfReleaseRequiresGovernedDataProduct,
    /// Release is blocked until all admitted participants are active.
    #[error("DH-OPRF PSI release requires active participants")]
    OprfReleaseRequiresActiveParticipants,
}

impl From<PsiBoundaryError> for SecureAggregateError {
    fn from(error: PsiBoundaryError) -> Self {
        match error {
            PsiBoundaryError::RequiresAllParticipants => Self::PsiRequiresAllParticipants,
            PsiBoundaryError::NamespaceEmpty => Self::PsiNamespaceEmpty,
            PsiBoundaryError::NamespaceContainsControl => Self::PsiNamespaceContainsControl,
            PsiBoundaryError::IdentifierEmpty => Self::PsiIdentifierEmpty,
            PsiBoundaryError::IdentifierContainsControl => Self::PsiIdentifierContainsControl,
            PsiBoundaryError::EmptyIdentifierDigest => Self::EmptyPsiIdentifierDigest,
            PsiBoundaryError::ParticipantDatasetEmpty => Self::PsiParticipantDatasetEmpty,
            PsiBoundaryError::ParticipantDatasetCountMismatch => {
                Self::PsiParticipantDatasetCountMismatch
            }
            PsiBoundaryError::DuplicateIdentifierDigest => Self::DuplicatePsiIdentifierDigest,
            PsiBoundaryError::ReplayDetected => Self::PsiReplayDetected,
            PsiBoundaryError::OprfUnsupportedSuite => Self::PsiOprfUnsupportedSuite,
            PsiBoundaryError::OprfRuntimeBindingMissing => Self::PsiOprfRuntimeBindingMissing,
            PsiBoundaryError::OprfExternalReviewRequired => Self::PsiOprfExternalReviewRequired,
            PsiBoundaryError::EmptyOprfElement => Self::EmptyPsiOprfElement,
            PsiBoundaryError::OprfParticipantTranscriptEmpty => {
                Self::PsiOprfParticipantTranscriptEmpty
            }
            PsiBoundaryError::OprfParticipantTranscriptCountMismatch => {
                Self::PsiOprfParticipantTranscriptCountMismatch
            }
            PsiBoundaryError::DuplicateOprfElement => Self::DuplicatePsiOprfElement,
            PsiBoundaryError::OprfTranscriptReplayDetected => Self::PsiOprfTranscriptReplayDetected,
            PsiBoundaryError::OprfTranscriptAborted => Self::PsiOprfTranscriptAborted,
            PsiBoundaryError::OprfReleaseRequiresIntersection => {
                Self::PsiOprfReleaseRequiresIntersection
            }
            PsiBoundaryError::OprfReleaseRequiresPolicyApproval => {
                Self::PsiOprfReleaseRequiresPolicyApproval
            }
            PsiBoundaryError::OprfReleaseRequiresGovernedDataProduct => {
                Self::PsiOprfReleaseRequiresGovernedDataProduct
            }
            PsiBoundaryError::OprfReleaseRequiresActiveParticipants => {
                Self::PsiOprfReleaseRequiresActiveParticipants
            }
        }
    }
}

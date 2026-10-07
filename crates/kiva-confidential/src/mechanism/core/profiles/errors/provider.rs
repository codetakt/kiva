use super::SecureAggregateError;
use thiserror::Error;

/// Secret/confidential-compute provider boundary errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SecretComputeProviderBoundaryError {
    /// The provider does not support the requested profile.
    #[error("secret compute provider profile is unsupported")]
    UnsupportedProfile,
    /// The fixed reference primitive provider kind does not match its profile.
    #[error("reference primitive provider kind does not match its profile")]
    ReferenceProviderKindMismatch,
    /// The reference primitive provider is being used for a production claim.
    #[error("reference primitive provider cannot support a production claim")]
    ReferenceProviderProductionClaimRejected,
    /// External provider review evidence is required.
    #[error("external secret compute provider review evidence is required")]
    ExternalProviderReviewRequired,
    /// Provider parameter-set evidence is required.
    #[error("external secret compute provider parameter-set evidence is required")]
    ParameterSetRequired,
    /// The provider capability does not match the requested mechanism.
    #[error("secret compute provider capability does not match the mechanism")]
    CapabilityMismatch,
}

impl From<SecretComputeProviderBoundaryError> for SecureAggregateError {
    fn from(error: SecretComputeProviderBoundaryError) -> Self {
        match error {
            SecretComputeProviderBoundaryError::UnsupportedProfile => {
                Self::SecretComputeProviderUnsupportedProfile
            }
            SecretComputeProviderBoundaryError::ReferenceProviderKindMismatch => {
                Self::ReferencePrimitiveProviderKindMismatch
            }
            SecretComputeProviderBoundaryError::ReferenceProviderProductionClaimRejected => {
                Self::ReferencePrimitiveProviderProductionClaimRejected
            }
            SecretComputeProviderBoundaryError::ExternalProviderReviewRequired => {
                Self::ExternalProviderReviewRequired
            }
            SecretComputeProviderBoundaryError::ParameterSetRequired => {
                Self::SecretComputeProviderParameterSetRequired
            }
            SecretComputeProviderBoundaryError::CapabilityMismatch => {
                Self::SecretComputeProviderCapabilityMismatch
            }
        }
    }
}

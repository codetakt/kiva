#![allow(clippy::wildcard_imports)]

use super::*;

impl ConfidentialRepository {
    pub(crate) async fn reconstruct_shamir_threshold_secret(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShamirThresholdReconstructionRequest,
    ) -> Result<ConfidentialShamirThresholdReconstructionState, ConfidentialShamirReconstructionError>
    {
        dispatch_repository!(self, |repository| repository
            .reconstruct_shamir_threshold_secret(principal, request)
            .await)
    }

    pub(crate) async fn list_shamir_threshold_reconstructions(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialShamirThresholdReconstructionFilter,
    ) -> Result<
        Vec<ConfidentialShamirThresholdReconstructionState>,
        ConfidentialShamirReconstructionError,
    > {
        dispatch_repository!(self, |repository| repository
            .list_shamir_threshold_reconstructions(principal, filter)
            .await)
    }
}

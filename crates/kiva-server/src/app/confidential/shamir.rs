//! Shamir confidential-computation application methods.

use kiva_application::confidential as confidential_app;
use kiva_identity::AuthenticatedPrincipal;

use crate::{
    app::confidential::{ConfidentialApplicationAdapter, ConfidentialWorkflowPort},
    surfaces::confidential::{
        model::{
            ConfidentialShamirThresholdReconstructionFilter,
            ConfidentialShamirThresholdReconstructionRequest,
            ConfidentialShamirThresholdReconstructionState,
        },
        outcome::ConfidentialShamirReconstructionError,
    },
};

impl ConfidentialApplicationAdapter {
    pub(crate) async fn reconstruct_shamir_threshold_secret(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShamirThresholdReconstructionRequest,
    ) -> Result<ConfidentialShamirThresholdReconstructionState, ConfidentialShamirReconstructionError>
    {
        self.begin_workflow()
            .reconstruct_shamir_threshold_secret(principal, request)
            .await
    }

    pub(crate) async fn list_shamir_threshold_reconstructions(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialShamirThresholdReconstructionFilter,
    ) -> Result<
        Vec<ConfidentialShamirThresholdReconstructionState>,
        ConfidentialShamirReconstructionError,
    > {
        self.begin_workflow()
            .list_shamir_threshold_reconstructions(principal, filter)
            .await
    }
}

impl confidential_app::ConfidentialShamirRepository for ConfidentialWorkflowPort {
    type ReconstructionRequest = ConfidentialShamirThresholdReconstructionRequest;
    type ReconstructionFilter = ConfidentialShamirThresholdReconstructionFilter;
    type Reconstruction = ConfidentialShamirThresholdReconstructionState;
    type ReconstructionError = ConfidentialShamirReconstructionError;

    fn reconstruct_shamir_threshold_secret<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ReconstructionRequest,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<Self::Reconstruction, Self::ReconstructionError>,
    > {
        Box::pin(async move {
            self.repository
                .reconstruct_shamir_threshold_secret(principal, request)
                .await
        })
    }

    fn list_shamir_threshold_reconstructions<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: Self::ReconstructionFilter,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<Vec<Self::Reconstruction>, Self::ReconstructionError>,
    > {
        Box::pin(async move {
            self.repository
                .list_shamir_threshold_reconstructions(principal, filter)
                .await
        })
    }
}

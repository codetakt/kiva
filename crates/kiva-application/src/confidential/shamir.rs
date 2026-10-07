//! Shamir confidential-computation application workflow.

use kiva_identity::AuthenticatedPrincipal;

use super::{
    ApplicationFuture, ApplicationRuntimePort, ConfidentialApplicationError,
    ConfidentialApplicationWorkflow, ConfidentialWorkflowAdmissionRecorder,
    ConfidentialWorkflowKind, ConfidentialWorkflowRepository,
};

/// Port for Shamir threshold reconstruction workflows.
pub trait ConfidentialShamirRepository: ConfidentialWorkflowRepository {
    /// Reconstruction command.
    type ReconstructionRequest: Send + 'static;
    /// Reconstruction query filter.
    type ReconstructionFilter: Send + 'static;
    /// Reconstruction state view.
    type Reconstruction: Send + 'static;
    /// Reconstruction workflow error.
    type ReconstructionError: From<ConfidentialApplicationError> + Send + 'static;

    /// Reconstructs a threshold secret.
    fn reconstruct_shamir_threshold_secret<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ReconstructionRequest,
    ) -> ApplicationFuture<'a, Result<Self::Reconstruction, Self::ReconstructionError>>;

    /// Lists threshold reconstructions.
    fn list_shamir_threshold_reconstructions<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: Self::ReconstructionFilter,
    ) -> ApplicationFuture<'a, Result<Vec<Self::Reconstruction>, Self::ReconstructionError>>;
}

impl<R, D, A> ConfidentialApplicationWorkflow<R, D, A>
where
    R: ConfidentialShamirRepository,
    D: ConfidentialWorkflowAdmissionRecorder,
    A: ApplicationRuntimePort,
{
    /// Reconstructs a threshold secret after application-level workflow admission.
    pub async fn reconstruct_shamir_threshold_secret(
        &self,
        principal: &AuthenticatedPrincipal,
        request: R::ReconstructionRequest,
    ) -> Result<R::Reconstruction, R::ReconstructionError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ReconstructShamirThresholdSecret,
            |repository, _admission| async move {
                repository
                    .reconstruct_shamir_threshold_secret(principal, request)
                    .await
            },
        )
        .await
    }

    /// Lists threshold reconstructions after application-level workflow admission.
    pub async fn list_shamir_threshold_reconstructions(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: R::ReconstructionFilter,
    ) -> Result<Vec<R::Reconstruction>, R::ReconstructionError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ListShamirThresholdReconstructions,
            |repository, _admission| async move {
                repository
                    .list_shamir_threshold_reconstructions(principal, filter)
                    .await
            },
        )
        .await
    }
}

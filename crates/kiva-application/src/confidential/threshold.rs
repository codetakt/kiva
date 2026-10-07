//! Threshold-release confidential-computation application workflow.

use kiva_identity::AuthenticatedPrincipal;

use super::{
    ApplicationFuture, ApplicationRuntimePort, ConfidentialApplicationError,
    ConfidentialApplicationWorkflow, ConfidentialWorkflowAdmissionRecorder,
    ConfidentialWorkflowKind, ConfidentialWorkflowRepository,
};

/// Port for threshold release workflows.
pub trait ConfidentialThresholdRepository: ConfidentialWorkflowRepository {
    /// Threshold release registration command.
    type ThresholdReleaseRequest: Send + 'static;
    /// Threshold approval command.
    type ThresholdReleaseApprovalRequest: Send + 'static;
    /// Threshold release query filter.
    type ThresholdReleaseFilter: Send + 'static;
    /// Threshold release state view.
    type ThresholdRelease: Send + 'static;
    /// Store outcome.
    type StoreOutcome: Send + 'static;
    /// Threshold release workflow error.
    type ThresholdReleaseError: From<ConfidentialApplicationError> + Send + 'static;

    /// Registers a threshold release request.
    fn register_threshold_release_request<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ThresholdReleaseRequest,
    ) -> ApplicationFuture<'a, Result<Self::StoreOutcome, Self::ThresholdReleaseError>>;

    /// Approves a threshold release.
    fn approve_threshold_release<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ThresholdReleaseApprovalRequest,
    ) -> ApplicationFuture<'a, Result<Self::StoreOutcome, Self::ThresholdReleaseError>>;

    /// Releases an aggregate after threshold approval.
    fn release_threshold_aggregate<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ThresholdReleaseApprovalRequest,
    ) -> ApplicationFuture<'a, Result<Self::ThresholdRelease, Self::ThresholdReleaseError>>;

    /// Lists threshold release requests.
    fn list_threshold_release_requests(
        &self,
        filter: Self::ThresholdReleaseFilter,
    ) -> ApplicationFuture<'_, Result<Vec<Self::ThresholdRelease>, Self::ThresholdReleaseError>>;
}

impl<R, D, A> ConfidentialApplicationWorkflow<R, D, A>
where
    R: ConfidentialThresholdRepository,
    D: ConfidentialWorkflowAdmissionRecorder,
    A: ApplicationRuntimePort,
{
    /// Registers a threshold release request after application-level workflow admission.
    pub async fn register_threshold_release_request(
        &self,
        principal: &AuthenticatedPrincipal,
        request: R::ThresholdReleaseRequest,
    ) -> Result<R::StoreOutcome, R::ThresholdReleaseError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::RegisterThresholdReleaseRequest,
            |repository, _admission| async move {
                repository
                    .register_threshold_release_request(principal, request)
                    .await
            },
        )
        .await
    }

    /// Approves a threshold release after application-level workflow admission.
    pub async fn approve_threshold_release(
        &self,
        principal: &AuthenticatedPrincipal,
        request: R::ThresholdReleaseApprovalRequest,
    ) -> Result<R::StoreOutcome, R::ThresholdReleaseError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ApproveThresholdRelease,
            |repository, _admission| async move {
                repository
                    .approve_threshold_release(principal, request)
                    .await
            },
        )
        .await
    }

    /// Releases an aggregate after application-level workflow admission.
    pub async fn release_threshold_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        request: R::ThresholdReleaseApprovalRequest,
    ) -> Result<R::ThresholdRelease, R::ThresholdReleaseError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ReleaseThresholdAggregate,
            |repository, _admission| async move {
                repository
                    .release_threshold_aggregate(principal, request)
                    .await
            },
        )
        .await
    }

    /// Lists threshold release requests after application-level workflow admission.
    pub async fn list_threshold_release_requests(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: R::ThresholdReleaseFilter,
    ) -> Result<Vec<R::ThresholdRelease>, R::ThresholdReleaseError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ListThresholdReleaseRequests,
            |repository, _admission| async move {
                repository.list_threshold_release_requests(filter).await
            },
        )
        .await
    }
}

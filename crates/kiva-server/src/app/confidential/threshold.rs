//! Threshold release confidential application methods.

use kiva_application::confidential as confidential_app;
use kiva_identity::AuthenticatedPrincipal;

use crate::{
    app::confidential::{ConfidentialApplicationAdapter, ConfidentialWorkflowPort},
    core::outcome::StoreOutcome,
    surfaces::confidential::{
        model::{
            ConfidentialThresholdReleaseApprovalRequest, ConfidentialThresholdReleaseFilter,
            ConfidentialThresholdReleaseRequest, ConfidentialThresholdReleaseState,
        },
        outcome::ConfidentialThresholdReleaseError,
    },
};

impl ConfidentialApplicationAdapter {
    pub(crate) async fn register_threshold_release_request(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialThresholdReleaseRequest,
    ) -> Result<StoreOutcome, ConfidentialThresholdReleaseError> {
        self.begin_workflow()
            .register_threshold_release_request(principal, request)
            .await
    }

    pub(crate) async fn approve_threshold_release(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialThresholdReleaseApprovalRequest,
    ) -> Result<StoreOutcome, ConfidentialThresholdReleaseError> {
        self.begin_workflow()
            .approve_threshold_release(principal, request)
            .await
    }

    pub(crate) async fn release_threshold_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialThresholdReleaseApprovalRequest,
    ) -> Result<ConfidentialThresholdReleaseState, ConfidentialThresholdReleaseError> {
        self.begin_workflow()
            .release_threshold_aggregate(principal, request)
            .await
    }

    pub(crate) async fn list_threshold_release_requests(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialThresholdReleaseFilter,
    ) -> Result<Vec<ConfidentialThresholdReleaseState>, ConfidentialThresholdReleaseError> {
        self.begin_workflow()
            .list_threshold_release_requests(principal, filter)
            .await
    }
}

impl confidential_app::ConfidentialThresholdRepository for ConfidentialWorkflowPort {
    type ThresholdReleaseRequest = ConfidentialThresholdReleaseRequest;
    type ThresholdReleaseApprovalRequest = ConfidentialThresholdReleaseApprovalRequest;
    type ThresholdReleaseFilter = ConfidentialThresholdReleaseFilter;
    type ThresholdRelease = ConfidentialThresholdReleaseState;
    type StoreOutcome = StoreOutcome;
    type ThresholdReleaseError = ConfidentialThresholdReleaseError;

    fn register_threshold_release_request<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ThresholdReleaseRequest,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<Self::StoreOutcome, Self::ThresholdReleaseError>,
    > {
        Box::pin(async move {
            self.repository
                .register_threshold_release_request(principal, request)
                .await
        })
    }

    fn approve_threshold_release<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ThresholdReleaseApprovalRequest,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<Self::StoreOutcome, Self::ThresholdReleaseError>,
    > {
        Box::pin(async move {
            self.repository
                .approve_threshold_release(principal, request)
                .await
        })
    }

    fn release_threshold_aggregate<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ThresholdReleaseApprovalRequest,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<Self::ThresholdRelease, Self::ThresholdReleaseError>,
    > {
        Box::pin(async move {
            self.repository
                .release_threshold_aggregate(principal, request)
                .await
        })
    }

    fn list_threshold_release_requests(
        &self,
        filter: Self::ThresholdReleaseFilter,
    ) -> confidential_app::ApplicationFuture<
        '_,
        Result<Vec<Self::ThresholdRelease>, Self::ThresholdReleaseError>,
    > {
        Box::pin(async move {
            self.repository
                .list_threshold_release_requests(filter)
                .await
        })
    }
}

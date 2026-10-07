#![allow(clippy::wildcard_imports)]

use super::*;

impl ConfidentialRepository {
    pub(crate) async fn register_threshold_release_request(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialThresholdReleaseRequest,
    ) -> Result<StoreOutcome, ConfidentialThresholdReleaseError> {
        dispatch_repository!(self, |repository| repository
            .register_threshold_release_request(principal, request)
            .await)
    }

    pub(crate) async fn approve_threshold_release(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialThresholdReleaseApprovalRequest,
    ) -> Result<StoreOutcome, ConfidentialThresholdReleaseError> {
        dispatch_repository!(self, |repository| repository
            .approve_threshold_release(principal, request)
            .await)
    }

    pub(crate) async fn release_threshold_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialThresholdReleaseApprovalRequest,
    ) -> Result<ConfidentialThresholdReleaseState, ConfidentialThresholdReleaseError> {
        dispatch_repository!(self, |repository| repository
            .release_threshold_aggregate(principal, request)
            .await)
    }

    pub(crate) async fn list_threshold_release_requests(
        &self,
        filter: ConfidentialThresholdReleaseFilter,
    ) -> Result<Vec<ConfidentialThresholdReleaseState>, ConfidentialThresholdReleaseError> {
        dispatch_repository!(self, |repository| repository
            .list_threshold_release_requests(filter)
            .await)
    }
}

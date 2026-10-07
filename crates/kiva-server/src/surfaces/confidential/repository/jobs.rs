#![allow(clippy::wildcard_imports)]

use super::*;

impl ConfidentialRepository {
    pub(crate) async fn register_confidential_job(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialJobRequest,
    ) -> Result<ConfidentialJobRegistrationOutcome, ConfidentialJobError> {
        dispatch_repository!(self, |repository| repository
            .register_confidential_job(principal, request)
            .await)
    }

    pub(crate) async fn transition_confidential_job(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialJobTransitionRequest,
    ) -> Result<ConfidentialJobTransitionOutcome, ConfidentialJobError> {
        dispatch_repository!(self, |repository| repository
            .transition_confidential_job(principal, request)
            .await)
    }

    pub(crate) async fn list_confidential_jobs(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialJobFilter,
    ) -> Result<Vec<ConfidentialJobState>, ConfidentialJobError> {
        dispatch_repository!(self, |repository| repository
            .list_confidential_jobs(principal, filter)
            .await)
    }

    pub(crate) async fn list_confidential_job_audit_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialJobAuditEventFilter,
    ) -> Result<Vec<ConfidentialJobAuditEventState>, ConfidentialJobError> {
        dispatch_repository!(self, |repository| repository
            .list_confidential_job_audit_events(principal, filter)
            .await)
    }

    pub(crate) async fn release_artifact_is_available_for_pds_data_product(
        &self,
        principal: &AuthenticatedPrincipal,
        artifact: PdsReleaseArtifactRef,
    ) -> Result<bool, ConfidentialAggregateStoreError> {
        dispatch_repository!(self, |repository| {
            memory => Ok(repository
                .release_artifact_is_available_for_pds_data_product(principal, artifact)
                .await),
            postgres => repository
                .release_artifact_is_available_for_pds_data_product(principal, artifact)
                .await,
        })
    }
}

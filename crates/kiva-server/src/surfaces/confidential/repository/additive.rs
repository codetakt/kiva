#![allow(clippy::wildcard_imports)]

use super::*;

impl ConfidentialRepository {
    pub(crate) async fn list_share_signing_keys(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<Vec<ConfidentialShareSigningKey>, ConfidentialAggregateError> {
        dispatch_repository!(self, |repository| repository
            .list_share_signing_keys(principal)
            .await)
    }

    pub(crate) async fn list_replay_records(
        &self,
        transcript_id: SecureAggregateTranscriptId,
    ) -> Result<Vec<AcceptedAdditiveShareRecord>, ConfidentialAggregateError> {
        dispatch_repository!(self, |repository| repository
            .list_replay_records(transcript_id)
            .await)
    }

    pub(crate) async fn list_submitted_shares(
        &self,
        principal: &AuthenticatedPrincipal,
        transcript_id: SecureAggregateTranscriptId,
    ) -> Result<Vec<ConfidentialSubmittedShare>, ConfidentialAggregateError> {
        dispatch_repository!(self, |repository| repository
            .list_submitted_shares(principal, transcript_id)
            .await)
    }
    pub(crate) async fn register_share_signing_key(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSigningKeyRequest,
    ) -> Result<StoreOutcome, ConfidentialAggregateError> {
        dispatch_repository!(self, |repository| repository
            .register_share_signing_key(principal, request)
            .await)
    }

    pub(crate) async fn revoke_share_signing_key(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSigningKeyRevocationRequest,
    ) -> Result<ConfidentialShareSigningKeyRevocationOutcome, ConfidentialAggregateError> {
        dispatch_repository!(self, |repository| repository
            .revoke_share_signing_key(principal, request)
            .await)
    }

    pub(crate) async fn rotate_share_signing_key(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSigningKeyRotationRequest,
    ) -> Result<ConfidentialShareSigningKeyRotationOutcome, ConfidentialAggregateError> {
        dispatch_repository!(self, |repository| repository
            .rotate_share_signing_key(principal, request)
            .await)
    }

    pub(crate) async fn submit_additive_share(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSubmissionRequest,
    ) -> Result<StoreOutcome, ConfidentialShareSubmissionError> {
        dispatch_repository!(self, |repository| repository
            .submit_additive_share(principal, request)
            .await)
    }

    pub(crate) async fn release_signed_additive_aggregate(
        &self,
        request: &ConfidentialAdditiveAggregateRequest,
    ) -> Result<ReplayWindowedAdditiveAggregate, ConfidentialAggregateError> {
        dispatch_repository!(self, |repository| repository
            .release_signed_additive_aggregate(request)
            .await)
    }
}

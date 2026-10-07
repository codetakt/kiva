//! Additive confidential-computation application methods.

use kiva_application::confidential as confidential_app;
use kiva_confidential::{
    mechanism::additive::{AcceptedAdditiveShareRecord, ReplayWindowedAdditiveAggregate},
    primitive::SecureAggregateTranscriptId,
};
use kiva_identity::AuthenticatedPrincipal;

use crate::{
    app::confidential::{ConfidentialApplicationAdapter, ConfidentialWorkflowPort},
    core::outcome::StoreOutcome,
    surfaces::confidential::{
        model::{
            ConfidentialAdditiveAggregateRequest, ConfidentialShareSigningKey,
            ConfidentialShareSigningKeyRequest, ConfidentialShareSigningKeyRevocationOutcome,
            ConfidentialShareSigningKeyRevocationRequest,
            ConfidentialShareSigningKeyRotationOutcome, ConfidentialShareSigningKeyRotationRequest,
            ConfidentialShareSubmissionRequest, ConfidentialSubmittedShare,
        },
        outcome::{ConfidentialAggregateError, ConfidentialShareSubmissionError},
    },
};

impl ConfidentialApplicationAdapter {
    pub(crate) async fn list_share_signing_keys(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<Vec<ConfidentialShareSigningKey>, ConfidentialAggregateError> {
        self.begin_workflow()
            .list_share_signing_keys(principal)
            .await
    }

    pub(crate) async fn list_replay_records(
        &self,
        principal: &AuthenticatedPrincipal,
        transcript_id: SecureAggregateTranscriptId,
    ) -> Result<Vec<AcceptedAdditiveShareRecord>, ConfidentialAggregateError> {
        self.begin_workflow()
            .list_replay_records(principal, transcript_id)
            .await
    }

    pub(crate) async fn list_submitted_shares(
        &self,
        principal: &AuthenticatedPrincipal,
        transcript_id: SecureAggregateTranscriptId,
    ) -> Result<Vec<ConfidentialSubmittedShare>, ConfidentialAggregateError> {
        self.begin_workflow()
            .list_submitted_shares(principal, transcript_id)
            .await
    }

    pub(crate) async fn register_share_signing_key(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSigningKeyRequest,
    ) -> Result<StoreOutcome, ConfidentialAggregateError> {
        self.begin_workflow()
            .register_share_signing_key(principal, request)
            .await
    }

    pub(crate) async fn revoke_share_signing_key(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSigningKeyRevocationRequest,
    ) -> Result<ConfidentialShareSigningKeyRevocationOutcome, ConfidentialAggregateError> {
        self.begin_workflow()
            .revoke_share_signing_key(principal, request)
            .await
    }

    pub(crate) async fn rotate_share_signing_key(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSigningKeyRotationRequest,
    ) -> Result<ConfidentialShareSigningKeyRotationOutcome, ConfidentialAggregateError> {
        self.begin_workflow()
            .rotate_share_signing_key(principal, request)
            .await
    }

    pub(crate) async fn submit_additive_share(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSubmissionRequest,
    ) -> Result<StoreOutcome, ConfidentialShareSubmissionError> {
        self.begin_workflow()
            .submit_additive_share(principal, request)
            .await
    }

    pub(crate) async fn release_signed_additive_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        request: &ConfidentialAdditiveAggregateRequest,
    ) -> Result<ReplayWindowedAdditiveAggregate, ConfidentialAggregateError> {
        self.begin_workflow()
            .release_signed_additive_aggregate(principal, request)
            .await
    }
}

impl confidential_app::ConfidentialAdditiveRepository for ConfidentialWorkflowPort {
    type TranscriptId = SecureAggregateTranscriptId;
    type ShareSigningKey = ConfidentialShareSigningKey;
    type ReplayRecord = AcceptedAdditiveShareRecord;
    type SubmittedShare = ConfidentialSubmittedShare;
    type ShareSigningKeyRequest = ConfidentialShareSigningKeyRequest;
    type ShareSigningKeyRevocationRequest = ConfidentialShareSigningKeyRevocationRequest;
    type ShareSigningKeyRevocationOutcome = ConfidentialShareSigningKeyRevocationOutcome;
    type ShareSigningKeyRotationRequest = ConfidentialShareSigningKeyRotationRequest;
    type ShareSigningKeyRotationOutcome = ConfidentialShareSigningKeyRotationOutcome;
    type ShareSubmissionRequest = ConfidentialShareSubmissionRequest;
    type AggregateRequest = ConfidentialAdditiveAggregateRequest;
    type AggregateResult = ReplayWindowedAdditiveAggregate;
    type StoreOutcome = StoreOutcome;
    type AggregateError = ConfidentialAggregateError;
    type ShareSubmissionError = ConfidentialShareSubmissionError;

    fn list_share_signing_keys<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<Vec<Self::ShareSigningKey>, Self::AggregateError>,
    > {
        Box::pin(async move { self.repository.list_share_signing_keys(principal).await })
    }

    fn list_replay_records(
        &self,
        transcript_id: SecureAggregateTranscriptId,
    ) -> confidential_app::ApplicationFuture<
        '_,
        Result<Vec<Self::ReplayRecord>, Self::AggregateError>,
    > {
        Box::pin(async move { self.repository.list_replay_records(transcript_id).await })
    }

    fn list_submitted_shares<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        transcript_id: SecureAggregateTranscriptId,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<Vec<Self::SubmittedShare>, Self::AggregateError>,
    > {
        Box::pin(async move {
            self.repository
                .list_submitted_shares(principal, transcript_id)
                .await
        })
    }

    fn register_share_signing_key<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ShareSigningKeyRequest,
    ) -> confidential_app::ApplicationFuture<'a, Result<Self::StoreOutcome, Self::AggregateError>>
    {
        Box::pin(async move {
            self.repository
                .register_share_signing_key(principal, request)
                .await
        })
    }

    fn revoke_share_signing_key<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ShareSigningKeyRevocationRequest,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<Self::ShareSigningKeyRevocationOutcome, Self::AggregateError>,
    > {
        Box::pin(async move {
            self.repository
                .revoke_share_signing_key(principal, request)
                .await
        })
    }

    fn rotate_share_signing_key<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ShareSigningKeyRotationRequest,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<Self::ShareSigningKeyRotationOutcome, Self::AggregateError>,
    > {
        Box::pin(async move {
            self.repository
                .rotate_share_signing_key(principal, request)
                .await
        })
    }

    fn submit_additive_share<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ShareSubmissionRequest,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<Self::StoreOutcome, Self::ShareSubmissionError>,
    > {
        Box::pin(async move {
            self.repository
                .submit_additive_share(principal, request)
                .await
        })
    }

    fn release_signed_additive_aggregate<'a>(
        &'a self,
        request: &'a Self::AggregateRequest,
    ) -> confidential_app::ApplicationFuture<'a, Result<Self::AggregateResult, Self::AggregateError>>
    {
        Box::pin(async move {
            self.repository
                .release_signed_additive_aggregate(request)
                .await
        })
    }
}

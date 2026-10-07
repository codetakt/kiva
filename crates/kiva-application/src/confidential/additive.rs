//! Additive confidential-computation application workflow.

use kiva_identity::AuthenticatedPrincipal;

use super::{
    ApplicationFuture, ApplicationRuntimePort, ConfidentialApplicationError,
    ConfidentialApplicationWorkflow, ConfidentialWorkflowAdmissionRecorder,
    ConfidentialWorkflowKind, ConfidentialWorkflowRepository,
};

/// Port for additive-share and aggregate workflows.
pub trait ConfidentialAdditiveRepository: ConfidentialWorkflowRepository {
    /// Transcript identifier used by this adapter.
    type TranscriptId: Copy + Send + Sync + 'static;
    /// Share signing-key metadata view.
    type ShareSigningKey: Send + 'static;
    /// Accepted additive-share replay record view.
    type ReplayRecord: Send + 'static;
    /// Submitted share metadata view.
    type SubmittedShare: Send + 'static;
    /// Share signing-key registration command.
    type ShareSigningKeyRequest: Send + 'static;
    /// Share signing-key revocation command.
    type ShareSigningKeyRevocationRequest: Send + 'static;
    /// Share signing-key revocation outcome.
    type ShareSigningKeyRevocationOutcome: Send + 'static;
    /// Share signing-key rotation command.
    type ShareSigningKeyRotationRequest: Send + 'static;
    /// Share signing-key rotation outcome.
    type ShareSigningKeyRotationOutcome: Send + 'static;
    /// Submitted additive-share command.
    type ShareSubmissionRequest: Send + 'static;
    /// Aggregate release request.
    type AggregateRequest: ?Sized + Send + Sync + 'static;
    /// Aggregate release result.
    type AggregateResult: Send + 'static;
    /// Store outcome.
    type StoreOutcome: Send + 'static;
    /// Aggregate workflow error.
    type AggregateError: From<ConfidentialApplicationError> + Send + 'static;
    /// Share-submission workflow error.
    type ShareSubmissionError: From<ConfidentialApplicationError> + Send + 'static;

    /// Lists share signing keys visible to the principal.
    fn list_share_signing_keys<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
    ) -> ApplicationFuture<'a, Result<Vec<Self::ShareSigningKey>, Self::AggregateError>>;

    /// Lists replay records for a transcript.
    fn list_replay_records(
        &self,
        transcript_id: Self::TranscriptId,
    ) -> ApplicationFuture<'_, Result<Vec<Self::ReplayRecord>, Self::AggregateError>>;

    /// Lists submitted shares visible to the principal.
    fn list_submitted_shares<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        transcript_id: Self::TranscriptId,
    ) -> ApplicationFuture<'a, Result<Vec<Self::SubmittedShare>, Self::AggregateError>>;

    /// Registers a share signing key.
    fn register_share_signing_key<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ShareSigningKeyRequest,
    ) -> ApplicationFuture<'a, Result<Self::StoreOutcome, Self::AggregateError>>;

    /// Revokes a share signing key.
    fn revoke_share_signing_key<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ShareSigningKeyRevocationRequest,
    ) -> ApplicationFuture<'a, Result<Self::ShareSigningKeyRevocationOutcome, Self::AggregateError>>;

    /// Rotates a share signing key.
    fn rotate_share_signing_key<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ShareSigningKeyRotationRequest,
    ) -> ApplicationFuture<'a, Result<Self::ShareSigningKeyRotationOutcome, Self::AggregateError>>;

    /// Submits one additive share.
    fn submit_additive_share<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ShareSubmissionRequest,
    ) -> ApplicationFuture<'a, Result<Self::StoreOutcome, Self::ShareSubmissionError>>;

    /// Releases a signed additive aggregate.
    fn release_signed_additive_aggregate<'a>(
        &'a self,
        request: &'a Self::AggregateRequest,
    ) -> ApplicationFuture<'a, Result<Self::AggregateResult, Self::AggregateError>>;
}

impl<R, D, A> ConfidentialApplicationWorkflow<R, D, A>
where
    R: ConfidentialAdditiveRepository,
    D: ConfidentialWorkflowAdmissionRecorder,
    A: ApplicationRuntimePort,
{
    /// Lists share signing keys after application-level workflow admission.
    pub async fn list_share_signing_keys(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<Vec<R::ShareSigningKey>, R::AggregateError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ListShareSigningKeys,
            |repository, _admission| async move { repository.list_share_signing_keys(principal).await },
        )
        .await
    }

    /// Lists replay records after application-level workflow admission.
    pub async fn list_replay_records(
        &self,
        principal: &AuthenticatedPrincipal,
        transcript_id: R::TranscriptId,
    ) -> Result<Vec<R::ReplayRecord>, R::AggregateError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ListReplayRecords,
            |repository, _admission| async move { repository.list_replay_records(transcript_id).await },
        )
        .await
    }

    /// Lists submitted shares after application-level workflow admission.
    pub async fn list_submitted_shares(
        &self,
        principal: &AuthenticatedPrincipal,
        transcript_id: R::TranscriptId,
    ) -> Result<Vec<R::SubmittedShare>, R::AggregateError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ListSubmittedShares,
            |repository, _admission| async move {
                repository
                    .list_submitted_shares(principal, transcript_id)
                    .await
            },
        )
        .await
    }

    /// Registers a share signing key after application-level workflow admission.
    pub async fn register_share_signing_key(
        &self,
        principal: &AuthenticatedPrincipal,
        request: R::ShareSigningKeyRequest,
    ) -> Result<R::StoreOutcome, R::AggregateError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::RegisterShareSigningKey,
            |repository, _admission| async move {
                repository
                    .register_share_signing_key(principal, request)
                    .await
            },
        )
        .await
    }

    /// Revokes a share signing key after application-level workflow admission.
    pub async fn revoke_share_signing_key(
        &self,
        principal: &AuthenticatedPrincipal,
        request: R::ShareSigningKeyRevocationRequest,
    ) -> Result<R::ShareSigningKeyRevocationOutcome, R::AggregateError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::RevokeShareSigningKey,
            |repository, _admission| async move {
                repository
                    .revoke_share_signing_key(principal, request)
                    .await
            },
        )
        .await
    }

    /// Rotates a share signing key after application-level workflow admission.
    pub async fn rotate_share_signing_key(
        &self,
        principal: &AuthenticatedPrincipal,
        request: R::ShareSigningKeyRotationRequest,
    ) -> Result<R::ShareSigningKeyRotationOutcome, R::AggregateError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::RotateShareSigningKey,
            |repository, _admission| async move {
                repository
                    .rotate_share_signing_key(principal, request)
                    .await
            },
        )
        .await
    }

    /// Submits one additive share after application-level workflow admission.
    pub async fn submit_additive_share(
        &self,
        principal: &AuthenticatedPrincipal,
        request: R::ShareSubmissionRequest,
    ) -> Result<R::StoreOutcome, R::ShareSubmissionError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::SubmitAdditiveShare,
            |repository, _admission| async move {
                repository.submit_additive_share(principal, request).await
            },
        )
        .await
    }

    /// Releases a signed additive aggregate after application-level workflow admission.
    pub async fn release_signed_additive_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        request: &R::AggregateRequest,
    ) -> Result<R::AggregateResult, R::AggregateError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ReleaseSignedAdditiveAggregate,
            |repository, _admission| async move {
                repository.release_signed_additive_aggregate(request).await
            },
        )
        .await
    }
}

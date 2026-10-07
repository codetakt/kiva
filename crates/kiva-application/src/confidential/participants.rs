//! Confidential participant application workflow.

use kiva_identity::AuthenticatedPrincipal;

use super::{
    ApplicationFuture, ApplicationRuntimePort, ConfidentialApplicationError,
    ConfidentialApplicationWorkflow, ConfidentialWorkflowAdmissionRecorder,
    ConfidentialWorkflowKind, ConfidentialWorkflowRepository,
};

/// Port for confidential participant lifecycle workflows.
pub trait ConfidentialParticipantRepository: ConfidentialWorkflowRepository {
    /// Participant registration command.
    type ParticipantRequest: Send + 'static;
    /// Participant transition command.
    type ParticipantTransitionRequest: Send + 'static;
    /// Participant query filter.
    type ParticipantFilter: Send + 'static;
    /// Participant audit-event query filter.
    type ParticipantAuditEventFilter: Send + 'static;
    /// Participant state view.
    type Participant: Send + 'static;
    /// Participant audit-event state view.
    type ParticipantAuditEvent: Send + 'static;
    /// Participant registration outcome.
    type ParticipantRegistrationOutcome: Send + 'static;
    /// Participant transition outcome.
    type ParticipantTransitionOutcome: Send + 'static;
    /// Participant workflow error.
    type ParticipantError: From<ConfidentialApplicationError> + Send + 'static;

    /// Registers a participant.
    fn register_confidential_participant<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ParticipantRequest,
    ) -> ApplicationFuture<'a, Result<Self::ParticipantRegistrationOutcome, Self::ParticipantError>>;

    /// Transitions a participant.
    fn transition_confidential_participant<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ParticipantTransitionRequest,
    ) -> ApplicationFuture<'a, Result<Self::ParticipantTransitionOutcome, Self::ParticipantError>>;

    /// Lists participants.
    fn list_confidential_participants<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: Self::ParticipantFilter,
    ) -> ApplicationFuture<'a, Result<Vec<Self::Participant>, Self::ParticipantError>>;

    /// Lists participant audit events.
    fn list_confidential_participant_audit_events<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: Self::ParticipantAuditEventFilter,
    ) -> ApplicationFuture<'a, Result<Vec<Self::ParticipantAuditEvent>, Self::ParticipantError>>;
}

impl<R, D, A> ConfidentialApplicationWorkflow<R, D, A>
where
    R: ConfidentialParticipantRepository,
    D: ConfidentialWorkflowAdmissionRecorder,
    A: ApplicationRuntimePort,
{
    /// Registers a participant after application-level workflow admission.
    pub async fn register_confidential_participant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: R::ParticipantRequest,
    ) -> Result<R::ParticipantRegistrationOutcome, R::ParticipantError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::RegisterParticipant,
            |repository, _admission| async move {
                repository
                    .register_confidential_participant(principal, request)
                    .await
            },
        )
        .await
    }

    /// Transitions a participant after application-level workflow admission.
    pub async fn transition_confidential_participant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: R::ParticipantTransitionRequest,
    ) -> Result<R::ParticipantTransitionOutcome, R::ParticipantError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::TransitionParticipant,
            |repository, _admission| async move {
                repository
                    .transition_confidential_participant(principal, request)
                    .await
            },
        )
        .await
    }

    /// Lists participants after application-level workflow admission.
    pub async fn list_confidential_participants(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: R::ParticipantFilter,
    ) -> Result<Vec<R::Participant>, R::ParticipantError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ListParticipants,
            |repository, _admission| async move {
                repository
                    .list_confidential_participants(principal, filter)
                    .await
            },
        )
        .await
    }

    /// Lists participant audit events after application-level workflow admission.
    pub async fn list_confidential_participant_audit_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: R::ParticipantAuditEventFilter,
    ) -> Result<Vec<R::ParticipantAuditEvent>, R::ParticipantError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ListParticipantAuditEvents,
            |repository, _admission| async move {
                repository
                    .list_confidential_participant_audit_events(principal, filter)
                    .await
            },
        )
        .await
    }
}

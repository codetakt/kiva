//! Confidential participant application methods.

use kiva_application::confidential as confidential_app;
use kiva_identity::AuthenticatedPrincipal;

use crate::{
    app::confidential::{ConfidentialApplicationAdapter, ConfidentialWorkflowPort},
    surfaces::confidential::{
        model::{
            ConfidentialParticipantAuditEventFilter, ConfidentialParticipantAuditEventState,
            ConfidentialParticipantFilter, ConfidentialParticipantRequest,
            ConfidentialParticipantState, ConfidentialParticipantTransitionRequest,
        },
        outcome::{
            ConfidentialParticipantError, ConfidentialParticipantRegistrationOutcome,
            ConfidentialParticipantTransitionOutcome,
        },
    },
};

impl ConfidentialApplicationAdapter {
    pub(crate) async fn register_confidential_participant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialParticipantRequest,
    ) -> Result<ConfidentialParticipantRegistrationOutcome, ConfidentialParticipantError> {
        self.begin_workflow()
            .register_confidential_participant(principal, request)
            .await
    }

    pub(crate) async fn transition_confidential_participant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialParticipantTransitionRequest,
    ) -> Result<ConfidentialParticipantTransitionOutcome, ConfidentialParticipantError> {
        self.begin_workflow()
            .transition_confidential_participant(principal, request)
            .await
    }

    pub(crate) async fn list_confidential_participants(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialParticipantFilter,
    ) -> Result<Vec<ConfidentialParticipantState>, ConfidentialParticipantError> {
        self.begin_workflow()
            .list_confidential_participants(principal, filter)
            .await
    }

    pub(crate) async fn list_confidential_participant_audit_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialParticipantAuditEventFilter,
    ) -> Result<Vec<ConfidentialParticipantAuditEventState>, ConfidentialParticipantError> {
        self.begin_workflow()
            .list_confidential_participant_audit_events(principal, filter)
            .await
    }
}

impl confidential_app::ConfidentialParticipantRepository for ConfidentialWorkflowPort {
    type ParticipantRequest = ConfidentialParticipantRequest;
    type ParticipantTransitionRequest = ConfidentialParticipantTransitionRequest;
    type ParticipantFilter = ConfidentialParticipantFilter;
    type ParticipantAuditEventFilter = ConfidentialParticipantAuditEventFilter;
    type Participant = ConfidentialParticipantState;
    type ParticipantAuditEvent = ConfidentialParticipantAuditEventState;
    type ParticipantRegistrationOutcome = ConfidentialParticipantRegistrationOutcome;
    type ParticipantTransitionOutcome = ConfidentialParticipantTransitionOutcome;
    type ParticipantError = ConfidentialParticipantError;

    fn register_confidential_participant<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ParticipantRequest,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<Self::ParticipantRegistrationOutcome, Self::ParticipantError>,
    > {
        Box::pin(async move {
            self.repository
                .register_confidential_participant(principal, request)
                .await
        })
    }

    fn transition_confidential_participant<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ParticipantTransitionRequest,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<Self::ParticipantTransitionOutcome, Self::ParticipantError>,
    > {
        Box::pin(async move {
            self.repository
                .transition_confidential_participant(principal, request)
                .await
        })
    }

    fn list_confidential_participants<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: Self::ParticipantFilter,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<Vec<Self::Participant>, Self::ParticipantError>,
    > {
        Box::pin(async move {
            self.repository
                .list_confidential_participants(principal, filter)
                .await
        })
    }

    fn list_confidential_participant_audit_events<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: Self::ParticipantAuditEventFilter,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<Vec<Self::ParticipantAuditEvent>, Self::ParticipantError>,
    > {
        Box::pin(async move {
            self.repository
                .list_confidential_participant_audit_events(principal, filter)
                .await
        })
    }
}

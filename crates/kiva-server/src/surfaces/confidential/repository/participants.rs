#![allow(clippy::wildcard_imports)]

use super::*;

impl ConfidentialRepository {
    pub(crate) async fn register_confidential_participant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialParticipantRequest,
    ) -> Result<ConfidentialParticipantRegistrationOutcome, ConfidentialParticipantError> {
        dispatch_repository!(self, |repository| repository
            .register_confidential_participant(principal, request)
            .await)
    }

    pub(crate) async fn transition_confidential_participant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialParticipantTransitionRequest,
    ) -> Result<ConfidentialParticipantTransitionOutcome, ConfidentialParticipantError> {
        dispatch_repository!(self, |repository| repository
            .transition_confidential_participant(principal, request)
            .await)
    }

    pub(crate) async fn list_confidential_participants(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialParticipantFilter,
    ) -> Result<Vec<ConfidentialParticipantState>, ConfidentialParticipantError> {
        dispatch_repository!(self, |repository| repository
            .list_confidential_participants(principal, filter)
            .await)
    }

    pub(crate) async fn list_confidential_participant_audit_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialParticipantAuditEventFilter,
    ) -> Result<Vec<ConfidentialParticipantAuditEventState>, ConfidentialParticipantError> {
        dispatch_repository!(self, |repository| repository
            .list_confidential_participant_audit_events(principal, filter)
            .await)
    }
}

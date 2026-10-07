#![allow(clippy::wildcard_imports)]

use super::*;

impl MemoryConfidentialRepository {
    pub(crate) async fn register_confidential_participant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialParticipantRequest,
    ) -> Result<ConfidentialParticipantRegistrationOutcome, ConfidentialParticipantError> {
        let participant = ConfidentialParticipantState::from_request(principal, request)?;
        let key = ConfidentialParticipantKey::from_participant(&participant);
        let mut participants = self.participants.states.write().await;
        let outcome = match participants.get(&key) {
            Some(existing) if existing.same_registration(&participant) => {
                ConfidentialParticipantRegistrationOutcome::Unchanged(existing.clone())
            }
            Some(_existing) => ConfidentialParticipantRegistrationOutcome::Conflict,
            None => {
                participants.insert(key, participant.clone());
                ConfidentialParticipantRegistrationOutcome::Stored(participant.clone())
            }
        };
        drop(participants);
        if matches!(
            outcome,
            ConfidentialParticipantRegistrationOutcome::Stored(_)
        ) {
            self.record_confidential_participant_audit_event(
                principal,
                participant.participant_subject.clone(),
                participant.participant_index,
                None,
                participant.status,
                "participant invited",
                &participant.evidence_refs,
            )
            .await;
        }
        Ok(outcome)
    }

    pub(crate) async fn transition_confidential_participant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialParticipantTransitionRequest,
    ) -> Result<ConfidentialParticipantTransitionOutcome, ConfidentialParticipantError> {
        validate_confidential_participant_transition_request(&request)?;
        let key = ConfidentialParticipantKey::new(
            &principal.subject,
            request.participant_subject.trim(),
            request.participant_index,
        );
        let mut participants = self.participants.states.write().await;
        let Some(participant) = participants.get_mut(&key) else {
            return Err(ConfidentialParticipantError::NotFound);
        };
        participant.ensure_owner(principal)?;
        if participant.status == request.target_status {
            return Ok(ConfidentialParticipantTransitionOutcome::Unchanged(
                participant.clone(),
            ));
        }
        let previous_status = participant.status;
        participant.status =
            transition_confidential_participant_status(participant.status, request.target_status)
                .map_err(ConfidentialParticipantError::Transition)?;
        participant.updated_at = OffsetDateTime::now_utc();
        append_unique_strings(&mut participant.evidence_refs, &request.evidence_refs);
        let participant = participant.clone();
        drop(participants);
        self.record_confidential_participant_audit_event(
            principal,
            participant.participant_subject.clone(),
            participant.participant_index,
            Some(previous_status),
            participant.status,
            request
                .reason
                .unwrap_or_else(|| "participant lifecycle transition".to_owned()),
            &request.evidence_refs,
        )
        .await;
        Ok(ConfidentialParticipantTransitionOutcome::Updated(
            participant,
        ))
    }

    pub(crate) async fn list_confidential_participants(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialParticipantFilter,
    ) -> Result<Vec<ConfidentialParticipantState>, ConfidentialParticipantError> {
        let mut participants = {
            let participants = self.participants.states.read().await;
            participants
                .values()
                .filter(|participant| participant.owner_subject == principal.subject)
                .filter(|participant| filter.matches(participant))
                .cloned()
                .collect::<Vec<_>>()
        };
        participants.sort_by_key(|participant| {
            (
                participant.participant_subject.clone(),
                participant.participant_index.get(),
            )
        });
        Ok(participants)
    }

    pub(crate) async fn list_confidential_participant_audit_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialParticipantAuditEventFilter,
    ) -> Result<Vec<ConfidentialParticipantAuditEventState>, ConfidentialParticipantError> {
        let participant_subjects = {
            let participants = self.participants.states.read().await;
            participants
                .values()
                .filter(|participant| participant.owner_subject == principal.subject)
                .map(|participant| {
                    (
                        participant.participant_subject.clone(),
                        participant.participant_index,
                    )
                })
                .collect::<BTreeSet<_>>()
        };
        let mut events = {
            let events = self.participants.audit_events.read().await;
            events
                .values()
                .filter(|event| {
                    participant_subjects
                        .contains(&(event.participant_subject.clone(), event.participant_index))
                })
                .filter(|event| filter.matches(event))
                .cloned()
                .collect::<Vec<_>>()
        };
        events.sort_by_key(|event| (std::cmp::Reverse(event.created_at), event.event_id));
        Ok(events)
    }
    pub(crate) async fn ensure_active_confidential_participant(
        &self,
        owner_subject: &str,
        participant_subject: &str,
        participant_index: std::num::NonZeroUsize,
    ) -> Result<(), ConfidentialParticipantError> {
        let key =
            ConfidentialParticipantKey::new(owner_subject, participant_subject, participant_index);
        let status = {
            let participants = self.participants.states.read().await;
            participants.get(&key).map(|participant| participant.status)
        };
        let Some(status) = status else {
            return Err(ConfidentialParticipantError::NotFound);
        };
        if confidential_participant_accepts_new_keys(status) {
            Ok(())
        } else {
            Err(ConfidentialParticipantError::Inactive)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn record_confidential_participant_audit_event(
        &self,
        principal: &AuthenticatedPrincipal,
        participant_subject: String,
        participant_index: std::num::NonZeroUsize,
        previous_status: Option<ConfidentialParticipantStatus>,
        new_status: ConfidentialParticipantStatus,
        reason: impl Into<String>,
        evidence_refs: &[String],
    ) {
        let event = ConfidentialParticipantAuditEventState::new(
            participant_subject,
            participant_index,
            previous_status,
            new_status,
            principal,
            reason,
            evidence_refs.to_vec(),
        );
        let key = ConfidentialParticipantAuditEventKey::from_event_id(event.event_id);
        let mut events = self.participants.audit_events.write().await;
        events.insert(key, event);
    }
}

//! Confidential participant response projection.

use serde_json::Value;

use kiva_confidential::mechanism::workflow::ConfidentialParticipantStatus;
use kiva_confidential_contract::{
    ConfidentialParticipantAuditEvent, ConfidentialParticipantAuditEventList,
    ConfidentialParticipantMetadata, ConfidentialParticipantMetadataList,
};

use crate::{
    core::response::rfc3339,
    surfaces::confidential::model::{
        ConfidentialParticipantAuditEventState, ConfidentialParticipantState,
    },
};

use super::dto_value;

pub(crate) fn participant_response(
    participant: &ConfidentialParticipantState,
) -> Result<Value, serde_json::Error> {
    dto_value(&participant_metadata(participant))
}

pub(crate) fn participant_list_response(
    participants: &[ConfidentialParticipantState],
) -> Result<Value, serde_json::Error> {
    dto_value(&ConfidentialParticipantMetadataList {
        participants: participants.iter().map(participant_metadata).collect(),
    })
}

pub(crate) fn participant_audit_event_list_response(
    events: &[ConfidentialParticipantAuditEventState],
) -> Result<Value, serde_json::Error> {
    dto_value(&ConfidentialParticipantAuditEventList {
        events: events.iter().map(participant_audit_event).collect(),
    })
}

fn participant_metadata(
    participant: &ConfidentialParticipantState,
) -> ConfidentialParticipantMetadata {
    ConfidentialParticipantMetadata {
        owner_subject: participant.owner_subject.clone(),
        participant_subject: participant.participant_subject.clone(),
        participant_index: participant.participant_index.get(),
        status: participant.status.label().to_owned(),
        mechanism_roles: participant.mechanism_roles.clone(),
        authorization_scope: participant.authorization_scope.clone(),
        key_ceremony_ref: participant.key_ceremony_ref.clone(),
        recovery_contact_ref: participant.recovery_contact_ref.clone(),
        evidence_refs: participant.evidence_refs.clone(),
        created_at: rfc3339(participant.created_at),
        updated_at: rfc3339(participant.updated_at),
    }
}

fn participant_audit_event(
    event: &ConfidentialParticipantAuditEventState,
) -> ConfidentialParticipantAuditEvent {
    ConfidentialParticipantAuditEvent {
        event_id: event.event_id.to_string(),
        participant_subject: event.participant_subject.clone(),
        participant_index: event.participant_index.get(),
        previous_status: event
            .previous_status
            .map(ConfidentialParticipantStatus::label)
            .map(str::to_owned),
        new_status: event.new_status.label().to_owned(),
        actor_subject: event.actor_subject.clone(),
        reason: event.reason.clone(),
        evidence_refs: event.evidence_refs.clone(),
        created_at: rfc3339(event.created_at),
    }
}

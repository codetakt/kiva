//! Confidential-participant `OpenAPI` request DTO conversion.

use kiva_confidential_contract as contract;

use crate::surfaces::confidential::model as domain;

use super::common::confidential_participant_status;

pub(crate) fn confidential_participant_request_from_openapi(
    request: contract::ConfidentialParticipantRequest,
) -> domain::ConfidentialParticipantRequest {
    domain::ConfidentialParticipantRequest {
        participant_subject: request.participant_subject,
        participant_index: request.participant_index,
        mechanism_roles: request
            .mechanism_roles
            .into_iter()
            .map(|role| role.label().to_owned())
            .collect(),
        authorization_scope: request.authorization_scope,
        key_ceremony_ref: request.key_ceremony_ref,
        recovery_contact_ref: request.recovery_contact_ref,
        evidence_refs: request.evidence_refs,
    }
}

pub(crate) fn confidential_participant_transition_request_from_openapi(
    request: contract::ConfidentialParticipantTransitionRequest,
) -> domain::ConfidentialParticipantTransitionRequest {
    domain::ConfidentialParticipantTransitionRequest {
        participant_subject: request.participant_subject,
        participant_index: request.participant_index,
        target_status: confidential_participant_status(request.target_status),
        reason: request.reason,
        evidence_refs: request.evidence_refs,
    }
}

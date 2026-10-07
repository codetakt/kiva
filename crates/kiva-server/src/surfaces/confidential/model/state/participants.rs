//! Confidential participants server-local state.

#![allow(clippy::wildcard_imports)]

use super::super::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ConfidentialParticipantRequest {
    pub(crate) participant_subject: String,
    pub(crate) participant_index: std::num::NonZeroUsize,
    pub(crate) mechanism_roles: Vec<String>,
    pub(crate) authorization_scope: String,
    pub(crate) key_ceremony_ref: String,
    pub(crate) recovery_contact_ref: String,
    #[serde(default)]
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ConfidentialParticipantTransitionRequest {
    pub(crate) participant_subject: String,
    pub(crate) participant_index: std::num::NonZeroUsize,
    pub(crate) target_status: ConfidentialParticipantStatus,
    #[serde(default)]
    pub(crate) reason: Option<String>,
    #[serde(default)]
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfidentialParticipantState {
    pub(crate) owner_subject: String,
    pub(crate) participant_subject: String,
    pub(crate) participant_index: std::num::NonZeroUsize,
    pub(crate) status: ConfidentialParticipantStatus,
    pub(crate) mechanism_roles: Vec<String>,
    pub(crate) authorization_scope: String,
    pub(crate) key_ceremony_ref: String,
    pub(crate) recovery_contact_ref: String,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) updated_at: OffsetDateTime,
}

impl ConfidentialParticipantState {
    pub(crate) fn from_request(
        principal: &AuthenticatedPrincipal,
        request: ConfidentialParticipantRequest,
    ) -> Result<Self, ConfidentialParticipantError> {
        validate_non_empty_confidential_participant_text(
            "participant_subject",
            &request.participant_subject,
        )?;
        validate_non_empty_confidential_participant_text(
            "authorization_scope",
            &request.authorization_scope,
        )?;
        validate_non_empty_confidential_participant_text(
            "key_ceremony_ref",
            &request.key_ceremony_ref,
        )?;
        validate_non_empty_confidential_participant_text(
            "recovery_contact_ref",
            &request.recovery_contact_ref,
        )?;
        validate_confidential_participant_roles(&request.mechanism_roles)?;
        validate_confidential_participant_evidence_refs(&request.evidence_refs)?;
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            owner_subject: principal.subject.clone(),
            participant_subject: request.participant_subject.trim().to_owned(),
            participant_index: request.participant_index,
            status: ConfidentialParticipantStatus::Invited,
            mechanism_roles: canonical_unique_strings(request.mechanism_roles),
            authorization_scope: request.authorization_scope.trim().to_owned(),
            key_ceremony_ref: request.key_ceremony_ref.trim().to_owned(),
            recovery_contact_ref: request.recovery_contact_ref.trim().to_owned(),
            evidence_refs: canonical_unique_strings(request.evidence_refs),
            created_at: now,
            updated_at: now,
        })
    }

    pub(crate) fn same_registration(&self, other: &Self) -> bool {
        let requested_evidence_is_retained = other
            .evidence_refs
            .iter()
            .all(|evidence_ref| self.evidence_refs.contains(evidence_ref));
        self.owner_subject == other.owner_subject
            && self.participant_subject == other.participant_subject
            && self.participant_index == other.participant_index
            && self.mechanism_roles == other.mechanism_roles
            && self.authorization_scope == other.authorization_scope
            && self.key_ceremony_ref == other.key_ceremony_ref
            && self.recovery_contact_ref == other.recovery_contact_ref
            && requested_evidence_is_retained
    }

    pub(crate) fn ensure_owner(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<(), ConfidentialParticipantError> {
        if self.owner_subject == principal.subject {
            Ok(())
        } else {
            Err(ConfidentialParticipantError::Forbidden)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfidentialParticipantAuditEventState {
    pub(crate) event_id: Uuid,
    pub(crate) participant_subject: String,
    pub(crate) participant_index: std::num::NonZeroUsize,
    pub(crate) previous_status: Option<ConfidentialParticipantStatus>,
    pub(crate) new_status: ConfidentialParticipantStatus,
    pub(crate) actor_subject: String,
    pub(crate) reason: String,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: OffsetDateTime,
}

impl ConfidentialParticipantAuditEventState {
    pub(crate) fn new(
        participant_subject: String,
        participant_index: std::num::NonZeroUsize,
        previous_status: Option<ConfidentialParticipantStatus>,
        new_status: ConfidentialParticipantStatus,
        principal: &AuthenticatedPrincipal,
        reason: impl Into<String>,
        evidence_refs: Vec<String>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            participant_subject,
            participant_index,
            previous_status,
            new_status,
            actor_subject: principal.subject.clone(),
            reason: reason.into(),
            evidence_refs: canonical_unique_strings(evidence_refs),
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

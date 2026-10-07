//! Confidential jobs server-local state.

#![allow(clippy::wildcard_imports)]

use super::super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfidentialJobState {
    pub(crate) job_id: SecureAggregateTranscriptId,
    pub(crate) owner_subject: String,
    pub(crate) mechanism: String,
    pub(crate) mechanism_profile: String,
    pub(crate) workload_id: String,
    pub(crate) participant_subjects: Vec<String>,
    pub(crate) policy_binding_id: String,
    pub(crate) status: ConfidentialJobStatus,
    pub(crate) retry_count: i64,
    pub(crate) max_retry_count: i64,
    pub(crate) deadline_at: OffsetDateTime,
    pub(crate) admission_quota_key: String,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) updated_at: OffsetDateTime,
}

impl ConfidentialJobState {
    pub(crate) fn from_request(
        principal: &AuthenticatedPrincipal,
        request: ConfidentialJobRequest,
    ) -> Result<Self, ConfidentialJobError> {
        validate_confidential_job_mechanism(&request.mechanism)?;
        validate_non_empty_confidential_job_text("mechanism_profile", &request.mechanism_profile)?;
        validate_non_empty_confidential_job_text("workload_id", &request.workload_id)?;
        validate_non_empty_confidential_job_text("policy_binding_id", &request.policy_binding_id)?;
        validate_non_empty_confidential_job_text(
            "admission_quota_key",
            &request.admission_quota_key,
        )?;
        validate_confidential_job_subjects(&request.participant_subjects)?;
        validate_confidential_job_evidence_refs(&request.evidence_refs)?;
        if request.max_retry_count < 1 {
            return Err(ConfidentialJobError::InvalidRequest(
                "confidential job max_retry_count must be at least 1".to_owned(),
            ));
        }
        let deadline_at =
            OffsetDateTime::parse(&request.deadline_at, &Rfc3339).map_err(|_error| {
                ConfidentialJobError::InvalidRequest(
                    "confidential job deadline_at must be an RFC3339 timestamp".to_owned(),
                )
            })?;
        let deadline_at = postgres_microsecond_timestamp(deadline_at);
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            job_id: request.job_id,
            owner_subject: principal.subject.clone(),
            mechanism: request.mechanism,
            mechanism_profile: request.mechanism_profile,
            workload_id: request.workload_id,
            participant_subjects: canonical_confidential_job_subjects(request.participant_subjects),
            policy_binding_id: request.policy_binding_id,
            status: ConfidentialJobStatus::Draft,
            retry_count: 0,
            max_retry_count: request.max_retry_count,
            deadline_at,
            admission_quota_key: request.admission_quota_key,
            evidence_refs: canonical_unique_strings(request.evidence_refs),
            created_at: now,
            updated_at: now,
        })
    }

    pub(crate) fn same_registration(&self, other: &Self) -> bool {
        self.job_id == other.job_id
            && self.owner_subject == other.owner_subject
            && self.mechanism == other.mechanism
            && self.mechanism_profile == other.mechanism_profile
            && self.workload_id == other.workload_id
            && self.participant_subjects == other.participant_subjects
            && self.policy_binding_id == other.policy_binding_id
            && self.max_retry_count == other.max_retry_count
            && self.deadline_at == other.deadline_at
            && self.admission_quota_key == other.admission_quota_key
            && self.evidence_refs == other.evidence_refs
    }

    pub(crate) fn ensure_owner(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<(), ConfidentialJobError> {
        if self.owner_subject == principal.subject {
            Ok(())
        } else {
            Err(ConfidentialJobError::Forbidden)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfidentialJobAuditEventState {
    pub(crate) event_id: Uuid,
    pub(crate) job_id: SecureAggregateTranscriptId,
    pub(crate) previous_status: Option<ConfidentialJobStatus>,
    pub(crate) new_status: ConfidentialJobStatus,
    pub(crate) actor_subject: String,
    pub(crate) reason: String,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: OffsetDateTime,
}

impl ConfidentialJobAuditEventState {
    pub(crate) fn new(
        job_id: SecureAggregateTranscriptId,
        previous_status: Option<ConfidentialJobStatus>,
        new_status: ConfidentialJobStatus,
        actor_subject: String,
        reason: String,
        evidence_refs: Vec<String>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            job_id,
            previous_status,
            new_status,
            actor_subject,
            reason,
            evidence_refs: canonical_unique_strings(evidence_refs),
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

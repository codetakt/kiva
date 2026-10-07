//! Confidential job response projection.

use serde_json::Value;

use kiva_confidential::mechanism::job::ConfidentialJobStatus;
use kiva_confidential_contract::{
    ConfidentialJobAuditEvent, ConfidentialJobAuditEventList, ConfidentialJobMetadata,
    ConfidentialJobMetadataList,
};

use crate::{
    core::response::{hex_lower, rfc3339},
    surfaces::confidential::model::{ConfidentialJobAuditEventState, ConfidentialJobState},
};

use super::dto_value;

pub(crate) fn job_response(job: &ConfidentialJobState) -> Result<Value, serde_json::Error> {
    dto_value(&job_metadata(job))
}

pub(crate) fn job_list_response(jobs: &[ConfidentialJobState]) -> Result<Value, serde_json::Error> {
    dto_value(&ConfidentialJobMetadataList {
        jobs: jobs.iter().map(job_metadata).collect(),
    })
}

pub(crate) fn job_audit_event_list_response(
    events: &[ConfidentialJobAuditEventState],
) -> Result<Value, serde_json::Error> {
    dto_value(&ConfidentialJobAuditEventList {
        events: events.iter().map(job_audit_event).collect(),
    })
}

fn job_metadata(job: &ConfidentialJobState) -> ConfidentialJobMetadata {
    ConfidentialJobMetadata {
        job_id: hex_lower(&job.job_id.bytes()),
        owner_subject: job.owner_subject.clone(),
        mechanism: job.mechanism.clone(),
        mechanism_profile: job.mechanism_profile.clone(),
        workload_id: job.workload_id.clone(),
        participant_subjects: job.participant_subjects.clone(),
        policy_binding_id: job.policy_binding_id.clone(),
        status: job.status.label().to_owned(),
        retry_count: job.retry_count,
        max_retry_count: job.max_retry_count,
        deadline_at: rfc3339(job.deadline_at),
        admission_quota_key: job.admission_quota_key.clone(),
        evidence_refs: job.evidence_refs.clone(),
        created_at: rfc3339(job.created_at),
        updated_at: rfc3339(job.updated_at),
    }
}

fn job_audit_event(event: &ConfidentialJobAuditEventState) -> ConfidentialJobAuditEvent {
    ConfidentialJobAuditEvent {
        event_id: event.event_id.to_string(),
        job_id: hex_lower(&event.job_id.bytes()),
        previous_status: event
            .previous_status
            .map(ConfidentialJobStatus::label)
            .map(str::to_owned),
        new_status: event.new_status.label().to_owned(),
        actor_subject: event.actor_subject.clone(),
        reason: event.reason.clone(),
        evidence_refs: event.evidence_refs.clone(),
        created_at: rfc3339(event.created_at),
    }
}

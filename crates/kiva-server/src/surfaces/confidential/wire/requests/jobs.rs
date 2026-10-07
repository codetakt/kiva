//! Confidential-job `OpenAPI` request DTO conversion.

use kiva_confidential_contract as contract;

use crate::{core::response::HttpError, surfaces::confidential::model as domain};

use super::common::{confidential_job_status, transcript_id};

pub(crate) fn confidential_job_request_from_openapi(
    request: contract::ConfidentialJobRequest,
) -> Result<domain::ConfidentialJobRequest, HttpError> {
    Ok(domain::ConfidentialJobRequest {
        job_id: transcript_id(request.job_id, "job_id")?,
        mechanism: request.mechanism.label().to_owned(),
        mechanism_profile: request.mechanism_profile,
        workload_id: request.workload_id,
        participant_subjects: request.participant_subjects,
        policy_binding_id: request.policy_binding_id,
        deadline_at: request.deadline_at,
        admission_quota_key: request.admission_quota_key,
        max_retry_count: request.max_retry_count,
        evidence_refs: request.evidence_refs,
    })
}

pub(crate) fn confidential_job_transition_request_from_openapi(
    request: contract::ConfidentialJobTransitionRequest,
) -> Result<domain::ConfidentialJobTransitionRequest, HttpError> {
    Ok(domain::ConfidentialJobTransitionRequest {
        job_id: transcript_id(request.job_id, "job_id")?,
        target_status: confidential_job_status(request.target_status),
        reason: request.reason,
        evidence_refs: request.evidence_refs,
    })
}

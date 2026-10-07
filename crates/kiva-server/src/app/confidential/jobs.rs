//! Confidential job application methods and repository adapter.

use kiva_application::confidential as confidential_app;
use kiva_confidential::{
    mechanism::job::ConfidentialJobStatus as MechanismJobStatus,
    primitive::SecureAggregateTranscriptId as MechanismTranscriptId,
};
use kiva_identity::AuthenticatedPrincipal;

use crate::{
    app::confidential::{ConfidentialApplicationAdapter, ConfidentialWorkflowPort},
    surfaces::confidential::{
        model::{
            ConfidentialJobAuditEventFilter, ConfidentialJobAuditEventState, ConfidentialJobFilter,
            ConfidentialJobRequest, ConfidentialJobState, ConfidentialJobTransitionRequest,
        },
        outcome::{
            ConfidentialJobError, ConfidentialJobRegistrationOutcome,
            ConfidentialJobTransitionOutcome,
        },
    },
};

impl ConfidentialApplicationAdapter {
    pub(crate) async fn register_confidential_job(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialJobRequest,
    ) -> Result<ConfidentialJobRegistrationOutcome, ConfidentialJobError> {
        let command = registration_command_from_request(request);
        self.begin_workflow()
            .register_confidential_job(principal, command)
            .await
    }

    pub(crate) async fn transition_confidential_job(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialJobTransitionRequest,
    ) -> Result<ConfidentialJobTransitionOutcome, ConfidentialJobError> {
        let command = transition_command_from_request(request);
        self.begin_workflow()
            .transition_confidential_job(principal, command)
            .await
    }

    pub(crate) async fn list_confidential_jobs(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialJobFilter,
    ) -> Result<Vec<ConfidentialJobState>, ConfidentialJobError> {
        let filter = job_filter_to_application(filter);
        self.begin_workflow()
            .list_confidential_jobs(principal, filter)
            .await
    }

    pub(crate) async fn list_confidential_job_audit_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialJobAuditEventFilter,
    ) -> Result<Vec<ConfidentialJobAuditEventState>, ConfidentialJobError> {
        let filter = job_audit_filter_to_application(filter);
        self.begin_workflow()
            .list_confidential_job_audit_events(principal, filter)
            .await
    }
}

impl confidential_app::ConfidentialJobRepository for ConfidentialWorkflowPort {
    type Job = ConfidentialJobState;
    type JobAuditEvent = ConfidentialJobAuditEventState;
    type JobRegistrationOutcome = ConfidentialJobRegistrationOutcome;
    type JobTransitionOutcome = ConfidentialJobTransitionOutcome;
    type JobError = ConfidentialJobError;

    fn register_confidential_job<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        command: confidential_app::ConfidentialJobRegistrationCommand,
    ) -> confidential_app::ApplicationFuture<'a, Result<Self::JobRegistrationOutcome, Self::JobError>>
    {
        Box::pin(async move {
            let request = request_from_registration_command(command)?;
            self.repository
                .register_confidential_job(principal, request)
                .await
        })
    }

    fn transition_confidential_job<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        command: confidential_app::ConfidentialJobTransitionCommand,
    ) -> confidential_app::ApplicationFuture<'a, Result<Self::JobTransitionOutcome, Self::JobError>>
    {
        Box::pin(async move {
            let request = request_from_transition_command(command)?;
            self.repository
                .transition_confidential_job(principal, request)
                .await
        })
    }

    fn list_confidential_jobs<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: confidential_app::ConfidentialJobFilter,
    ) -> confidential_app::ApplicationFuture<'a, Result<Vec<Self::Job>, Self::JobError>> {
        let filter = job_filter_from_application(filter);
        Box::pin(async move {
            self.repository
                .list_confidential_jobs(principal, filter)
                .await
        })
    }

    fn list_confidential_job_audit_events<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: confidential_app::ConfidentialJobAuditEventFilter,
    ) -> confidential_app::ApplicationFuture<'a, Result<Vec<Self::JobAuditEvent>, Self::JobError>>
    {
        let filter = job_audit_filter_from_application(filter);
        Box::pin(async move {
            self.repository
                .list_confidential_job_audit_events(principal, filter)
                .await
        })
    }
}

fn registration_command_from_request(
    request: ConfidentialJobRequest,
) -> confidential_app::ConfidentialJobRegistrationCommand {
    confidential_app::ConfidentialJobRegistrationCommand {
        job_id: application_transcript_id(request.job_id),
        mechanism: request.mechanism,
        mechanism_profile: request.mechanism_profile,
        workload_id: request.workload_id,
        participant_subjects: request.participant_subjects,
        policy_binding_id: request.policy_binding_id,
        deadline_at: request.deadline_at,
        admission_quota_key: request.admission_quota_key,
        max_retry_count: request.max_retry_count,
        evidence_refs: request.evidence_refs,
    }
}

fn request_from_registration_command(
    command: confidential_app::ConfidentialJobRegistrationCommand,
) -> Result<ConfidentialJobRequest, ConfidentialJobError> {
    Ok(ConfidentialJobRequest {
        job_id: mechanism_transcript_id(command.job_id)?,
        mechanism: command.mechanism,
        mechanism_profile: command.mechanism_profile,
        workload_id: command.workload_id,
        participant_subjects: command.participant_subjects,
        policy_binding_id: command.policy_binding_id,
        deadline_at: command.deadline_at,
        admission_quota_key: command.admission_quota_key,
        max_retry_count: command.max_retry_count,
        evidence_refs: command.evidence_refs,
    })
}

fn transition_command_from_request(
    request: ConfidentialJobTransitionRequest,
) -> confidential_app::ConfidentialJobTransitionCommand {
    confidential_app::ConfidentialJobTransitionCommand {
        job_id: application_transcript_id(request.job_id),
        target_status: application_job_status(request.target_status),
        reason: request.reason,
        evidence_refs: request.evidence_refs,
    }
}

fn request_from_transition_command(
    command: confidential_app::ConfidentialJobTransitionCommand,
) -> Result<ConfidentialJobTransitionRequest, ConfidentialJobError> {
    Ok(ConfidentialJobTransitionRequest {
        job_id: mechanism_transcript_id(command.job_id)?,
        target_status: mechanism_job_status(command.target_status),
        reason: command.reason,
        evidence_refs: command.evidence_refs,
    })
}

fn job_filter_to_application(
    filter: ConfidentialJobFilter,
) -> confidential_app::ConfidentialJobFilter {
    confidential_app::ConfidentialJobFilter {
        job_id: filter.job_id.map(application_transcript_id),
        status: filter.status.map(application_job_status),
        mechanism: filter.mechanism,
    }
}

fn job_filter_from_application(
    filter: confidential_app::ConfidentialJobFilter,
) -> ConfidentialJobFilter {
    ConfidentialJobFilter {
        job_id: filter
            .job_id
            .and_then(|job_id| mechanism_transcript_id(job_id).ok()),
        status: filter.status.map(mechanism_job_status),
        mechanism: filter.mechanism,
    }
}

fn job_audit_filter_to_application(
    filter: ConfidentialJobAuditEventFilter,
) -> confidential_app::ConfidentialJobAuditEventFilter {
    confidential_app::ConfidentialJobAuditEventFilter {
        job_id: filter.job_id.map(application_transcript_id),
    }
}

fn job_audit_filter_from_application(
    filter: confidential_app::ConfidentialJobAuditEventFilter,
) -> ConfidentialJobAuditEventFilter {
    ConfidentialJobAuditEventFilter {
        job_id: filter
            .job_id
            .and_then(|job_id| mechanism_transcript_id(job_id).ok()),
    }
}

const fn application_job_status(
    status: MechanismJobStatus,
) -> confidential_app::ConfidentialJobStatus {
    match status {
        MechanismJobStatus::Draft => confidential_app::ConfidentialJobStatus::Draft,
        MechanismJobStatus::Admitted => confidential_app::ConfidentialJobStatus::Admitted,
        MechanismJobStatus::Running => confidential_app::ConfidentialJobStatus::Running,
        MechanismJobStatus::AwaitingRelease => {
            confidential_app::ConfidentialJobStatus::AwaitingRelease
        }
        MechanismJobStatus::Released => confidential_app::ConfidentialJobStatus::Released,
        MechanismJobStatus::Failed => confidential_app::ConfidentialJobStatus::Failed,
        MechanismJobStatus::Cancelled => confidential_app::ConfidentialJobStatus::Cancelled,
        MechanismJobStatus::Expired => confidential_app::ConfidentialJobStatus::Expired,
        MechanismJobStatus::Retired => confidential_app::ConfidentialJobStatus::Retired,
    }
}

const fn mechanism_job_status(
    status: confidential_app::ConfidentialJobStatus,
) -> MechanismJobStatus {
    match status {
        confidential_app::ConfidentialJobStatus::Draft => MechanismJobStatus::Draft,
        confidential_app::ConfidentialJobStatus::Admitted => MechanismJobStatus::Admitted,
        confidential_app::ConfidentialJobStatus::Running => MechanismJobStatus::Running,
        confidential_app::ConfidentialJobStatus::AwaitingRelease => {
            MechanismJobStatus::AwaitingRelease
        }
        confidential_app::ConfidentialJobStatus::Released => MechanismJobStatus::Released,
        confidential_app::ConfidentialJobStatus::Failed => MechanismJobStatus::Failed,
        confidential_app::ConfidentialJobStatus::Cancelled => MechanismJobStatus::Cancelled,
        confidential_app::ConfidentialJobStatus::Expired => MechanismJobStatus::Expired,
        confidential_app::ConfidentialJobStatus::Retired => MechanismJobStatus::Retired,
    }
}

const fn application_transcript_id(
    transcript_id: MechanismTranscriptId,
) -> confidential_app::SecureAggregateTranscriptId {
    confidential_app::SecureAggregateTranscriptId::from_validated_bytes(transcript_id.bytes())
}

fn mechanism_transcript_id(
    transcript_id: confidential_app::SecureAggregateTranscriptId,
) -> Result<MechanismTranscriptId, ConfidentialJobError> {
    MechanismTranscriptId::new(transcript_id.bytes()).map_err(|_error| {
        ConfidentialJobError::InvalidRequest(
            "confidential application transcript identifier was invalid".to_owned(),
        )
    })
}

impl From<confidential_app::ConfidentialApplicationError> for ConfidentialJobError {
    fn from(error: confidential_app::ConfidentialApplicationError) -> Self {
        Self::InvalidRequest(format!(
            "confidential application admission failed: {}",
            error.message()
        ))
    }
}

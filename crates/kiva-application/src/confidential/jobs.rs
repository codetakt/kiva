//! Confidential job lifecycle application workflow.

use kiva_confidential_core::{ConfidentialJobStatus, SecureAggregateTranscriptId};
use kiva_identity::AuthenticatedPrincipal;

use super::{
    ApplicationFuture, ApplicationRuntimePort, ConfidentialApplicationError,
    ConfidentialApplicationWorkflow, ConfidentialWorkflowAdmissionRecorder,
    ConfidentialWorkflowKind, ConfidentialWorkflowRepository,
};

/// Port for confidential job lifecycle workflows.
pub trait ConfidentialJobRepository: ConfidentialWorkflowRepository {
    /// Persisted confidential job view.
    type Job: Send + 'static;
    /// Persisted confidential job audit event view.
    type JobAuditEvent: Send + 'static;
    /// Registration outcome.
    type JobRegistrationOutcome: Send + 'static;
    /// Transition outcome.
    type JobTransitionOutcome: Send + 'static;
    /// Job lifecycle error.
    type JobError: From<ConfidentialApplicationError> + Send + 'static;

    /// Registers a confidential job after application admission.
    fn register_confidential_job<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        command: ConfidentialJobRegistrationCommand,
    ) -> ApplicationFuture<'a, Result<Self::JobRegistrationOutcome, Self::JobError>>;

    /// Transitions a confidential job after application admission.
    fn transition_confidential_job<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        command: ConfidentialJobTransitionCommand,
    ) -> ApplicationFuture<'a, Result<Self::JobTransitionOutcome, Self::JobError>>;

    /// Lists confidential jobs visible to the principal.
    fn list_confidential_jobs<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: ConfidentialJobFilter,
    ) -> ApplicationFuture<'a, Result<Vec<Self::Job>, Self::JobError>>;

    /// Lists confidential job audit events visible to the principal.
    fn list_confidential_job_audit_events<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: ConfidentialJobAuditEventFilter,
    ) -> ApplicationFuture<'a, Result<Vec<Self::JobAuditEvent>, Self::JobError>>;
}

/// Pure command for registering durable confidential job metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfidentialJobRegistrationCommand {
    /// Stable job identifier.
    pub job_id: SecureAggregateTranscriptId,
    /// Mechanism family label.
    pub mechanism: String,
    /// Mechanism profile identifier.
    pub mechanism_profile: String,
    /// Workload identifier.
    pub workload_id: String,
    /// Participant subjects bound to the job.
    pub participant_subjects: Vec<String>,
    /// Policy binding identifier.
    pub policy_binding_id: String,
    /// Deadline timestamp in RFC3339 form.
    pub deadline_at: String,
    /// Admission quota key.
    pub admission_quota_key: String,
    /// Maximum retry count.
    pub max_retry_count: i64,
    /// Evidence references supporting admission.
    pub evidence_refs: Vec<String>,
}

/// Pure command for moving a confidential job through its lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfidentialJobTransitionCommand {
    /// Stable job identifier.
    pub job_id: SecureAggregateTranscriptId,
    /// Requested lifecycle status.
    pub target_status: ConfidentialJobStatus,
    /// Optional transition reason.
    pub reason: Option<String>,
    /// Evidence references supporting the transition.
    pub evidence_refs: Vec<String>,
}

/// Pure confidential job query filter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfidentialJobFilter {
    /// Optional job identifier.
    pub job_id: Option<SecureAggregateTranscriptId>,
    /// Optional lifecycle status.
    pub status: Option<ConfidentialJobStatus>,
    /// Optional mechanism label.
    pub mechanism: Option<String>,
}

/// Pure confidential job audit-event query filter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConfidentialJobAuditEventFilter {
    /// Optional job identifier.
    pub job_id: Option<SecureAggregateTranscriptId>,
}

impl<R, D, A> ConfidentialApplicationWorkflow<R, D, A>
where
    R: ConfidentialJobRepository,
    D: ConfidentialWorkflowAdmissionRecorder,
    A: ApplicationRuntimePort,
{
    /// Registers a confidential job after application-level workflow admission.
    pub async fn register_confidential_job(
        &self,
        principal: &AuthenticatedPrincipal,
        command: ConfidentialJobRegistrationCommand,
    ) -> Result<R::JobRegistrationOutcome, R::JobError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::RegisterJob,
            |repository, _admission| async move {
                repository
                    .register_confidential_job(principal, command)
                    .await
            },
        )
        .await
    }

    /// Transitions a confidential job after application-level workflow admission.
    pub async fn transition_confidential_job(
        &self,
        principal: &AuthenticatedPrincipal,
        command: ConfidentialJobTransitionCommand,
    ) -> Result<R::JobTransitionOutcome, R::JobError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::TransitionJob,
            |repository, _admission| async move {
                repository
                    .transition_confidential_job(principal, command)
                    .await
            },
        )
        .await
    }

    /// Lists confidential jobs visible to the principal.
    pub async fn list_confidential_jobs(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialJobFilter,
    ) -> Result<Vec<R::Job>, R::JobError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ListJobs,
            |repository, _admission| async move {
                repository.list_confidential_jobs(principal, filter).await
            },
        )
        .await
    }

    /// Lists confidential job audit events visible to the principal.
    pub async fn list_confidential_job_audit_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialJobAuditEventFilter,
    ) -> Result<Vec<R::JobAuditEvent>, R::JobError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ListJobAuditEvents,
            |repository, _admission| async move {
                repository
                    .list_confidential_job_audit_events(principal, filter)
                    .await
            },
        )
        .await
    }
}

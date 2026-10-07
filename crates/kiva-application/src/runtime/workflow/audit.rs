//! Application workflow audit evidence.

use uuid::Uuid;

use super::{ApplicationWorkflowKind, ApplicationWorkflowSpec};

/// Workflow audit status emitted at application boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationWorkflowAuditStatus {
    /// Workflow boundary was started.
    Started,
    /// Workflow completed successfully.
    Completed,
    /// Workflow failed after boundary admission.
    Failed,
}

/// Auditable workflow boundary event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationWorkflowAuditEvent {
    workflow_spec: ApplicationWorkflowSpec,
    workflow_id: Uuid,
    status: ApplicationWorkflowAuditStatus,
    reason: Option<String>,
}

impl ApplicationWorkflowAuditEvent {
    /// Creates a workflow audit event.
    #[must_use]
    pub const fn new(
        workflow_spec: ApplicationWorkflowSpec,
        workflow_id: Uuid,
        status: ApplicationWorkflowAuditStatus,
        reason: Option<String>,
    ) -> Self {
        Self {
            workflow_spec,
            workflow_id,
            status,
            reason,
        }
    }

    /// Returns the workflow kind.
    #[must_use]
    pub const fn workflow_kind(&self) -> ApplicationWorkflowKind {
        self.workflow_spec.kind()
    }

    /// Returns the workflow specification.
    #[must_use]
    pub const fn workflow_spec(&self) -> ApplicationWorkflowSpec {
        self.workflow_spec
    }

    /// Returns the workflow label.
    #[must_use]
    pub const fn workflow_name(&self) -> &'static str {
        self.workflow_spec.label()
    }

    /// Returns the workflow identifier.
    #[must_use]
    pub const fn workflow_id(&self) -> Uuid {
        self.workflow_id
    }

    /// Returns the audit status.
    #[must_use]
    pub const fn status(&self) -> ApplicationWorkflowAuditStatus {
        self.status
    }

    /// Returns the optional failure reason.
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

/// Active application workflow boundary marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationWorkflowRun {
    workflow_spec: ApplicationWorkflowSpec,
    workflow_id: Uuid,
}

impl ApplicationWorkflowRun {
    /// Creates a workflow boundary marker.
    #[must_use]
    pub const fn new(workflow_spec: ApplicationWorkflowSpec, workflow_id: Uuid) -> Self {
        Self {
            workflow_spec,
            workflow_id,
        }
    }

    /// Returns the workflow kind.
    #[must_use]
    pub const fn workflow_kind(self) -> ApplicationWorkflowKind {
        self.workflow_spec.kind()
    }

    /// Returns the workflow specification.
    #[must_use]
    pub const fn workflow_spec(self) -> ApplicationWorkflowSpec {
        self.workflow_spec
    }

    /// Returns the workflow name.
    #[must_use]
    pub const fn workflow_name(self) -> &'static str {
        self.workflow_spec.label()
    }

    /// Returns the workflow identifier.
    #[must_use]
    pub const fn workflow_id(self) -> Uuid {
        self.workflow_id
    }

    /// Completes this workflow boundary.
    #[must_use]
    pub const fn complete(self) -> ApplicationWorkflowAuditEvent {
        self.completed_event()
    }

    /// Returns the started audit event for this workflow boundary.
    #[must_use]
    pub const fn started_event(self) -> ApplicationWorkflowAuditEvent {
        ApplicationWorkflowAuditEvent::new(
            self.workflow_spec,
            self.workflow_id,
            ApplicationWorkflowAuditStatus::Started,
            None,
        )
    }

    /// Returns the completed audit event for this workflow boundary.
    #[must_use]
    pub const fn completed_event(self) -> ApplicationWorkflowAuditEvent {
        ApplicationWorkflowAuditEvent::new(
            self.workflow_spec,
            self.workflow_id,
            ApplicationWorkflowAuditStatus::Completed,
            None,
        )
    }

    /// Returns the failed audit event for this workflow boundary.
    #[must_use]
    pub const fn failed_event(self, reason: String) -> ApplicationWorkflowAuditEvent {
        ApplicationWorkflowAuditEvent::new(
            self.workflow_spec,
            self.workflow_id,
            ApplicationWorkflowAuditStatus::Failed,
            Some(reason),
        )
    }
}

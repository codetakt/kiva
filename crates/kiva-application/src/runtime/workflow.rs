//! Application workflow evidence types.

use std::{future::Future, pin::Pin};

mod admission;
mod audit;
mod context;
mod executor;
mod operation;

use uuid::Uuid;

use super::ApplicationRuntimeError;

pub use admission::{Admitted, PrincipalBound, UnscopedAdmission, WorkflowBound};
pub use audit::{
    ApplicationWorkflowAuditEvent, ApplicationWorkflowAuditStatus, ApplicationWorkflowRun,
};
pub use context::ApplicationWorkflowContext;
pub use executor::ApplicationWorkflowFailure;
pub use operation::{
    ApplicationAdmissionPolicy, ApplicationOperation, ApplicationOperationGateway,
    ApplicationOperationPlan,
};

/// Boxed future returned by application ports.
pub type ApplicationFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Typed application workflow label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationWorkflowKind {
    label: &'static str,
}

impl ApplicationWorkflowKind {
    /// Creates a workflow kind from a stable application-owned label.
    #[must_use]
    pub(crate) const fn new(label: &'static str) -> Self {
        Self { label }
    }

    /// Returns the stable workflow label used by logs and evidence.
    #[must_use]
    pub const fn label(self) -> &'static str {
        self.label
    }
}

/// Stable application workflow specification used by runtime and operation evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationWorkflowSpec {
    kind: ApplicationWorkflowKind,
}

impl ApplicationWorkflowSpec {
    /// Creates a workflow specification from a stable application workflow label.
    #[must_use]
    pub(crate) const fn new(kind: ApplicationWorkflowKind) -> Self {
        Self { kind }
    }

    /// Returns the application workflow kind.
    #[must_use]
    pub const fn kind(self) -> ApplicationWorkflowKind {
        self.kind
    }

    /// Returns the stable workflow label used by logs and evidence.
    #[must_use]
    pub const fn label(self) -> &'static str {
        self.kind.label()
    }
}

/// Domain-specific workflow kind that can be projected into application evidence.
pub trait WorkflowKind: Copy + Eq + Send + Sync + 'static {
    /// Returns the application-wide workflow label.
    fn application_kind(self) -> ApplicationWorkflowKind;

    /// Returns the application workflow specification.
    fn workflow_spec(self) -> ApplicationWorkflowSpec;

    /// Returns the stable workflow label used by logs and evidence.
    fn label(self) -> &'static str {
        self.application_kind().label()
    }
}

impl WorkflowKind for ApplicationWorkflowKind {
    fn application_kind(self) -> ApplicationWorkflowKind {
        self
    }

    fn workflow_spec(self) -> ApplicationWorkflowSpec {
        ApplicationWorkflowSpec::new(self)
    }
}

/// Workflow boundary port for application workflow observation.
pub trait ApplicationWorkflowBoundary: Clone + Send + Sync + 'static {
    /// Begins an application workflow boundary.
    fn begin(
        &self,
        workflow_spec: ApplicationWorkflowSpec,
        workflow_id: Uuid,
    ) -> ApplicationFuture<'_, Result<ApplicationWorkflowRun, ApplicationRuntimeError>>;

    /// Records successful workflow completion.
    fn complete(
        &self,
        workflow_run: ApplicationWorkflowRun,
    ) -> ApplicationFuture<'_, Result<ApplicationWorkflowAuditEvent, ApplicationRuntimeError>> {
        Box::pin(async move { Ok(workflow_run.completed_event()) })
    }

    /// Records failed workflow completion.
    fn fail(
        &self,
        workflow_run: ApplicationWorkflowRun,
        reason: String,
    ) -> ApplicationFuture<'_, Result<ApplicationWorkflowAuditEvent, ApplicationRuntimeError>> {
        Box::pin(async move { Ok(workflow_run.failed_event(reason)) })
    }
}

/// Admitted workflow record shared by application surfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowAdmission<K>
where
    K: WorkflowKind,
{
    principal_subject: String,
    workflow_kind: K,
}

impl<K> WorkflowAdmission<K>
where
    K: WorkflowKind,
{
    /// Creates a workflow admission record.
    #[must_use]
    pub fn new(principal_subject: impl Into<String>, workflow_kind: K) -> Self {
        Self {
            principal_subject: principal_subject.into(),
            workflow_kind,
        }
    }

    /// Returns the authenticated principal subject admitted to the workflow.
    #[must_use]
    pub fn principal_subject(&self) -> &str {
        &self.principal_subject
    }

    /// Returns the admitted workflow kind.
    #[must_use]
    pub const fn workflow_kind(&self) -> K {
        self.workflow_kind
    }

    /// Returns the admitted workflow specification.
    #[must_use]
    pub fn workflow_spec(&self) -> ApplicationWorkflowSpec {
        self.workflow_kind.workflow_spec()
    }

    /// Returns the admitted workflow name.
    #[must_use]
    pub fn workflow_name(&self) -> &'static str {
        self.workflow_kind.label()
    }
}

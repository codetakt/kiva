//! Server-independent application runtime ports.

mod workflow;

use time::OffsetDateTime;
use uuid::Uuid;

pub use workflow::{
    Admitted, ApplicationAdmissionPolicy, ApplicationFuture, ApplicationOperation,
    ApplicationOperationGateway, ApplicationOperationPlan, ApplicationWorkflowAuditEvent,
    ApplicationWorkflowAuditStatus, ApplicationWorkflowBoundary, ApplicationWorkflowContext,
    ApplicationWorkflowFailure, ApplicationWorkflowKind, ApplicationWorkflowRun,
    ApplicationWorkflowSpec, PrincipalBound, UnscopedAdmission, WorkflowAdmission, WorkflowBound,
    WorkflowKind,
};

/// Clock port for deterministic application workflows.
pub trait ApplicationClock: Clone + Send + Sync + 'static {
    /// Returns the current UTC timestamp for application commands.
    fn now_utc(&self) -> OffsetDateTime;
}

/// Identifier port for deterministic workflow correlation.
pub trait ApplicationIdGenerator: Clone + Send + Sync + 'static {
    /// Returns the next application workflow identifier.
    fn next_uuid(&self) -> Uuid;
}

/// Combined runtime port consumed by application workflows.
pub trait ApplicationRuntimePort: Clone + Send + Sync + 'static {
    /// Returns the current UTC timestamp.
    fn now_utc(&self) -> OffsetDateTime;

    /// Returns the next workflow identifier.
    fn next_workflow_id(&self) -> Uuid;

    /// Begins a logical boundary for the named workflow.
    fn begin_workflow_boundary(
        &self,
        workflow_spec: ApplicationWorkflowSpec,
        workflow_id: Uuid,
    ) -> ApplicationFuture<'_, Result<ApplicationWorkflowRun, ApplicationRuntimeError>>;

    /// Completes a logical workflow boundary.
    fn complete_workflow_boundary(
        &self,
        workflow_run: ApplicationWorkflowRun,
    ) -> ApplicationFuture<'_, Result<ApplicationWorkflowAuditEvent, ApplicationRuntimeError>>;

    /// Fails a logical workflow boundary.
    fn fail_workflow_boundary(
        &self,
        workflow_run: ApplicationWorkflowRun,
        reason: String,
    ) -> ApplicationFuture<'_, Result<ApplicationWorkflowAuditEvent, ApplicationRuntimeError>>;
}

/// Runtime error raised before an application workflow can be persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationRuntimeError {
    message: String,
}

impl ApplicationRuntimeError {
    /// Creates a runtime error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the stable runtime error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// System UTC clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl ApplicationClock for SystemClock {
    fn now_utc(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

/// `UUIDv4` workflow identifier generator.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemIdGenerator;

impl ApplicationIdGenerator for SystemIdGenerator {
    fn next_uuid(&self) -> Uuid {
        Uuid::new_v4()
    }
}

/// In-process workflow boundary implementation for application workflows.
#[derive(Debug, Clone, Copy, Default)]
pub struct InProcessWorkflowBoundary;

impl ApplicationWorkflowBoundary for InProcessWorkflowBoundary {
    fn begin(
        &self,
        workflow_spec: ApplicationWorkflowSpec,
        workflow_id: Uuid,
    ) -> ApplicationFuture<'_, Result<ApplicationWorkflowRun, ApplicationRuntimeError>> {
        Box::pin(async move { Ok(ApplicationWorkflowRun::new(workflow_spec, workflow_id)) })
    }
}

/// Application runtime assembled from clock, identifier, and workflow boundary ports.
#[derive(Debug, Clone)]
pub struct ApplicationRuntime<C, I, W>
where
    C: ApplicationClock,
    I: ApplicationIdGenerator,
    W: ApplicationWorkflowBoundary,
{
    clock: C,
    id_generator: I,
    workflow_boundary: W,
}

impl<C, I, W> ApplicationRuntime<C, I, W>
where
    C: ApplicationClock,
    I: ApplicationIdGenerator,
    W: ApplicationWorkflowBoundary,
{
    /// Creates an application runtime from explicit ports.
    #[must_use]
    pub const fn new(clock: C, id_generator: I, workflow_boundary: W) -> Self {
        Self {
            clock,
            id_generator,
            workflow_boundary,
        }
    }

    /// Returns the clock port.
    #[must_use]
    pub const fn clock(&self) -> &C {
        &self.clock
    }

    /// Returns the identifier port.
    #[must_use]
    pub const fn id_generator(&self) -> &I {
        &self.id_generator
    }

    /// Returns the workflow boundary port.
    #[must_use]
    pub const fn workflow_boundary(&self) -> &W {
        &self.workflow_boundary
    }
}

impl<C, I, W> ApplicationRuntimePort for ApplicationRuntime<C, I, W>
where
    C: ApplicationClock,
    I: ApplicationIdGenerator,
    W: ApplicationWorkflowBoundary,
{
    fn now_utc(&self) -> OffsetDateTime {
        self.clock.now_utc()
    }

    fn next_workflow_id(&self) -> Uuid {
        self.id_generator.next_uuid()
    }

    fn begin_workflow_boundary(
        &self,
        workflow_spec: ApplicationWorkflowSpec,
        workflow_id: Uuid,
    ) -> ApplicationFuture<'_, Result<ApplicationWorkflowRun, ApplicationRuntimeError>> {
        self.workflow_boundary.begin(workflow_spec, workflow_id)
    }

    fn complete_workflow_boundary(
        &self,
        workflow_run: ApplicationWorkflowRun,
    ) -> ApplicationFuture<'_, Result<ApplicationWorkflowAuditEvent, ApplicationRuntimeError>> {
        self.workflow_boundary.complete(workflow_run)
    }

    fn fail_workflow_boundary(
        &self,
        workflow_run: ApplicationWorkflowRun,
        reason: String,
    ) -> ApplicationFuture<'_, Result<ApplicationWorkflowAuditEvent, ApplicationRuntimeError>> {
        self.workflow_boundary.fail(workflow_run, reason)
    }
}

/// Default application runtime used by the server composition root.
pub type DefaultApplicationRuntime =
    ApplicationRuntime<SystemClock, SystemIdGenerator, InProcessWorkflowBoundary>;

impl Default for DefaultApplicationRuntime {
    fn default() -> Self {
        Self::new(SystemClock, SystemIdGenerator, InProcessWorkflowBoundary)
    }
}

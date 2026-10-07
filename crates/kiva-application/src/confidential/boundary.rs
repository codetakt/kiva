//! Confidential-computation workflow admission boundary.

use std::future::Future;

use crate::runtime::{
    Admitted, ApplicationAdmissionPolicy, ApplicationFuture, ApplicationOperationGateway,
    ApplicationOperationPlan, ApplicationRuntimeError, ApplicationRuntimePort,
    ApplicationWorkflowFailure, DefaultApplicationRuntime, WorkflowBound,
};
use kiva_identity::AuthenticatedPrincipal;

use super::{
    ConfidentialCommand, ConfidentialDomainEvent, ConfidentialWorkflowAdmission,
    ConfidentialWorkflowKind,
};

/// Port used by confidential workflows to cross into persistence adapters.
pub trait ConfidentialWorkflowRepository: Clone + Send + Sync + 'static {
    /// Stable boundary label for architecture evidence.
    fn boundary_name(&self) -> &'static str;
}

/// Port used by confidential workflows to record admission evidence.
pub trait ConfidentialWorkflowAdmissionRecorder: Clone + Send + Sync + 'static {
    /// Records an application-admitted confidential workflow domain event.
    fn record_workflow_event<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        event: ConfidentialDomainEvent,
    ) -> ApplicationFuture<'a, Result<ConfidentialDomainEvent, ConfidentialApplicationError>>;
}

/// Error raised before a confidential workflow can be admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfidentialApplicationError {
    message: String,
}

impl ConfidentialApplicationError {
    /// Creates an application boundary error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the stable error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<ApplicationRuntimeError> for ConfidentialApplicationError {
    fn from(error: ApplicationRuntimeError) -> Self {
        Self::new(error.message().to_owned())
    }
}

/// Capability aggregate for confidential application workflows.
#[derive(Debug, Clone)]
pub struct ConfidentialApplicationCapabilities<R, D = R, A = DefaultApplicationRuntime>
where
    R: ConfidentialWorkflowRepository,
    D: ConfidentialWorkflowAdmissionRecorder,
    A: ApplicationRuntimePort,
{
    pub(in crate::confidential) repository: R,
    pub(in crate::confidential) admission_recorder: D,
    runtime: A,
}

impl<R> ConfidentialApplicationCapabilities<R>
where
    R: ConfidentialWorkflowRepository + ConfidentialWorkflowAdmissionRecorder,
{
    /// Creates a confidential application capability aggregate.
    #[must_use]
    pub fn new(repository: R) -> Self {
        Self::with_admission_recorder(repository.clone(), repository)
    }
}

impl<R, D> ConfidentialApplicationCapabilities<R, D, DefaultApplicationRuntime>
where
    R: ConfidentialWorkflowRepository,
    D: ConfidentialWorkflowAdmissionRecorder,
{
    /// Creates a confidential application capability aggregate with an explicit admission recorder.
    #[must_use]
    pub fn with_admission_recorder(repository: R, admission_recorder: D) -> Self {
        Self::with_runtime(
            repository,
            admission_recorder,
            DefaultApplicationRuntime::default(),
        )
    }
}

impl<R, D, A> ConfidentialApplicationCapabilities<R, D, A>
where
    R: ConfidentialWorkflowRepository,
    D: ConfidentialWorkflowAdmissionRecorder,
    A: ApplicationRuntimePort,
{
    /// Creates a confidential application capability aggregate with an explicit runtime.
    #[must_use]
    pub const fn with_runtime(repository: R, admission_recorder: D, runtime: A) -> Self {
        Self {
            repository,
            admission_recorder,
            runtime,
        }
    }

    /// Returns the stable workflow boundary name.
    pub fn workflow_boundary_name(&self) -> &'static str {
        self.repository.boundary_name()
    }

    /// Returns the application runtime port.
    pub const fn runtime(&self) -> &A {
        &self.runtime
    }

    /// Begins a confidential application workflow over this capability set.
    pub fn begin_workflow(&self) -> ConfidentialApplicationWorkflow<R, D, A> {
        ConfidentialApplicationWorkflow {
            capabilities: self.clone(),
        }
    }
}

/// Per-invocation confidential application workflow.
#[derive(Debug, Clone)]
pub struct ConfidentialApplicationWorkflow<R, D = R, A = DefaultApplicationRuntime>
where
    R: ConfidentialWorkflowRepository,
    D: ConfidentialWorkflowAdmissionRecorder,
    A: ApplicationRuntimePort,
{
    pub(in crate::confidential) capabilities: ConfidentialApplicationCapabilities<R, D, A>,
}

impl<R, D, A> ConfidentialApplicationWorkflow<R, D, A>
where
    R: ConfidentialWorkflowRepository,
    D: ConfidentialWorkflowAdmissionRecorder,
    A: ApplicationRuntimePort,
{
    /// Returns the shared application operation gateway.
    pub const fn operation_gateway(&self) -> ApplicationOperationGateway<'_, A> {
        ApplicationOperationGateway::new(self.capabilities.runtime())
    }

    pub(in crate::confidential) async fn run_confidential_workflow<T, E, Fut>(
        &self,
        principal: &AuthenticatedPrincipal,
        workflow_kind: ConfidentialWorkflowKind,
        operation: impl FnOnce(R, Admitted<ConfidentialWorkflowAdmission, WorkflowBound>) -> Fut + Send,
    ) -> Result<T, E>
    where
        T: Send,
        E: From<ConfidentialApplicationError> + Send,
        Fut: Future<Output = Result<T, E>> + Send,
    {
        let command = ConfidentialCommand::new(workflow_kind);
        let plan = ApplicationOperationPlan::with_admission_policy(
            command.workflow_kind(),
            ConfidentialWorkflowAdmissionPolicy::new(principal),
        );
        self.operation_gateway()
            .run_admitted_with_runtime_error(
                plan,
                command,
                |error| E::from(ConfidentialApplicationError::from(error)),
                |error| {
                    let reason = error.message().to_owned();
                    ApplicationWorkflowFailure::new(reason, E::from(error))
                },
                |admission| async move {
                    let event = ConfidentialDomainEvent::workflow_admitted(&admission);
                    self.capabilities
                        .admission_recorder
                        .record_workflow_event(principal, event)
                        .await
                        .map_err(|error| {
                            let reason = error.message().to_owned();
                            ApplicationWorkflowFailure::new(reason, E::from(error))
                        })?;
                    operation(self.capabilities.repository.clone(), admission)
                        .await
                        .map_err(|error| {
                            ApplicationWorkflowFailure::new(
                                "confidential workflow operation failed",
                                error,
                            )
                        })
                },
            )
            .await
    }

    /// Admits a named workflow for the authenticated principal.
    pub async fn admit(
        &self,
        principal: &AuthenticatedPrincipal,
        workflow_kind: ConfidentialWorkflowKind,
    ) -> Result<ConfidentialWorkflowAdmission, ConfidentialApplicationError> {
        self.run_confidential_workflow(
            principal,
            workflow_kind,
            |_repository, admission| async move { Ok(admission.into_inner()) },
        )
        .await
    }
}

#[derive(Debug, Clone, Copy)]
struct ConfidentialWorkflowAdmissionPolicy<'a> {
    principal: &'a AuthenticatedPrincipal,
}

impl<'a> ConfidentialWorkflowAdmissionPolicy<'a> {
    const fn new(principal: &'a AuthenticatedPrincipal) -> Self {
        Self { principal }
    }
}

impl ApplicationAdmissionPolicy for ConfidentialWorkflowAdmissionPolicy<'_> {
    type Request = ConfidentialCommand;
    type Admission = Admitted<ConfidentialWorkflowAdmission, WorkflowBound>;
    type Error = ConfidentialApplicationError;

    fn admit(
        &self,
        context: crate::runtime::ApplicationWorkflowContext,
        command: Self::Request,
    ) -> Result<Self::Admission, Self::Error> {
        let _requested = command.admission_requested(self.principal);
        Ok(context.admit_workflow(ConfidentialWorkflowAdmission::new(
            self.principal.subject.clone(),
            command.workflow_kind(),
        )))
    }
}

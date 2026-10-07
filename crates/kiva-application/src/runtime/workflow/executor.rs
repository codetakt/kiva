//! Shared application workflow execution lifecycle.

use std::future::Future;

use super::{ApplicationWorkflowContext, ApplicationWorkflowRun, ApplicationWorkflowSpec};
use crate::runtime::{ApplicationRuntimeError, ApplicationRuntimePort};

/// Shared begin/complete/fail executor for application workflows.
#[derive(Debug, Clone, Copy)]
pub(super) struct ApplicationWorkflowExecutor<'a, A>
where
    A: ApplicationRuntimePort,
{
    runtime: &'a A,
}

impl<'a, A> ApplicationWorkflowExecutor<'a, A>
where
    A: ApplicationRuntimePort,
{
    /// Creates a workflow executor over an application runtime.
    #[must_use]
    pub(super) const fn new(runtime: &'a A) -> Self {
        Self { runtime }
    }

    /// Begins one workflow boundary.
    async fn begin(
        &self,
        workflow_spec: ApplicationWorkflowSpec,
    ) -> Result<ApplicationWorkflowRun, ApplicationRuntimeError> {
        let workflow_id = self.runtime.next_workflow_id();
        self.runtime
            .begin_workflow_boundary(workflow_spec, workflow_id)
            .await
    }

    /// Completes one workflow boundary.
    async fn complete(
        &self,
        workflow_run: ApplicationWorkflowRun,
    ) -> Result<(), ApplicationRuntimeError> {
        self.runtime
            .complete_workflow_boundary(workflow_run)
            .await
            .map(|_event| ())
    }

    /// Fails one workflow boundary with a stable reason.
    async fn fail(
        &self,
        workflow_run: ApplicationWorkflowRun,
        reason: &str,
    ) -> Result<(), ApplicationRuntimeError> {
        self.runtime
            .fail_workflow_boundary(workflow_run, reason.to_owned())
            .await
            .map(|_event| ())
    }

    /// Runs one application workflow with shared begin/complete/fail semantics.
    pub(super) async fn run<T, E, Fut>(
        &self,
        workflow_spec: ApplicationWorkflowSpec,
        operation: impl FnOnce(ApplicationWorkflowContext) -> Fut,
    ) -> Result<T, E>
    where
        E: From<ApplicationRuntimeError>,
        Fut: Future<Output = Result<T, ApplicationWorkflowFailure<E>>>,
    {
        self.run_detailed(workflow_spec, operation)
            .await
            .map_err(ApplicationWorkflowExecutionError::into_primary_error)
    }

    /// Runs one application workflow and preserves operation and audit errors.
    async fn run_detailed<T, E, Fut>(
        &self,
        workflow_spec: ApplicationWorkflowSpec,
        operation: impl FnOnce(ApplicationWorkflowContext) -> Fut,
    ) -> Result<T, ApplicationWorkflowExecutionError<E>>
    where
        E: From<ApplicationRuntimeError>,
        Fut: Future<Output = Result<T, ApplicationWorkflowFailure<E>>>,
    {
        let workflow_run = self
            .begin(workflow_spec)
            .await
            .map_err(ApplicationWorkflowExecutionError::Begin)?;
        let context = ApplicationWorkflowContext::new(workflow_run);
        match operation(context).await {
            Ok(value) => {
                self.complete(workflow_run)
                    .await
                    .map_err(ApplicationWorkflowExecutionError::Complete)?;
                Ok(value)
            }
            Err(failure) => {
                let reason = failure.reason().to_owned();
                let operation_error = failure.into_error();
                let audit_error = self.fail(workflow_run, &reason).await.err();
                Err(ApplicationWorkflowExecutionError::Operation {
                    operation_error,
                    _audit_error: audit_error,
                })
            }
        }
    }

    /// Runs one application workflow while mapping runtime errors through a caller-owned adapter.
    pub(super) async fn run_with_runtime_error<T, E, Fut>(
        &self,
        workflow_spec: ApplicationWorkflowSpec,
        map_runtime_error: impl Fn(ApplicationRuntimeError) -> E,
        operation: impl FnOnce(ApplicationWorkflowContext) -> Fut,
    ) -> Result<T, E>
    where
        Fut: Future<Output = Result<T, ApplicationWorkflowFailure<E>>>,
    {
        let workflow_run = self
            .begin(workflow_spec)
            .await
            .map_err(&map_runtime_error)?;
        let context = ApplicationWorkflowContext::new(workflow_run);
        match operation(context).await {
            Ok(value) => {
                self.complete(workflow_run)
                    .await
                    .map_err(map_runtime_error)?;
                Ok(value)
            }
            Err(failure) => {
                let reason = failure.reason().to_owned();
                let operation_error = failure.into_error();
                let _audit_error = self.fail(workflow_run, &reason).await.err();
                Err(operation_error)
            }
        }
    }
}

/// Application workflow execution error preserving audit failures.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ApplicationWorkflowExecutionError<E> {
    /// Workflow begin failed before operation execution.
    Begin(ApplicationRuntimeError),
    /// Workflow operation failed, with optional failure-audit error.
    Operation {
        /// Primary operation error.
        operation_error: E,
        /// Failure-audit error when recording the failure also failed.
        _audit_error: Option<ApplicationRuntimeError>,
    },
    /// Workflow completion audit failed after operation success.
    Complete(ApplicationRuntimeError),
}

impl<E> ApplicationWorkflowExecutionError<E>
where
    E: From<ApplicationRuntimeError>,
{
    /// Converts to the primary caller-facing error.
    #[must_use]
    fn into_primary_error(self) -> E {
        match self {
            Self::Begin(error) | Self::Complete(error) => E::from(error),
            Self::Operation {
                operation_error, ..
            } => operation_error,
        }
    }
}

/// Application workflow failure with an auditable reason and typed error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationWorkflowFailure<E> {
    reason: String,
    error: E,
}

impl<E> ApplicationWorkflowFailure<E> {
    /// Creates a workflow failure.
    #[must_use]
    pub fn new(reason: impl Into<String>, error: E) -> Self {
        Self {
            reason: reason.into(),
            error,
        }
    }

    /// Returns the stable audit reason.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Consumes this failure and returns the typed error.
    #[must_use]
    pub fn into_error(self) -> E {
        self.error
    }
}

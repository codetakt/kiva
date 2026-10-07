//! Application operation execution and admission contracts.

use std::future::Future;

#[path = "operation/gateway.rs"]
mod gateway;

use super::{
    ApplicationWorkflowContext, ApplicationWorkflowFailure, ApplicationWorkflowSpec, WorkflowKind,
    executor::ApplicationWorkflowExecutor,
};
use crate::runtime::{ApplicationRuntimeError, ApplicationRuntimePort};

pub use gateway::ApplicationOperationGateway;

/// Executable application operation metadata.
pub trait ApplicationOperation: Copy + Send + Sync + 'static {
    /// Returns the workflow specification that observes this operation.
    fn workflow_spec(self) -> ApplicationWorkflowSpec;
}

impl ApplicationOperation for ApplicationWorkflowSpec {
    fn workflow_spec(self) -> ApplicationWorkflowSpec {
        self
    }
}

impl<K> ApplicationOperation for K
where
    K: WorkflowKind,
{
    fn workflow_spec(self) -> ApplicationWorkflowSpec {
        WorkflowKind::workflow_spec(self)
    }
}

/// Synchronous admission policy used before values cross an application port.
pub trait ApplicationAdmissionPolicy {
    /// Input value checked by the admission policy.
    type Request;
    /// Value emitted after admission succeeds.
    type Admission;
    /// Policy-specific admission error.
    type Error;

    /// Admits one value within the active workflow context.
    fn admit(
        &self,
        context: ApplicationWorkflowContext,
        request: Self::Request,
    ) -> Result<Self::Admission, Self::Error>;
}

/// Typed application operation plus its optional admission policy.
#[derive(Debug, Clone, Copy)]
pub struct ApplicationOperationPlan<O, P = ()> {
    operation: O,
    admission_policy: P,
}

impl<O> ApplicationOperationPlan<O> {
    /// Creates an operation plan without a typed admission policy.
    #[must_use]
    pub const fn new(operation: O) -> Self {
        Self {
            operation,
            admission_policy: (),
        }
    }
}

impl<O, P> ApplicationOperationPlan<O, P> {
    /// Creates an operation plan with the admission policy used inside it.
    #[must_use]
    pub const fn with_admission_policy(operation: O, admission_policy: P) -> Self {
        Self {
            operation,
            admission_policy,
        }
    }

    /// Returns the operation metadata.
    #[must_use]
    pub const fn operation(&self) -> O
    where
        O: Copy,
    {
        self.operation
    }

    /// Returns the admission policy.
    #[must_use]
    pub const fn admission_policy(&self) -> &P {
        &self.admission_policy
    }

    /// Admits one request with the plan's admission policy.
    pub fn admit(
        &self,
        context: ApplicationWorkflowContext,
        request: P::Request,
    ) -> Result<P::Admission, P::Error>
    where
        P: ApplicationAdmissionPolicy,
    {
        self.admission_policy.admit(context, request)
    }

    /// Admits one request and executes the admitted port operation.
    pub async fn execute_admitted<T, E, Fut>(
        &self,
        context: ApplicationWorkflowContext,
        request: P::Request,
        map_admission_error: impl Fn(P::Error) -> ApplicationWorkflowFailure<E> + Send,
        execute: impl FnOnce(P::Admission) -> Fut + Send,
    ) -> Result<T, ApplicationWorkflowFailure<E>>
    where
        O: Sync,
        P: ApplicationAdmissionPolicy + Sync,
        P::Request: Send,
        P::Admission: Send,
        P::Error: Send,
        T: Send,
        E: Send,
        Fut: Future<Output = Result<T, ApplicationWorkflowFailure<E>>> + Send,
    {
        let admission = self
            .admission_policy
            .admit(context, request)
            .map_err(map_admission_error)?;
        execute(admission).await
    }
}

/// Internal operation kernel over the workflow lifecycle.
#[derive(Debug, Clone, Copy)]
struct ApplicationOperationKernel<'a, A>
where
    A: ApplicationRuntimePort,
{
    workflow_executor: ApplicationWorkflowExecutor<'a, A>,
}

impl<'a, A> ApplicationOperationKernel<'a, A>
where
    A: ApplicationRuntimePort,
{
    /// Creates an operation kernel over an application runtime.
    #[must_use]
    const fn new(runtime: &'a A) -> Self {
        Self {
            workflow_executor: ApplicationWorkflowExecutor::new(runtime),
        }
    }

    /// Executes one application operation with shared begin/complete/fail semantics.
    async fn execute<T, E, Fut>(
        &self,
        operation: impl ApplicationOperation,
        execute: impl FnOnce(ApplicationWorkflowContext) -> Fut,
    ) -> Result<T, E>
    where
        E: From<ApplicationRuntimeError>,
        Fut: Future<Output = Result<T, ApplicationWorkflowFailure<E>>>,
    {
        self.workflow_executor
            .run(operation.workflow_spec(), execute)
            .await
    }

    /// Executes one planned application operation with shared begin/complete/fail semantics.
    async fn execute_plan<T, E, Fut, O, P>(
        &self,
        plan: ApplicationOperationPlan<O, P>,
        execute: impl FnOnce(ApplicationWorkflowContext) -> Fut,
    ) -> Result<T, E>
    where
        O: ApplicationOperation,
        E: From<ApplicationRuntimeError>,
        Fut: Future<Output = Result<T, ApplicationWorkflowFailure<E>>>,
    {
        self.execute(plan.operation, execute).await
    }

    /// Executes one application operation while mapping runtime errors through a caller-owned adapter.
    async fn execute_with_runtime_error<T, E, Fut>(
        &self,
        operation: impl ApplicationOperation,
        map_runtime_error: impl Fn(ApplicationRuntimeError) -> E,
        execute: impl FnOnce(ApplicationWorkflowContext) -> Fut,
    ) -> Result<T, E>
    where
        Fut: Future<Output = Result<T, ApplicationWorkflowFailure<E>>>,
    {
        self.workflow_executor
            .run_with_runtime_error(operation.workflow_spec(), map_runtime_error, execute)
            .await
    }
}

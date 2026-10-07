//! Application operation gateway over lifecycle, admission, and execution.

use std::future::Future;

use super::{
    ApplicationAdmissionPolicy, ApplicationOperation, ApplicationOperationKernel,
    ApplicationOperationPlan,
};
use crate::runtime::{
    ApplicationRuntimeError, ApplicationRuntimePort, ApplicationWorkflowContext,
    ApplicationWorkflowFailure,
};

/// Shared application entrypoint for operation lifecycle, admission, and execution.
#[derive(Debug, Clone, Copy)]
pub struct ApplicationOperationGateway<'a, A>
where
    A: ApplicationRuntimePort,
{
    kernel: ApplicationOperationKernel<'a, A>,
}

impl<'a, A> ApplicationOperationGateway<'a, A>
where
    A: ApplicationRuntimePort,
{
    /// Creates an operation gateway over an application runtime.
    #[must_use]
    pub const fn new(runtime: &'a A) -> Self {
        Self {
            kernel: ApplicationOperationKernel::new(runtime),
        }
    }

    /// Executes one application operation with shared begin/complete/fail semantics.
    pub async fn execute<T, E, Fut>(
        &self,
        operation: impl ApplicationOperation,
        execute: impl FnOnce(ApplicationWorkflowContext) -> Fut,
    ) -> Result<T, E>
    where
        E: From<ApplicationRuntimeError>,
        Fut: Future<Output = Result<T, ApplicationWorkflowFailure<E>>>,
    {
        self.kernel.execute(operation, execute).await
    }

    /// Executes one planned application operation with shared begin/complete/fail semantics.
    pub async fn execute_plan<T, E, Fut, O, P>(
        &self,
        plan: ApplicationOperationPlan<O, P>,
        execute: impl FnOnce(ApplicationWorkflowContext) -> Fut,
    ) -> Result<T, E>
    where
        O: ApplicationOperation,
        E: From<ApplicationRuntimeError>,
        Fut: Future<Output = Result<T, ApplicationWorkflowFailure<E>>>,
    {
        self.kernel.execute_plan(plan, execute).await
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
        self.kernel
            .execute_with_runtime_error(operation, map_runtime_error, execute)
            .await
    }

    /// Runs one operation plan, admits its request, and executes with the admitted value.
    pub async fn run_admitted_with_runtime_error<T, E, Fut, O, P>(
        &self,
        plan: ApplicationOperationPlan<O, P>,
        request: P::Request,
        map_runtime_error: impl Fn(ApplicationRuntimeError) -> E + Send + Sync,
        map_admission_error: impl Fn(P::Error) -> ApplicationWorkflowFailure<E> + Send,
        execute: impl FnOnce(P::Admission) -> Fut + Send,
    ) -> Result<T, E>
    where
        O: ApplicationOperation + Copy,
        P: ApplicationAdmissionPolicy + Send + Sync,
        P::Request: Send,
        P::Admission: Send,
        P::Error: Send,
        T: Send,
        E: Send,
        Fut: Future<Output = Result<T, ApplicationWorkflowFailure<E>>> + Send,
    {
        self.execute_with_runtime_error(plan.operation(), map_runtime_error, |context| async move {
            plan.execute_admitted(context, request, map_admission_error, execute)
                .await
        })
        .await
    }
}

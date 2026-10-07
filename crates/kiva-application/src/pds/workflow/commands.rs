//! PDS application command workflows.

mod consent;
mod data_products;
mod governance;
mod lifecycle;
#[cfg(test)]
mod tests;

use kiva_identity::AuthenticatedPrincipal;

use crate::runtime::{
    Admitted, ApplicationAdmissionPolicy, ApplicationOperationPlan, ApplicationRuntimeError,
    ApplicationRuntimePort, ApplicationWorkflowContext, ApplicationWorkflowFailure, PrincipalBound,
};

use super::{
    super::{
        capabilities::{PdsApplicationPortSet, PdsRecordWritePort},
        model::{
            ApplicationRevocationOutcome, ApplicationStoreOutcome, ApplicationWriteOutcome,
            PdsApplicationCommandError, PdsApplicationStoreError,
        },
    },
    PdsApplicationWorkflow, PdsWorkflowKind,
};

pub(in crate::pds) fn invalid_command_request(
    error: kiva_pds::ValidationError,
) -> PdsApplicationCommandError {
    PdsApplicationCommandError::InvalidRequest(error.into_message())
}

pub(in crate::pds) fn invalid_governance_role_overlap() -> PdsApplicationCommandError {
    PdsApplicationCommandError::InvalidRequest(
        "PDS governance profile must declare distinct public-sector duty subjects".to_owned(),
    )
}

pub(in crate::pds) fn invalid_governance_identity_authority_overlap() -> PdsApplicationCommandError
{
    PdsApplicationCommandError::InvalidRequest(
        "PDS governance profile identity authority must be absent or distinct from public-sector duty subjects"
            .to_owned(),
    )
}

impl<P, A> PdsApplicationWorkflow<P, A>
where
    P: PdsApplicationPortSet,
    A: ApplicationRuntimePort,
{
    pub(in crate::pds) async fn run_write_workflow<T, E, Fut>(
        &self,
        workflow_kind: PdsWorkflowKind,
        operation: impl FnOnce(ApplicationWorkflowContext) -> Fut,
    ) -> Result<T, E>
    where
        E: From<ApplicationRuntimeError>,
        Fut: Future<Output = Result<T, WorkflowFailure<E>>>,
    {
        debug_assert!(workflow_kind.is_command_boundary());
        let plan = ApplicationOperationPlan::new(workflow_kind);
        self.operation_gateway().execute_plan(plan, operation).await
    }

    pub(in crate::pds) async fn append_store_event(
        &self,
        context: ApplicationWorkflowContext,
        principal: &AuthenticatedPrincipal,
        event: impl Into<kiva_pds::PdsDomainEvent>,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationStoreError> {
        execute_principal_bound_domain_event(context, principal, event.into(), |event| async move {
            self.capabilities
                .pds_writes()
                .append_domain_event(principal, event)
                .await
                .and_then(ApplicationWriteOutcome::into_store)
        })
        .await
    }

    pub(in crate::pds) async fn append_revocation_event(
        &self,
        context: ApplicationWorkflowContext,
        principal: &AuthenticatedPrincipal,
        event: impl Into<kiva_pds::PdsDomainEvent>,
    ) -> Result<ApplicationRevocationOutcome, PdsApplicationStoreError> {
        execute_principal_bound_domain_event(context, principal, event.into(), |event| async move {
            self.capabilities
                .pds_writes()
                .append_domain_event(principal, event)
                .await
                .and_then(ApplicationWriteOutcome::into_revocation)
        })
        .await
    }
}

pub(in crate::pds) type WorkflowFailure<E> = ApplicationWorkflowFailure<E>;

#[cfg(test)]
fn admit_domain_event_for_principal(
    context: ApplicationWorkflowContext,
    principal: &AuthenticatedPrincipal,
    event: kiva_pds::PdsDomainEvent,
) -> Result<Admitted<kiva_pds::PdsDomainEvent, PrincipalBound>, PdsApplicationStoreError> {
    ApplicationOperationPlan::with_admission_policy(
        context.workflow_spec(),
        PdsDomainEventAdmissionPolicy::new(principal),
    )
    .admit(context, event)
}

async fn execute_principal_bound_domain_event<T, Fut>(
    context: ApplicationWorkflowContext,
    principal: &AuthenticatedPrincipal,
    event: kiva_pds::PdsDomainEvent,
    execute: impl FnOnce(Admitted<kiva_pds::PdsDomainEvent, PrincipalBound>) -> Fut + Send,
) -> Result<T, PdsApplicationStoreError>
where
    T: Send,
    Fut: Future<Output = Result<T, PdsApplicationStoreError>> + Send,
{
    ApplicationOperationPlan::with_admission_policy(
        context.workflow_spec(),
        PdsDomainEventAdmissionPolicy::new(principal),
    )
    .execute_admitted(
        context,
        event,
        |error| WorkflowFailure::new("pds domain event admission failed", error),
        |event| async move {
            execute(event).await.map_err(|error| {
                WorkflowFailure::new("pds domain event port execution failed", error)
            })
        },
    )
    .await
    .map_err(ApplicationWorkflowFailure::into_error)
}

#[derive(Debug, Clone, Copy)]
struct PdsDomainEventAdmissionPolicy<'a> {
    principal: &'a AuthenticatedPrincipal,
}

impl<'a> PdsDomainEventAdmissionPolicy<'a> {
    const fn new(principal: &'a AuthenticatedPrincipal) -> Self {
        Self { principal }
    }
}

impl ApplicationAdmissionPolicy for PdsDomainEventAdmissionPolicy<'_> {
    type Request = kiva_pds::PdsDomainEvent;
    type Admission = Admitted<kiva_pds::PdsDomainEvent, PrincipalBound>;
    type Error = PdsApplicationStoreError;

    fn admit(
        &self,
        context: ApplicationWorkflowContext,
        event: Self::Request,
    ) -> Result<Self::Admission, Self::Error> {
        let event_subject = event.controlling_subject();
        if event_subject == self.principal.subject {
            Ok(context.admit_principal(self.principal.subject.clone(), event))
        } else {
            Err(PdsApplicationStoreError::InvalidRequest(
                "PDS domain event subject must match the authenticated principal".to_owned(),
            ))
        }
    }
}

//! PDS lifecycle command workflows.

use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::{
    LifecycleCaseCommandInput as PdsLifecycleCaseCommandInput,
    LifecycleEventCommandInput as PdsLifecycleEventCommandInput,
    RecordLifecycleEventCommand as PdsRecordLifecycleEventCommand,
    RegisterRetentionPolicyCommand as PdsRegisterRetentionPolicyCommand,
    RetentionPolicyCommandInput as PdsRetentionPolicyCommandInput,
    UpsertLifecycleCaseCommand as PdsUpsertLifecycleCaseCommand,
};

use crate::runtime::ApplicationRuntimePort;

use super::super::{PdsApplicationWorkflow, PdsWorkflowKind};
use super::{WorkflowFailure, invalid_command_request};
use crate::pds::{ApplicationStoreOutcome, PdsApplicationPortSet, PdsApplicationWriteError};

impl<P, A> PdsApplicationWorkflow<P, A>
where
    P: PdsApplicationPortSet,
    A: ApplicationRuntimePort,
{
    pub(in crate::pds) async fn register_lifecycle_event(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsLifecycleEventCommandInput,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationWriteError> {
        self.run_write_workflow(
            PdsWorkflowKind::RegisterLifecycleEvent,
            |context| async move {
                let command = PdsRecordLifecycleEventCommand::new(
                    principal.subject.clone(),
                    request,
                    self.runtime().now_utc(),
                );
                let event = command
                    .lifecycle_event_recording_requested()
                    .map_err(|error| {
                        WorkflowFailure::new(
                            "pds lifecycle event command validation failed",
                            invalid_command_request(error).into(),
                        )
                    })?;
                self.append_store_event(context, principal, event)
                    .await
                    .map_err(|error| {
                        WorkflowFailure::new(
                            "pds lifecycle event persistence failed",
                            PdsApplicationWriteError::Pds(error),
                        )
                    })
            },
        )
        .await
    }

    pub(in crate::pds) async fn register_retention_policy(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsRetentionPolicyCommandInput,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationWriteError> {
        self.run_write_workflow(
            PdsWorkflowKind::RegisterRetentionPolicy,
            |context| async move {
                let command = PdsRegisterRetentionPolicyCommand::new(
                    principal.subject.clone(),
                    request,
                    self.runtime().now_utc(),
                );
                let event = command
                    .retention_policy_registration_requested()
                    .map_err(|error| {
                        WorkflowFailure::new(
                            "pds retention policy command validation failed",
                            invalid_command_request(error).into(),
                        )
                    })?;
                self.append_store_event(context, principal, event)
                    .await
                    .map_err(|error| {
                        WorkflowFailure::new(
                            "pds retention policy persistence failed",
                            PdsApplicationWriteError::Pds(error),
                        )
                    })
            },
        )
        .await
    }

    pub(in crate::pds) async fn register_lifecycle_case(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsLifecycleCaseCommandInput,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationWriteError> {
        self.run_write_workflow(
            PdsWorkflowKind::RegisterLifecycleCase,
            |context| async move {
                let command = PdsUpsertLifecycleCaseCommand::new(
                    principal.subject.clone(),
                    request,
                    self.runtime().now_utc(),
                );
                let event = command.lifecycle_case_upsert_requested().map_err(|error| {
                    WorkflowFailure::new(
                        "pds lifecycle case command validation failed",
                        invalid_command_request(error).into(),
                    )
                })?;
                self.append_store_event(context, principal, event)
                    .await
                    .map_err(|error| {
                        WorkflowFailure::new(
                            "pds lifecycle case persistence failed",
                            PdsApplicationWriteError::Pds(error),
                        )
                    })
            },
        )
        .await
    }
}

//! PDS public-sector governance command workflow.

use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::{
    GovernanceProfileCommandInput as PdsGovernanceProfileCommandInput,
    RegisterGovernanceProfileCommand as PdsRegisterGovernanceProfileCommand,
};

use crate::runtime::ApplicationRuntimePort;

use super::super::{PdsApplicationWorkflow, PdsWorkflowKind};
use super::{
    WorkflowFailure, invalid_command_request, invalid_governance_identity_authority_overlap,
    invalid_governance_role_overlap,
};
use crate::pds::{ApplicationStoreOutcome, PdsApplicationPortSet, PdsApplicationWriteError};

impl<P, A> PdsApplicationWorkflow<P, A>
where
    P: PdsApplicationPortSet,
    A: ApplicationRuntimePort,
{
    pub(in crate::pds) async fn register_governance_profile(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsGovernanceProfileCommandInput,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationWriteError> {
        self.run_write_workflow(
            PdsWorkflowKind::RegisterGovernanceProfile,
            |context| async move {
                let command = PdsRegisterGovernanceProfileCommand::new(
                    principal.subject.clone(),
                    request,
                    self.runtime().now_utc(),
                );
                let event = command
                    .governance_profile_registration_requested()
                    .map_err(|error| {
                        WorkflowFailure::new(
                            "pds governance profile command validation failed",
                            invalid_command_request(error).into(),
                        )
                    })?;
                if !event.distinct_role_subjects() {
                    return Err(WorkflowFailure::new(
                        "pds governance role overlap",
                        invalid_governance_role_overlap().into(),
                    ));
                }
                if !event.identity_authority_absent_or_distinct() {
                    return Err(WorkflowFailure::new(
                        "pds governance identity authority overlaps with duty subject",
                        invalid_governance_identity_authority_overlap().into(),
                    ));
                }
                self.append_store_event(context, principal, event)
                    .await
                    .map_err(|error| {
                        WorkflowFailure::new(
                            "pds governance profile persistence failed",
                            PdsApplicationWriteError::Pds(error),
                        )
                    })
            },
        )
        .await
    }
}

//! PDS consent command workflows.

use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::{
    ConsentGrantCommandInput as PdsConsentGrantCommandInput,
    ConsentRevocationCommandInput as PdsConsentRevocationCommandInput,
    RegisterConsentGrantCommand as PdsRegisterConsentGrantCommand,
    RevokeConsentGrantCommand as PdsRevokeConsentGrantCommand,
};

use crate::runtime::ApplicationRuntimePort;

use super::super::{PdsApplicationWorkflow, PdsWorkflowKind};
use super::{WorkflowFailure, invalid_command_request};
use crate::pds::{
    ApplicationRevocationOutcome, ApplicationStoreOutcome, PdsApplicationPortSet,
    PdsApplicationWriteError,
};

impl<P, A> PdsApplicationWorkflow<P, A>
where
    P: PdsApplicationPortSet,
    A: ApplicationRuntimePort,
{
    pub(in crate::pds) async fn register_consent_grant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsConsentGrantCommandInput,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationWriteError> {
        self.run_write_workflow(
            PdsWorkflowKind::RegisterConsentGrant,
            |context| async move {
                let command = PdsRegisterConsentGrantCommand::new(
                    principal.subject.clone(),
                    request,
                    self.runtime().now_utc(),
                );
                let event = command
                    .consent_grant_registration_requested()
                    .map_err(|error| {
                        WorkflowFailure::new(
                            "pds consent grant command validation failed",
                            invalid_command_request(error).into(),
                        )
                    })?;
                self.append_store_event(context, principal, event)
                    .await
                    .map_err(|error| {
                        WorkflowFailure::new(
                            "pds consent grant persistence failed",
                            PdsApplicationWriteError::Pds(error),
                        )
                    })
            },
        )
        .await
    }

    pub(in crate::pds) async fn revoke_consent_grant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsConsentRevocationCommandInput,
    ) -> Result<ApplicationRevocationOutcome, PdsApplicationWriteError> {
        self.run_write_workflow(PdsWorkflowKind::RevokeConsentGrant, |context| async move {
            let command = PdsRevokeConsentGrantCommand::new(
                principal.subject.clone(),
                request,
                self.runtime().now_utc(),
            );
            let event = command.consent_grant_revocation_requested();
            self.append_revocation_event(context, principal, event)
                .await
                .map_err(|error| {
                    WorkflowFailure::new(
                        "pds consent revocation persistence failed",
                        PdsApplicationWriteError::Pds(error),
                    )
                })
        })
        .await
    }
}

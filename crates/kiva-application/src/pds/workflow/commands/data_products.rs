//! PDS data-product command workflow.

use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::DataProductRegistrationCommandInput as PdsDataProductRegistrationCommandInput;

use crate::runtime::ApplicationRuntimePort;

use super::super::{PdsApplicationWorkflow, PdsWorkflowKind};
use super::{WorkflowFailure, invalid_command_request};
use crate::pds::{
    ApplicationStoreOutcome, PdsApplicationPortSet, RegisterDataProductError,
    ReleaseArtifactAvailability,
};

impl<P, A> PdsApplicationWorkflow<P, A>
where
    P: PdsApplicationPortSet,
    A: ApplicationRuntimePort,
{
    pub(in crate::pds) async fn register_data_product(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsDataProductRegistrationCommandInput,
    ) -> Result<ApplicationStoreOutcome, RegisterDataProductError> {
        self.run_write_workflow(PdsWorkflowKind::RegisterDataProduct, |context| async move {
            let command = kiva_pds::RegisterDataProductCommand::new(
                principal.subject.clone(),
                request,
                self.runtime().now_utc(),
            );
            let event = command
                .data_product_registration_requested()
                .map_err(|error| {
                    WorkflowFailure::new(
                        "pds data product command validation failed",
                        invalid_command_request(error).into(),
                    )
                })?;
            let artifact_available = self
                .release_artifacts()
                .is_available_for_pds_data_product(principal, event.release_artifact())
                .await
                .map_err(|error| {
                    WorkflowFailure::new(
                        "pds data product release artifact availability check failed",
                        RegisterDataProductError::ArtifactStore(error),
                    )
                })?;
            if !artifact_available {
                return Err(WorkflowFailure::new(
                    "pds data product release artifact unavailable",
                    RegisterDataProductError::ArtifactUnavailable,
                ));
            }
            self.append_store_event(context, principal, event)
                .await
                .map_err(|error| {
                    WorkflowFailure::new(
                        "pds data product persistence failed",
                        RegisterDataProductError::Pds(error),
                    )
                })
        })
        .await
    }
}

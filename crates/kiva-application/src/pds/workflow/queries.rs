//! PDS application query workflows.

use kiva_identity::AuthenticatedPrincipal;

use crate::runtime::{ApplicationRuntimePort, ApplicationWorkflowFailure};

use super::{
    super::{
        capabilities::PdsRecordWritePort,
        capabilities::{PdsApplicationPortSet, PdsSubjectProjection},
        model::{MetadataExport, PdsReadModelError, PortabilityExport, RecordInventory},
    },
    PdsApplicationWorkflow, PdsWorkflowKind,
};

impl<P, A> PdsApplicationWorkflow<P, A>
where
    P: PdsApplicationPortSet,
    A: ApplicationRuntimePort,
{
    pub(in crate::pds) async fn record_inventory(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<RecordInventory, PdsReadModelError> {
        let workflow_kind = PdsWorkflowKind::ReadRecordInventory;
        debug_assert!(workflow_kind.is_query_boundary());
        self.operation_gateway()
            .execute(workflow_kind, |_context| async {
                self.capabilities
                    .subject_projection()
                    .subject_snapshot(principal)
                    .await
                    .map(super::super::model::PdsSubjectSnapshot::into_record_inventory)
                    .map_err(|error| {
                        ApplicationWorkflowFailure::new(
                            "pds record inventory read model failed",
                            error,
                        )
                    })
            })
            .await
    }

    pub(in crate::pds) async fn metadata_export(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<MetadataExport, PdsReadModelError> {
        let workflow_kind = PdsWorkflowKind::ExportMetadata;
        debug_assert!(workflow_kind.is_query_boundary());
        self.operation_gateway()
            .execute(workflow_kind, |_context| async {
                self.capabilities
                    .subject_projection()
                    .subject_snapshot(principal)
                    .await
                    .map(super::super::model::PdsSubjectSnapshot::into_metadata_export)
                    .map_err(|error| {
                        ApplicationWorkflowFailure::new(
                            "pds metadata export read model failed",
                            error,
                        )
                    })
            })
            .await
    }

    pub(in crate::pds) async fn portability_export(
        &self,
        principal: &AuthenticatedPrincipal,
        cursor: Option<&str>,
        page_size: usize,
    ) -> Result<PortabilityExport, PdsReadModelError> {
        let workflow_kind = PdsWorkflowKind::ExportPortability;
        debug_assert!(workflow_kind.is_query_boundary());
        self.operation_gateway()
            .execute(workflow_kind, |context| async move {
                let generated_at = self.runtime().now_utc();
                let audit_reference = self.runtime().next_workflow_id();
                let snapshot = self
                    .capabilities
                    .subject_projection()
                    .portability_snapshot(principal, cursor, page_size)
                    .await;
                let (outcome, statement_count) = match &snapshot {
                    Ok(snapshot) => (
                        kiva_pds::PortabilityExportAuditOutcome::Completed,
                        u32::try_from(snapshot.statement_page.statements.len()).map_err(|_error| {
                            ApplicationWorkflowFailure::new(
                                "pds portability export page exceeded audit count bounds",
                                PdsReadModelError::Pds(
                                    super::super::model::PdsApplicationStoreError::InvalidStoredData,
                                ),
                            )
                        })?,
                    ),
                    Err(_error) => (kiva_pds::PortabilityExportAuditOutcome::Rejected, 0),
                };
                let audit = kiva_pds::PortabilityExportAuditRecorded::new(
                    audit_reference,
                    principal.subject.clone(),
                    outcome,
                    statement_count,
                    generated_at,
                );
                let admitted = context.admit_principal(principal.subject.clone(), audit.into());
                self.capabilities
                    .pds_writes()
                    .append_domain_event(principal, admitted)
                    .await
                    .map_err(PdsReadModelError::Pds)
                    .map_err(|error| {
                        ApplicationWorkflowFailure::new(
                            "pds portability export audit persistence failed",
                            error,
                        )
                    })?;
                snapshot
                    .map(|snapshot| snapshot.into_export(generated_at, audit_reference))
                    .map_err(|error| {
                        ApplicationWorkflowFailure::new(
                            "pds portability export projection failed",
                            error,
                        )
                    })
            })
            .await
    }
}

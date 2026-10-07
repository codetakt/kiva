//! PDS application query entry points.

use kiva_identity::AuthenticatedPrincipal;

use crate::runtime::ApplicationRuntimePort;

use super::{
    PdsApplication,
    capabilities::PdsApplicationPortSet,
    model::{MetadataExport, PdsReadModelError, PortabilityExport, RecordInventory},
};

impl<P, A> PdsApplication<P, A>
where
    P: PdsApplicationPortSet,
    A: ApplicationRuntimePort,
{
    /// Returns subject-scoped record inventory for the authenticated principal.
    pub async fn record_inventory(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<RecordInventory, PdsReadModelError> {
        self.capabilities
            .begin_workflow()
            .record_inventory(principal)
            .await
    }

    /// Returns a subject-scoped metadata export for the authenticated principal.
    pub async fn metadata_export(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<MetadataExport, PdsReadModelError> {
        self.capabilities
            .begin_workflow()
            .metadata_export(principal)
            .await
    }

    /// Returns one audited subject-bound portability export page.
    pub async fn portability_export(
        &self,
        principal: &AuthenticatedPrincipal,
        cursor: Option<&str>,
        page_size: usize,
    ) -> Result<PortabilityExport, PdsReadModelError> {
        self.capabilities
            .begin_workflow()
            .portability_export(principal, cursor, page_size)
            .await
    }
}

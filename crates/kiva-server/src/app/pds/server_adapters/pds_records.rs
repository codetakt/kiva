//! PDS repository adapter for PDS application records.

mod outcomes;
mod reads;
mod writes;

use kiva_application::{
    pds::{ApplicationFuture, PdsRecordReadPort, PdsRecordWritePort},
    runtime::{Admitted, PrincipalBound},
};
use kiva_identity::AuthenticatedPrincipal;

use crate::surfaces::pds::{repository::PdsRepository, service::PdsQueryGateway};

use super::super::{
    ApplicationWriteOutcome, DataProductRecord, PdsApplicationStoreError, PdsMetadataRecords,
};
use writes::PdsDomainEventProjector;

#[derive(Debug, Clone)]
pub(crate) struct PdsRecordPort {
    repository: PdsRepository,
}

impl PdsRecordPort {
    pub(super) fn from_server_adapter(repository: &PdsRepository) -> Self {
        Self {
            repository: repository.clone(),
        }
    }

    pub(super) fn query_gateway(&self) -> PdsQueryGateway {
        PdsQueryGateway::new(&self.repository)
    }
}

impl PdsRecordReadPort for PdsRecordPort {
    fn list_data_products<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
    ) -> ApplicationFuture<'a, Result<Vec<DataProductRecord>, PdsApplicationStoreError>> {
        Box::pin(async move { self.list_data_products_impl(principal).await })
    }

    fn list_metadata_records<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
    ) -> ApplicationFuture<'a, Result<PdsMetadataRecords, PdsApplicationStoreError>> {
        Box::pin(async move { self.list_metadata_records_impl(principal).await })
    }
}

impl PdsRecordWritePort for PdsRecordPort {
    fn append_domain_event<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        event: Admitted<kiva_pds::PdsDomainEvent, PrincipalBound>,
    ) -> ApplicationFuture<'a, Result<ApplicationWriteOutcome, PdsApplicationStoreError>> {
        self.project_domain_event(principal, event)
    }
}

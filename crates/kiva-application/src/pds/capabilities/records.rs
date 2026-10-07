//! Record and artifact ports for PDS application capabilities.

use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::{PdsDomainEvent, ReleaseArtifactRef};

use crate::runtime::{Admitted, ApplicationFuture, PrincipalBound};

use super::super::model::{
    ApplicationWriteOutcome, DataProductRecord, PdsApplicationStoreError, PdsMetadataRecords,
    PortabilityStatementPage, ReleaseArtifactAvailabilityError, StatementReadError,
    StatementRecord,
};

/// Port for reading xAPI statement metadata needed by PDS read models.
pub trait XapiStatementReader: Clone + Send + Sync + 'static {
    /// Lists statement metadata indexed by the subject actor IFI.
    fn list_metadata_by_actor_ifi<'a>(
        &'a self,
        actor_ifi: &'a str,
    ) -> ApplicationFuture<'a, Result<Vec<StatementRecord>, StatementReadError>>;

    /// Lists one bounded page of preserved Statement bodies indexed by subject actor IFI.
    fn list_portability_page_by_actor_ifi<'a>(
        &'a self,
        actor_ifi: &'a str,
        cursor: Option<&'a str>,
        page_size: usize,
    ) -> ApplicationFuture<'a, Result<PortabilityStatementPage, StatementReadError>>;
}

/// Port for reading PDS records used by application read models.
pub trait PdsRecordReadPort: Clone + Send + Sync + 'static {
    /// Lists approved data products visible to the authenticated principal.
    fn list_data_products<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
    ) -> ApplicationFuture<'a, Result<Vec<DataProductRecord>, PdsApplicationStoreError>>;

    /// Lists the PDS metadata families needed for subject exports.
    fn list_metadata_records<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
    ) -> ApplicationFuture<'a, Result<PdsMetadataRecords, PdsApplicationStoreError>>;
}

/// Port for writing admitted PDS record events used by command workflows.
pub trait PdsRecordWritePort: Clone + Send + Sync + 'static {
    /// Appends one admitted PDS domain event and projects its write outcome.
    fn append_domain_event<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        event: Admitted<PdsDomainEvent, PrincipalBound>,
    ) -> ApplicationFuture<'a, Result<ApplicationWriteOutcome, PdsApplicationStoreError>>;
}

/// Port for checking confidential release artifacts without importing confidential state.
pub trait ReleaseArtifactAvailability: Clone + Send + Sync + 'static {
    /// Returns whether the artifact can back a PDS data product for this principal.
    fn is_available_for_pds_data_product<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        artifact: ReleaseArtifactRef,
    ) -> ApplicationFuture<'a, Result<bool, ReleaseArtifactAvailabilityError>>;
}

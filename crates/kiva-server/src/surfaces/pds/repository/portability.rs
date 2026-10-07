use crate::core::outcome::StoreOutcome;

use super::{super::outcome::PdsRepositoryError, PdsRepository};

impl PdsRepository {
    pub(crate) async fn record_portability_export_audit(
        &self,
        event: &kiva_pds::PortabilityExportAuditRecorded,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        dispatch_repository!(self, |repository| {
            memory => Ok(repository.record_portability_export_audit(event).await),
            postgres => repository.record_portability_export_audit(event).await,
        })
    }
}

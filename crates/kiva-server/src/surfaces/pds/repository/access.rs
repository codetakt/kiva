//! PDS repository facade methods for access checks and audit events.

use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::AccessCheckIntent as PdsAccessCheckIntent;

use crate::surfaces::pds::{
    model::{PdsAccessAuditEventFilter, PdsAccessAuditEventState},
    outcome::PdsRepositoryError,
    repository::PdsRepository,
};

impl PdsRepository {
    pub(crate) async fn record_analytics_read(
        &self,
        event: PdsAccessAuditEventState,
    ) -> Result<(), PdsRepositoryError> {
        dispatch_repository!(self, |repository| repository
            .record_analytics_read(event)
            .await)
    }

    pub(crate) async fn evaluate_access_check(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsAccessCheckIntent,
    ) -> Result<PdsAccessAuditEventState, PdsRepositoryError> {
        dispatch_repository!(self, |repository| repository
            .evaluate_access_check(principal, request)
            .await)
    }

    pub(crate) async fn list_access_audit_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsAccessAuditEventFilter,
    ) -> Result<Vec<PdsAccessAuditEventState>, PdsRepositoryError> {
        dispatch_repository!(self, |repository| repository
            .list_access_audit_events(principal, filter)
            .await)
    }
}

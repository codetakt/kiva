//! PDS application write adapter methods.

mod consent;
mod data_products;
mod governance;
mod lifecycle;

use kiva_identity::AuthenticatedPrincipal;

use crate::app::pds::{ApplicationFuture, ApplicationWriteOutcome, PdsApplicationStoreError};
use kiva_application::runtime::{Admitted, PrincipalBound};

use super::PdsRecordPort;

pub(super) trait PdsDomainEventProjector {
    fn project_domain_event<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        event: Admitted<kiva_pds::PdsDomainEvent, PrincipalBound>,
    ) -> ApplicationFuture<'a, Result<ApplicationWriteOutcome, PdsApplicationStoreError>>;
}

impl PdsDomainEventProjector for PdsRecordPort {
    fn project_domain_event<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        event: Admitted<kiva_pds::PdsDomainEvent, PrincipalBound>,
    ) -> ApplicationFuture<'a, Result<ApplicationWriteOutcome, PdsApplicationStoreError>> {
        Box::pin(async move { self.project_domain_event_impl(principal, event).await })
    }
}

impl PdsRecordPort {
    async fn project_domain_event_impl(
        &self,
        _principal: &AuthenticatedPrincipal,
        event: Admitted<kiva_pds::PdsDomainEvent, PrincipalBound>,
    ) -> Result<ApplicationWriteOutcome, PdsApplicationStoreError> {
        match event.into_inner() {
            kiva_pds::PdsDomainEvent::ConsentGrantRegistrationRequested(event) => self
                .register_consent_grant_impl(event)
                .await
                .map(ApplicationWriteOutcome::Store),
            kiva_pds::PdsDomainEvent::ConsentGrantRevocationRequested(event) => self
                .revoke_consent_grant_impl(event)
                .await
                .map(ApplicationWriteOutcome::Revocation),
            kiva_pds::PdsDomainEvent::DataProductRegistrationRequested(event) => self
                .register_data_product_impl(event)
                .await
                .map(ApplicationWriteOutcome::Store),
            kiva_pds::PdsDomainEvent::LifecycleEventRecordingRequested(event) => self
                .register_lifecycle_event_impl(event)
                .await
                .map(ApplicationWriteOutcome::Store),
            kiva_pds::PdsDomainEvent::RetentionPolicyRegistrationRequested(event) => self
                .register_retention_policy_impl(event)
                .await
                .map(ApplicationWriteOutcome::Store),
            kiva_pds::PdsDomainEvent::LifecycleCaseUpsertRequested(event) => self
                .register_lifecycle_case_impl(event)
                .await
                .map(ApplicationWriteOutcome::Store),
            kiva_pds::PdsDomainEvent::GovernanceProfileRegistrationRequested(event) => self
                .register_governance_profile_impl(event)
                .await
                .map(ApplicationWriteOutcome::Store),
            kiva_pds::PdsDomainEvent::PortabilityExportAuditRecorded(event) => self
                .repository
                .record_portability_export_audit(&event)
                .await
                .map(super::outcomes::application_store_outcome)
                .map(ApplicationWriteOutcome::Store)
                .map_err(super::super::records::pds_error),
        }
    }
}

//! Consent-family PDS write projections.

use crate::app::pds::{
    ApplicationRevocationOutcome, ApplicationStoreOutcome, PdsApplicationStoreError,
};

use super::super::super::records::pds_error;
use super::super::{
    PdsRecordPort,
    outcomes::{application_revocation_outcome, application_store_outcome},
};

impl PdsRecordPort {
    pub(super) async fn register_consent_grant_impl(
        &self,
        event: kiva_pds::ConsentGrantRegistrationRequested,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationStoreError> {
        self.repository
            .register_consent_grant(event)
            .await
            .map(application_store_outcome)
            .map_err(pds_error)
    }

    pub(super) async fn revoke_consent_grant_impl(
        &self,
        event: kiva_pds::ConsentGrantRevocationRequested,
    ) -> Result<ApplicationRevocationOutcome, PdsApplicationStoreError> {
        self.repository
            .revoke_consent_grant(event)
            .await
            .map(application_revocation_outcome)
            .map_err(pds_error)
    }
}

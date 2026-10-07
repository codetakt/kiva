//! Governance-family PDS write projections.

use crate::app::pds::{ApplicationStoreOutcome, PdsApplicationStoreError};

use super::super::super::records::pds_error;
use super::super::{PdsRecordPort, outcomes::application_store_outcome};

impl PdsRecordPort {
    pub(super) async fn register_governance_profile_impl(
        &self,
        event: kiva_pds::GovernanceProfileRegistrationRequested,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationStoreError> {
        self.repository
            .register_governance_profile(event)
            .await
            .map(application_store_outcome)
            .map_err(pds_error)
    }
}

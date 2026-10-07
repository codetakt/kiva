//! Lifecycle-family PDS write projections.

use crate::app::pds::{ApplicationStoreOutcome, PdsApplicationStoreError};

use super::super::super::records::pds_error;
use super::super::{PdsRecordPort, outcomes::application_store_outcome};

impl PdsRecordPort {
    pub(super) async fn register_lifecycle_event_impl(
        &self,
        event: kiva_pds::LifecycleEventRecordingRequested,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationStoreError> {
        self.repository
            .register_lifecycle_event(event)
            .await
            .map(application_store_outcome)
            .map_err(pds_error)
    }

    pub(super) async fn register_retention_policy_impl(
        &self,
        event: kiva_pds::RetentionPolicyRegistrationRequested,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationStoreError> {
        self.repository
            .register_retention_policy(event)
            .await
            .map(application_store_outcome)
            .map_err(pds_error)
    }

    pub(super) async fn register_lifecycle_case_impl(
        &self,
        event: kiva_pds::LifecycleCaseUpsertRequested,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationStoreError> {
        self.repository
            .register_lifecycle_case(event)
            .await
            .map(application_store_outcome)
            .map_err(pds_error)
    }
}

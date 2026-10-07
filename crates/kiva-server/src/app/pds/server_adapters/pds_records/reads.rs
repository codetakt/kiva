//! PDS application read adapter methods.

use kiva_identity::AuthenticatedPrincipal;

use crate::{
    app::pds::{DataProductRecord, PdsApplicationStoreError, PdsMetadataRecords},
    surfaces::pds::{
        model::{
            PdsConsentGrantFilter, PdsDataProductFilter, PdsGovernanceProfileFilter,
            PdsLifecycleCaseFilter, PdsLifecycleEventFilter, PdsRetentionPolicyFilter,
        },
        service as pds_service,
    },
};

use super::{
    super::records::{
        consent_grant_record, data_product_record, governance_profile_record,
        lifecycle_case_record, lifecycle_event_record, pds_error, retention_policy_record,
    },
    PdsRecordPort,
};

impl PdsRecordPort {
    pub(super) async fn list_data_products_impl(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<Vec<DataProductRecord>, PdsApplicationStoreError> {
        pds_service::list_data_products(
            self.query_gateway(),
            principal,
            PdsDataProductFilter::default(),
        )
        .await
        .map(|products| products.into_iter().map(data_product_record).collect())
        .map_err(pds_error)
    }

    pub(super) async fn list_metadata_records_impl(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<PdsMetadataRecords, PdsApplicationStoreError> {
        let consent_grants = pds_service::list_consent_grants(
            self.query_gateway(),
            principal,
            PdsConsentGrantFilter::default(),
        )
        .await
        .map_err(pds_error)?
        .into_iter()
        .map(consent_grant_record)
        .collect();
        let lifecycle_events = pds_service::list_lifecycle_events(
            self.query_gateway(),
            principal,
            PdsLifecycleEventFilter::default(),
        )
        .await
        .map_err(pds_error)?
        .into_iter()
        .map(lifecycle_event_record)
        .collect();
        let retention_policies = pds_service::list_retention_policies(
            self.query_gateway(),
            principal,
            PdsRetentionPolicyFilter::default(),
        )
        .await
        .map_err(pds_error)?
        .into_iter()
        .map(retention_policy_record)
        .collect();
        let lifecycle_cases = pds_service::list_lifecycle_cases(
            self.query_gateway(),
            principal,
            PdsLifecycleCaseFilter::default(),
        )
        .await
        .map_err(pds_error)?
        .into_iter()
        .map(lifecycle_case_record)
        .collect();
        let governance_profiles = pds_service::list_governance_profiles(
            self.query_gateway(),
            principal,
            PdsGovernanceProfileFilter::default(),
        )
        .await
        .map_err(pds_error)?
        .into_iter()
        .map(governance_profile_record)
        .collect();
        Ok(PdsMetadataRecords {
            consent_grants,
            lifecycle_events,
            retention_policies,
            lifecycle_cases,
            governance_profiles,
        })
    }
}

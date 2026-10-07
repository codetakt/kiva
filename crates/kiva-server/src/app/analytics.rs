//! Server adapters for the HTTP-independent analytics application boundary.

use kiva_application::analytics::{
    AnalyticsAggregate, AnalyticsApplication, AnalyticsProduct, AnalyticsReadAudit,
    AnalyticsReadOutcome, AnalyticsServingPort, AnalyticsStoreError,
};
use kiva_confidential::primitive::SecureAggregateTranscriptId;
use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::{
    DataProductSourceKind, LifecycleCaseStatus, LifecycleEventType, LifecycleResourceKind,
};
use kiva_policy::policy::{Action, Decision, DenyReason, PrincipalRole, ResourceClass};
use uuid::Uuid;

use std::collections::BTreeSet;

use crate::surfaces::{
    confidential::{
        model::{ConfidentialDifferentialPrivacyReleaseFilter, ConfidentialThresholdReleaseFilter},
        repository::ConfidentialRepository,
        storage::field_element_i64,
    },
    pds::{
        model::{
            PdsAccessAuditEventState, PdsDataProductFilter, PdsDataProductState,
            PdsLifecycleCaseFilter, PdsLifecycleEventFilter,
        },
        repository::PdsRepository,
    },
};

/// Composition adapter over PDS product metadata, confidential releases, and audit storage.
#[derive(Debug, Clone)]
pub(crate) struct AnalyticsApplicationAdapter {
    pds: PdsRepository,
    confidential: ConfidentialRepository,
}

impl AnalyticsApplicationAdapter {
    pub(crate) const fn new(pds: PdsRepository, confidential: ConfidentialRepository) -> Self {
        Self { pds, confidential }
    }

    pub(crate) fn application(&self) -> AnalyticsApplication<Self> {
        AnalyticsApplication::new(self.clone())
    }

    async fn admitted_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        product: &PdsDataProductState,
    ) -> Result<Option<AnalyticsAggregate>, AnalyticsStoreError> {
        let release_id = SecureAggregateTranscriptId::new(product.release_artifact().artifact_id())
            .map_err(|_error| AnalyticsStoreError::InvalidStoredData)?;
        match product.source_kind() {
            DataProductSourceKind::ThresholdRelease => {
                let releases = self
                    .confidential
                    .list_threshold_release_requests(ConfidentialThresholdReleaseFilter {
                        release_id: Some(release_id),
                        transcript_id: None,
                    })
                    .await
                    .map_err(|_error| AnalyticsStoreError::Store)?;
                Ok(releases.into_iter().find_map(|release| {
                    let released_at = release.released_at?;
                    (release.requester_subject == principal.subject)
                        .then(|| field_element_i64(release.aggregate).ok())
                        .flatten()
                        .map(|value| AnalyticsAggregate {
                            data_product_id: product.data_product_id,
                            release_kind: product.source_kind().label().to_owned(),
                            value,
                            released_at,
                        })
                }))
            }
            DataProductSourceKind::DifferentialPrivacyRelease => {
                let releases = self
                    .confidential
                    .list_differential_privacy_releases(
                        principal,
                        ConfidentialDifferentialPrivacyReleaseFilter {
                            release: Some(release_id),
                            budget: None,
                            transcript: None,
                        },
                    )
                    .await
                    .map_err(|_error| AnalyticsStoreError::Store)?;
                Ok(releases
                    .into_iter()
                    .next()
                    .map(|release| AnalyticsAggregate {
                        data_product_id: product.data_product_id,
                        release_kind: product.source_kind().label().to_owned(),
                        value: release.noised_aggregate.noised_value(),
                        released_at: release.created_at,
                    }))
            }
            DataProductSourceKind::ShamirThresholdReconstruction => Ok(None),
        }
    }

    async fn product_state(
        &self,
        principal: &AuthenticatedPrincipal,
        data_product_id: Uuid,
    ) -> Result<Option<PdsDataProductState>, AnalyticsStoreError> {
        self.pds
            .list_data_products(
                principal,
                PdsDataProductFilter {
                    data_product_id: Some(data_product_id),
                },
            )
            .await
            .map(|mut products| products.pop())
            .map_err(|_error| AnalyticsStoreError::Store)
    }

    async fn withdrawn_product_ids(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<BTreeSet<Uuid>, AnalyticsStoreError> {
        let cases = self
            .pds
            .list_lifecycle_cases(principal, PdsLifecycleCaseFilter::default())
            .await
            .map_err(|_error| AnalyticsStoreError::Store)?;
        let events = self
            .pds
            .list_lifecycle_events(principal, PdsLifecycleEventFilter::default())
            .await
            .map_err(|_error| AnalyticsStoreError::Store)?;
        cases
            .into_iter()
            .filter(|case| {
                case.resource_kind == LifecycleResourceKind::DataProduct
                    && case.status == LifecycleCaseStatus::Tombstoned
            })
            .map(|case| case.resource_id)
            .chain(
                events
                    .into_iter()
                    .filter(|event| {
                        event.resource_kind == LifecycleResourceKind::DataProduct
                            && event.event_type == LifecycleEventType::TombstoneRecorded
                    })
                    .map(|event| event.resource_id),
            )
            .map(|resource_id| {
                Uuid::parse_str(&resource_id)
                    .map_err(|_error| AnalyticsStoreError::InvalidStoredData)
            })
            .collect()
    }
}

fn product_metadata(product: &PdsDataProductState) -> AnalyticsProduct {
    AnalyticsProduct {
        data_product_id: product.data_product_id,
        title: product.title.clone(),
        purpose: product.purpose.clone(),
        release_kind: product.source_kind().label().to_owned(),
        approved_at: product.created_at,
    }
}

impl AnalyticsServingPort for AnalyticsApplicationAdapter {
    fn list_products<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
    ) -> kiva_application::runtime::ApplicationFuture<
        'a,
        Result<Vec<AnalyticsProduct>, AnalyticsStoreError>,
    > {
        Box::pin(async move {
            let products = self
                .pds
                .list_data_products(principal, PdsDataProductFilter::default())
                .await
                .map_err(|_error| AnalyticsStoreError::Store)?;
            let withdrawn = self.withdrawn_product_ids(principal).await?;
            let mut admitted = Vec::new();
            for product in products {
                if !withdrawn.contains(&product.data_product_id)
                    && self
                        .admitted_aggregate(principal, &product)
                        .await?
                        .is_some()
                {
                    admitted.push(product_metadata(&product));
                }
            }
            admitted.sort_by_key(|product| (product.approved_at, product.data_product_id));
            Ok(admitted)
        })
    }

    fn read_product<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        data_product_id: Uuid,
    ) -> kiva_application::runtime::ApplicationFuture<
        'a,
        Result<Option<AnalyticsProduct>, AnalyticsStoreError>,
    > {
        Box::pin(async move {
            let Some(product) = self.product_state(principal, data_product_id).await? else {
                return Ok(None);
            };
            if self
                .withdrawn_product_ids(principal)
                .await?
                .contains(&data_product_id)
            {
                return Ok(None);
            }
            if self
                .admitted_aggregate(principal, &product)
                .await?
                .is_none()
            {
                return Ok(None);
            }
            Ok(Some(product_metadata(&product)))
        })
    }

    fn read_aggregate<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        data_product_id: Uuid,
    ) -> kiva_application::runtime::ApplicationFuture<
        'a,
        Result<Option<AnalyticsAggregate>, AnalyticsStoreError>,
    > {
        Box::pin(async move {
            let Some(product) = self.product_state(principal, data_product_id).await? else {
                return Ok(None);
            };
            if self
                .withdrawn_product_ids(principal)
                .await?
                .contains(&data_product_id)
            {
                return Ok(None);
            }
            self.admitted_aggregate(principal, &product).await
        })
    }

    fn record_read(
        &self,
        audit: AnalyticsReadAudit,
    ) -> kiva_application::runtime::ApplicationFuture<'_, Result<(), AnalyticsStoreError>> {
        Box::pin(async move {
            self.pds
                .record_analytics_read(PdsAccessAuditEventState {
                    event_id: audit.event_id,
                    requester_subject: audit.requester_subject,
                    role: PrincipalRole::Analyst,
                    resource: ResourceClass::AggregateInsight,
                    action: Action::Read,
                    grant_id: None,
                    data_product_id: audit.data_product_id,
                    decision: match audit.outcome {
                        AnalyticsReadOutcome::Allowed => Decision::Allow,
                        AnalyticsReadOutcome::NotFound => {
                            Decision::Deny(DenyReason::DataProductUnavailable)
                        }
                    },
                    created_at: audit.occurred_at,
                })
                .await
                .map_err(|_error| AnalyticsStoreError::Store)
        })
    }
}

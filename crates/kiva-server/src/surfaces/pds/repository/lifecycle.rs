//! PDS repository facade methods for lifecycle and retention resources.

use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::{
    LifecycleCaseUpsertRequested as PdsLifecycleCaseUpsertRequested,
    LifecycleEventRecordingRequested as PdsLifecycleEventRecordingRequested,
    RetentionPolicyRegistrationRequested as PdsRetentionPolicyRegistrationRequested,
};

use crate::{
    core::outcome::StoreOutcome,
    surfaces::pds::{
        model::{
            PdsLifecycleCaseFilter, PdsLifecycleCaseProjection, PdsLifecycleCaseState,
            PdsLifecycleEventFilter, PdsLifecycleEventProjection, PdsLifecycleEventState,
            PdsRetentionPolicyFilter, PdsRetentionPolicyProjection, PdsRetentionPolicyState,
        },
        outcome::PdsRepositoryError,
        repository::PdsRepository,
    },
};

impl PdsRepository {
    pub(crate) async fn register_lifecycle_event(
        &self,
        requested: PdsLifecycleEventRecordingRequested,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let projection = PdsLifecycleEventProjection::from_admitted_event(&requested);
        dispatch_repository!(self, |repository| repository
            .register_lifecycle_event(projection)
            .await)
    }

    pub(crate) async fn list_lifecycle_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsLifecycleEventFilter,
    ) -> Result<Vec<PdsLifecycleEventState>, PdsRepositoryError> {
        dispatch_repository!(self, |repository| repository
            .list_lifecycle_events(principal, filter)
            .await)
    }

    pub(crate) async fn register_retention_policy(
        &self,
        event: PdsRetentionPolicyRegistrationRequested,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let projection = PdsRetentionPolicyProjection::from_admitted_event(&event);
        dispatch_repository!(self, |repository| repository
            .register_retention_policy(projection)
            .await)
    }

    pub(crate) async fn list_retention_policies(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsRetentionPolicyFilter,
    ) -> Result<Vec<PdsRetentionPolicyState>, PdsRepositoryError> {
        dispatch_repository!(self, |repository| repository
            .list_retention_policies(principal, filter)
            .await)
    }

    pub(crate) async fn register_lifecycle_case(
        &self,
        event: PdsLifecycleCaseUpsertRequested,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let projection = PdsLifecycleCaseProjection::from_admitted_event(&event);
        dispatch_repository!(self, |repository| repository
            .register_lifecycle_case(projection)
            .await)
    }

    pub(crate) async fn list_lifecycle_cases(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsLifecycleCaseFilter,
    ) -> Result<Vec<PdsLifecycleCaseState>, PdsRepositoryError> {
        dispatch_repository!(self, |repository| repository
            .list_lifecycle_cases(principal, filter)
            .await)
    }
}

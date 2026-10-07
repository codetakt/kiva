//! PDS lifecycle server-local state.

use kiva_pds::{
    DataClass as PdsDataClass, LegalBasis as PdsLegalBasis,
    LifecycleCaseStatus as PdsLifecycleCaseStatus, LifecycleCaseType as PdsLifecycleCaseType,
    LifecycleEventType as PdsLifecycleEventType, LifecycleResourceKind as PdsLifecycleResourceKind,
    TombstonePublicationRule as PdsTombstonePublicationRule,
};
use kiva_policy::policy::pds_lifecycle_case_transition_is_allowed;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    core::outcome::StoreOutcome,
    surfaces::pds::{
        model::policy_adapter::pds_lifecycle_case_policy_status, outcome::PdsRepositoryError,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdsLifecycleEventState {
    pub(crate) event_id: Uuid,
    pub(crate) data_subject: String,
    pub(crate) event_type: PdsLifecycleEventType,
    pub(crate) resource_kind: PdsLifecycleResourceKind,
    pub(crate) resource_id: String,
    pub(crate) reason: String,
    pub(crate) created_at: OffsetDateTime,
}

impl PdsLifecycleEventState {
    pub(crate) fn same_registration(&self, other: &Self) -> bool {
        self.event_id == other.event_id
            && self.data_subject == other.data_subject
            && self.event_type == other.event_type
            && self.resource_kind == other.resource_kind
            && self.resource_id == other.resource_id
            && self.reason == other.reason
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdsRetentionPolicyState {
    pub(crate) policy_id: Uuid,
    pub(crate) owner_subject: String,
    pub(crate) resource_kind: PdsLifecycleResourceKind,
    pub(crate) data_class: PdsDataClass,
    pub(crate) retention_period_days: u32,
    pub(crate) review_interval_days: u32,
    pub(crate) tombstone_after_days: Option<u32>,
    pub(crate) tombstone_publication: PdsTombstonePublicationRule,
    pub(crate) legal_basis: PdsLegalBasis,
    pub(crate) description: String,
    pub(crate) created_at: OffsetDateTime,
}

impl PdsRetentionPolicyState {
    pub(crate) fn same_registration(&self, other: &Self) -> bool {
        self.policy_id == other.policy_id
            && self.owner_subject == other.owner_subject
            && self.resource_kind == other.resource_kind
            && self.data_class == other.data_class
            && self.retention_period_days == other.retention_period_days
            && self.review_interval_days == other.review_interval_days
            && self.tombstone_after_days == other.tombstone_after_days
            && self.tombstone_publication == other.tombstone_publication
            && self.legal_basis == other.legal_basis
            && self.description == other.description
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdsLifecycleCaseState {
    pub(crate) case_id: Uuid,
    pub(crate) data_subject: String,
    pub(crate) case_type: PdsLifecycleCaseType,
    pub(crate) resource_kind: PdsLifecycleResourceKind,
    pub(crate) resource_id: String,
    pub(crate) status: PdsLifecycleCaseStatus,
    pub(crate) retention_policy_id: Option<Uuid>,
    pub(crate) summary: String,
    pub(crate) resolution: Option<String>,
    pub(crate) tombstone_publication: Option<PdsTombstonePublicationRule>,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) updated_at: OffsetDateTime,
}

impl PdsLifecycleCaseState {
    pub(crate) fn same_workflow_identity(&self, other: &Self) -> bool {
        self.case_id == other.case_id
            && self.data_subject == other.data_subject
            && self.case_type == other.case_type
            && self.resource_kind == other.resource_kind
            && self.resource_id == other.resource_id
            && self.retention_policy_id == other.retention_policy_id
            && self.summary == other.summary
            && self.tombstone_publication == other.tombstone_publication
    }

    pub(crate) fn same_registration(&self, other: &Self) -> bool {
        self.same_workflow_identity(other)
            && self.status == other.status
            && self.resolution == other.resolution
    }

    pub(crate) fn transition_from(
        &self,
        requested: &Self,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        if !self.same_workflow_identity(requested) {
            return Ok(StoreOutcome::Conflict);
        }
        if self.same_registration(requested) {
            return Ok(StoreOutcome::Unchanged);
        }
        if self.status.requires_resolution() {
            return Ok(StoreOutcome::Conflict);
        }
        if !pds_lifecycle_case_transition_is_allowed(
            Some(pds_lifecycle_case_policy_status(self.status)),
            pds_lifecycle_case_policy_status(requested.status),
        ) {
            return Err(PdsRepositoryError::InvalidRequest(
                "PDS lifecycle case status transition is not allowed".to_owned(),
            ));
        }
        Ok(StoreOutcome::Stored)
    }
}

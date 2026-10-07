//! Retention-policy PDS wire projection.

use kiva_pds::{
    DataClass, LegalBasis, LifecycleResourceKind, RetentionPolicy as RetentionPolicyDto,
    RetentionPolicyList, TombstonePublicationRule,
};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    app::pds as pds_app, core::response::rfc3339, surfaces::pds::model::PdsRetentionPolicyState,
};

use super::super::common::dto_value;

pub(crate) fn retention_policy_list_response(
    policies: &[PdsRetentionPolicyState],
) -> Result<Value, serde_json::Error> {
    dto_value(&RetentionPolicyList {
        policies: policies.iter().map(retention_policy_dto).collect(),
    })
}

pub(in crate::surfaces::pds::wire) trait RetentionPolicyProjection {
    fn policy_id(&self) -> Uuid;
    fn owner_subject(&self) -> &str;
    fn resource_kind(&self) -> LifecycleResourceKind;
    fn data_class(&self) -> DataClass;
    fn retention_period_days(&self) -> u32;
    fn review_interval_days(&self) -> u32;
    fn tombstone_after_days(&self) -> Option<u32>;
    fn tombstone_publication(&self) -> TombstonePublicationRule;
    fn legal_basis(&self) -> LegalBasis;
    fn description(&self) -> &str;
    fn created_at(&self) -> OffsetDateTime;
}

impl RetentionPolicyProjection for PdsRetentionPolicyState {
    fn policy_id(&self) -> Uuid {
        self.policy_id
    }

    fn owner_subject(&self) -> &str {
        &self.owner_subject
    }

    fn resource_kind(&self) -> LifecycleResourceKind {
        self.resource_kind
    }

    fn data_class(&self) -> DataClass {
        self.data_class
    }

    fn retention_period_days(&self) -> u32 {
        self.retention_period_days
    }

    fn review_interval_days(&self) -> u32 {
        self.review_interval_days
    }

    fn tombstone_after_days(&self) -> Option<u32> {
        self.tombstone_after_days
    }

    fn tombstone_publication(&self) -> TombstonePublicationRule {
        self.tombstone_publication
    }

    fn legal_basis(&self) -> LegalBasis {
        self.legal_basis
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

impl RetentionPolicyProjection for pds_app::RetentionPolicyRecord {
    fn policy_id(&self) -> Uuid {
        self.policy_id
    }

    fn owner_subject(&self) -> &str {
        &self.owner_subject
    }

    fn resource_kind(&self) -> LifecycleResourceKind {
        self.resource_kind
    }

    fn data_class(&self) -> DataClass {
        self.data_class
    }

    fn retention_period_days(&self) -> u32 {
        self.retention_period_days
    }

    fn review_interval_days(&self) -> u32 {
        self.review_interval_days
    }

    fn tombstone_after_days(&self) -> Option<u32> {
        self.tombstone_after_days
    }

    fn tombstone_publication(&self) -> TombstonePublicationRule {
        self.tombstone_publication
    }

    fn legal_basis(&self) -> LegalBasis {
        self.legal_basis
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

pub(in crate::surfaces::pds::wire) fn retention_policy_dto<T: RetentionPolicyProjection>(
    policy: &T,
) -> RetentionPolicyDto {
    RetentionPolicyDto {
        policy_id: policy.policy_id().to_string(),
        owner_subject: policy.owner_subject().to_owned(),
        resource_kind: policy.resource_kind(),
        data_class: policy.data_class(),
        retention_period_days: policy.retention_period_days(),
        review_interval_days: policy.review_interval_days(),
        tombstone_after_days: policy.tombstone_after_days(),
        tombstone_publication: policy.tombstone_publication(),
        legal_basis: policy.legal_basis(),
        description: policy.description().to_owned(),
        created_at: rfc3339(policy.created_at()),
    }
}

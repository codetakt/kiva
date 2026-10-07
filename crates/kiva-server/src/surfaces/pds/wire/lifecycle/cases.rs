//! Lifecycle-case PDS wire projection.

use kiva_pds::{
    LifecycleCase as LifecycleCaseDto, LifecycleCaseList, LifecycleCaseStatus, LifecycleCaseType,
    LifecycleResourceKind, TombstonePublicationRule,
};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    app::pds as pds_app, core::response::rfc3339, surfaces::pds::model::PdsLifecycleCaseState,
};

use super::super::common::dto_value;

pub(crate) fn lifecycle_case_list_response(
    cases: &[PdsLifecycleCaseState],
) -> Result<Value, serde_json::Error> {
    dto_value(&LifecycleCaseList {
        cases: cases.iter().map(lifecycle_case_dto).collect(),
    })
}

pub(in crate::surfaces::pds::wire) trait LifecycleCaseProjection {
    fn case_id(&self) -> Uuid;
    fn data_subject(&self) -> &str;
    fn case_type(&self) -> LifecycleCaseType;
    fn resource_kind(&self) -> LifecycleResourceKind;
    fn resource_id(&self) -> &str;
    fn status(&self) -> LifecycleCaseStatus;
    fn retention_policy_id(&self) -> Option<Uuid>;
    fn summary(&self) -> &str;
    fn resolution(&self) -> Option<&str>;
    fn tombstone_publication(&self) -> Option<TombstonePublicationRule>;
    fn created_at(&self) -> OffsetDateTime;
    fn updated_at(&self) -> OffsetDateTime;
}

impl LifecycleCaseProjection for PdsLifecycleCaseState {
    fn case_id(&self) -> Uuid {
        self.case_id
    }

    fn data_subject(&self) -> &str {
        &self.data_subject
    }

    fn case_type(&self) -> LifecycleCaseType {
        self.case_type
    }

    fn resource_kind(&self) -> LifecycleResourceKind {
        self.resource_kind
    }

    fn resource_id(&self) -> &str {
        &self.resource_id
    }

    fn status(&self) -> LifecycleCaseStatus {
        self.status
    }

    fn retention_policy_id(&self) -> Option<Uuid> {
        self.retention_policy_id
    }

    fn summary(&self) -> &str {
        &self.summary
    }

    fn resolution(&self) -> Option<&str> {
        self.resolution.as_deref()
    }

    fn tombstone_publication(&self) -> Option<TombstonePublicationRule> {
        self.tombstone_publication
    }

    fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    fn updated_at(&self) -> OffsetDateTime {
        self.updated_at
    }
}

impl LifecycleCaseProjection for pds_app::LifecycleCaseRecord {
    fn case_id(&self) -> Uuid {
        self.case_id
    }

    fn data_subject(&self) -> &str {
        &self.data_subject
    }

    fn case_type(&self) -> LifecycleCaseType {
        self.case_type
    }

    fn resource_kind(&self) -> LifecycleResourceKind {
        self.resource_kind
    }

    fn resource_id(&self) -> &str {
        &self.resource_id
    }

    fn status(&self) -> LifecycleCaseStatus {
        self.status
    }

    fn retention_policy_id(&self) -> Option<Uuid> {
        self.retention_policy_id
    }

    fn summary(&self) -> &str {
        &self.summary
    }

    fn resolution(&self) -> Option<&str> {
        self.resolution.as_deref()
    }

    fn tombstone_publication(&self) -> Option<TombstonePublicationRule> {
        self.tombstone_publication
    }

    fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    fn updated_at(&self) -> OffsetDateTime {
        self.updated_at
    }
}

pub(in crate::surfaces::pds::wire) fn lifecycle_case_dto<T: LifecycleCaseProjection>(
    case: &T,
) -> LifecycleCaseDto {
    LifecycleCaseDto {
        case_id: case.case_id().to_string(),
        data_subject: case.data_subject().to_owned(),
        case_type: case.case_type(),
        resource_kind: case.resource_kind(),
        resource_id: case.resource_id().to_owned(),
        status: case.status(),
        retention_policy_id: case
            .retention_policy_id()
            .map(|policy_id| policy_id.to_string()),
        summary: case.summary().to_owned(),
        resolution: case.resolution().map(str::to_owned),
        tombstone_publication: case.tombstone_publication(),
        created_at: rfc3339(case.created_at()),
        updated_at: rfc3339(case.updated_at()),
    }
}

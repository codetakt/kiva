//! Lifecycle-event PDS wire projection.

use kiva_pds::{
    LifecycleEvent as LifecycleEventDto, LifecycleEventList, LifecycleEventType,
    LifecycleResourceKind,
};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    app::pds as pds_app, core::response::rfc3339, surfaces::pds::model::PdsLifecycleEventState,
};

use super::super::common::dto_value;

pub(crate) fn lifecycle_event_list_response(
    events: &[PdsLifecycleEventState],
) -> Result<Value, serde_json::Error> {
    dto_value(&LifecycleEventList {
        events: events.iter().map(lifecycle_event_dto).collect(),
    })
}

pub(in crate::surfaces::pds::wire) trait LifecycleEventProjection {
    fn event_id(&self) -> Uuid;
    fn data_subject(&self) -> &str;
    fn event_type(&self) -> LifecycleEventType;
    fn resource_kind(&self) -> LifecycleResourceKind;
    fn resource_id(&self) -> &str;
    fn reason(&self) -> &str;
    fn created_at(&self) -> OffsetDateTime;
}

impl LifecycleEventProjection for PdsLifecycleEventState {
    fn event_id(&self) -> Uuid {
        self.event_id
    }

    fn data_subject(&self) -> &str {
        &self.data_subject
    }

    fn event_type(&self) -> LifecycleEventType {
        self.event_type
    }

    fn resource_kind(&self) -> LifecycleResourceKind {
        self.resource_kind
    }

    fn resource_id(&self) -> &str {
        &self.resource_id
    }

    fn reason(&self) -> &str {
        &self.reason
    }

    fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

impl LifecycleEventProjection for pds_app::LifecycleEventRecord {
    fn event_id(&self) -> Uuid {
        self.event_id
    }

    fn data_subject(&self) -> &str {
        &self.data_subject
    }

    fn event_type(&self) -> LifecycleEventType {
        self.event_type
    }

    fn resource_kind(&self) -> LifecycleResourceKind {
        self.resource_kind
    }

    fn resource_id(&self) -> &str {
        &self.resource_id
    }

    fn reason(&self) -> &str {
        &self.reason
    }

    fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

pub(in crate::surfaces::pds::wire) fn lifecycle_event_dto<T: LifecycleEventProjection>(
    event: &T,
) -> LifecycleEventDto {
    LifecycleEventDto {
        event_id: event.event_id().to_string(),
        data_subject: event.data_subject().to_owned(),
        event_type: event.event_type(),
        resource_kind: event.resource_kind(),
        resource_id: event.resource_id().to_owned(),
        reason: event.reason().to_owned(),
        created_at: rfc3339(event.created_at()),
    }
}

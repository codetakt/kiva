//! Access-check and access-audit PDS wire projection.

use kiva_pds::{
    AccessAuditEvent as AccessAuditEventDto, AccessAuditEventList,
    AccessCheckResponse as AccessCheckResponseDto,
};
use serde_json::Value;

use crate::{
    core::response::rfc3339,
    surfaces::pds::model::{
        PdsAccessAuditEventState, pds_action_label, pds_decision_label, pds_deny_reason,
        pds_deny_reason_label, pds_principal_role_label, pds_resource_class_label,
    },
};

use super::common::dto_value;

pub(crate) fn access_check_response(
    event: &PdsAccessAuditEventState,
) -> Result<Value, serde_json::Error> {
    dto_value(&AccessCheckResponseDto {
        event_id: event.event_id.to_string(),
        decision: pds_decision_label(event.decision).to_owned(),
        deny_reason: pds_deny_reason(event.decision)
            .map(pds_deny_reason_label)
            .map(str::to_owned),
    })
}

pub(crate) fn access_audit_event_list_response(
    events: &[PdsAccessAuditEventState],
) -> Result<Value, serde_json::Error> {
    dto_value(&AccessAuditEventList {
        events: events.iter().map(access_audit_event_dto).collect(),
    })
}

fn access_audit_event_dto(event: &PdsAccessAuditEventState) -> AccessAuditEventDto {
    AccessAuditEventDto {
        event_id: event.event_id.to_string(),
        requester_subject: event.requester_subject.clone(),
        role: pds_principal_role_label(event.role).to_owned(),
        resource: pds_resource_class_label(event.resource).to_owned(),
        action: pds_action_label(event.action).to_owned(),
        grant_id: event.grant_id.map(|grant_id| grant_id.to_string()),
        data_product_id: event
            .data_product_id
            .map(|data_product_id| data_product_id.to_string()),
        decision: pds_decision_label(event.decision).to_owned(),
        deny_reason: pds_deny_reason(event.decision)
            .map(pds_deny_reason_label)
            .map(str::to_owned),
        created_at: rfc3339(event.created_at),
    }
}

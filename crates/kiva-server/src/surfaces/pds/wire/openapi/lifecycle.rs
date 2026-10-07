//! OpenAPI-to-PDS conversion for lifecycle request DTOs.

use kiva_pds::{
    LifecycleCaseCommandInput as PdsLifecycleCaseCommandInput,
    LifecycleEventCommandInput as PdsLifecycleEventCommandInput,
    RetentionPolicyCommandInput as PdsRetentionPolicyCommandInput,
};
use kiva_pds_openapi_contract as pds_openapi;

use super::common::{
    data_class_from_openapi, legal_basis_from_openapi, lifecycle_case_status_from_openapi,
    lifecycle_case_type_from_openapi, lifecycle_event_type_from_openapi,
    lifecycle_resource_kind_from_openapi, tombstone_publication_rule_from_openapi,
};

pub(crate) fn lifecycle_event_command_input_from_openapi(
    request: pds_openapi::LifecycleEventRequest,
) -> PdsLifecycleEventCommandInput {
    PdsLifecycleEventCommandInput {
        event_id: request.event_id,
        event_type: lifecycle_event_type_from_openapi(request.event_type),
        resource_kind: lifecycle_resource_kind_from_openapi(request.resource_kind),
        resource_id: request.resource_id,
        reason: request.reason,
    }
}

pub(crate) fn retention_policy_command_input_from_openapi(
    request: pds_openapi::RetentionPolicyRequest,
) -> PdsRetentionPolicyCommandInput {
    PdsRetentionPolicyCommandInput {
        policy_id: request.policy_id,
        resource_kind: lifecycle_resource_kind_from_openapi(request.resource_kind),
        data_class: data_class_from_openapi(request.data_class),
        retention_period_days: request.retention_period_days,
        review_interval_days: request.review_interval_days,
        tombstone_after_days: request.tombstone_after_days,
        tombstone_publication: tombstone_publication_rule_from_openapi(
            request.tombstone_publication,
        ),
        legal_basis: legal_basis_from_openapi(request.legal_basis),
        description: request.description,
    }
}

pub(crate) fn lifecycle_case_command_input_from_openapi(
    request: pds_openapi::LifecycleCaseRequest,
) -> PdsLifecycleCaseCommandInput {
    PdsLifecycleCaseCommandInput {
        case_id: request.case_id,
        case_type: lifecycle_case_type_from_openapi(request.case_type),
        resource_kind: lifecycle_resource_kind_from_openapi(request.resource_kind),
        resource_id: request.resource_id,
        status: lifecycle_case_status_from_openapi(request.status),
        retention_policy_id: request.retention_policy_id,
        summary: request.summary,
        resolution: request.resolution,
        tombstone_publication: request
            .tombstone_publication
            .map(tombstone_publication_rule_from_openapi),
    }
}

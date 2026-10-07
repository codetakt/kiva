//! OpenAPI-to-PDS conversion for access request DTOs.

use kiva_pds::AccessCheckIntent as PdsAccessCheckIntent;
use kiva_pds_openapi_contract as pds_openapi;

use super::common::{
    policy_action_label_from_openapi, principal_role_label_from_openapi,
    resource_class_label_from_openapi,
};

pub(crate) fn access_check_intent_from_openapi(
    request: &pds_openapi::AccessCheckRequest,
) -> PdsAccessCheckIntent {
    let &pds_openapi::AccessCheckRequest {
        role,
        resource,
        action,
        grant_id,
        data_product_id,
    } = request;
    PdsAccessCheckIntent {
        role: principal_role_label_from_openapi(role).to_owned(),
        resource: resource_class_label_from_openapi(resource).to_owned(),
        action: policy_action_label_from_openapi(action).to_owned(),
        grant_id,
        data_product_id,
    }
}

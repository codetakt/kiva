//! PDS generated `OpenAPI` request DTO conversion.

mod access;
mod common;
mod consent;
mod data_products;
mod governance;
mod lifecycle;
mod provider_bindings;

pub(crate) use access::access_check_intent_from_openapi;
pub(crate) use consent::{
    consent_grant_command_input_from_openapi, consent_revocation_command_input_from_openapi,
};
pub(crate) use data_products::{
    data_product_read_intent_from_openapi, data_product_registration_command_input_from_openapi,
};
pub(crate) use governance::governance_profile_command_input_from_openapi;
pub(crate) use lifecycle::{
    lifecycle_case_command_input_from_openapi, lifecycle_event_command_input_from_openapi,
    retention_policy_command_input_from_openapi,
};
pub(crate) use provider_bindings::provider_binding_registration_command_input_from_openapi;

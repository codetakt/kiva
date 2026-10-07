//! Confidential-computation `OpenAPI` request DTO conversion.

mod additive;
mod common;
mod differential_privacy;
mod jobs;
mod participants;
mod shamir;
mod threshold;

pub(crate) use additive::{
    additive_aggregate_request_from_openapi, share_signing_key_request_from_openapi,
    share_signing_key_revocation_request_from_openapi,
    share_signing_key_rotation_request_from_openapi, share_submission_request_from_openapi,
};
pub(crate) use differential_privacy::{
    differential_privacy_budget_request_from_openapi,
    differential_privacy_budget_spend_request_from_openapi,
    differential_privacy_release_request_from_openapi,
    differential_privacy_sampled_release_request_from_openapi,
};
pub(crate) use jobs::{
    confidential_job_request_from_openapi, confidential_job_transition_request_from_openapi,
};
pub(crate) use participants::{
    confidential_participant_request_from_openapi,
    confidential_participant_transition_request_from_openapi,
};
pub(crate) use shamir::shamir_threshold_reconstruction_request_from_openapi;
pub(crate) use threshold::{
    threshold_release_approval_request_from_openapi, threshold_release_request_from_openapi,
};

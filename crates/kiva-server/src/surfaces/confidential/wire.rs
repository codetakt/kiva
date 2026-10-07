//! Confidential-computation HTTP response mapping.

use serde::Serialize;
use serde_json::Value;

mod additive;
mod differential_privacy;
mod jobs;
mod participants;
mod requests;
mod shamir;
mod threshold;

pub(crate) use additive::{
    aggregate_response, replay_record_list_response, share_signing_key_list_response,
    submitted_share_list_response,
};
pub(crate) use differential_privacy::{
    differential_privacy_budget_list_response, differential_privacy_budget_response,
    differential_privacy_composition_summary_list_response,
    differential_privacy_release_list_response, differential_privacy_release_response,
};
pub(crate) use jobs::{job_audit_event_list_response, job_list_response, job_response};
pub(crate) use participants::{
    participant_audit_event_list_response, participant_list_response, participant_response,
};
pub(crate) use requests::{
    additive_aggregate_request_from_openapi, confidential_job_request_from_openapi,
    confidential_job_transition_request_from_openapi,
    confidential_participant_request_from_openapi,
    confidential_participant_transition_request_from_openapi,
    differential_privacy_budget_request_from_openapi,
    differential_privacy_budget_spend_request_from_openapi,
    differential_privacy_release_request_from_openapi,
    differential_privacy_sampled_release_request_from_openapi,
    shamir_threshold_reconstruction_request_from_openapi, share_signing_key_request_from_openapi,
    share_signing_key_revocation_request_from_openapi,
    share_signing_key_rotation_request_from_openapi, share_submission_request_from_openapi,
    threshold_release_approval_request_from_openapi, threshold_release_request_from_openapi,
};
pub(crate) use shamir::{
    shamir_threshold_reconstruction_list_response, shamir_threshold_reconstruction_response,
};
pub(crate) use threshold::{threshold_release_list_response, threshold_release_response};

fn dto_value<T: Serialize>(value: &T) -> Result<Value, serde_json::Error> {
    serde_json::to_value(value)
}

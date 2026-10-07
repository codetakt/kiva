//! Confidential-computation HTTP response mapping.

mod common;
mod differential_privacy;
mod jobs;
mod participants;
mod shamir;
mod threshold;

pub(crate) use common::{
    confidential_application_admission_error_response, confidential_secure_error_response,
    confidential_store_error_response,
};
pub(crate) use differential_privacy::{
    confidential_differential_privacy_budget_error_response,
    confidential_differential_privacy_release_error_response,
};
pub(crate) use jobs::confidential_job_error_response;
pub(crate) use participants::confidential_participant_error_response;
pub(crate) use shamir::confidential_shamir_reconstruction_error_response;
pub(crate) use threshold::confidential_threshold_release_error_response;

//! Confidential job error response projection.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use kiva_confidential::mechanism::job::{
    ConfidentialJobAdmissionError, ConfidentialJobTransitionError,
};

use crate::{core::response::HttpError, surfaces::confidential::outcome::ConfidentialJobError};

use super::confidential_store_error_response;

pub(crate) fn confidential_job_error_response(error: ConfidentialJobError) -> Response {
    match error {
        ConfidentialJobError::Admission(ConfidentialJobAdmissionError::TooFewParticipants) => {
            HttpError::new(
                StatusCode::BAD_REQUEST,
                "confidential job admission requires at least two participants",
            )
            .into_response()
        }
        ConfidentialJobError::Admission(ConfidentialJobAdmissionError::MissingPolicyBinding) => {
            HttpError::new(
                StatusCode::BAD_REQUEST,
                "confidential job admission requires a policy binding",
            )
            .into_response()
        }
        ConfidentialJobError::Admission(ConfidentialJobAdmissionError::DeadlineNotFuture) => {
            HttpError::new(
                StatusCode::CONFLICT,
                "confidential job deadline is not in the future",
            )
            .into_response()
        }
        ConfidentialJobError::Admission(ConfidentialJobAdmissionError::AdmissionQuotaExceeded) => {
            HttpError::new(
                StatusCode::CONFLICT,
                "confidential job admission quota is exhausted",
            )
            .into_response()
        }
        ConfidentialJobError::Admission(ConfidentialJobAdmissionError::SchedulerBackpressure) => {
            HttpError::new(
                StatusCode::CONFLICT,
                "confidential job scheduler is applying backpressure",
            )
            .into_response()
        }
        ConfidentialJobError::Admission(ConfidentialJobAdmissionError::RetryBudgetExceeded) => {
            HttpError::new(
                StatusCode::CONFLICT,
                "confidential job retry budget is exhausted",
            )
            .into_response()
        }
        ConfidentialJobError::Transition(ConfidentialJobTransitionError::InvalidTransition) => {
            HttpError::new(
                StatusCode::CONFLICT,
                "confidential job lifecycle transition is invalid",
            )
            .into_response()
        }
        ConfidentialJobError::Store(error) => confidential_store_error_response(&error),
        ConfidentialJobError::InvalidRequest(message) => {
            HttpError::new(StatusCode::BAD_REQUEST, message).into_response()
        }
        ConfidentialJobError::NotFound => {
            HttpError::new(StatusCode::NOT_FOUND, "confidential job was not found").into_response()
        }
        ConfidentialJobError::Forbidden => HttpError::new(
            StatusCode::FORBIDDEN,
            "confidential job belongs to a different subject",
        )
        .into_response(),
        ConfidentialJobError::Conflict => HttpError::new(
            StatusCode::CONFLICT,
            "confidential job already exists with different data",
        )
        .into_response(),
    }
}

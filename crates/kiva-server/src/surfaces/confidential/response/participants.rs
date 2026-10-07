//! Confidential participant error response projection.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use kiva_confidential::mechanism::workflow::ConfidentialParticipantTransitionError;

use crate::{
    core::response::HttpError, surfaces::confidential::outcome::ConfidentialParticipantError,
};

use super::{confidential_application_admission_error_response, confidential_store_error_response};

pub(crate) fn confidential_participant_error_response(
    error: ConfidentialParticipantError,
) -> Response {
    match error {
        ConfidentialParticipantError::ApplicationAdmission(message) => {
            confidential_application_admission_error_response(&message)
        }
        ConfidentialParticipantError::Transition(
            ConfidentialParticipantTransitionError::InvalidTransition,
        ) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential participant lifecycle transition is invalid",
        )
        .into_response(),
        ConfidentialParticipantError::Store(error) => confidential_store_error_response(&error),
        ConfidentialParticipantError::InvalidRequest(message) => {
            HttpError::new(StatusCode::BAD_REQUEST, message).into_response()
        }
        ConfidentialParticipantError::NotFound => HttpError::new(
            StatusCode::NOT_FOUND,
            "confidential participant was not found",
        )
        .into_response(),
        ConfidentialParticipantError::Forbidden => HttpError::new(
            StatusCode::FORBIDDEN,
            "confidential participant belongs to a different subject",
        )
        .into_response(),
        ConfidentialParticipantError::Conflict => HttpError::new(
            StatusCode::CONFLICT,
            "confidential participant already exists with different data",
        )
        .into_response(),
        ConfidentialParticipantError::Inactive => HttpError::new(
            StatusCode::CONFLICT,
            "confidential participant is not active for this operation",
        )
        .into_response(),
    }
}

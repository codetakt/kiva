//! Threshold release error response projection.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    core::response::HttpError, surfaces::confidential::outcome::ConfidentialThresholdReleaseError,
};

use super::{
    confidential_application_admission_error_response, confidential_secure_error_response,
    confidential_store_error_response,
};

pub(crate) fn confidential_threshold_release_error_response(
    error: ConfidentialThresholdReleaseError,
) -> Response {
    match error {
        ConfidentialThresholdReleaseError::ApplicationAdmission(message) => {
            confidential_application_admission_error_response(&message)
        }
        ConfidentialThresholdReleaseError::Secure(error) => {
            confidential_secure_error_response(error).into_response()
        }
        ConfidentialThresholdReleaseError::Store(error) => {
            confidential_store_error_response(&error)
        }
        ConfidentialThresholdReleaseError::InvalidAuthoritySet => HttpError::new(
            StatusCode::BAD_REQUEST,
            "confidential threshold release requires two distinct non-empty authority subjects",
        )
        .into_response(),
        ConfidentialThresholdReleaseError::NotFound => HttpError::new(
            StatusCode::NOT_FOUND,
            "confidential threshold release request was not found",
        )
        .into_response(),
        ConfidentialThresholdReleaseError::Forbidden => HttpError::new(
            StatusCode::FORBIDDEN,
            "confidential threshold release authority does not match the authenticated subject",
        )
        .into_response(),
        ConfidentialThresholdReleaseError::MissingComputedAggregate => HttpError::new(
            StatusCode::CONFLICT,
            "confidential threshold release requires a matching computed aggregate result",
        )
        .into_response(),
        ConfidentialThresholdReleaseError::MissingApproval => HttpError::new(
            StatusCode::CONFLICT,
            "confidential threshold release requires both authority approvals",
        )
        .into_response(),
    }
}

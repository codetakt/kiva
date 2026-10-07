//! Shamir reconstruction error response projection.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    core::response::HttpError,
    surfaces::confidential::outcome::ConfidentialShamirReconstructionError,
};

use super::{
    confidential_application_admission_error_response, confidential_secure_error_response,
    confidential_store_error_response,
};

pub(crate) fn confidential_shamir_reconstruction_error_response(
    error: ConfidentialShamirReconstructionError,
) -> Response {
    match error {
        ConfidentialShamirReconstructionError::ApplicationAdmission(message) => {
            confidential_application_admission_error_response(&message)
        }
        ConfidentialShamirReconstructionError::Workflow(error) => {
            HttpError::new(StatusCode::BAD_REQUEST, error.to_string()).into_response()
        }
        ConfidentialShamirReconstructionError::Secure(error) => {
            confidential_secure_error_response(error).into_response()
        }
        ConfidentialShamirReconstructionError::Store(error) => {
            confidential_store_error_response(&error)
        }
        ConfidentialShamirReconstructionError::NotFound => HttpError::new(
            StatusCode::NOT_FOUND,
            "confidential Shamir reconstruction was not found",
        )
        .into_response(),
        ConfidentialShamirReconstructionError::Conflict => HttpError::new(
            StatusCode::CONFLICT,
            "confidential Shamir reconstruction already exists with different data",
        )
        .into_response(),
    }
}

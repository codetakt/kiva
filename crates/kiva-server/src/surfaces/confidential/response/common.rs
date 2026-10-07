//! Shared confidential-computation error response projection.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use kiva_confidential::mechanism::additive::SecureAggregateError;

use crate::{
    core::response::HttpError, surfaces::confidential::outcome::ConfidentialAggregateStoreError,
};

pub(crate) fn confidential_application_admission_error_response(message: &str) -> Response {
    tracing::error!(
        error = message,
        "confidential application workflow admission failed"
    );
    HttpError::new(
        StatusCode::FORBIDDEN,
        "confidential application workflow was not admitted",
    )
    .into_response()
}

pub(crate) fn confidential_store_error_response(
    error: &ConfidentialAggregateStoreError,
) -> Response {
    match error {
        ConfidentialAggregateStoreError::Store { source } => {
            tracing::error!(
                error = ?source,
                "confidential aggregate store operation failed"
            );
        }
        ConfidentialAggregateStoreError::InvalidStoredArtifactLength => {
            tracing::error!("confidential replay record contained invalid artifact length");
        }
        ConfidentialAggregateStoreError::InvalidStoredReplayRecord(source) => {
            tracing::error!(
                error = ?source,
                "confidential replay record contained invalid data"
            );
        }
        ConfidentialAggregateStoreError::InvalidStoredSigningKey(source) => {
            tracing::error!(
                error = ?source,
                "confidential signing key registration contained invalid data"
            );
        }
        ConfidentialAggregateStoreError::InvalidStoredShamirReconstruction => {
            tracing::error!("confidential Shamir reconstruction contained invalid data");
        }
        ConfidentialAggregateStoreError::InvalidStoredDifferentialPrivacyBudget(source) => {
            tracing::error!(
                error = ?source,
                "confidential differential privacy budget contained invalid data"
            );
        }
        ConfidentialAggregateStoreError::InvalidStoredDifferentialPrivacyRelease => {
            tracing::error!("confidential differential privacy release contained invalid data");
        }
        ConfidentialAggregateStoreError::InvalidStoredConfidentialJob => {
            tracing::error!("confidential job lifecycle metadata contained invalid data");
        }
        ConfidentialAggregateStoreError::InvalidStoredConfidentialParticipant => {
            tracing::error!("confidential participant lifecycle metadata contained invalid data");
        }
        ConfidentialAggregateStoreError::ParticipantIndexTooLarge => {
            tracing::error!("confidential replay record participant index exceeded i64");
        }
    }
    HttpError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        "confidential aggregate store operation failed",
    )
    .into_response()
}

pub(crate) fn confidential_secure_error_response(error: SecureAggregateError) -> HttpError {
    let status = match error {
        SecureAggregateError::EnvelopeReplayDetected => StatusCode::CONFLICT,
        _error => StatusCode::BAD_REQUEST,
    };
    HttpError::new(status, error.to_string())
}

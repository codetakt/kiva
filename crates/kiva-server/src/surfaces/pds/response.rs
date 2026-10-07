//! PDS HTTP response mapping.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::app::pds as pds_app;
use crate::core::response::HttpError;
use crate::surfaces::pds::outcome::PdsRepositoryError;

pub(crate) fn pds_store_error_response(error: &PdsRepositoryError) -> Response {
    match error {
        PdsRepositoryError::Runtime(message) => {
            tracing::error!(error = %message, "PDS operation runtime failed");
        }
        PdsRepositoryError::Store { source } => {
            tracing::error!(error = ?source, "PDS repository operation failed");
        }
        PdsRepositoryError::InvalidRequest(message) => {
            return HttpError::new(StatusCode::BAD_REQUEST, message.clone()).into_response();
        }
        PdsRepositoryError::InvalidStoredData => {
            tracing::error!("PDS repository returned invalid stored data");
        }
    }
    HttpError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        "PDS repository operation failed",
    )
    .into_response()
}

pub(crate) fn pds_read_model_error_response(error: &pds_app::PdsReadModelError) -> Response {
    match error {
        pds_app::PdsReadModelError::Runtime(error) => {
            tracing::error!(error = error.message(), "PDS read-model runtime failed");
            HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "PDS application runtime failed",
            )
            .into_response()
        }
        pds_app::PdsReadModelError::Statements(pds_app::StatementReadError::Store) => {
            tracing::error!("statement store operation failed while building PDS read model");
            HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "statement store operation failed",
            )
            .into_response()
        }
        pds_app::PdsReadModelError::Statements(pds_app::StatementReadError::InvalidCursor) => {
            HttpError::new(StatusCode::BAD_REQUEST, "PDS portability cursor is invalid")
                .into_response()
        }
        pds_app::PdsReadModelError::Pds(error) => pds_application_store_error_response(error),
    }
}

pub(crate) fn pds_application_store_error_response(
    error: &pds_app::PdsApplicationStoreError,
) -> Response {
    match error {
        pds_app::PdsApplicationStoreError::InvalidRequest(message) => {
            HttpError::new(StatusCode::BAD_REQUEST, message.clone()).into_response()
        }
        pds_app::PdsApplicationStoreError::Store => {
            tracing::error!("PDS application store operation failed");
            HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "PDS repository operation failed",
            )
            .into_response()
        }
        pds_app::PdsApplicationStoreError::InvalidStoredData => {
            tracing::error!("PDS application read invalid stored data");
            HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "PDS repository operation failed",
            )
            .into_response()
        }
    }
}

pub(crate) fn pds_write_error_response(error: &pds_app::PdsApplicationWriteError) -> Response {
    match error {
        pds_app::PdsApplicationWriteError::Runtime(error) => {
            tracing::error!(error = error.message(), "PDS application runtime failed");
            HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "PDS application runtime failed",
            )
            .into_response()
        }
        pds_app::PdsApplicationWriteError::Pds(error) => {
            pds_application_store_error_response(error)
        }
    }
}

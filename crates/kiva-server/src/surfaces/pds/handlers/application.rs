//! Application-workflow response helpers for PDS HTTP handlers.

use super::prelude::{HttpError, IntoResponse, Response, StatusCode, empty_response};
use crate::app::pds as pds_app;

pub(super) fn pds_application_store_outcome_response(
    outcome: pds_app::ApplicationStoreOutcome,
    conflict_message: &'static str,
) -> Response {
    match outcome {
        pds_app::ApplicationStoreOutcome::Stored | pds_app::ApplicationStoreOutcome::Unchanged => {
            empty_response(StatusCode::NO_CONTENT)
        }
        pds_app::ApplicationStoreOutcome::Conflict => {
            HttpError::new(StatusCode::CONFLICT, conflict_message).into_response()
        }
    }
}

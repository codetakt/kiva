//! Core system route handlers.

use axum::{Router, extract::State, http::StatusCode, response::Response, routing::get};
use serde_json::json;

use crate::core::{
    paths::CONFORMANCE_RATE_LIMIT_PATH, response::json_response, state::SystemHttpContext,
};

pub(crate) fn router(state: SystemHttpContext) -> Router {
    Router::new()
        .route(CONFORMANCE_RATE_LIMIT_PATH, get(conformance_rate_limit))
        .with_state(state)
}

pub(crate) async fn conformance_rate_limit(State(state): State<SystemHttpContext>) -> Response {
    if let Some(response) = state.rate_limit_response(CONFORMANCE_RATE_LIMIT_PATH).await {
        return response;
    }
    json_response(
        StatusCode::OK,
        json!({
            "status": "ok",
            "fixture": "kiva-conformance-rate-limit",
        }),
    )
}

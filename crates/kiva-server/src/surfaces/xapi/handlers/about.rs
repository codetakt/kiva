//! xAPI About resource handler.
#![allow(clippy::wildcard_imports)]

use super::common::*;

pub(crate) async fn about(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = state.rate_limit_response("/about").await {
        return response;
    }
    match state.authorize_about_request(&headers, &query).await {
        Ok(_credentials) => json_projection_response(StatusCode::OK, wire::about_response()),
        Err(error) => error.into_response(),
    }
}

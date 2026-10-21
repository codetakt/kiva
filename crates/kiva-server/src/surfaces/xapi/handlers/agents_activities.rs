//! xAPI Agent and Activity support resource handlers.
#![allow(clippy::wildcard_imports)]

use super::common::*;

pub(crate) async fn get_agent_person(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Err(error) = state.authorize_xapi_read_request(&headers, &query).await {
        return error.into_response();
    }
    if let Err(error) = validate_query_parameters(&query, &["agent"]) {
        return error.into_response();
    }
    let agent = match required_agent(&query) {
        Ok(agent) => agent,
        Err(error) => return error.into_response(),
    };
    person_from_agent(&agent).map_or_else(
        || {
            HttpError::new(
                StatusCode::BAD_REQUEST,
                "agent must include an inverse functional identifier",
            )
            .into_response()
        },
        |person| json_projection_response(StatusCode::OK, wire::person_response(person)),
    )
}

pub(crate) async fn get_activity(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Err(error) = state.authorize_xapi_read_request(&headers, &query).await {
        return error.into_response();
    }
    if let Err(error) = validate_query_parameters(&query, &["activityId"]) {
        return error.into_response();
    }
    let activity_id = match required_iri(&query, "activityId") {
        Ok(activity_id) => activity_id,
        Err(error) => return error.into_response(),
    };
    let statements = match xapi_service::list_statements(state.xapi_services(), None).await {
        Ok(statements) => statements,
        Err(error) => return store_error_response(&error),
    };
    let definition = wire::merged_activity_definition(&statements, activity_id);
    json_projection_response(
        StatusCode::OK,
        wire::activity_response(activity_id, definition),
    )
}

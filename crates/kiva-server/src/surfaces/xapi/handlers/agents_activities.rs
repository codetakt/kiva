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
    let sources = wire::activity_definition_sources(&statements, activity_id);
    let ids = sources
        .iter()
        .filter_map(|(index, _, _)| {
            statements[*index]["id"]
                .as_str()
                .and_then(|id| uuid::Uuid::parse_str(id).ok())
        })
        .collect::<Vec<_>>();
    let raw_values = match xapi_service::statement_opaque_values(state.xapi_services(), &ids).await
    {
        Ok(values) => values,
        Err(error) => return store_error_response(&error),
    };
    let mut opaque = crate::surfaces::xapi::lossless::OpaqueValues::new();
    let mut seen = std::collections::BTreeSet::new();
    for (index, path, source) in sources {
        let raw = statements[index]["id"]
            .as_str()
            .and_then(|id| uuid::Uuid::parse_str(id).ok())
            .and_then(|id| raw_values.get(&id));
        if let Some(extensions) = source
            .get("extensions")
            .and_then(serde_json::Value::as_object)
        {
            for key in extensions.keys() {
                if seen.insert(key.clone()) {
                    let source_path = crate::surfaces::xapi::lossless::pointer(
                        &format!("{path}/extensions"),
                        key,
                    );
                    if let Some(value) = raw.and_then(|raw| raw.get(&source_path)) {
                        opaque.insert(
                            crate::surfaces::xapi::lossless::pointer("/definition/extensions", key),
                            value.clone(),
                        );
                    }
                }
            }
        }
    }
    crate::core::response::json_bytes_response(
        StatusCode::OK,
        wire::activity_response(activity_id, definition)
            .and_then(|value| crate::surfaces::xapi::lossless::render(&value, &opaque)),
    )
}

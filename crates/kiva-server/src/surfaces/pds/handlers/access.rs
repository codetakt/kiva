//! PDS HTTP handler bodies for this resource family.

use super::{
    prelude::{
        ApplicationOperationId, BTreeMap, Bytes, HeaderMap, HttpError, IntoResponse,
        PDS_ACCESS_AUDIT_EVENTS_PATH, PDS_ACCESS_CHECKS_PATH, PdsAccessAuditEventFilter,
        PdsHttpContext, PdsOpenApiAccessCheckRequest, PdsRepositoryError, Query, Response, State,
        StatusCode, json_projection_response, pds_service, pds_store_error_response, wire,
    },
    request::parse_pds_openapi_body,
};

pub(crate) async fn post_pds_access_check(
    State(state): State<PdsHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_ACCESS_CHECKS_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_read_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let request = match parse_pds_openapi_body::<PdsOpenApiAccessCheckRequest>(
        &body,
        "PDS access check request body",
    ) {
        Ok(request) => request,
        Err(error) => return error.into_response(),
    };
    let request = wire::access_check_intent_from_openapi(&request);

    let principal = principal.into_principal();
    match state
        .run_pds_operation(
            ApplicationOperationId::PdsEvaluateAccessCheck,
            move |services| async move {
                pds_service::evaluate_access_check(services, &principal, request).await
            },
        )
        .await
    {
        Ok(event) => json_projection_response(StatusCode::OK, wire::access_check_response(&event)),
        Err(PdsRepositoryError::InvalidRequest(message)) => {
            HttpError::new(StatusCode::BAD_REQUEST, message).into_response()
        }
        Err(error) => pds_store_error_response(&error),
    }
}

pub(crate) async fn get_pds_access_audit_events(
    State(state): State<PdsHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = state
        .rate_limit_response(PDS_ACCESS_AUDIT_EVENTS_PATH)
        .await
    {
        return response;
    }
    let principal = match state.authorize_pds_audit_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let filter = match PdsAccessAuditEventFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    let principal = principal.into_principal();
    match state
        .run_pds_operation(
            ApplicationOperationId::PdsListAccessAuditEvents,
            move |services| async move {
                pds_service::list_access_audit_events(services, &principal, filter).await
            },
        )
        .await
    {
        Ok(events) => json_projection_response(
            StatusCode::OK,
            wire::access_audit_event_list_response(&events),
        ),
        Err(error) => pds_store_error_response(&error),
    }
}

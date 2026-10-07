//! PDS HTTP handler bodies for this resource family.

use super::{
    application::pds_application_store_outcome_response,
    prelude::{
        ApplicationOperationId, BTreeMap, Bytes, HeaderMap, IntoResponse, PDS_LIFECYCLE_CASES_PATH,
        PDS_LIFECYCLE_EVENTS_PATH, PDS_RETENTION_POLICIES_PATH, PdsApplicationHttpContext,
        PdsHttpContext, PdsLifecycleCaseFilter, PdsLifecycleEventFilter,
        PdsOpenApiLifecycleCaseRequest, PdsOpenApiLifecycleEventRequest,
        PdsOpenApiRetentionPolicyRequest, PdsRetentionPolicyFilter, Query, Response, State,
        StatusCode, json_projection_response, pds_service, pds_store_error_response,
        pds_write_error_response, wire,
    },
    request::parse_pds_openapi_body,
};

pub(crate) async fn post_pds_lifecycle_event(
    State(state): State<PdsApplicationHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_LIFECYCLE_EVENTS_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_write_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let request = match parse_pds_openapi_body::<PdsOpenApiLifecycleEventRequest>(
        &body,
        "PDS lifecycle event request body",
    ) {
        Ok(request) => request,
        Err(error) => return error.into_response(),
    };
    let request = wire::lifecycle_event_command_input_from_openapi(request);

    match state
        .pds_application()
        .register_lifecycle_event(principal.as_principal(), request)
        .await
    {
        Ok(outcome) => pds_application_store_outcome_response(
            outcome,
            "PDS lifecycle event already exists with different data",
        ),
        Err(error) => pds_write_error_response(&error),
    }
}

pub(crate) async fn get_pds_lifecycle_events(
    State(state): State<PdsHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_LIFECYCLE_EVENTS_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_read_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let filter = match PdsLifecycleEventFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    let principal = principal.into_principal();
    match state
        .run_pds_operation(
            ApplicationOperationId::PdsListLifecycleEvents,
            move |services| async move {
                pds_service::list_lifecycle_events(services, &principal, filter).await
            },
        )
        .await
    {
        Ok(events) => {
            json_projection_response(StatusCode::OK, wire::lifecycle_event_list_response(&events))
        }
        Err(error) => pds_store_error_response(&error),
    }
}

pub(crate) async fn post_pds_retention_policy(
    State(state): State<PdsApplicationHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_RETENTION_POLICIES_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_write_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let request = match parse_pds_openapi_body::<PdsOpenApiRetentionPolicyRequest>(
        &body,
        "PDS retention policy request body",
    ) {
        Ok(request) => request,
        Err(error) => return error.into_response(),
    };
    let request = wire::retention_policy_command_input_from_openapi(request);

    match state
        .pds_application()
        .register_retention_policy(principal.as_principal(), request)
        .await
    {
        Ok(outcome) => pds_application_store_outcome_response(
            outcome,
            "PDS retention policy already exists with different data",
        ),
        Err(error) => pds_write_error_response(&error),
    }
}

pub(crate) async fn get_pds_retention_policies(
    State(state): State<PdsHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_RETENTION_POLICIES_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_read_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let filter = match PdsRetentionPolicyFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    let principal = principal.into_principal();
    match state
        .run_pds_operation(
            ApplicationOperationId::PdsListRetentionPolicies,
            move |services| async move {
                pds_service::list_retention_policies(services, &principal, filter).await
            },
        )
        .await
    {
        Ok(policies) => json_projection_response(
            StatusCode::OK,
            wire::retention_policy_list_response(&policies),
        ),
        Err(error) => pds_store_error_response(&error),
    }
}

pub(crate) async fn post_pds_lifecycle_case(
    State(state): State<PdsApplicationHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_LIFECYCLE_CASES_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_write_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let request = match parse_pds_openapi_body::<PdsOpenApiLifecycleCaseRequest>(
        &body,
        "PDS lifecycle case request body",
    ) {
        Ok(request) => request,
        Err(error) => return error.into_response(),
    };
    let request = wire::lifecycle_case_command_input_from_openapi(request);

    match state
        .pds_application()
        .register_lifecycle_case(principal.as_principal(), request)
        .await
    {
        Ok(outcome) => pds_application_store_outcome_response(
            outcome,
            "PDS lifecycle case conflicts with existing case metadata",
        ),
        Err(error) => pds_write_error_response(&error),
    }
}

pub(crate) async fn get_pds_lifecycle_cases(
    State(state): State<PdsHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_LIFECYCLE_CASES_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_read_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let filter = match PdsLifecycleCaseFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    let principal = principal.into_principal();
    match state
        .run_pds_operation(
            ApplicationOperationId::PdsListLifecycleCases,
            move |services| async move {
                pds_service::list_lifecycle_cases(services, &principal, filter).await
            },
        )
        .await
    {
        Ok(cases) => {
            json_projection_response(StatusCode::OK, wire::lifecycle_case_list_response(&cases))
        }
        Err(error) => pds_store_error_response(&error),
    }
}

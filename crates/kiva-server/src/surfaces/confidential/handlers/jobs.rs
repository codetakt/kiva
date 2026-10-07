#![allow(clippy::wildcard_imports)]

use super::*;

pub(crate) async fn post_confidential_job(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_write_endpoint(
        &state,
        CONFIDENTIAL_JOBS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request = match parse_confidential_openapi_request::<
        confidential_contract::ConfidentialJobRequest,
        _,
    >(
        &body,
        "confidential job request body",
        wire::confidential_job_request_from_openapi,
    ) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .register_confidential_job(principal.as_principal(), request)
        .await
    {
        Ok(
            ConfidentialJobRegistrationOutcome::Stored(job)
            | ConfidentialJobRegistrationOutcome::Unchanged(job),
        ) => json_projection_response(StatusCode::OK, wire::job_response(&job)),
        Ok(ConfidentialJobRegistrationOutcome::Conflict) => {
            confidential_job_error_response(ConfidentialJobError::Conflict)
        }
        Err(error) => confidential_job_error_response(error),
    }
}

pub(crate) async fn get_confidential_jobs(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    let principal = match authorize_confidential_read_endpoint(
        &state,
        CONFIDENTIAL_JOBS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let filter = match ConfidentialJobFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    match state
        .confidential_application()
        .list_confidential_jobs(principal.as_principal(), filter)
        .await
    {
        Ok(jobs) => json_projection_response(StatusCode::OK, wire::job_list_response(&jobs)),
        Err(error) => confidential_job_error_response(error),
    }
}

pub(crate) async fn post_confidential_job_transition(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    if let Err(response) =
        enforce_confidential_endpoint_rate_limit(&state, CONFIDENTIAL_JOB_TRANSITIONS_PATH).await
    {
        return response;
    }
    let request = match parse_confidential_openapi_request::<
        confidential_contract::ConfidentialJobTransitionRequest,
        _,
    >(
        &body,
        "confidential job transition body",
        wire::confidential_job_transition_request_from_openapi,
    ) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let principal = if request.target_status == ConfidentialJobStatus::Released {
        match authorize_confidential_release_scope(&state, &headers, &query).await {
            Ok(principal) => principal.into_principal(),
            Err(response) => return response,
        }
    } else {
        match authorize_confidential_write_scope(&state, &headers, &query).await {
            Ok(principal) => principal.into_principal(),
            Err(response) => return response,
        }
    };

    match state
        .confidential_application()
        .transition_confidential_job(&principal, request)
        .await
    {
        Ok(
            ConfidentialJobTransitionOutcome::Updated(job)
            | ConfidentialJobTransitionOutcome::Unchanged(job),
        ) => json_projection_response(StatusCode::OK, wire::job_response(&job)),
        Err(error) => confidential_job_error_response(error),
    }
}

pub(crate) async fn get_confidential_job_audit_events(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    let principal = match authorize_confidential_read_endpoint(
        &state,
        CONFIDENTIAL_JOB_AUDIT_EVENTS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let filter = match ConfidentialJobAuditEventFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    match state
        .confidential_application()
        .list_confidential_job_audit_events(principal.as_principal(), filter)
        .await
    {
        Ok(events) => {
            json_projection_response(StatusCode::OK, wire::job_audit_event_list_response(&events))
        }
        Err(error) => confidential_job_error_response(error),
    }
}

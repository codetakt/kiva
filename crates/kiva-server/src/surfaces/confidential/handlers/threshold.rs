#![allow(clippy::wildcard_imports)]

use super::*;

pub(crate) async fn post_confidential_threshold_release_request(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_write_endpoint(
        &state,
        CONFIDENTIAL_THRESHOLD_RELEASE_REQUESTS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request = match parse_confidential_openapi_request::<
        confidential_contract::ThresholdReleaseRequest,
        _,
    >(
        &body,
        "confidential threshold release request body",
        wire::threshold_release_request_from_openapi,
    ) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .register_threshold_release_request(principal.as_principal(), request)
        .await
    {
        Ok(StoreOutcome::Stored | StoreOutcome::Unchanged) => {
            empty_response(StatusCode::NO_CONTENT)
        }
        Ok(StoreOutcome::Conflict) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential threshold release request already exists with different data",
        )
        .into_response(),
        Err(error) => confidential_threshold_release_error_response(error),
    }
}

pub(crate) async fn post_confidential_threshold_release_approval(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_release_endpoint(
        &state,
        CONFIDENTIAL_THRESHOLD_RELEASE_APPROVALS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request = match parse_confidential_openapi_request::<
        confidential_contract::ThresholdReleaseApproval,
        _,
    >(
        &body,
        "confidential threshold release approval body",
        wire::threshold_release_approval_request_from_openapi,
    ) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .approve_threshold_release(principal.as_principal(), request)
        .await
    {
        Ok(StoreOutcome::Stored | StoreOutcome::Unchanged) => {
            empty_response(StatusCode::NO_CONTENT)
        }
        Ok(StoreOutcome::Conflict) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential threshold release approval conflicts with current state",
        )
        .into_response(),
        Err(error) => confidential_threshold_release_error_response(error),
    }
}

pub(crate) async fn post_confidential_threshold_release(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_release_endpoint(
        &state,
        CONFIDENTIAL_THRESHOLD_RELEASES_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request = match parse_confidential_openapi_request::<
        confidential_contract::ThresholdReleaseApproval,
        _,
    >(
        &body,
        "confidential threshold release body",
        wire::threshold_release_approval_request_from_openapi,
    ) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .release_threshold_aggregate(principal.as_principal(), request)
        .await
    {
        Ok(release) => {
            json_projection_response(StatusCode::OK, wire::threshold_release_response(&release))
        }
        Err(error) => confidential_threshold_release_error_response(error),
    }
}

pub(crate) async fn get_confidential_threshold_release_requests(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    let principal = match authorize_confidential_read_endpoint(
        &state,
        CONFIDENTIAL_THRESHOLD_RELEASE_REQUESTS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let filter = match ConfidentialThresholdReleaseFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    match state
        .confidential_application()
        .list_threshold_release_requests(principal.as_principal(), filter)
        .await
    {
        Ok(releases) => json_projection_response(
            StatusCode::OK,
            wire::threshold_release_list_response(&releases),
        ),
        Err(error) => confidential_threshold_release_error_response(error),
    }
}

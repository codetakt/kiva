#![allow(clippy::wildcard_imports)]

use super::*;

pub(crate) async fn post_confidential_shamir_threshold_reconstruction(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_compute_endpoint(
        &state,
        CONFIDENTIAL_SHAMIR_THRESHOLD_RECONSTRUCTIONS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request = match parse_confidential_openapi_request::<
        confidential_contract::ShamirThresholdReconstruction,
        _,
    >(
        &body,
        "confidential Shamir reconstruction body",
        wire::shamir_threshold_reconstruction_request_from_openapi,
    ) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .reconstruct_shamir_threshold_secret(principal.as_principal(), request)
        .await
    {
        Ok(reconstruction) => json_projection_response(
            StatusCode::OK,
            wire::shamir_threshold_reconstruction_response(&reconstruction),
        ),
        Err(error) => confidential_shamir_reconstruction_error_response(error),
    }
}

pub(crate) async fn get_confidential_shamir_threshold_reconstructions(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    let principal = match authorize_confidential_read_endpoint(
        &state,
        CONFIDENTIAL_SHAMIR_THRESHOLD_RECONSTRUCTIONS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let filter = match ConfidentialShamirThresholdReconstructionFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    match state
        .confidential_application()
        .list_shamir_threshold_reconstructions(principal.as_principal(), filter)
        .await
    {
        Ok(reconstructions) => json_projection_response(
            StatusCode::OK,
            wire::shamir_threshold_reconstruction_list_response(&reconstructions),
        ),
        Err(error) => confidential_shamir_reconstruction_error_response(error),
    }
}

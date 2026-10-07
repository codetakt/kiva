//! PDS HTTP handler bodies for this resource family.

use super::{
    application::pds_application_store_outcome_response,
    prelude::{
        ApplicationOperationId, BTreeMap, Bytes, HeaderMap, IntoResponse,
        PDS_GOVERNANCE_PROFILES_PATH, PdsApplicationHttpContext, PdsGovernanceProfileFilter,
        PdsHttpContext, PdsOpenApiGovernanceProfileRequest, Query, Response, State, StatusCode,
        json_projection_response, pds_service, pds_store_error_response, pds_write_error_response,
        wire,
    },
    request::parse_pds_openapi_body,
};

pub(crate) async fn post_pds_governance_profile(
    State(state): State<PdsApplicationHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    if let Some(response) = state
        .rate_limit_response(PDS_GOVERNANCE_PROFILES_PATH)
        .await
    {
        return response;
    }
    let principal = match state.authorize_pds_write_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let request = match parse_pds_openapi_body::<PdsOpenApiGovernanceProfileRequest>(
        &body,
        "PDS governance profile request body",
    ) {
        Ok(request) => request,
        Err(error) => return error.into_response(),
    };
    let request = wire::governance_profile_command_input_from_openapi(request);

    match state
        .pds_application()
        .register_governance_profile(principal.as_principal(), request)
        .await
    {
        Ok(outcome) => pds_application_store_outcome_response(
            outcome,
            "PDS governance profile already exists with different data",
        ),
        Err(error) => pds_write_error_response(&error),
    }
}

pub(crate) async fn get_pds_governance_profiles(
    State(state): State<PdsHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = state
        .rate_limit_response(PDS_GOVERNANCE_PROFILES_PATH)
        .await
    {
        return response;
    }
    let principal = match state.authorize_pds_read_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let filter = match PdsGovernanceProfileFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    let principal = principal.into_principal();
    match state
        .run_pds_operation(
            ApplicationOperationId::PdsListGovernanceProfiles,
            move |services| async move {
                pds_service::list_governance_profiles(services, &principal, filter).await
            },
        )
        .await
    {
        Ok(profiles) => json_projection_response(
            StatusCode::OK,
            wire::governance_profile_list_response(&profiles),
        ),
        Err(error) => pds_store_error_response(&error),
    }
}

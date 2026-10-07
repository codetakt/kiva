//! PDS HTTP handler bodies for provider-binding registry metadata.

use super::{
    prelude::{
        ApplicationOperationId, BTreeMap, Bytes, HeaderMap, HttpError, IntoResponse,
        PDS_PROVIDER_BINDINGS_PATH, PdsHttpContext, PdsOpenApiProviderBindingRequest,
        PdsProviderBindingFilter, Query, Response, State, StatusCode, empty_response,
        json_projection_response, pds_service, pds_store_error_response, wire,
    },
    request::parse_pds_openapi_body,
};

use crate::core::outcome::StoreOutcome;

pub(crate) async fn get_pds_provider_bindings(
    State(state): State<PdsHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_PROVIDER_BINDINGS_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_read_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let filter = match PdsProviderBindingFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    let principal = principal.into_principal();
    match state
        .run_pds_operation(
            ApplicationOperationId::PdsListProviderBindings,
            move |services| async move {
                pds_service::list_provider_bindings(services, &principal, filter).await
            },
        )
        .await
    {
        Ok(bindings) => json_projection_response(
            StatusCode::OK,
            wire::provider_binding_list_response(&bindings),
        ),
        Err(error) => pds_store_error_response(&error),
    }
}

pub(crate) async fn post_pds_provider_binding(
    State(state): State<PdsHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_PROVIDER_BINDINGS_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_write_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let request = match parse_pds_openapi_body::<PdsOpenApiProviderBindingRequest>(
        &body,
        "PDS provider binding request body",
    ) {
        Ok(request) => request,
        Err(error) => return error.into_response(),
    };
    let request = wire::provider_binding_registration_command_input_from_openapi(request);
    let principal = principal.into_principal();

    match state
        .run_pds_operation(
            ApplicationOperationId::PdsRegisterProviderBinding,
            move |services| async move {
                let binding = pds_service::admit_provider_binding_registration(
                    &principal,
                    request,
                    time::OffsetDateTime::now_utc(),
                )?;
                pds_service::register_provider_binding(services, binding).await
            },
        )
        .await
    {
        Ok(StoreOutcome::Stored | StoreOutcome::Unchanged) => {
            empty_response(StatusCode::NO_CONTENT)
        }
        Ok(StoreOutcome::Conflict) => HttpError::new(
            StatusCode::CONFLICT,
            "PDS provider binding already exists with different data",
        )
        .into_response(),
        Err(error) => pds_store_error_response(&error),
    }
}

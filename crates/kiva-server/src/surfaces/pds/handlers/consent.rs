//! PDS HTTP handler bodies for this resource family.

use super::{
    application::pds_application_store_outcome_response,
    prelude::{
        ApplicationOperationId, BTreeMap, Bytes, HeaderMap, HttpError, IntoResponse,
        PDS_CONSENT_GRANTS_PATH, PDS_CONSENT_REVOCATIONS_PATH, PdsApplicationHttpContext,
        PdsConsentGrantFilter, PdsHttpContext, PdsOpenApiConsentGrantRequest,
        PdsOpenApiConsentRevocationRequest, Query, Response, State, StatusCode, empty_response,
        json_projection_response, pds_service, pds_store_error_response, pds_write_error_response,
        wire,
    },
    request::parse_pds_openapi_body,
};

use crate::app::pds as pds_app;

pub(crate) async fn get_pds_consent_grants(
    State(state): State<PdsHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_CONSENT_GRANTS_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_read_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let filter = match PdsConsentGrantFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    let principal = principal.into_principal();
    match state
        .run_pds_operation(
            ApplicationOperationId::PdsListConsentGrants,
            move |services| async move {
                pds_service::list_consent_grants(services, &principal, filter).await
            },
        )
        .await
    {
        Ok(grants) => {
            json_projection_response(StatusCode::OK, wire::consent_grant_list_response(&grants))
        }
        Err(error) => pds_store_error_response(&error),
    }
}

pub(crate) async fn post_pds_consent_grant(
    State(state): State<PdsApplicationHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_CONSENT_GRANTS_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_delegate_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let request = match parse_pds_openapi_body::<PdsOpenApiConsentGrantRequest>(
        &body,
        "PDS consent grant request body",
    ) {
        Ok(request) => request,
        Err(error) => return error.into_response(),
    };
    let request = wire::consent_grant_command_input_from_openapi(request);

    match state
        .pds_application()
        .register_consent_grant(principal.as_principal(), request)
        .await
    {
        Ok(outcome) => pds_application_store_outcome_response(
            outcome,
            "PDS consent grant already exists with different or revoked data",
        ),
        Err(error) => pds_write_error_response(&error),
    }
}

pub(crate) async fn post_pds_consent_revocation(
    State(state): State<PdsApplicationHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    if let Some(response) = state
        .rate_limit_response(PDS_CONSENT_REVOCATIONS_PATH)
        .await
    {
        return response;
    }
    let principal = match state.authorize_pds_revoke_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let request = match parse_pds_openapi_body::<PdsOpenApiConsentRevocationRequest>(
        &body,
        "PDS consent revocation request body",
    ) {
        Ok(request) => request,
        Err(error) => return error.into_response(),
    };
    let request = wire::consent_revocation_command_input_from_openapi(request);

    match state
        .pds_application()
        .revoke_consent_grant(principal.as_principal(), request)
        .await
    {
        Ok(
            pds_app::ApplicationRevocationOutcome::Revoked
            | pds_app::ApplicationRevocationOutcome::AlreadyRevoked,
        ) => empty_response(StatusCode::NO_CONTENT),
        Ok(pds_app::ApplicationRevocationOutcome::NotFound) => {
            HttpError::new(StatusCode::NOT_FOUND, "PDS consent grant was not found").into_response()
        }
        Ok(pds_app::ApplicationRevocationOutcome::Forbidden) => HttpError::new(
            StatusCode::FORBIDDEN,
            "PDS consent grant belongs to a different data subject",
        )
        .into_response(),
        Err(error) => pds_write_error_response(&error),
    }
}

//! PDS HTTP handler bodies for this resource family.

use super::{
    application::pds_application_store_outcome_response,
    prelude::{
        ApplicationOperationId, BTreeMap, Bytes, Decision, HeaderMap, HttpError, IntoResponse,
        PDS_DATA_PRODUCT_READS_PATH, PDS_DATA_PRODUCTS_PATH, PDS_METADATA_EXPORTS_PATH,
        PDS_RECORD_INVENTORY_PATH, PdsApplicationHttpContext, PdsDataProductFilter, PdsHttpContext,
        PdsOpenApiDataProductReadRequest, PdsOpenApiDataProductRequest, PdsRepositoryError, Query,
        Response, State, StatusCode, json_projection_response, pds_read_model_error_response,
        pds_service, pds_store_error_response, wire,
    },
    request::parse_pds_openapi_body,
};

use crate::app::pds as pds_app;
use crate::surfaces::pds::response::pds_application_store_error_response;

pub(crate) async fn get_pds_data_products(
    State(state): State<PdsHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_DATA_PRODUCTS_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_read_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let filter = match PdsDataProductFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    let principal = principal.into_principal();
    match state
        .run_pds_operation(
            ApplicationOperationId::PdsListDataProducts,
            move |services| async move {
                pds_service::list_data_products(services, &principal, filter).await
            },
        )
        .await
    {
        Ok(data_products) => json_projection_response(
            StatusCode::OK,
            wire::data_product_list_response(&data_products),
        ),
        Err(error) => pds_store_error_response(&error),
    }
}

pub(crate) async fn post_pds_data_product(
    State(state): State<PdsApplicationHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_DATA_PRODUCTS_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_write_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let request = match parse_pds_openapi_body::<PdsOpenApiDataProductRequest>(
        &body,
        "PDS data product request body",
    ) {
        Ok(request) => request,
        Err(error) => return error.into_response(),
    };
    let request = wire::data_product_registration_command_input_from_openapi(request);

    match state
        .pds_application()
        .register_data_product(principal.as_principal(), request)
        .await
    {
        Ok(outcome) => pds_application_store_outcome_response(
            outcome,
            "PDS data product already exists with different data",
        ),
        Err(pds_app::RegisterDataProductError::ArtifactUnavailable) => HttpError::new(
            StatusCode::CONFLICT,
            "PDS data product release reference must point to an available confidential release owned by the authenticated subject",
        )
        .into_response(),
        Err(pds_app::RegisterDataProductError::ArtifactStore(error)) => {
            match error {
                pds_app::ReleaseArtifactAvailabilityError::Store => {
                    tracing::error!("PDS data product release-artifact availability check failed");
                }
                pds_app::ReleaseArtifactAvailabilityError::InvalidStoredData => {
                    tracing::error!(
                        "PDS data product release-artifact availability check read invalid stored data"
                    );
                }
            }
            HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "PDS data product release-artifact availability check failed",
            )
            .into_response()
        }
        Err(pds_app::RegisterDataProductError::Runtime(error)) => {
            tracing::error!(
                error = error.message(),
                "PDS data product application runtime failed"
            );
            HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "PDS data product application runtime failed",
            )
            .into_response()
        }
        Err(pds_app::RegisterDataProductError::Pds(
            pds_app::PdsApplicationStoreError::InvalidRequest(
            message,
        ))) => HttpError::new(StatusCode::BAD_REQUEST, message).into_response(),
        Err(pds_app::RegisterDataProductError::Pds(error)) => {
            pds_application_store_error_response(&error)
        }
    }
}

pub(crate) async fn get_pds_record_inventory(
    State(state): State<PdsApplicationHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_RECORD_INVENTORY_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_read_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let inventory = match state
        .pds_application()
        .record_inventory(principal.as_principal())
        .await
    {
        Ok(inventory) => inventory,
        Err(error) => return pds_read_model_error_response(&error),
    };
    let view = wire::RecordInventoryView::from_inventory(&inventory);
    json_projection_response(StatusCode::OK, wire::record_inventory_response(&view))
}

pub(crate) async fn get_pds_metadata_exports(
    State(state): State<PdsApplicationHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_METADATA_EXPORTS_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_read_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let export = match state
        .pds_application()
        .metadata_export(principal.as_principal())
        .await
    {
        Ok(export) => export,
        Err(error) => return pds_read_model_error_response(&error),
    };
    let view = wire::MetadataExportView::from_export(&export);
    json_projection_response(StatusCode::OK, wire::metadata_export_response(&view))
}

pub(crate) async fn post_pds_data_product_read(
    State(state): State<PdsHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    if let Some(response) = state.rate_limit_response(PDS_DATA_PRODUCT_READS_PATH).await {
        return response;
    }
    let principal = match state.authorize_pds_read_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let request = match parse_pds_openapi_body::<PdsOpenApiDataProductReadRequest>(
        &body,
        "PDS data product read request body",
    ) {
        Ok(request) => request,
        Err(error) => return error.into_response(),
    };
    let request = wire::data_product_read_intent_from_openapi(request);

    let principal = principal.into_principal();
    match state
        .run_pds_operation(
            ApplicationOperationId::PdsReadDataProduct,
            move |services| async move {
                pds_service::read_data_product(services, &principal, request).await
            },
        )
        .await
    {
        Ok(outcome) => {
            let status = if matches!(outcome.event.decision, Decision::Allow) {
                StatusCode::OK
            } else {
                StatusCode::FORBIDDEN
            };
            json_projection_response(status, wire::data_product_read_response(&outcome))
        }
        Err(PdsRepositoryError::InvalidRequest(message)) => {
            HttpError::new(StatusCode::BAD_REQUEST, message).into_response()
        }
        Err(error) => pds_store_error_response(&error),
    }
}

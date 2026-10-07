//! Learner-initiated data portability HTTP handler.

use super::prelude::{
    BTreeMap, HeaderMap, HttpError, IntoResponse, PDS_DATA_PORTABILITY_EXPORTS_PATH,
    PdsApplicationHttpContext, Query, Response, State, StatusCode, json_projection_response,
    pds_read_model_error_response, wire,
};

const DEFAULT_PAGE_SIZE: usize = 20;
const MAX_PAGE_SIZE: usize = 100;

pub(crate) async fn get_pds_data_portability_exports(
    State(state): State<PdsApplicationHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = state
        .rate_limit_response(PDS_DATA_PORTABILITY_EXPORTS_PATH)
        .await
    {
        return response;
    }
    let principal = match state.authorize_pds_read_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    if query.keys().any(|key| key != "cursor" && key != "pageSize") {
        return HttpError::new(
            StatusCode::BAD_REQUEST,
            "PDS portability query parameter is not supported",
        )
        .into_response();
    }
    let page_size = match query.get("pageSize") {
        Some(value) => match value.parse::<usize>() {
            Ok(value) => value,
            Err(_error) => {
                return HttpError::new(
                    StatusCode::BAD_REQUEST,
                    "PDS portability pageSize must be an integer",
                )
                .into_response();
            }
        },
        None => DEFAULT_PAGE_SIZE,
    };
    if !(1..=MAX_PAGE_SIZE).contains(&page_size) {
        return HttpError::new(
            StatusCode::BAD_REQUEST,
            "PDS portability pageSize must be between 1 and 100",
        )
        .into_response();
    }
    let export = match state
        .pds_application()
        .portability_export(
            principal.as_principal(),
            query.get("cursor").map(String::as_str),
            page_size,
        )
        .await
    {
        Ok(export) => export,
        Err(error) => return pds_read_model_error_response(&error),
    };
    json_projection_response(StatusCode::OK, wire::portability_export_response(&export))
}

//! Governed analytics HTTP handlers.

use std::collections::BTreeMap;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use kiva_application::analytics::AnalyticsQueryError;
use uuid::Uuid;

use crate::core::{
    paths::{ANALYTICS_PRODUCT_AGGREGATE_PATH, ANALYTICS_PRODUCT_PATH, ANALYTICS_PRODUCTS_PATH},
    response::HttpError,
    state::AnalyticsHttpContext,
};

use super::wire;

const DEFAULT_PAGE_SIZE: usize = 20;
const MAX_PAGE_SIZE: usize = 100;

fn analytics_error(error: &AnalyticsQueryError) -> Response {
    match error {
        AnalyticsQueryError::NotFound => {
            HttpError::new(StatusCode::NOT_FOUND, "Analytics product was not found").into_response()
        }
        AnalyticsQueryError::Store(_) | AnalyticsQueryError::Runtime(_) => {
            tracing::error!("governed analytics read failed");
            HttpError::new(StatusCode::INTERNAL_SERVER_ERROR, "Analytics read failed")
                .into_response()
        }
    }
}

fn invalid_stored_data() -> Response {
    tracing::error!("analytics application returned an unsupported aggregate release kind");
    HttpError::new(StatusCode::INTERNAL_SERVER_ERROR, "Analytics read failed").into_response()
}

fn page(
    products: Vec<kiva_application::analytics::AnalyticsProduct>,
    query: &BTreeMap<String, String>,
) -> Result<
    (
        Vec<kiva_application::analytics::AnalyticsProduct>,
        Option<String>,
    ),
    HttpError,
> {
    if query.keys().any(|key| key != "cursor" && key != "pageSize") {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "Analytics query parameter is not supported",
        ));
    }
    let page_size = query
        .get("pageSize")
        .map_or(Ok(DEFAULT_PAGE_SIZE), |value| {
            value.parse::<usize>().map_err(|_error| {
                HttpError::new(
                    StatusCode::BAD_REQUEST,
                    "Analytics pageSize must be an integer",
                )
            })
        })?;
    if !(1..=MAX_PAGE_SIZE).contains(&page_size) {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "Analytics pageSize must be between 1 and 100",
        ));
    }
    let start = if let Some(cursor) = query.get("cursor") {
        let cursor = Uuid::parse_str(cursor).map_err(|_error| {
            HttpError::new(StatusCode::BAD_REQUEST, "Analytics cursor is invalid")
        })?;
        products
            .iter()
            .position(|product| product.data_product_id == cursor)
            .map(|index| index + 1)
            .ok_or_else(|| HttpError::new(StatusCode::BAD_REQUEST, "Analytics cursor is invalid"))?
    } else {
        0
    };
    let mut selected = products
        .into_iter()
        .skip(start)
        .take(page_size + 1)
        .collect::<Vec<_>>();
    let has_more = selected.len() > page_size;
    if has_more {
        selected.truncate(page_size);
    }
    let next_cursor = has_more
        .then(|| {
            selected
                .last()
                .map(|product| product.data_product_id.to_string())
        })
        .flatten();
    Ok((selected, next_cursor))
}

pub(crate) async fn get_analytics_products(
    State(state): State<AnalyticsHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = state.rate_limit_response(ANALYTICS_PRODUCTS_PATH).await {
        return response;
    }
    let principal = match state
        .authorize_analytics_read_request(&headers, &query)
        .await
    {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let products = match state
        .analytics_application()
        .application()
        .list_products(principal.as_principal())
        .await
    {
        Ok(products) => products,
        Err(error) => return analytics_error(&error),
    };
    let (products, next_cursor) = match page(products, &query) {
        Ok(page) => page,
        Err(error) => return error.into_response(),
    };
    wire::product_page(products, next_cursor).map_or_else(invalid_stored_data, |response| {
        (StatusCode::OK, Json(response)).into_response()
    })
}

pub(crate) async fn get_analytics_product(
    State(state): State<AnalyticsHttpContext>,
    Path(data_product_id): Path<Uuid>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = state.rate_limit_response(ANALYTICS_PRODUCT_PATH).await {
        return response;
    }
    let principal = match state
        .authorize_analytics_read_request(&headers, &query)
        .await
    {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    match state
        .analytics_application()
        .application()
        .read_product(principal.as_principal(), data_product_id)
        .await
    {
        Ok(product) => wire::product(product).map_or_else(invalid_stored_data, |response| {
            (StatusCode::OK, Json(response)).into_response()
        }),
        Err(error) => analytics_error(&error),
    }
}

pub(crate) async fn get_analytics_product_aggregate(
    State(state): State<AnalyticsHttpContext>,
    Path(data_product_id): Path<Uuid>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = state
        .rate_limit_response(ANALYTICS_PRODUCT_AGGREGATE_PATH)
        .await
    {
        return response;
    }
    let principal = match state
        .authorize_analytics_read_request(&headers, &query)
        .await
    {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    match state
        .analytics_application()
        .application()
        .read_aggregate(principal.as_principal(), data_product_id)
        .await
    {
        Ok(aggregate) => wire::aggregate(&aggregate).map_or_else(invalid_stored_data, |response| {
            (StatusCode::OK, Json(response)).into_response()
        }),
        Err(error) => analytics_error(&error),
    }
}

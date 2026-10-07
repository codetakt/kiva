//! Governed analytics HTTP routes.

use axum::{Router, routing::get};

use crate::{
    core::{
        paths::{
            ANALYTICS_PRODUCT_AGGREGATE_PATH, ANALYTICS_PRODUCT_PATH, ANALYTICS_PRODUCTS_PATH,
        },
        state::AnalyticsHttpContext,
    },
    surfaces::analytics::handlers::{
        get_analytics_product, get_analytics_product_aggregate, get_analytics_products,
    },
};

pub(crate) fn router(state: AnalyticsHttpContext) -> Router {
    debug_assert_eq!(
        AnalyticsHttpContext::surface_profile_spec().application_surface,
        Some("analytics")
    );
    Router::new()
        .route(ANALYTICS_PRODUCTS_PATH, get(get_analytics_products))
        .route(ANALYTICS_PRODUCT_PATH, get(get_analytics_product))
        .route(
            ANALYTICS_PRODUCT_AGGREGATE_PATH,
            get(get_analytics_product_aggregate),
        )
        .with_state(state)
}

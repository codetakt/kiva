//! HTTP server composition for Kiva.

#![allow(clippy::redundant_pub_crate)]

use std::io;

use axum::{Router, middleware};

use crate::core::system;
use crate::surfaces::{
    analytics::routes as analytics_routes, confidential::routes as confidential_routes,
    pds::routes as pds_routes, xapi::routes as xapi_routes,
};

macro_rules! dispatch_repository {
    ($repository:expr, |$inner:ident| { memory => $memory:expr, postgres => $postgres:expr $(,)? }) => {
        match $repository {
            Self::Memory($inner) => $memory,
            Self::Postgres($inner) => $postgres,
        }
    };
    ($repository:expr, |$inner:ident| $body:expr) => {
        match $repository {
            Self::Memory($inner) => $body,
            Self::Postgres($inner) => $body,
        }
    };
}

mod app;
mod core;
mod deployment;
mod surfaces;

#[cfg(test)]
mod test_support;

pub(crate) use crate::core::state::AppState;
#[cfg(test)]
pub(crate) use test_support::*;
/// Builds the Kiva HTTP router.
pub fn router() -> Router {
    let state = AppState::default();
    router_with_state(&state)
}

/// Builds the Kiva HTTP router from process environment configuration.
///
/// When `DATABASE_URL` is present, Statements are stored in `PostgreSQL`. When
/// it is absent, Kiva uses an in-memory store for local conformance smoke tests.
pub async fn router_from_env() -> Result<Router, io::Error> {
    let token_verifier = deployment::idp_token_verifier_from_env()?;
    let state = AppState::from_env(token_verifier).await?;
    let router = router_with_state(&state);
    if deployment::adl_basic_auth_compatibility_from_env()? {
        Ok(router.layer(middleware::from_fn(deployment::translate_adl_basic_auth)))
    } else {
        Ok(router)
    }
}

fn router_with_state(state: &AppState) -> Router {
    Router::new()
        .merge(system::router(state.system_http_context()))
        .merge(analytics_routes::router(state.analytics_http_context()))
        .merge(xapi_routes::router(state.xapi_http_context()))
        .merge(pds_routes::router(
            state.pds_http_context(),
            state.pds_application_http_context(),
        ))
        .merge(confidential_routes::router(
            state.confidential_http_context(),
        ))
}

#[cfg(test)]
mod tests;

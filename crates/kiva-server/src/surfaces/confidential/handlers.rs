//! Confidential-computation HTTP handler bodies.
#![allow(clippy::wildcard_imports)]

use std::{collections::BTreeMap, future::Future};

use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use kiva_confidential::mechanism::job::ConfidentialJobStatus;
use kiva_confidential_contract as confidential_contract;
use kiva_identity::{
    AuthorizedConfidentialCompute, AuthorizedConfidentialRead, AuthorizedConfidentialRelease,
    AuthorizedConfidentialWrite, AuthorizedPrincipal,
};
use serde::de::DeserializeOwned;

use crate::core::{
    extract::parse_json_request,
    outcome::StoreOutcome,
    paths::*,
    request_util::required_transcript_id_query,
    response::{HttpError, empty_response, json_projection_response},
    state::ConfidentialHttpContext,
};
use crate::surfaces::confidential::{
    model::*,
    outcome::*,
    response::{
        confidential_application_admission_error_response,
        confidential_differential_privacy_budget_error_response,
        confidential_differential_privacy_release_error_response, confidential_job_error_response,
        confidential_participant_error_response, confidential_secure_error_response,
        confidential_shamir_reconstruction_error_response, confidential_store_error_response,
        confidential_threshold_release_error_response,
    },
    wire,
};

mod additive;
mod differential_privacy;
mod jobs;
mod participants;
mod shamir;
mod threshold;

pub(crate) use additive::*;
pub(crate) use differential_privacy::*;
pub(crate) use jobs::*;
pub(crate) use participants::*;
pub(crate) use shamir::*;
pub(crate) use threshold::*;

pub(super) async fn authorize_confidential_endpoint<O, F>(
    state: &ConfidentialHttpContext,
    path: &str,
    authorize: F,
) -> Result<O, Response>
where
    F: Future<Output = Result<O, HttpError>>,
{
    enforce_confidential_endpoint_rate_limit(state, path).await?;
    authorize.await.map_err(IntoResponse::into_response)
}

pub(super) async fn enforce_confidential_endpoint_rate_limit(
    state: &ConfidentialHttpContext,
    path: &str,
) -> Result<(), Response> {
    if let Some(response) = state.rate_limit_response(path).await {
        return Err(response);
    }
    Ok(())
}

#[allow(clippy::result_large_err)]
pub(super) fn parse_confidential_json_request<T>(
    body: &Bytes,
    context: &'static str,
) -> Result<T, Response>
where
    T: DeserializeOwned,
{
    parse_json_request::<T>(body, context).map_err(IntoResponse::into_response)
}

#[allow(clippy::result_large_err)]
pub(super) fn parse_confidential_openapi_request<T, U>(
    body: &Bytes,
    context: &'static str,
    convert: impl FnOnce(T) -> Result<U, HttpError>,
) -> Result<U, Response>
where
    T: DeserializeOwned,
{
    let request = parse_confidential_json_request::<T>(body, context)?;
    convert(request).map_err(IntoResponse::into_response)
}

pub(super) async fn authorize_confidential_read_endpoint(
    state: &ConfidentialHttpContext,
    path: &str,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
) -> Result<AuthorizedPrincipal<AuthorizedConfidentialRead>, Response> {
    authorize_confidential_endpoint(
        state,
        path,
        state.authorize_confidential_read_request(headers, query),
    )
    .await
}

pub(super) async fn authorize_confidential_write_endpoint(
    state: &ConfidentialHttpContext,
    path: &str,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
) -> Result<AuthorizedPrincipal<AuthorizedConfidentialWrite>, Response> {
    authorize_confidential_endpoint(
        state,
        path,
        state.authorize_confidential_write_request(headers, query),
    )
    .await
}

pub(super) async fn authorize_confidential_compute_endpoint(
    state: &ConfidentialHttpContext,
    path: &str,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
) -> Result<AuthorizedPrincipal<AuthorizedConfidentialCompute>, Response> {
    authorize_confidential_endpoint(
        state,
        path,
        state.authorize_confidential_compute_request(headers, query),
    )
    .await
}

pub(super) async fn authorize_confidential_release_endpoint(
    state: &ConfidentialHttpContext,
    path: &str,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
) -> Result<AuthorizedPrincipal<AuthorizedConfidentialRelease>, Response> {
    authorize_confidential_endpoint(
        state,
        path,
        state.authorize_confidential_release_request(headers, query),
    )
    .await
}

#[allow(clippy::result_large_err)]
pub(super) async fn authorize_confidential_write_scope(
    state: &ConfidentialHttpContext,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
) -> Result<AuthorizedPrincipal<AuthorizedConfidentialWrite>, Response> {
    state
        .authorize_confidential_write_request(headers, query)
        .await
        .map_err(IntoResponse::into_response)
}

#[allow(clippy::result_large_err)]
pub(super) async fn authorize_confidential_release_scope(
    state: &ConfidentialHttpContext,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
) -> Result<AuthorizedPrincipal<AuthorizedConfidentialRelease>, Response> {
    state
        .authorize_confidential_release_request(headers, query)
        .await
        .map_err(IntoResponse::into_response)
}

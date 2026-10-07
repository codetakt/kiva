//! Shared HTTP request guard helpers for bounded contexts.

use std::collections::BTreeMap;

use axum::{http::HeaderMap, response::Response};
use kiva_identity::{
    AuthenticatedPrincipal, AuthorizedPrincipal as AuthorizedOperationPrincipal,
    AuthorizedResourceOperation,
};

use crate::core::{
    auth::{
        AuthConfig, authorize_about_request, authorize_typed_analytics_request,
        authorize_typed_confidential_request, authorize_typed_xapi_request,
    },
    response::{HttpError, rate_limit_response},
    runtime::RuntimeLimits,
};

#[derive(Debug, Clone)]
pub(crate) struct RequestGuards {
    limits: RuntimeLimits,
    auth: AuthConfig,
}

impl RequestGuards {
    pub(crate) const fn new(limits: RuntimeLimits, auth: AuthConfig) -> Self {
        Self { limits, auth }
    }

    pub(crate) const fn runtime_limits(&self) -> &RuntimeLimits {
        &self.limits
    }

    pub(crate) async fn rate_limit_response(&self, path: &str) -> Option<Response> {
        rate_limit_response(&self.limits, path).await
    }

    pub(crate) async fn authorize_about_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<Option<AuthenticatedPrincipal>, HttpError> {
        authorize_about_request(&self.auth, headers, query).await
    }

    pub(crate) async fn authorize_typed_pds_request<O>(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<AuthorizedOperationPrincipal<O>, HttpError>
    where
        O: AuthorizedResourceOperation,
    {
        crate::core::auth::authorize_typed_pds_request::<O>(&self.auth, headers, query).await
    }

    pub(crate) async fn authorize_typed_analytics_request<O>(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<AuthorizedOperationPrincipal<O>, HttpError>
    where
        O: AuthorizedResourceOperation,
    {
        authorize_typed_analytics_request::<O>(&self.auth, headers, query).await
    }

    pub(crate) async fn authorize_typed_confidential_request<O>(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<AuthorizedOperationPrincipal<O>, HttpError>
    where
        O: AuthorizedResourceOperation,
    {
        authorize_typed_confidential_request::<O>(&self.auth, headers, query).await
    }

    pub(crate) async fn authorize_typed_xapi_request<O>(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<Option<AuthorizedOperationPrincipal<O>>, HttpError>
    where
        O: AuthorizedResourceOperation,
    {
        authorize_typed_xapi_request::<O>(&self.auth, headers, query).await
    }
}

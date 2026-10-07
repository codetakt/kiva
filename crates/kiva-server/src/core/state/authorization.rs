//! Typed authorization methods for bounded HTTP contexts.

use std::collections::BTreeMap;

use axum::http::HeaderMap;
use kiva_identity::{
    AuthenticatedPrincipal, AuthorizedAnalyticsRead, AuthorizedConfidentialCompute,
    AuthorizedConfidentialRead, AuthorizedConfidentialRelease, AuthorizedConfidentialWrite,
    AuthorizedPdsAudit, AuthorizedPdsDelegate, AuthorizedPdsRead, AuthorizedPdsRevoke,
    AuthorizedPdsWrite, AuthorizedPrincipal as AuthorizedOperationPrincipal, AuthorizedXapiRead,
    AuthorizedXapiWrite,
};

use crate::core::response::HttpError;

use super::{
    AnalyticsHttpContext, ConfidentialHttpContext, PdsApplicationHttpContext, PdsHttpContext,
    XapiHttpContext,
};

impl AnalyticsHttpContext {
    pub(crate) async fn authorize_analytics_read_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<AuthorizedOperationPrincipal<AuthorizedAnalyticsRead>, HttpError> {
        self.guards
            .authorize_typed_analytics_request::<AuthorizedAnalyticsRead>(headers, query)
            .await
    }
}

impl XapiHttpContext {
    pub(crate) async fn authorize_about_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<Option<AuthenticatedPrincipal>, HttpError> {
        self.guards.authorize_about_request(headers, query).await
    }

    pub(crate) async fn authorize_xapi_read_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<Option<AuthorizedOperationPrincipal<AuthorizedXapiRead>>, HttpError> {
        self.guards
            .authorize_typed_xapi_request::<AuthorizedXapiRead>(headers, query)
            .await
    }

    pub(crate) async fn authorize_xapi_write_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<Option<AuthorizedOperationPrincipal<AuthorizedXapiWrite>>, HttpError> {
        self.guards
            .authorize_typed_xapi_request::<AuthorizedXapiWrite>(headers, query)
            .await
    }
}

impl PdsHttpContext {
    pub(crate) async fn authorize_pds_read_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<AuthorizedOperationPrincipal<AuthorizedPdsRead>, HttpError> {
        self.guards
            .authorize_typed_pds_request::<AuthorizedPdsRead>(headers, query)
            .await
    }

    pub(crate) async fn authorize_pds_audit_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<AuthorizedOperationPrincipal<AuthorizedPdsAudit>, HttpError> {
        self.guards
            .authorize_typed_pds_request::<AuthorizedPdsAudit>(headers, query)
            .await
    }

    pub(crate) async fn authorize_pds_write_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<AuthorizedOperationPrincipal<AuthorizedPdsWrite>, HttpError> {
        self.guards
            .authorize_typed_pds_request::<AuthorizedPdsWrite>(headers, query)
            .await
    }
}

impl PdsApplicationHttpContext {
    pub(crate) async fn authorize_pds_read_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<AuthorizedOperationPrincipal<AuthorizedPdsRead>, HttpError> {
        self.guards
            .authorize_typed_pds_request::<AuthorizedPdsRead>(headers, query)
            .await
    }

    pub(crate) async fn authorize_pds_delegate_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<AuthorizedOperationPrincipal<AuthorizedPdsDelegate>, HttpError> {
        self.guards
            .authorize_typed_pds_request::<AuthorizedPdsDelegate>(headers, query)
            .await
    }

    pub(crate) async fn authorize_pds_revoke_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<AuthorizedOperationPrincipal<AuthorizedPdsRevoke>, HttpError> {
        self.guards
            .authorize_typed_pds_request::<AuthorizedPdsRevoke>(headers, query)
            .await
    }

    pub(crate) async fn authorize_pds_write_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<AuthorizedOperationPrincipal<AuthorizedPdsWrite>, HttpError> {
        self.guards
            .authorize_typed_pds_request::<AuthorizedPdsWrite>(headers, query)
            .await
    }
}

impl ConfidentialHttpContext {
    pub(crate) async fn authorize_confidential_read_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<AuthorizedOperationPrincipal<AuthorizedConfidentialRead>, HttpError> {
        self.guards
            .authorize_typed_confidential_request::<AuthorizedConfidentialRead>(headers, query)
            .await
    }

    pub(crate) async fn authorize_confidential_write_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<AuthorizedOperationPrincipal<AuthorizedConfidentialWrite>, HttpError> {
        self.guards
            .authorize_typed_confidential_request::<AuthorizedConfidentialWrite>(headers, query)
            .await
    }

    pub(crate) async fn authorize_confidential_compute_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<AuthorizedOperationPrincipal<AuthorizedConfidentialCompute>, HttpError> {
        self.guards
            .authorize_typed_confidential_request::<AuthorizedConfidentialCompute>(headers, query)
            .await
    }

    pub(crate) async fn authorize_confidential_release_request(
        &self,
        headers: &HeaderMap,
        query: &BTreeMap<String, String>,
    ) -> Result<AuthorizedOperationPrincipal<AuthorizedConfidentialRelease>, HttpError> {
        self.guards
            .authorize_typed_confidential_request::<AuthorizedConfidentialRelease>(headers, query)
            .await
    }
}

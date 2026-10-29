//! HTTP context projections derived from server state.

use std::future::Future;

use axum::response::Response;
use kiva_application::operation_catalog::ApplicationOperationId;

use crate::{
    app::{
        analytics::AnalyticsApplicationAdapter, confidential::ConfidentialApplicationAdapter,
        pds::PdsApplicationAdapter,
    },
    core::{
        request_guards::RequestGuards,
        runtime::RuntimeLimits,
        state::surface_operation::{
            AnalyticsSurfaceProfile, ConfidentialSurfaceProfile, PdsApplicationSurfaceProfile,
            PdsRepositorySurfaceProfile, SurfaceOperationRunner, SurfaceProfileSpec,
        },
    },
    surfaces::{
        pds::{outcome::PdsRepositoryError, repository::PdsRepository, service::PdsQueryGateway},
        xapi::{
            repository::{DocumentRepository, StatementRepository},
            service::XapiServices,
        },
    },
};

#[derive(Debug, Clone)]
pub(crate) struct XapiHttpContext {
    statements: StatementRepository,
    documents: DocumentRepository,
    signature_policy: crate::core::signature_policy::SignaturePolicy,
    pub(super) guards: RequestGuards,
}

#[derive(Debug, Clone)]
pub(crate) struct AnalyticsHttpContext {
    application: AnalyticsApplicationAdapter,
    pub(super) guards: RequestGuards,
}

#[derive(Debug, Clone)]
pub(crate) struct PdsHttpContext {
    repository: PdsRepository,
    pub(super) guards: RequestGuards,
}

#[derive(Debug, Clone)]
pub(crate) struct PdsApplicationHttpContext {
    application: PdsApplicationAdapter,
    pub(super) guards: RequestGuards,
}

#[derive(Debug, Clone)]
pub(crate) struct ConfidentialHttpContext {
    application: ConfidentialApplicationAdapter,
    pub(super) guards: RequestGuards,
}

#[derive(Debug, Clone)]
pub(crate) struct SystemHttpContext {
    guards: RequestGuards,
}

impl XapiHttpContext {
    pub(super) const fn new(
        statements: StatementRepository,
        documents: DocumentRepository,
        signature_policy: crate::core::signature_policy::SignaturePolicy,
        guards: RequestGuards,
    ) -> Self {
        Self {
            statements,
            documents,
            signature_policy,
            guards,
        }
    }

    pub(crate) const fn xapi_services(&self) -> XapiServices<'_> {
        XapiServices::new(&self.statements, &self.documents)
    }

    pub(crate) const fn signature_policy(&self) -> &crate::core::signature_policy::SignaturePolicy {
        &self.signature_policy
    }

    pub(crate) async fn statement_watermark(
        &self,
    ) -> Result<time::OffsetDateTime, crate::surfaces::xapi::outcome::StatementStoreError> {
        self.statements.consistent_through().await
    }

    pub(crate) const fn runtime_limits(&self) -> &RuntimeLimits {
        self.guards.runtime_limits()
    }

    pub(crate) async fn rate_limit_response(&self, path: &str) -> Option<Response> {
        self.guards.rate_limit_response(path).await
    }
}

impl AnalyticsHttpContext {
    pub(crate) const fn surface_profile_spec() -> SurfaceProfileSpec {
        SurfaceProfileSpec::for_profile::<AnalyticsSurfaceProfile>()
    }

    pub(super) const fn new(
        application: AnalyticsApplicationAdapter,
        guards: RequestGuards,
    ) -> Self {
        Self {
            application,
            guards,
        }
    }

    pub(crate) fn analytics_application(&self) -> AnalyticsApplicationAdapter {
        self.application.clone()
    }

    pub(crate) async fn rate_limit_response(&self, path: &str) -> Option<Response> {
        self.guards.rate_limit_response(path).await
    }
}

impl PdsHttpContext {
    pub(crate) const fn surface_profile_spec() -> SurfaceProfileSpec {
        SurfaceProfileSpec::for_profile::<PdsRepositorySurfaceProfile>()
    }

    pub(super) const fn new(repository: PdsRepository, guards: RequestGuards) -> Self {
        Self { repository, guards }
    }

    pub(crate) fn pds_query_gateway(&self) -> PdsQueryGateway {
        PdsQueryGateway::new(&self.repository)
    }

    pub(crate) async fn run_pds_operation<T, Fut>(
        &self,
        operation_id: ApplicationOperationId,
        execute: impl FnOnce(PdsQueryGateway) -> Fut,
    ) -> Result<T, PdsRepositoryError>
    where
        Fut: Future<Output = Result<T, PdsRepositoryError>>,
    {
        SurfaceOperationRunner::<PdsRepositorySurfaceProfile>::run(
            operation_id,
            self.pds_query_gateway(),
            execute,
        )
        .await
    }

    pub(crate) async fn rate_limit_response(&self, path: &str) -> Option<Response> {
        self.guards.rate_limit_response(path).await
    }
}

impl PdsApplicationHttpContext {
    pub(crate) const fn surface_profile_spec() -> SurfaceProfileSpec {
        SurfaceProfileSpec::for_profile::<PdsApplicationSurfaceProfile>()
    }

    pub(super) const fn new(application: PdsApplicationAdapter, guards: RequestGuards) -> Self {
        Self {
            application,
            guards,
        }
    }

    pub(crate) fn pds_application(&self) -> PdsApplicationAdapter {
        self.application.clone()
    }

    pub(crate) async fn rate_limit_response(&self, path: &str) -> Option<Response> {
        self.guards.rate_limit_response(path).await
    }
}

impl ConfidentialHttpContext {
    pub(crate) const fn surface_profile_spec() -> SurfaceProfileSpec {
        SurfaceProfileSpec::for_profile::<ConfidentialSurfaceProfile>()
    }

    pub(super) const fn new(
        application: ConfidentialApplicationAdapter,
        guards: RequestGuards,
    ) -> Self {
        Self {
            application,
            guards,
        }
    }

    pub(crate) fn confidential_application(&self) -> ConfidentialApplicationAdapter {
        self.application.clone()
    }

    pub(crate) async fn rate_limit_response(&self, path: &str) -> Option<Response> {
        self.guards.rate_limit_response(path).await
    }
}

impl SystemHttpContext {
    pub(super) const fn new(guards: RequestGuards) -> Self {
        Self { guards }
    }

    pub(crate) async fn rate_limit_response(&self, path: &str) -> Option<Response> {
        self.guards.rate_limit_response(path).await
    }
}

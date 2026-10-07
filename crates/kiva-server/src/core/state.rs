//! Server state construction and service handles.

mod authorization;
mod generated_surface_profiles;
mod http_context;
mod surface_operation;

use crate::{
    app::{
        analytics::AnalyticsApplicationAdapter,
        confidential::ConfidentialApplicationAdapter,
        pds::{PdsApplicationAdapter, ReleaseArtifactAvailabilityPort},
    },
    core::{auth::AuthConfig, request_guards::RequestGuards, runtime::RuntimeLimits},
    surfaces::{
        confidential::repository::ConfidentialRepository,
        pds::repository::PdsRepository,
        xapi::repository::{DocumentRepository, StatementRepository},
    },
};

pub(crate) use http_context::{
    AnalyticsHttpContext, ConfidentialHttpContext, PdsApplicationHttpContext, PdsHttpContext,
    SystemHttpContext, XapiHttpContext,
};

#[derive(Debug, Clone)]
pub(crate) struct XapiApp {
    pub(crate) statements: StatementRepository,
    pub(crate) documents: DocumentRepository,
}

#[derive(Debug, Clone)]
pub(crate) struct PdsApp {
    pub(crate) repository: PdsRepository,
}

#[derive(Debug, Clone)]
pub(crate) struct ApplicationApp {
    pub(crate) analytics: AnalyticsApplicationAdapter,
    pub(crate) pds: PdsApplicationAdapter,
    pub(crate) confidential: ConfidentialApplicationAdapter,
}

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    pub(crate) xapi: XapiApp,
    pub(crate) pds: PdsApp,
    pub(crate) application: ApplicationApp,
    pub(crate) limits: RuntimeLimits,
    pub(crate) auth: AuthConfig,
}

impl Default for AppState {
    fn default() -> Self {
        Self::from_repositories(
            StatementRepository::default(),
            DocumentRepository::default(),
            PdsRepository::default(),
            ConfidentialRepository::default(),
            RuntimeLimits::default(),
            AuthConfig::default(),
        )
    }
}

impl AppState {
    pub(crate) fn from_repositories(
        statements: StatementRepository,
        documents: DocumentRepository,
        pds: PdsRepository,
        confidential: ConfidentialRepository,
        limits: RuntimeLimits,
        auth: AuthConfig,
    ) -> Self {
        debug_assert!(surface_operation::surface_profile_registry_is_valid());

        let release_artifacts =
            ReleaseArtifactAvailabilityPort::from_server_adapter(confidential.clone());
        let pds_application =
            PdsApplicationAdapter::from_server_adapters(&statements, &pds, &release_artifacts);
        let confidential_application =
            ConfidentialApplicationAdapter::from_server_adapters(confidential.clone());
        let analytics_application = AnalyticsApplicationAdapter::new(pds.clone(), confidential);
        Self {
            xapi: XapiApp {
                statements,
                documents,
            },
            pds: PdsApp { repository: pds },
            application: ApplicationApp {
                analytics: analytics_application,
                pds: pds_application,
                confidential: confidential_application,
            },
            limits,
            auth,
        }
    }

    pub(crate) fn xapi_http_context(&self) -> XapiHttpContext {
        XapiHttpContext::new(
            self.xapi.statements.clone(),
            self.xapi.documents.clone(),
            self.request_guards(),
        )
    }

    pub(crate) fn analytics_http_context(&self) -> AnalyticsHttpContext {
        AnalyticsHttpContext::new(self.application.analytics.clone(), self.request_guards())
    }

    pub(crate) fn pds_http_context(&self) -> PdsHttpContext {
        PdsHttpContext::new(self.pds.repository.clone(), self.request_guards())
    }

    pub(crate) fn pds_application_http_context(&self) -> PdsApplicationHttpContext {
        PdsApplicationHttpContext::new(self.application.pds.clone(), self.request_guards())
    }

    pub(crate) fn confidential_http_context(&self) -> ConfidentialHttpContext {
        ConfidentialHttpContext::new(self.application.confidential.clone(), self.request_guards())
    }

    pub(crate) fn system_http_context(&self) -> SystemHttpContext {
        SystemHttpContext::new(self.request_guards())
    }

    fn request_guards(&self) -> RequestGuards {
        RequestGuards::new(self.limits.clone(), self.auth.clone())
    }

    #[cfg(test)]
    pub(crate) const fn pds_repository(&self) -> &PdsRepository {
        &self.pds.repository
    }
}

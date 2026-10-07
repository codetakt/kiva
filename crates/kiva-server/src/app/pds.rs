//! Server adapter assembly for PDS application workflows.

mod server_adapters;

pub(crate) use kiva_application::pds::{
    ApplicationFuture, ApplicationRevocationOutcome, ApplicationStoreOutcome,
    ApplicationWriteOutcome, ConsentGrantRecord, DataProductRecord, GovernanceProfileRecord,
    LifecycleCaseRecord, LifecycleEventRecord, MetadataExport, PdsApplicationStoreError,
    PdsApplicationWriteError, PdsMetadataRecords, PdsReadModelError, PortabilityExport,
    PortabilityStatementPage, PortabilityStatementRecord, RecordInventory,
    RegisterDataProductError, ReleaseArtifactAvailabilityError, RetentionPolicyRecord,
    StatementReadError, StatementRecord,
};

use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::{
    ConsentGrantCommandInput as PdsConsentGrantCommandInput,
    ConsentRevocationCommandInput as PdsConsentRevocationCommandInput,
    DataProductRegistrationCommandInput as PdsDataProductRegistrationCommandInput,
    GovernanceProfileCommandInput as PdsGovernanceProfileCommandInput,
    LifecycleCaseCommandInput as PdsLifecycleCaseCommandInput,
    LifecycleEventCommandInput as PdsLifecycleEventCommandInput,
    RetentionPolicyCommandInput as PdsRetentionPolicyCommandInput,
};

pub(crate) use server_adapters::ReleaseArtifactAvailabilityPort;

use crate::surfaces::{pds::repository::PdsRepository, xapi::repository::StatementRepository};

use self::server_adapters::{PdsRecordPort, PdsSubjectIdentityMapper, XapiStatementReadPort};

pub(crate) type PdsApplicationPorts = kiva_application::pds::PdsApplicationPorts<
    XapiStatementReadPort,
    PdsRecordPort,
    PdsRecordPort,
    ReleaseArtifactAvailabilityPort,
    PdsSubjectIdentityMapper,
>;
pub(crate) type PdsApplicationCapabilities =
    kiva_application::pds::PdsApplicationCapabilities<PdsApplicationPorts>;
pub(crate) type PdsApplication = kiva_application::pds::PdsApplication<PdsApplicationPorts>;

#[derive(Debug, Clone)]
pub(crate) struct PdsApplicationAdapter {
    capabilities: PdsApplicationCapabilities,
}

impl PdsApplicationAdapter {
    pub(crate) fn from_server_adapters(
        statements: &StatementRepository,
        pds: &PdsRepository,
        release_artifacts: &ReleaseArtifactAvailabilityPort,
    ) -> Self {
        Self {
            capabilities: server_adapters::capabilities_from_server_adapters(
                statements,
                pds,
                release_artifacts,
            ),
        }
    }

    pub(crate) fn application(&self) -> PdsApplication {
        PdsApplication::new(self.capabilities.clone())
    }

    pub(crate) async fn record_inventory(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<RecordInventory, PdsReadModelError> {
        self.application().record_inventory(principal).await
    }

    pub(crate) async fn metadata_export(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<MetadataExport, PdsReadModelError> {
        self.application().metadata_export(principal).await
    }

    pub(crate) async fn portability_export(
        &self,
        principal: &AuthenticatedPrincipal,
        cursor: Option<&str>,
        page_size: usize,
    ) -> Result<PortabilityExport, PdsReadModelError> {
        self.application()
            .portability_export(principal, cursor, page_size)
            .await
    }

    pub(crate) async fn register_data_product(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsDataProductRegistrationCommandInput,
    ) -> Result<ApplicationStoreOutcome, RegisterDataProductError> {
        self.application()
            .register_data_product(principal, request)
            .await
    }

    pub(crate) async fn register_consent_grant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsConsentGrantCommandInput,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationWriteError> {
        self.application()
            .register_consent_grant(principal, request)
            .await
    }

    pub(crate) async fn revoke_consent_grant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsConsentRevocationCommandInput,
    ) -> Result<ApplicationRevocationOutcome, PdsApplicationWriteError> {
        self.application()
            .revoke_consent_grant(principal, request)
            .await
    }

    pub(crate) async fn register_lifecycle_event(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsLifecycleEventCommandInput,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationWriteError> {
        self.application()
            .register_lifecycle_event(principal, request)
            .await
    }

    pub(crate) async fn register_retention_policy(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsRetentionPolicyCommandInput,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationWriteError> {
        self.application()
            .register_retention_policy(principal, request)
            .await
    }

    pub(crate) async fn register_lifecycle_case(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsLifecycleCaseCommandInput,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationWriteError> {
        self.application()
            .register_lifecycle_case(principal, request)
            .await
    }

    pub(crate) async fn register_governance_profile(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsGovernanceProfileCommandInput,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationWriteError> {
        self.application()
            .register_governance_profile(principal, request)
            .await
    }
}

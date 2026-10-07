//! PDS application command entry points.

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

use crate::runtime::ApplicationRuntimePort;

use super::{
    PdsApplication,
    capabilities::PdsApplicationPortSet,
    model::{
        ApplicationRevocationOutcome, ApplicationStoreOutcome, PdsApplicationWriteError,
        RegisterDataProductError,
    },
};

impl<P, A> PdsApplication<P, A>
where
    P: PdsApplicationPortSet,
    A: ApplicationRuntimePort,
{
    /// Registers an approved PDS data product after command and release-artifact admission.
    pub async fn register_data_product(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsDataProductRegistrationCommandInput,
    ) -> Result<ApplicationStoreOutcome, RegisterDataProductError> {
        self.capabilities
            .begin_workflow()
            .register_data_product(principal, request)
            .await
    }

    /// Registers a PDS consent grant after command admission.
    pub async fn register_consent_grant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsConsentGrantCommandInput,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationWriteError> {
        self.capabilities
            .begin_workflow()
            .register_consent_grant(principal, request)
            .await
    }

    /// Revokes a PDS consent grant after command admission.
    pub async fn revoke_consent_grant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsConsentRevocationCommandInput,
    ) -> Result<ApplicationRevocationOutcome, PdsApplicationWriteError> {
        self.capabilities
            .begin_workflow()
            .revoke_consent_grant(principal, request)
            .await
    }

    /// Records a PDS lifecycle event after command admission.
    pub async fn register_lifecycle_event(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsLifecycleEventCommandInput,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationWriteError> {
        self.capabilities
            .begin_workflow()
            .register_lifecycle_event(principal, request)
            .await
    }

    /// Registers a PDS retention policy after command admission.
    pub async fn register_retention_policy(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsRetentionPolicyCommandInput,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationWriteError> {
        self.capabilities
            .begin_workflow()
            .register_retention_policy(principal, request)
            .await
    }

    /// Registers or updates a PDS lifecycle case after command admission.
    pub async fn register_lifecycle_case(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsLifecycleCaseCommandInput,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationWriteError> {
        self.capabilities
            .begin_workflow()
            .register_lifecycle_case(principal, request)
            .await
    }

    /// Registers a public-sector PDS governance profile after command admission.
    pub async fn register_governance_profile(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsGovernanceProfileCommandInput,
    ) -> Result<ApplicationStoreOutcome, PdsApplicationWriteError> {
        self.capabilities
            .begin_workflow()
            .register_governance_profile(principal, request)
            .await
    }
}

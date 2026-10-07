//! PDS read and direct-access surface services.

use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::{
    AccessCheckIntent as PdsAccessCheckIntent, DataProductReadIntent as PdsDataProductReadIntent,
};

#[path = "service/provider_bindings.rs"]
mod provider_bindings;

pub(crate) use provider_bindings::admit_provider_binding_registration;

use crate::surfaces::pds::{
    model::{
        PdsAccessAuditEventFilter, PdsAccessAuditEventState, PdsConsentGrantFilter,
        PdsConsentGrantState, PdsDataProductFilter, PdsDataProductReadOutcome, PdsDataProductState,
        PdsGovernanceProfileFilter, PdsGovernanceProfileState, PdsLifecycleCaseFilter,
        PdsLifecycleCaseState, PdsLifecycleEventFilter, PdsLifecycleEventState,
        PdsProviderBindingFilter, PdsProviderBindingState, PdsRetentionPolicyFilter,
        PdsRetentionPolicyState,
    },
    outcome::PdsRepositoryError,
    repository::PdsRepository,
};

#[derive(Clone)]
pub(crate) struct PdsQueryGateway {
    repository: PdsRepository,
}

impl PdsQueryGateway {
    pub(crate) fn new(repository: &PdsRepository) -> Self {
        Self {
            repository: repository.clone(),
        }
    }
}

pub(crate) async fn list_consent_grants(
    services: PdsQueryGateway,
    principal: &AuthenticatedPrincipal,
    filter: PdsConsentGrantFilter,
) -> Result<Vec<PdsConsentGrantState>, PdsRepositoryError> {
    services
        .repository
        .list_consent_grants(principal, filter)
        .await
}

pub(crate) async fn list_data_products(
    services: PdsQueryGateway,
    principal: &AuthenticatedPrincipal,
    filter: PdsDataProductFilter,
) -> Result<Vec<PdsDataProductState>, PdsRepositoryError> {
    services
        .repository
        .list_data_products(principal, filter)
        .await
}

pub(crate) async fn list_provider_bindings(
    services: PdsQueryGateway,
    principal: &AuthenticatedPrincipal,
    filter: PdsProviderBindingFilter,
) -> Result<Vec<PdsProviderBindingState>, PdsRepositoryError> {
    services
        .repository
        .list_provider_bindings(principal, filter)
        .await
}

pub(crate) async fn register_provider_binding(
    services: PdsQueryGateway,
    binding: PdsProviderBindingState,
) -> Result<crate::core::outcome::StoreOutcome, PdsRepositoryError> {
    services.repository.register_provider_binding(binding).await
}

pub(crate) async fn list_access_audit_events(
    services: PdsQueryGateway,
    principal: &AuthenticatedPrincipal,
    filter: PdsAccessAuditEventFilter,
) -> Result<Vec<PdsAccessAuditEventState>, PdsRepositoryError> {
    services
        .repository
        .list_access_audit_events(principal, filter)
        .await
}

pub(crate) async fn list_lifecycle_events(
    services: PdsQueryGateway,
    principal: &AuthenticatedPrincipal,
    filter: PdsLifecycleEventFilter,
) -> Result<Vec<PdsLifecycleEventState>, PdsRepositoryError> {
    services
        .repository
        .list_lifecycle_events(principal, filter)
        .await
}

pub(crate) async fn list_retention_policies(
    services: PdsQueryGateway,
    principal: &AuthenticatedPrincipal,
    filter: PdsRetentionPolicyFilter,
) -> Result<Vec<PdsRetentionPolicyState>, PdsRepositoryError> {
    services
        .repository
        .list_retention_policies(principal, filter)
        .await
}

pub(crate) async fn list_lifecycle_cases(
    services: PdsQueryGateway,
    principal: &AuthenticatedPrincipal,
    filter: PdsLifecycleCaseFilter,
) -> Result<Vec<PdsLifecycleCaseState>, PdsRepositoryError> {
    services
        .repository
        .list_lifecycle_cases(principal, filter)
        .await
}

pub(crate) async fn list_governance_profiles(
    services: PdsQueryGateway,
    principal: &AuthenticatedPrincipal,
    filter: PdsGovernanceProfileFilter,
) -> Result<Vec<PdsGovernanceProfileState>, PdsRepositoryError> {
    services
        .repository
        .list_governance_profiles(principal, filter)
        .await
}

pub(crate) async fn read_data_product(
    services: PdsQueryGateway,
    principal: &AuthenticatedPrincipal,
    request: PdsDataProductReadIntent,
) -> Result<PdsDataProductReadOutcome, PdsRepositoryError> {
    services
        .repository
        .read_data_product(principal, request)
        .await
}

pub(crate) async fn evaluate_access_check(
    services: PdsQueryGateway,
    principal: &AuthenticatedPrincipal,
    request: PdsAccessCheckIntent,
) -> Result<PdsAccessAuditEventState, PdsRepositoryError> {
    services
        .repository
        .evaluate_access_check(principal, request)
        .await
}

//! PDS application workflow boundary.

mod commands;
mod queries;

use crate::{
    operation_catalog,
    runtime::{
        ApplicationOperationGateway, ApplicationRuntimePort, ApplicationWorkflowKind,
        ApplicationWorkflowSpec, WorkflowKind,
    },
};

use super::capabilities::{PdsApplicationCapabilities, PdsApplicationPortSet};

/// Stable PDS application workflow kinds.
#[allow(clippy::redundant_pub_crate)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PdsWorkflowKind {
    /// Consent-grant read model.
    ListConsentGrants,
    /// Approved data-product registration.
    RegisterDataProduct,
    /// Consent-grant registration.
    RegisterConsentGrant,
    /// Consent-grant revocation.
    RevokeConsentGrant,
    /// Approved data-product read model.
    ListDataProducts,
    /// Policy-gated data-product read.
    ReadDataProduct,
    /// Provider-binding read model.
    ListProviderBindings,
    /// Provider-binding metadata registration.
    RegisterProviderBinding,
    /// Access decision evaluation.
    EvaluateAccessCheck,
    /// Access-audit-event read model.
    ListAccessAuditEvents,
    /// Lifecycle-event read model.
    ListLifecycleEvents,
    /// Lifecycle-event recording.
    RegisterLifecycleEvent,
    /// Retention-policy read model.
    ListRetentionPolicies,
    /// Retention-policy registration.
    RegisterRetentionPolicy,
    /// Lifecycle-case read model.
    ListLifecycleCases,
    /// Lifecycle-case upsert.
    RegisterLifecycleCase,
    /// Public-sector governance-profile read model.
    ListGovernanceProfiles,
    /// Public-sector governance-profile registration.
    RegisterGovernanceProfile,
    /// Subject-visible record-inventory read model.
    ReadRecordInventory,
    /// Subject-visible metadata export read model.
    ExportMetadata,
    /// Subject-bound Statement and PDS metadata portability export.
    ExportPortability,
}

/// Stable command/query boundary classification for PDS workflows.
#[allow(clippy::redundant_pub_crate)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PdsWorkflowBoundary {
    /// Mutating workflow admitted through the application command path.
    Command,
    /// Subject- or tenant-scoped read model.
    Query,
    /// Policy decision or direct-access workflow that may record audit evidence.
    DirectAccess,
}

impl PdsWorkflowKind {
    pub(crate) const fn application_kind(self) -> ApplicationWorkflowKind {
        self.workflow_spec().kind()
    }

    pub(crate) const fn workflow_spec(self) -> ApplicationWorkflowSpec {
        operation_catalog::pds_workflow_spec(self)
    }

    pub(crate) const fn boundary(self) -> PdsWorkflowBoundary {
        match self {
            Self::RegisterDataProduct
            | Self::RegisterConsentGrant
            | Self::RevokeConsentGrant
            | Self::RegisterProviderBinding
            | Self::RegisterLifecycleEvent
            | Self::RegisterRetentionPolicy
            | Self::RegisterLifecycleCase
            | Self::RegisterGovernanceProfile => PdsWorkflowBoundary::Command,
            Self::EvaluateAccessCheck => PdsWorkflowBoundary::DirectAccess,
            Self::ListConsentGrants
            | Self::ListDataProducts
            | Self::ReadDataProduct
            | Self::ListProviderBindings
            | Self::ListAccessAuditEvents
            | Self::ListLifecycleEvents
            | Self::ListRetentionPolicies
            | Self::ListLifecycleCases
            | Self::ListGovernanceProfiles
            | Self::ReadRecordInventory
            | Self::ExportMetadata
            | Self::ExportPortability => PdsWorkflowBoundary::Query,
        }
    }

    pub(crate) const fn is_command_boundary(self) -> bool {
        matches!(self.boundary(), PdsWorkflowBoundary::Command)
    }

    pub(crate) const fn is_query_boundary(self) -> bool {
        matches!(self.boundary(), PdsWorkflowBoundary::Query)
    }
}

impl WorkflowKind for PdsWorkflowKind {
    fn application_kind(self) -> ApplicationWorkflowKind {
        self.application_kind()
    }

    fn workflow_spec(self) -> ApplicationWorkflowSpec {
        self.workflow_spec()
    }
}

/// Per-invocation PDS application workflow over an explicit capability set.
#[derive(Debug, Clone)]
pub struct PdsApplicationWorkflow<P, A>
where
    P: PdsApplicationPortSet,
    A: ApplicationRuntimePort,
{
    capabilities: PdsApplicationCapabilities<P, A>,
}

impl<P, A> PdsApplicationWorkflow<P, A>
where
    P: PdsApplicationPortSet,
    A: ApplicationRuntimePort,
{
    const fn new(capabilities: PdsApplicationCapabilities<P, A>) -> Self {
        Self { capabilities }
    }

    /// Returns the release-artifact availability port used by this workflow.
    pub fn release_artifacts(&self) -> &P::ReleaseArtifacts {
        self.capabilities.release_artifacts()
    }

    /// Returns the application runtime used by this workflow.
    pub const fn runtime(&self) -> &A {
        self.capabilities.runtime()
    }

    /// Returns the shared application operation gateway.
    pub const fn operation_gateway(&self) -> ApplicationOperationGateway<'_, A> {
        ApplicationOperationGateway::new(self.runtime())
    }
}

impl<P, A> PdsApplicationCapabilities<P, A>
where
    P: PdsApplicationPortSet,
    A: ApplicationRuntimePort,
{
    /// Begins a PDS application workflow using a cloned capability set.
    pub fn begin_workflow(&self) -> PdsApplicationWorkflow<P, A> {
        PdsApplicationWorkflow::new(self.clone())
    }
}

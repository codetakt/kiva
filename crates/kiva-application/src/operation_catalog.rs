//! Application operation catalog facade.

mod generated;

use crate::runtime::{ApplicationOperation, ApplicationWorkflowSpec};

pub use generated::operation_ids::{
    ApplicationOperationId, application_operation_spec_by_catalog_id,
    application_operation_spec_by_openapi_operation,
};
use generated::registry::APPLICATION_OPERATION_SPECS;
pub(crate) use generated::workflow_metadata::{
    analytics_workflow_spec, confidential_workflow_family, confidential_workflow_rust_kind,
    confidential_workflow_spec, pds_workflow_spec,
};

/// Public application surface represented by one operation catalog entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationOperationSurface {
    /// Governed aggregate analytics operation.
    Analytics,
    /// Personal Data Store application operation.
    Pds,
    /// Confidential-computation application operation.
    Confidential,
}

impl ApplicationOperationSurface {
    /// Returns the stable surface label used by the architecture catalog.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Analytics => "analytics",
            Self::Pds => "pds",
            Self::Confidential => "confidential",
        }
    }
}

/// One operation catalog entry linking Rust workflow, `OpenAPI`, and scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationOperationSpec {
    /// Stable operation catalog entry identifier.
    pub id: &'static str,
    /// Application surface that owns the operation.
    pub surface: ApplicationOperationSurface,
    /// Stable application workflow specification.
    pub workflow_spec: ApplicationWorkflowSpec,
    /// Rust workflow marker expected to own the operation.
    pub rust_workflow_kind: &'static str,
    /// Repository-relative `OpenAPI` contract path.
    pub openapi_contract: &'static str,
    /// `OpenAPI` operationId.
    pub operation_id: &'static str,
    /// Required OAuth/OIDC resource-server scopes.
    pub required_scopes: &'static [&'static str],
}

impl ApplicationOperationSpec {
    /// Returns the stable application workflow label.
    #[must_use]
    pub const fn workflow_label(self) -> &'static str {
        self.workflow_spec.label()
    }

    /// Returns the Rust workflow marker expected to own the operation.
    #[must_use]
    pub const fn rust_workflow_kind(self) -> &'static str {
        self.rust_workflow_kind
    }
}

/// Returns the generated application operation registry.
#[must_use]
pub const fn application_operation_specs() -> &'static [ApplicationOperationSpec] {
    APPLICATION_OPERATION_SPECS
}

impl ApplicationOperation for ApplicationOperationSpec {
    fn workflow_spec(self) -> ApplicationWorkflowSpec {
        self.workflow_spec
    }
}

impl ApplicationOperation for ApplicationOperationId {
    fn workflow_spec(self) -> ApplicationWorkflowSpec {
        self.spec().workflow_spec
    }
}

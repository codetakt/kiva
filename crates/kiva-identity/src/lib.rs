//! Authenticated resource-server identity boundary types for Kiva.

use std::{collections::BTreeSet, marker::PhantomData};

use serde::{Deserialize, Serialize};

/// OAuth/OIDC scope labels enforced at the Kiva resource-server boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OAuthScope {
    /// Read xAPI LRS resources.
    XapiRead,
    /// Write xAPI LRS resources.
    XapiWrite,
    /// Read PDS metadata and governed views.
    PdsRead,
    /// Write PDS metadata such as lifecycle records, products, and profiles.
    PdsWrite,
    /// Create PDS consent/delegation grants.
    PdsDelegate,
    /// Revoke PDS consent/delegation grants.
    PdsRevoke,
    /// Read PDS audit evidence.
    PdsAudit,
    /// Read confidential-computation metadata.
    ConfidentialRead,
    /// Write confidential-computation setup metadata.
    ConfidentialWrite,
    /// Run confidential-computation operations.
    ConfidentialCompute,
    /// Release confidential-computation outputs.
    ConfidentialRelease,
    /// Read governed aggregate analytics products.
    AnalyticsRead,
}

impl OAuthScope {
    /// Returns the wire label used in OAuth/OIDC `scope` claims.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::XapiRead => "xapi:read",
            Self::XapiWrite => "xapi:write",
            Self::PdsRead => "pds:read",
            Self::PdsWrite => "pds:write",
            Self::PdsDelegate => "pds:delegate",
            Self::PdsRevoke => "pds:revoke",
            Self::PdsAudit => "pds:audit",
            Self::ConfidentialRead => "confidential:read",
            Self::ConfidentialWrite => "confidential:write",
            Self::ConfidentialCompute => "confidential:compute",
            Self::ConfidentialRelease => "confidential:release",
            Self::AnalyticsRead => "analytics:read",
        }
    }
}

/// Protected operation classes enforced by the Kiva resource server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceServerOperation {
    /// Read xAPI LRS resources.
    XapiRead,
    /// Write xAPI LRS resources.
    XapiWrite,
    /// Read PDS metadata and governed views.
    PdsRead,
    /// Write PDS metadata such as lifecycle records, products, and profiles.
    PdsWrite,
    /// Create PDS consent/delegation grants.
    PdsDelegate,
    /// Revoke PDS consent/delegation grants.
    PdsRevoke,
    /// Read PDS audit evidence.
    PdsAudit,
    /// Read confidential-computation metadata.
    ConfidentialRead,
    /// Write confidential-computation setup metadata.
    ConfidentialWrite,
    /// Run confidential-computation operations.
    ConfidentialCompute,
    /// Release confidential-computation outputs.
    ConfidentialRelease,
    /// Read governed aggregate analytics products.
    AnalyticsRead,
}

impl ResourceServerOperation {
    /// Returns a stable operation label for diagnostics and evidence.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::XapiRead => "xapi:read",
            Self::XapiWrite => "xapi:write",
            Self::PdsRead => "pds:read",
            Self::PdsWrite => "pds:write",
            Self::PdsDelegate => "pds:delegate",
            Self::PdsRevoke => "pds:revoke",
            Self::PdsAudit => "pds:audit",
            Self::ConfidentialRead => "confidential:read",
            Self::ConfidentialWrite => "confidential:write",
            Self::ConfidentialCompute => "confidential:compute",
            Self::ConfidentialRelease => "confidential:release",
            Self::AnalyticsRead => "analytics:read",
        }
    }
}

/// Returns whether a bearer-token scope authorizes a Kiva operation.
///
/// The mapping is intentionally exact. Coarse inheritance such as write
/// implying read belongs in the authorization server's issued scope set, not in
/// Kiva's resource-server enforcement.
#[must_use]
pub const fn oauth_scope_allows_operation(
    scope: OAuthScope,
    operation: ResourceServerOperation,
) -> bool {
    matches!(
        (scope, operation),
        (OAuthScope::XapiRead, ResourceServerOperation::XapiRead)
            | (OAuthScope::XapiWrite, ResourceServerOperation::XapiWrite)
            | (OAuthScope::PdsRead, ResourceServerOperation::PdsRead)
            | (OAuthScope::PdsWrite, ResourceServerOperation::PdsWrite)
            | (
                OAuthScope::PdsDelegate,
                ResourceServerOperation::PdsDelegate
            )
            | (OAuthScope::PdsRevoke, ResourceServerOperation::PdsRevoke)
            | (OAuthScope::PdsAudit, ResourceServerOperation::PdsAudit)
            | (
                OAuthScope::ConfidentialRead,
                ResourceServerOperation::ConfidentialRead
            )
            | (
                OAuthScope::ConfidentialWrite,
                ResourceServerOperation::ConfidentialWrite
            )
            | (
                OAuthScope::ConfidentialCompute,
                ResourceServerOperation::ConfidentialCompute
            )
            | (
                OAuthScope::ConfidentialRelease,
                ResourceServerOperation::ConfidentialRelease
            )
            | (
                OAuthScope::AnalyticsRead,
                ResourceServerOperation::AnalyticsRead
            )
    )
}

/// Permission to write xAPI Statements after resource-server authentication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementWritePermission {
    /// Statement writes are allowed for this principal.
    Allowed,
    /// Statement writes are denied for this principal.
    Denied,
}

/// Authenticated OAuth/OIDC resource-server principal admitted by Kiva.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPrincipal {
    /// Stable authenticated subject identifier used by Kiva identity boundaries.
    pub subject: String,
    statement_write_permission: StatementWritePermission,
    scopes: BTreeSet<OAuthScope>,
}

impl AuthenticatedPrincipal {
    /// Creates an authenticated resource-server principal.
    #[must_use]
    pub fn new(
        subject: impl Into<String>,
        statement_write_permission: StatementWritePermission,
        scopes: BTreeSet<OAuthScope>,
    ) -> Self {
        Self {
            subject: subject.into(),
            statement_write_permission,
            scopes,
        }
    }

    /// Returns whether this principal may write xAPI Statements.
    #[must_use]
    pub const fn can_write_statements(&self) -> bool {
        matches!(
            self.statement_write_permission,
            StatementWritePermission::Allowed
        )
    }

    /// Returns whether this principal has a scope for the requested operation.
    #[must_use]
    pub fn allows_operation(&self, operation: ResourceServerOperation) -> bool {
        self.scopes
            .iter()
            .any(|scope| oauth_scope_allows_operation(*scope, operation))
    }

    /// Returns the accepted OAuth/OIDC resource-server scopes.
    #[must_use]
    pub const fn scopes(&self) -> &BTreeSet<OAuthScope> {
        &self.scopes
    }
}

/// Type-level resource-server operation admitted for an authenticated principal.
pub trait AuthorizedResourceOperation: Copy + Eq + 'static {
    /// The concrete operation authorized by this marker.
    const OPERATION: ResourceServerOperation;
}

macro_rules! authorized_operation_marker {
    ($name:ident, $operation:expr, $docs:literal) => {
        #[doc = $docs]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name;

        impl AuthorizedResourceOperation for $name {
            const OPERATION: ResourceServerOperation = $operation;
        }
    };
}

authorized_operation_marker!(
    AuthorizedXapiRead,
    ResourceServerOperation::XapiRead,
    "Authorized xAPI read operation marker."
);
authorized_operation_marker!(
    AuthorizedXapiWrite,
    ResourceServerOperation::XapiWrite,
    "Authorized xAPI write operation marker."
);
authorized_operation_marker!(
    AuthorizedPdsRead,
    ResourceServerOperation::PdsRead,
    "Authorized PDS read operation marker."
);
authorized_operation_marker!(
    AuthorizedPdsWrite,
    ResourceServerOperation::PdsWrite,
    "Authorized PDS write operation marker."
);
authorized_operation_marker!(
    AuthorizedPdsDelegate,
    ResourceServerOperation::PdsDelegate,
    "Authorized PDS delegation operation marker."
);
authorized_operation_marker!(
    AuthorizedPdsRevoke,
    ResourceServerOperation::PdsRevoke,
    "Authorized PDS revocation operation marker."
);
authorized_operation_marker!(
    AuthorizedPdsAudit,
    ResourceServerOperation::PdsAudit,
    "Authorized PDS audit operation marker."
);
authorized_operation_marker!(
    AuthorizedConfidentialRead,
    ResourceServerOperation::ConfidentialRead,
    "Authorized confidential-computation read operation marker."
);
authorized_operation_marker!(
    AuthorizedConfidentialWrite,
    ResourceServerOperation::ConfidentialWrite,
    "Authorized confidential-computation write operation marker."
);
authorized_operation_marker!(
    AuthorizedConfidentialCompute,
    ResourceServerOperation::ConfidentialCompute,
    "Authorized confidential-computation execution operation marker."
);
authorized_operation_marker!(
    AuthorizedConfidentialRelease,
    ResourceServerOperation::ConfidentialRelease,
    "Authorized confidential-computation release operation marker."
);
authorized_operation_marker!(
    AuthorizedAnalyticsRead,
    ResourceServerOperation::AnalyticsRead,
    "Authorized governed analytics read operation marker."
);

/// Authenticated principal paired with a compile-time resource operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedPrincipal<O>
where
    O: AuthorizedResourceOperation,
{
    principal: AuthenticatedPrincipal,
    operation: PhantomData<O>,
}

impl<O> AuthorizedPrincipal<O>
where
    O: AuthorizedResourceOperation,
{
    /// Creates a typed authorization witness when the principal allows the operation.
    #[must_use]
    pub fn new(principal: AuthenticatedPrincipal) -> Option<Self> {
        principal.allows_operation(O::OPERATION).then_some(Self {
            principal,
            operation: PhantomData,
        })
    }

    /// Returns the underlying authenticated principal.
    #[must_use]
    pub const fn as_principal(&self) -> &AuthenticatedPrincipal {
        &self.principal
    }

    /// Consumes the witness and returns the underlying authenticated principal.
    #[must_use]
    pub fn into_principal(self) -> AuthenticatedPrincipal {
        self.principal
    }
}

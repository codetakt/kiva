use std::collections::{BTreeSet, HashMap};

use kiva_identity::{AuthenticatedPrincipal, OAuthScope, StatementWritePermission};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

/// Fail-closed token verification and claim-mapping errors.
#[derive(Debug, Error)]
pub enum IdpAdapterError {
    /// JWT verification is not configured.
    #[error("JWT verification is not configured")]
    JwtDisabled,
    /// Introspection is not configured.
    #[error("token introspection is not configured")]
    IntrospectionDisabled,
    /// The token is malformed or cryptographically invalid.
    #[error("JWT verification failed")]
    InvalidJwt,
    /// No configured asymmetric key matches the token.
    #[error("JWT signing key is unavailable")]
    SigningKeyUnavailable,
    /// The token uses a forbidden or mismatched algorithm.
    #[error("JWT algorithm is not allowed")]
    AlgorithmNotAllowed,
    /// A JWKS endpoint or document could not be consumed.
    #[error("JWKS retrieval failed")]
    JwksUnavailable,
    /// An introspection endpoint could not be consumed.
    #[error("token introspection failed")]
    IntrospectionUnavailable,
    /// RFC 7662 reported an inactive token.
    #[error("token is inactive")]
    InactiveToken,
    /// A required claim is missing or has the wrong shape.
    #[error("required token claim is invalid")]
    InvalidClaims,
    /// Issuer or audience does not match deployment configuration.
    #[error("token issuer or audience is invalid")]
    IssuerAudienceMismatch,
    /// The token is expired or not active yet.
    #[error("token lifetime is invalid")]
    InvalidLifetime,
    /// A scope cannot be mapped to Kiva's exact scope vocabulary.
    #[error("token contains an unsupported scope")]
    UnsupportedScope,
    /// Deployment scope-mapping configuration would weaken Kiva authorization.
    #[error("identity-provider scope mapping configuration is invalid")]
    InvalidScopeConfiguration,
}

/// Deterministic mapping from `IdP` claim names into a Kiva principal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimMapping {
    tenant_claim: Option<String>,
    ignored_scopes: BTreeSet<String>,
}

impl ClaimMapping {
    /// Creates a mapping with an optional IdP-specific tenant claim name.
    #[must_use]
    pub const fn new(tenant_claim: Option<String>) -> Self {
        Self {
            tenant_claim,
            ignored_scopes: BTreeSet::new(),
        }
    }

    /// Creates a mapping with an explicit set of non-authorizing scope labels.
    ///
    /// Kiva authorization scope labels cannot be ignored because doing so would
    /// weaken the resource-server enforcement boundary.
    pub fn try_with_ignored_scopes(
        tenant_claim: Option<String>,
        ignored_scopes: impl IntoIterator<Item = String>,
    ) -> Result<Self, IdpAdapterError> {
        let ignored_scopes = ignored_scopes.into_iter().collect::<BTreeSet<_>>();
        if ignored_scopes
            .iter()
            .any(|label| label.is_empty() || parse_scope(label).is_ok())
        {
            return Err(IdpAdapterError::InvalidScopeConfiguration);
        }
        Ok(Self {
            tenant_claim,
            ignored_scopes,
        })
    }

    pub(super) fn principal(
        &self,
        claims: &VerifiedClaims,
    ) -> Result<AuthenticatedPrincipal, IdpAdapterError> {
        let subject = claims
            .subject
            .as_deref()
            .filter(|subject| !subject.is_empty())
            .ok_or(IdpAdapterError::InvalidClaims)?;
        let subject = match self.tenant_claim.as_deref() {
            Some(claim_name) => {
                let tenant = claims
                    .additional
                    .get(claim_name)
                    .and_then(Value::as_str)
                    .filter(|tenant| !tenant.is_empty())
                    .ok_or(IdpAdapterError::InvalidClaims)?;
                format!("{tenant}:{subject}")
            }
            None => subject.to_owned(),
        };
        let scopes = self.mapped_scopes(claims)?;
        let statement_write_permission = if scopes.contains(&OAuthScope::XapiWrite) {
            StatementWritePermission::Allowed
        } else {
            StatementWritePermission::Denied
        };
        Ok(AuthenticatedPrincipal::new(
            subject,
            statement_write_permission,
            scopes,
        ))
    }

    fn mapped_scopes(
        &self,
        claims: &VerifiedClaims,
    ) -> Result<BTreeSet<OAuthScope>, IdpAdapterError> {
        let mut labels = BTreeSet::new();
        for claim in [&claims.scope, &claims.scp].into_iter().flatten() {
            labels.extend(claim.values());
        }
        let mut scopes = BTreeSet::new();
        for label in labels {
            match parse_scope(label) {
                Ok(scope) => {
                    scopes.insert(scope);
                }
                Err(IdpAdapterError::UnsupportedScope) if self.ignored_scopes.contains(label) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(scopes)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum Audience {
    One(String),
    Many(Vec<String>),
}

impl Audience {
    pub(super) fn contains(&self, expected: &str) -> bool {
        match self {
            Self::One(audience) => audience == expected,
            Self::Many(audiences) => audiences.iter().any(|audience| audience == expected),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum ScopeClaim {
    Text(String),
    List(Vec<String>),
}

impl ScopeClaim {
    fn values(&self) -> Vec<&str> {
        match self {
            Self::Text(value) => value.split_ascii_whitespace().collect(),
            Self::List(values) => values.iter().map(String::as_str).collect(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct VerifiedClaims {
    #[serde(rename = "sub")]
    pub(super) subject: Option<String>,
    #[serde(rename = "iss")]
    pub(super) issuer: Option<String>,
    #[serde(rename = "aud")]
    pub(super) audience: Option<Audience>,
    pub(super) exp: Option<i64>,
    pub(super) nbf: Option<i64>,
    pub(super) scope: Option<ScopeClaim>,
    pub(super) scp: Option<ScopeClaim>,
    #[serde(flatten)]
    pub(super) additional: HashMap<String, Value>,
}

fn parse_scope(label: &str) -> Result<OAuthScope, IdpAdapterError> {
    match label {
        "xapi:read" => Ok(OAuthScope::XapiRead),
        "xapi:write" => Ok(OAuthScope::XapiWrite),
        "pds:read" => Ok(OAuthScope::PdsRead),
        "pds:write" => Ok(OAuthScope::PdsWrite),
        "pds:delegate" => Ok(OAuthScope::PdsDelegate),
        "pds:revoke" => Ok(OAuthScope::PdsRevoke),
        "pds:audit" => Ok(OAuthScope::PdsAudit),
        "confidential:read" => Ok(OAuthScope::ConfidentialRead),
        "confidential:write" => Ok(OAuthScope::ConfidentialWrite),
        "confidential:compute" => Ok(OAuthScope::ConfidentialCompute),
        "confidential:release" => Ok(OAuthScope::ConfidentialRelease),
        "analytics:read" => Ok(OAuthScope::AnalyticsRead),
        _ => Err(IdpAdapterError::UnsupportedScope),
    }
}

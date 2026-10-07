//! Authentication and resource-server authorization boundary.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, io,
    sync::Arc,
};

use axum::http::{HeaderMap, StatusCode, header::AUTHORIZATION};
use kiva_http_core::{BearerHeaderError, bearer_token, xapi_version_is_supported};
use kiva_identity::{
    AuthenticatedPrincipal, AuthorizedPrincipal as AuthorizedOperationPrincipal,
    AuthorizedResourceOperation, OAuthScope, ResourceServerOperation, StatementWritePermission,
};
use kiva_idp_adapter::BearerTokenVerifier;

use crate::core::{
    config::{
        KIVA_CONFORMANCE_INVALID_TOKEN_ENV, KIVA_CONFORMANCE_READER_TOKEN_ENV,
        KIVA_CONFORMANCE_WRITER_A_TOKEN_ENV, KIVA_REQUIRE_AUTHORIZATION_ENV, XAPI_VERSION_HEADER,
    },
    response::HttpError,
    runtime::{optional_env_bool, optional_env_string},
};

/// Authorizes an xAPI request and validates the xAPI version header.
pub(crate) async fn authorize_xapi_request(
    auth: &AuthConfig,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
    operation: ResourceServerOperation,
) -> Result<Option<AuthenticatedPrincipal>, HttpError> {
    let principal = authorize_credentials(auth, headers, query).await?;
    if auth.require_authorization && principal.is_none() {
        return Err(HttpError::new(
            StatusCode::UNAUTHORIZED,
            "Authorization header is required",
        ));
    }
    if let Some(principal) = principal.as_ref() {
        require_scope(principal, operation)?;
    }
    validate_xapi_version_header(headers)?;
    Ok(principal)
}

/// Authorizes an xAPI about request.
pub(crate) async fn authorize_about_request(
    auth: &AuthConfig,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
) -> Result<Option<AuthenticatedPrincipal>, HttpError> {
    authorize_credentials(auth, headers, query).await
}

/// Authorizes a confidential-computation request.
pub(crate) async fn authorize_confidential_request(
    auth: &AuthConfig,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
    operation: ResourceServerOperation,
) -> Result<AuthenticatedPrincipal, HttpError> {
    authorize_required_surface_request(
        auth,
        headers,
        query,
        operation,
        "Authorization header is required for confidential computation requests",
    )
    .await
}

/// Authorizes a PDS request.
pub(crate) async fn authorize_pds_request(
    auth: &AuthConfig,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
    operation: ResourceServerOperation,
) -> Result<AuthenticatedPrincipal, HttpError> {
    authorize_required_surface_request(
        auth,
        headers,
        query,
        operation,
        "Authorization header is required for PDS requests",
    )
    .await
}

/// Authorizes a governed analytics request.
pub(crate) async fn authorize_analytics_request(
    auth: &AuthConfig,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
    operation: ResourceServerOperation,
) -> Result<AuthenticatedPrincipal, HttpError> {
    authorize_required_surface_request(
        auth,
        headers,
        query,
        operation,
        "Authorization header is required for analytics requests",
    )
    .await
}

async fn authorize_required_surface_request(
    auth: &AuthConfig,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
    operation: ResourceServerOperation,
    missing_authorization_message: &'static str,
) -> Result<AuthenticatedPrincipal, HttpError> {
    let principal = authorize_credentials(auth, headers, query)
        .await?
        .ok_or_else(|| HttpError::new(StatusCode::UNAUTHORIZED, missing_authorization_message))?;
    require_scope(&principal, operation)?;
    Ok(principal)
}

pub(crate) async fn authorize_typed_pds_request<O>(
    auth: &AuthConfig,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
) -> Result<AuthorizedOperationPrincipal<O>, HttpError>
where
    O: AuthorizedResourceOperation,
{
    let principal = authorize_pds_request(auth, headers, query, O::OPERATION).await?;
    authorize_operation_principal(principal, "PDS")
}

pub(crate) async fn authorize_typed_analytics_request<O>(
    auth: &AuthConfig,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
) -> Result<AuthorizedOperationPrincipal<O>, HttpError>
where
    O: AuthorizedResourceOperation,
{
    let principal = authorize_analytics_request(auth, headers, query, O::OPERATION).await?;
    authorize_operation_principal(principal, "analytics")
}

pub(crate) async fn authorize_typed_confidential_request<O>(
    auth: &AuthConfig,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
) -> Result<AuthorizedOperationPrincipal<O>, HttpError>
where
    O: AuthorizedResourceOperation,
{
    let principal = authorize_confidential_request(auth, headers, query, O::OPERATION).await?;
    authorize_operation_principal(principal, "confidential computation")
}

pub(crate) async fn authorize_typed_xapi_request<O>(
    auth: &AuthConfig,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
) -> Result<Option<AuthorizedOperationPrincipal<O>>, HttpError>
where
    O: AuthorizedResourceOperation,
{
    let principal = authorize_xapi_request(auth, headers, query, O::OPERATION).await?;
    principal
        .map(|principal| authorize_operation_principal(principal, "xAPI"))
        .transpose()
}

fn authorize_operation_principal<O>(
    principal: AuthenticatedPrincipal,
    surface: &'static str,
) -> Result<AuthorizedOperationPrincipal<O>, HttpError>
where
    O: AuthorizedResourceOperation,
{
    AuthorizedOperationPrincipal::new(principal).ok_or_else(|| {
        HttpError::new(
            StatusCode::FORBIDDEN,
            format!("Bearer token does not authorize the requested {surface} operation"),
        )
    })
}

async fn authorize_credentials(
    auth: &AuthConfig,
    headers: &HeaderMap,
    query: &BTreeMap<String, String>,
) -> Result<Option<AuthenticatedPrincipal>, HttpError> {
    if query.contains_key("access_token") {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "OAuth bearer tokens must be sent in the Authorization header",
        ));
    }
    match validate_bearer_header(headers)? {
        Some(token) => auth.principal_for_bearer_token(&token).await.map(Some),
        None => Ok(None),
    }
}

/// Server authentication configuration.
#[derive(Clone)]
pub(crate) struct AuthConfig {
    pub(crate) require_authorization: bool,
    refused_bearer_tokens: BTreeSet<String>,
    statement_read_only_tokens: BTreeSet<String>,
    bearer_subjects: BTreeMap<String, String>,
    bearer_scopes: BTreeMap<String, BTreeSet<OAuthScope>>,
    token_verifier: Option<Arc<dyn BearerTokenVerifier>>,
}

impl fmt::Debug for AuthConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthConfig")
            .field("require_authorization", &self.require_authorization)
            .field("external_token_verifier", &self.token_verifier.is_some())
            .finish_non_exhaustive()
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        let mut config = Self {
            require_authorization: false,
            refused_bearer_tokens: BTreeSet::new(),
            statement_read_only_tokens: BTreeSet::new(),
            bearer_subjects: BTreeMap::new(),
            bearer_scopes: BTreeMap::new(),
            token_verifier: None,
        };

        for token in [
            "invalid-credential",
            "invalid_credential",
            "refused-credential",
            "refused",
        ] {
            config.register_refused_token(token);
        }
        for token in [
            "statement-reader",
            "statement_reader",
            "reader",
            "read-only",
            "readonly",
        ] {
            config.register_statement_reader_token(token);
            config.register_bearer_scopes(token, [OAuthScope::XapiRead]);
        }
        for token in ["statement-writer-a", "statement_writer_a"] {
            config.register_bearer_subject(token, "statement-writer-a");
            config.register_bearer_scopes(token, [OAuthScope::XapiRead, OAuthScope::XapiWrite]);
        }
        for token in ["statement-reader", "statement_reader"] {
            config.register_bearer_subject(token, "statement-reader");
        }
        for token in [
            "confidential-coordinator",
            "another-coordinator",
            "learner-a",
            "learner-b",
            "analyst-a",
            "public-admin",
            "another-public-admin",
            "policy-authority-left",
            "policy-authority-right",
            "policy-auditor",
            "school-data-custodian",
            "regional-platform-operator",
            "district-policy-authority",
            "district-analyst",
            "public-auditor",
            "identity-authority",
        ] {
            config.register_bearer_scopes(token, all_kiva_extension_fixture_scopes());
        }

        config
    }
}

impl AuthConfig {
    /// Builds the default authentication configuration with an explicit
    /// authorization requirement.
    #[cfg(test)]
    pub(crate) fn with_required_authorization(require_authorization: bool) -> Self {
        Self {
            require_authorization,
            ..Self::default()
        }
    }

    /// Builds authentication configuration from process environment variables.
    pub(crate) fn from_env(
        token_verifier: Option<Arc<dyn BearerTokenVerifier>>,
    ) -> Result<Self, io::Error> {
        let mut config = Self {
            require_authorization: optional_env_bool(KIVA_REQUIRE_AUTHORIZATION_ENV)?
                .unwrap_or(false),
            token_verifier,
            ..Self::default()
        };
        if let Some(token) = optional_env_string(KIVA_CONFORMANCE_WRITER_A_TOKEN_ENV)? {
            config.register_bearer_subject(&token, "statement-writer-a");
            config.register_bearer_scopes(&token, [OAuthScope::XapiRead, OAuthScope::XapiWrite]);
        }
        if let Some(token) = optional_env_string(KIVA_CONFORMANCE_READER_TOKEN_ENV)? {
            config.register_statement_reader_token(&token);
            config.register_bearer_subject(&token, "statement-reader");
            config.register_bearer_scopes(&token, [OAuthScope::XapiRead]);
        }
        if let Some(token) = optional_env_string(KIVA_CONFORMANCE_INVALID_TOKEN_ENV)? {
            config.register_refused_token(&token);
        }
        Ok(config)
    }

    /// Registers a bearer token that must be refused.
    pub(crate) fn register_refused_token(&mut self, token: &str) {
        self.refused_bearer_tokens.insert(token.to_owned());
    }

    /// Registers a bearer token that may read but not write xAPI Statements.
    pub(crate) fn register_statement_reader_token(&mut self, token: &str) {
        self.statement_read_only_tokens.insert(token.to_owned());
    }

    /// Registers a deterministic subject for an opaque fixture token.
    pub(crate) fn register_bearer_subject(&mut self, token: &str, subject: &str) {
        self.bearer_subjects
            .insert(token.to_owned(), subject.to_owned());
    }

    /// Registers OAuth scopes for an opaque fixture token.
    pub(crate) fn register_bearer_scopes(
        &mut self,
        token: &str,
        scopes: impl IntoIterator<Item = OAuthScope>,
    ) {
        self.bearer_scopes
            .entry(token.to_owned())
            .or_default()
            .extend(scopes);
    }

    /// Resolves a bearer token to an authenticated principal.
    pub(crate) async fn principal_for_bearer_token(
        &self,
        token: &str,
    ) -> Result<AuthenticatedPrincipal, HttpError> {
        if self.refused_bearer_tokens.contains(token) {
            return Err(HttpError::new(
                StatusCode::UNAUTHORIZED,
                "credentials were refused",
            ));
        }

        if let Some(verifier) = &self.token_verifier {
            return verifier.verify(token).await.map_err(|_error| {
                HttpError::new(StatusCode::UNAUTHORIZED, "credentials were refused")
            });
        }

        let statement_write_permission = if self.statement_read_only_tokens.contains(token) {
            StatementWritePermission::Denied
        } else {
            StatementWritePermission::Allowed
        };
        Ok(AuthenticatedPrincipal::new(
            self.subject_for_bearer_token(token),
            statement_write_permission,
            self.bearer_scopes.get(token).cloned().unwrap_or_default(),
        ))
    }

    #[cfg(test)]
    pub(crate) fn with_token_verifier(
        require_authorization: bool,
        verifier: Arc<dyn BearerTokenVerifier>,
    ) -> Self {
        Self {
            require_authorization,
            token_verifier: Some(verifier),
            ..Self::default()
        }
    }

    fn subject_for_bearer_token(&self, token: &str) -> String {
        self.bearer_subjects
            .get(token)
            .cloned()
            .unwrap_or_else(|| token.replace('_', "-"))
    }
}

fn require_scope(
    principal: &AuthenticatedPrincipal,
    operation: ResourceServerOperation,
) -> Result<(), HttpError> {
    if principal.allows_operation(operation) {
        Ok(())
    } else {
        Err(HttpError::new(
            StatusCode::FORBIDDEN,
            format!(
                "OAuth scope {} is required for this request",
                operation.label()
            ),
        ))
    }
}

const fn all_kiva_extension_fixture_scopes() -> [OAuthScope; 10] {
    [
        OAuthScope::PdsRead,
        OAuthScope::PdsWrite,
        OAuthScope::PdsDelegate,
        OAuthScope::PdsRevoke,
        OAuthScope::PdsAudit,
        OAuthScope::ConfidentialRead,
        OAuthScope::ConfidentialWrite,
        OAuthScope::ConfidentialCompute,
        OAuthScope::ConfidentialRelease,
        OAuthScope::AnalyticsRead,
    ]
}

fn validate_xapi_version_header(headers: &HeaderMap) -> Result<(), HttpError> {
    let Some(value) = headers.get(XAPI_VERSION_HEADER) else {
        return Ok(());
    };
    let value = value.to_str().map_err(|_error| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            "X-Experience-API-Version must be valid ASCII",
        )
    })?;
    if xapi_version_is_supported(value) {
        Ok(())
    } else {
        Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "X-Experience-API-Version must be 2.0 or 2.0.0",
        ))
    }
}

fn validate_bearer_header(headers: &HeaderMap) -> Result<Option<String>, HttpError> {
    let Some(value) = headers.get(AUTHORIZATION) else {
        return Ok(None);
    };
    let value = value.to_str().map_err(|_error| {
        HttpError::new(
            StatusCode::UNAUTHORIZED,
            "Authorization header must be valid ASCII",
        )
    })?;
    bearer_token(Some(value))
        .map(|token| token.map(str::to_owned))
        .map_err(|error| match error {
            BearerHeaderError::UnsupportedScheme => HttpError::new(
                StatusCode::UNAUTHORIZED,
                "Authorization header must use Bearer credentials",
            ),
            BearerHeaderError::EmptyToken => {
                HttpError::new(StatusCode::UNAUTHORIZED, "Bearer token must not be empty")
            }
        })
}

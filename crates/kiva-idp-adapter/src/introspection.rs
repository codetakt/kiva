use kiva_identity::AuthenticatedPrincipal;
use reqwest::{Client, Url};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{ClaimMapping, IdpAdapterError, claims::VerifiedClaims};

/// RFC 7662 introspection settings for one trusted authorization server.
#[derive(Debug, Clone)]
pub struct IntrospectionConfig {
    /// Introspection endpoint URL.
    pub endpoint: Url,
    /// Client identifier used for endpoint authentication.
    pub client_id: String,
    /// Client secret used for endpoint authentication.
    pub client_secret: String,
    /// Required issuer in the introspection response.
    pub issuer: String,
    /// Required audience in the introspection response.
    pub audience: String,
    /// IdP-specific claim mapping.
    pub claim_mapping: ClaimMapping,
}

/// Verifies opaque bearer tokens with RFC 7662 introspection.
#[derive(Debug, Clone)]
pub struct IntrospectionTokenVerifier {
    config: IntrospectionConfig,
    client: Client,
}

impl IntrospectionTokenVerifier {
    /// Creates an opaque-token verifier.
    #[must_use]
    pub fn new(config: IntrospectionConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub(crate) async fn verify_token(
        &self,
        token: &str,
    ) -> Result<AuthenticatedPrincipal, IdpAdapterError> {
        let response = self
            .client
            .post(self.config.endpoint.clone())
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&[("token", token)])
            .send()
            .await
            .map_err(|_error| IdpAdapterError::IntrospectionUnavailable)?
            .error_for_status()
            .map_err(|_error| IdpAdapterError::IntrospectionUnavailable)?
            .json::<IntrospectionResponse>()
            .await
            .map_err(|_error| IdpAdapterError::IntrospectionUnavailable)?;
        if !response.active {
            return Err(IdpAdapterError::InactiveToken);
        }
        validate_claims(&response.claims, &self.config)?;
        self.config.claim_mapping.principal(&response.claims)
    }
}

#[derive(Debug, Deserialize)]
struct IntrospectionResponse {
    active: bool,
    #[serde(flatten)]
    claims: VerifiedClaims,
}

fn validate_claims(
    claims: &VerifiedClaims,
    config: &IntrospectionConfig,
) -> Result<(), IdpAdapterError> {
    if claims.issuer.as_deref() != Some(config.issuer.as_str())
        || !claims
            .audience
            .as_ref()
            .is_some_and(|audience| audience.contains(&config.audience))
    {
        return Err(IdpAdapterError::IssuerAudienceMismatch);
    }
    let now = OffsetDateTime::now_utc().unix_timestamp();
    if claims.exp.is_none_or(|expiration| expiration <= now)
        || claims.nbf.is_some_and(|not_before| not_before > now)
    {
        return Err(IdpAdapterError::InvalidLifetime);
    }
    Ok(())
}

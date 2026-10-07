use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use reqwest::{Client, Url};
use serde::Deserialize;

use crate::{ClaimMapping, IdpAdapterError, claims::VerifiedClaims};
use kiva_identity::AuthenticatedPrincipal;

/// JSON Web Key Set containing asymmetric RSA verification keys.
#[derive(Debug, Clone, Deserialize)]
pub struct JwkSet {
    keys: Vec<RsaJwk>,
}

#[derive(Debug, Clone, Deserialize)]
struct RsaJwk {
    kid: String,
    kty: String,
    #[serde(default)]
    alg: Option<String>,
    n: String,
    e: String,
}

/// Source for configured JSON Web Keys.
#[derive(Debug, Clone)]
pub enum JwksSource {
    /// Deployment-provided local key set.
    Static(JwkSet),
    /// HTTPS or fixture HTTP JWKS endpoint.
    Remote(Url),
}

/// JWT validation settings for one trusted issuer and audience.
#[derive(Debug, Clone)]
pub struct JwtVerifierConfig {
    /// Expected token issuer.
    pub issuer: String,
    /// Expected resource-server audience.
    pub audience: String,
    /// JWKS source used for each verification.
    pub jwks: JwksSource,
    /// IdP-specific claim mapping.
    pub claim_mapping: ClaimMapping,
}

/// Verifies signed JWT bearer tokens against an asymmetric JWKS.
#[derive(Debug, Clone)]
pub struct JwtTokenVerifier {
    config: JwtVerifierConfig,
    client: Client,
}

impl JwtTokenVerifier {
    /// Creates a JWT verifier.
    #[must_use]
    pub fn new(config: JwtVerifierConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub(crate) async fn verify_token(
        &self,
        token: &str,
    ) -> Result<AuthenticatedPrincipal, IdpAdapterError> {
        let header = decode_header(token).map_err(|_error| IdpAdapterError::InvalidJwt)?;
        let algorithm = allowed_algorithm(header.alg)?;
        let kid = header
            .kid
            .as_deref()
            .filter(|kid| !kid.is_empty())
            .ok_or(IdpAdapterError::SigningKeyUnavailable)?;
        let jwks = self.jwks().await?;
        let jwk = jwks
            .keys
            .iter()
            .find(|key| key.kid == kid && key.kty == "RSA")
            .ok_or(IdpAdapterError::SigningKeyUnavailable)?;
        if jwk
            .alg
            .as_deref()
            .is_some_and(|configured| configured != algorithm_name(algorithm))
        {
            return Err(IdpAdapterError::AlgorithmNotAllowed);
        }
        let key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
            .map_err(|_error| IdpAdapterError::SigningKeyUnavailable)?;
        let mut validation = Validation::new(algorithm);
        validation.set_issuer(&[self.config.issuer.as_str()]);
        validation.set_audience(&[self.config.audience.as_str()]);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.required_spec_claims = ["exp", "iss", "aud", "sub"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let claims = decode::<VerifiedClaims>(token, &key, &validation)
            .map_err(|_error| IdpAdapterError::InvalidJwt)?
            .claims;
        self.config.claim_mapping.principal(&claims)
    }

    async fn jwks(&self) -> Result<JwkSet, IdpAdapterError> {
        match &self.config.jwks {
            JwksSource::Static(jwks) => Ok(jwks.clone()),
            JwksSource::Remote(url) => self
                .client
                .get(url.clone())
                .send()
                .await
                .map_err(|_error| IdpAdapterError::JwksUnavailable)?
                .error_for_status()
                .map_err(|_error| IdpAdapterError::JwksUnavailable)?
                .json::<JwkSet>()
                .await
                .map_err(|_error| IdpAdapterError::JwksUnavailable),
        }
    }
}

const fn allowed_algorithm(algorithm: Algorithm) -> Result<Algorithm, IdpAdapterError> {
    match algorithm {
        Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => Ok(algorithm),
        _ => Err(IdpAdapterError::AlgorithmNotAllowed),
    }
}

const fn algorithm_name(algorithm: Algorithm) -> &'static str {
    match algorithm {
        Algorithm::RS256 => "RS256",
        Algorithm::RS384 => "RS384",
        Algorithm::RS512 => "RS512",
        _ => "forbidden",
    }
}

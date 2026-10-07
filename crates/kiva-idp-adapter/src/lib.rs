//! OAuth/OIDC identity-provider integration outside Kiva's resource-server core.

#![allow(clippy::redundant_pub_crate)]

mod claims;
mod introspection;
mod jwks;

#[cfg(test)]
mod tests;

use std::{future::Future, pin::Pin};

pub use claims::{ClaimMapping, IdpAdapterError};
pub use introspection::{IntrospectionConfig, IntrospectionTokenVerifier};
pub use jwks::{JwkSet, JwksSource, JwtTokenVerifier, JwtVerifierConfig};
use kiva_identity::AuthenticatedPrincipal;

/// Asynchronous result returned by a bearer-token verifier.
pub type VerificationFuture<'a> =
    Pin<Box<dyn Future<Output = Result<AuthenticatedPrincipal, IdpAdapterError>> + Send + 'a>>;

/// Converts an untrusted bearer token into a verified Kiva principal.
pub trait BearerTokenVerifier: Send + Sync {
    /// Verifies one bearer token without granting permissions on ambiguous input.
    fn verify<'a>(&'a self, token: &'a str) -> VerificationFuture<'a>;
}

/// Selects JWT verification or RFC 7662 introspection from the token shape.
#[derive(Debug, Clone)]
pub struct IdpTokenVerifier {
    jwt: Option<JwtTokenVerifier>,
    introspection: Option<IntrospectionTokenVerifier>,
}

impl IdpTokenVerifier {
    /// Creates a verifier from one or both supported token verification modes.
    #[must_use]
    pub const fn new(
        jwt: Option<JwtTokenVerifier>,
        introspection: Option<IntrospectionTokenVerifier>,
    ) -> Self {
        Self { jwt, introspection }
    }
}

impl BearerTokenVerifier for IdpTokenVerifier {
    fn verify<'a>(&'a self, token: &'a str) -> VerificationFuture<'a> {
        Box::pin(async move {
            if token.split('.').count() == 3 {
                let verifier = self.jwt.as_ref().ok_or(IdpAdapterError::JwtDisabled)?;
                verifier.verify_token(token).await
            } else {
                let verifier = self
                    .introspection
                    .as_ref()
                    .ok_or(IdpAdapterError::IntrospectionDisabled)?;
                verifier.verify_token(token).await
            }
        })
    }
}

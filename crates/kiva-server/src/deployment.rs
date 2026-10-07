//! Deployment-edge composition for external identity providers.

use std::{io, sync::Arc};

use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};

use kiva_idp_adapter::{
    BearerTokenVerifier, ClaimMapping, IdpTokenVerifier, IntrospectionConfig,
    IntrospectionTokenVerifier, JwkSet, JwksSource, JwtTokenVerifier, JwtVerifierConfig,
};

use crate::core::{
    config::{
        KIVA_ADL_COMPAT_BASIC_AUTH_ENV, KIVA_IDP_AUDIENCE_ENV, KIVA_IDP_CLIENT_ID_ENV,
        KIVA_IDP_CLIENT_SECRET_ENV, KIVA_IDP_IGNORED_SCOPES_ENV, KIVA_IDP_INTROSPECTION_URL_ENV,
        KIVA_IDP_ISSUER_ENV, KIVA_IDP_JWKS_FILE_ENV, KIVA_IDP_JWKS_URL_ENV,
        KIVA_IDP_TENANT_CLAIM_ENV,
    },
    runtime::{optional_env_bool, optional_env_string},
};

const ADL_BASIC_AUTHORIZATION: &str = "Basic dW5kZWZpbmVkOnVuZGVmaW5lZA==";
const ADL_FIXTURE_BEARER_AUTHORIZATION: &str = "Bearer statement-writer-a";

pub(crate) fn adl_basic_auth_compatibility_from_env() -> Result<bool, io::Error> {
    Ok(optional_env_bool(KIVA_ADL_COMPAT_BASIC_AUTH_ENV)?.unwrap_or(false))
}

pub(crate) async fn translate_adl_basic_auth(mut request: Request, next: Next) -> Response {
    if is_xapi_path(request.uri().path()) {
        translate_adl_basic_auth_headers(request.headers_mut());
    }
    next.run(request).await
}

#[cfg(test)]
pub(crate) fn translate_adl_basic_auth_for_path(path: &str, headers: &mut HeaderMap) {
    if !is_xapi_path(path) {
        return;
    }
    translate_adl_basic_auth_headers(headers);
}

fn translate_adl_basic_auth_headers(headers: &mut HeaderMap) {
    if headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        == Some(ADL_BASIC_AUTHORIZATION)
    {
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static(ADL_FIXTURE_BEARER_AUTHORIZATION),
        );
    }
}

fn is_xapi_path(path: &str) -> bool {
    matches!(
        path,
        "/about"
            | "/statements"
            | "/activities"
            | "/agents"
            | "/activities/state"
            | "/activities/profile"
            | "/agents/profile"
    )
}

pub(crate) fn idp_token_verifier_from_env()
-> Result<Option<Arc<dyn BearerTokenVerifier>>, io::Error> {
    let jwks_url = optional_env_string(KIVA_IDP_JWKS_URL_ENV)?;
    let jwks_file = optional_env_string(KIVA_IDP_JWKS_FILE_ENV)?;
    let introspection_url = optional_env_string(KIVA_IDP_INTROSPECTION_URL_ENV)?;
    let tenant_claim = optional_env_string(KIVA_IDP_TENANT_CLAIM_ENV)?;
    let ignored_scopes = optional_env_string(KIVA_IDP_IGNORED_SCOPES_ENV)?;
    let claim_mapping = claim_mapping_from_configuration(tenant_claim, ignored_scopes.as_deref())?;
    if jwks_url.is_none() && jwks_file.is_none() && introspection_url.is_none() {
        return Ok(None);
    }
    if jwks_url.is_some() && jwks_file.is_some() {
        return Err(invalid_idp_config(
            "configure only one of KIVA_IDP_JWKS_URL or KIVA_IDP_JWKS_FILE",
        ));
    }

    let issuer = required_idp_env(KIVA_IDP_ISSUER_ENV)?;
    let audience = required_idp_env(KIVA_IDP_AUDIENCE_ENV)?;
    let jwks = match (jwks_url, jwks_file) {
        (Some(url), None) => Some(JwksSource::Remote(parse_url(&url, KIVA_IDP_JWKS_URL_ENV)?)),
        (None, Some(path)) => Some(JwksSource::Static(jwks_from_file(&path)?)),
        (None, None) => None,
        (Some(_), Some(_)) => {
            return Err(invalid_idp_config(
                "configure only one of KIVA_IDP_JWKS_URL or KIVA_IDP_JWKS_FILE",
            ));
        }
    };
    let jwt = jwks.map(|jwks| {
        JwtTokenVerifier::new(JwtVerifierConfig {
            issuer: issuer.clone(),
            audience: audience.clone(),
            jwks,
            claim_mapping: claim_mapping.clone(),
        })
    });
    let introspection = introspection_url
        .map(|endpoint| {
            Ok::<IntrospectionTokenVerifier, io::Error>(IntrospectionTokenVerifier::new(
                IntrospectionConfig {
                    endpoint: parse_url(&endpoint, KIVA_IDP_INTROSPECTION_URL_ENV)?,
                    client_id: required_idp_env(KIVA_IDP_CLIENT_ID_ENV)?,
                    client_secret: required_idp_env(KIVA_IDP_CLIENT_SECRET_ENV)?,
                    issuer: issuer.clone(),
                    audience: audience.clone(),
                    claim_mapping: claim_mapping.clone(),
                },
            ))
        })
        .transpose()?;
    Ok(Some(Arc::new(IdpTokenVerifier::new(jwt, introspection))))
}

fn claim_mapping_from_configuration(
    tenant_claim: Option<String>,
    ignored_scopes: Option<&str>,
) -> Result<ClaimMapping, io::Error> {
    let ignored_scopes = ignored_scopes
        .into_iter()
        .flat_map(str::split_ascii_whitespace)
        .map(str::to_owned);
    ClaimMapping::try_with_ignored_scopes(tenant_claim, ignored_scopes).map_err(|_error| {
        invalid_idp_config("KIVA_IDP_IGNORED_SCOPES must not contain Kiva authorization scopes")
    })
}

fn jwks_from_file(path: &str) -> Result<JwkSet, io::Error> {
    let body = std::fs::read(path)?;
    serde_json::from_slice(&body)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

fn parse_url(value: &str, name: &'static str) -> Result<url::Url, io::Error> {
    value
        .parse()
        .map_err(|_error| invalid_idp_config(&format!("{name} must be a URL")))
}

fn required_idp_env(name: &'static str) -> Result<String, io::Error> {
    optional_env_string(name)?.ok_or_else(|| invalid_idp_config(&format!("{name} is required")))
}

fn invalid_idp_config(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::claim_mapping_from_configuration;

    #[test]
    fn startup_configuration_rejects_ignored_kiva_scope() {
        let result = claim_mapping_from_configuration(None, Some("openid xapi:read profile"));
        assert!(result.is_err_and(|error| error.kind() == io::ErrorKind::InvalidInput));
    }
}

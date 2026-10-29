//! Locally configured PKIX trust and authenticated out-of-band status for x5c.
//!
//! Received certificates never supply their own trust or revocation evidence.
//! No AIA, CRL, OCSP, or other network retrieval is performed by this provider.

use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    sync::Arc,
};

use axum::http::StatusCode;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use x509_parser::prelude::{FromDer, X509Certificate};

use super::response::{HttpError, hex_lower};

mod der;
mod provider;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) use tests::name_constraints::integration_cases as name_constraint_cases;

const POLICY_ENV: &str = "KIVA_XAPI_CERTIFICATE_POLICY";

#[cfg(test)]
std::thread_local! {
    static JWS_OBSERVATIONS: std::cell::RefCell<Vec<serde_json::Value>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
pub(crate) fn take_jws_observations() -> Vec<serde_json::Value> {
    JWS_OBSERVATIONS.with(|values| std::mem::take(&mut *values.borrow_mut()))
}

#[cfg(test)]
fn observe_jws(
    kid: Option<&str>,
    message: &[u8],
    signature: &str,
    algorithm: jsonwebtoken::Algorithm,
    outcome: &str,
) {
    JWS_OBSERVATIONS.with(|values| values.borrow_mut().push(serde_json::json!({"kid":kid,"message_sha256":hex_lower(&Sha256::digest(message)),"signature_sha256":hex_lower(&Sha256::digest(signature.as_bytes())),"algorithm":format!("{algorithm:?}"),"outcome":outcome})));
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SignaturePolicy {
    config: Option<Arc<CertificatePolicy>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CertificatePolicy {
    trust_anchors_der: Vec<String>,
    intended_service: IntendedService,
    allowed_extended_key_usages: Vec<String>,
    initial_policy_set: Vec<String>,
    #[serde(default)]
    explicit_policy: bool,
    #[serde(default)]
    inhibit_any_policy: bool,
    #[serde(default)]
    inhibit_policy_mapping: bool,
    #[serde(default)]
    validation_time: Option<String>,
    out_of_band_status: Vec<CertificateStatus>,
    /// Independently trusted RSA SPKIs, selected by exact decoded JOSE kid.
    #[serde(default)]
    trusted_jws_keys: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum IntendedService {
    Integrity,
    NonRepudiation,
    IntegrityOrNonRepudiation,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CertificateStatus {
    certificate_sha256: String,
    issuer_spki_sha256: String,
    authenticated_source: String,
    valid_from: String,
    valid_until: String,
    status: RevocationStatus,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum RevocationStatus {
    Good,
    Revoked,
    Unknown,
}

fn invalid(message: impl Into<String>) -> HttpError {
    HttpError::new(StatusCode::BAD_REQUEST, message)
}

fn provider_error(error: impl std::fmt::Display) -> HttpError {
    invalid(format!("x5c certificate validation failed: {error}"))
}

fn timestamp(value: &str) -> Result<OffsetDateTime, HttpError> {
    OffsetDateTime::parse(value, &Rfc3339).map_err(provider_error)
}

impl SignaturePolicy {
    pub(crate) fn from_env() -> Result<Self, io::Error> {
        let path = match std::env::var(POLICY_ENV) {
            Ok(path) if !path.is_empty() => path,
            Err(std::env::VarError::NotPresent) => return Ok(Self::default()),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "invalid certificate policy path",
                ));
            }
        };
        let encoded = std::fs::read(path)?;
        Self::from_json(&encoded)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.message))
    }

    pub(crate) fn from_json(encoded: &[u8]) -> Result<Self, HttpError> {
        let config: CertificatePolicy = serde_json::from_slice(encoded).map_err(provider_error)?;
        if config.trust_anchors_der.is_empty() && config.trusted_jws_keys.is_empty() {
            return Err(invalid(
                "certificate policy requires explicit trust anchors",
            ));
        }
        if config.initial_policy_set.is_empty() {
            return Err(invalid(
                "certificate policy requires an explicit initial policy set",
            ));
        }
        for oid in config
            .initial_policy_set
            .iter()
            .chain(&config.allowed_extended_key_usages)
        {
            ::der::asn1::ObjectIdentifier::new(oid).map_err(provider_error)?;
        }
        for encoded in &config.trust_anchors_der {
            parse_certificate(encoded)?;
        }
        for (kid, key) in &config.trusted_jws_keys {
            if kid.is_empty() {
                return Err(invalid("configured JWS kid must not be empty"));
            }
            der::rsa_spki(&STANDARD.decode(key).map_err(provider_error)?)?;
        }
        if let Some(value) = &config.validation_time {
            timestamp(value)?;
        }
        let mut bindings = BTreeSet::new();
        for status in &config.out_of_band_status {
            if status.authenticated_source.trim().is_empty()
                || [&status.certificate_sha256, &status.issuer_spki_sha256]
                    .iter()
                    .any(|hash| {
                        hash.len() != 64
                            || !hash
                                .bytes()
                                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                    })
                || !bindings.insert((&status.certificate_sha256, &status.issuer_spki_sha256))
                || timestamp(&status.valid_from)? > timestamp(&status.valid_until)?
            {
                return Err(invalid(
                    "invalid or ambiguous out-of-band certificate status binding",
                ));
            }
        }
        Ok(Self {
            config: Some(Arc::new(config)),
        })
    }

    /// D-10=A: unknown key is not-evaluated; a known failed signature rejects.
    pub(crate) fn verify_without_certificate(
        &self,
        kid: Option<&str>,
        message: &[u8],
        signature: &str,
        algorithm: jsonwebtoken::Algorithm,
    ) -> Result<(), HttpError> {
        let configured = self
            .config
            .as_ref()
            .and_then(|config| kid.and_then(|kid| config.trusted_jws_keys.get(kid)));
        let Some(encoded) = configured else {
            #[cfg(test)]
            observe_jws(kid, message, signature, algorithm, "not-evaluated");
            tracing::debug!(
                crypto = "not-evaluated",
                "JWS has no independently configured key"
            );
            return Ok(());
        };
        let bytes = STANDARD.decode(encoded).map_err(provider_error)?;
        let key = der::rsa_spki(&bytes)?;
        let key = jsonwebtoken::DecodingKey::from_rsa_der(&key);
        let verification = jsonwebtoken::crypto::verify(signature, message, &key, algorithm);
        #[cfg(test)]
        observe_jws(
            kid,
            message,
            signature,
            algorithm,
            if matches!(verification, Ok(true)) {
                "verified"
            } else {
                "failed"
            },
        );
        if !verification.map_err(provider_error)? {
            return Err(invalid(
                "JWS signature does not match the independently configured key",
            ));
        }
        tracing::debug!(
            crypto = "verified",
            "JWS verified with the independently configured key"
        );
        Ok(())
    }

    pub(crate) fn validate_chain(&self, encoded: &[String]) -> Result<Vec<u8>, HttpError> {
        use ::der::Encode;
        let config = self.config.as_ref().ok_or_else(|| {
            invalid("x5c requires locally configured certificate trust and status evidence")
        })?;
        let certificates = encoded
            .iter()
            .map(|value| parse_certificate(value))
            .collect::<Result<Vec<_>, _>>()?;
        if certificates.is_empty() {
            return Err(invalid("x5c must not be empty"));
        }
        if certificates.iter().collect::<BTreeSet<_>>().len() != certificates.len() {
            return Err(invalid("x5c must not repeat a certificate"));
        }
        let now = config
            .validation_time
            .as_deref()
            .map_or_else(|| Ok(OffsetDateTime::now_utc()), timestamp)?;
        let anchors = config
            .trust_anchors_der
            .iter()
            .map(|encoded| parse_certificate(encoded))
            .collect::<Result<Vec<_>, _>>()?;
        let validated = provider::validate(&certificates, &anchors, config, now)?;
        let mut path = certificates;
        if path.len() > 1 && path.last() == anchors.get(validated.anchor_index) {
            path.pop();
        }
        path.push(anchors[validated.anchor_index].clone());
        validate_status(&path, config, now)?;
        der::rsa_spki(&validated.leaf_spki.to_der().map_err(provider_error)?)
    }
}

fn validate_status(
    path: &[Vec<u8>],
    config: &CertificatePolicy,
    now: OffsetDateTime,
) -> Result<(), HttpError> {
    // A directly trusted signer still needs signer validity, intended use
    // and explicit status; it never inherits a fabricated 'good' result.
    let path_length = path.len().saturating_sub(1).max(1);
    for (index, certificate) in path.iter().take(path_length).enumerate() {
        let der = certificate;
        let view = der::provider_view(der)?;
        let (_, parsed) = X509Certificate::from_der(&view).map_err(provider_error)?;
        if !der::valid_at(der, now)? {
            return Err(invalid("x5c certificate is outside its validity interval"));
        }
        let issuer = path.get(index + 1).unwrap_or(certificate);
        let issuer_view = der::provider_view(issuer)?;
        let (_, issuer_certificate) =
            X509Certificate::from_der(&issuer_view).map_err(provider_error)?;
        let issuer_key = issuer_certificate.public_key().raw;
        let certificate_hash = hex_lower(&Sha256::digest(der));
        let issuer_hash = hex_lower(&Sha256::digest(issuer_key));
        let status = config
            .out_of_band_status
            .iter()
            .find(|status| {
                status.certificate_sha256 == certificate_hash
                    && status.issuer_spki_sha256 == issuer_hash
            })
            .ok_or_else(|| {
                invalid("x5c revocation status is unavailable for the certificate and issuer")
            })?;
        if now < timestamp(&status.valid_from)? || now > timestamp(&status.valid_until)? {
            return Err(invalid(
                "x5c revocation status is indeterminate outside its authenticated interval",
            ));
        }
        match status.status {
            RevocationStatus::Good => {}
            RevocationStatus::Revoked => {
                return Err(invalid(
                    "x5c certificate is revoked by its authenticated status provider",
                ));
            }
            RevocationStatus::Unknown => {
                return Err(invalid(
                    "x5c revocation status is indeterminate according to its authenticated provider",
                ));
            }
        }
        if index == 0 {
            validate_intended_use(&parsed, config)?;
        }
    }
    Ok(())
}

fn parse_certificate(encoded: &str) -> Result<Vec<u8>, HttpError> {
    let bytes = STANDARD.decode(encoded).map_err(provider_error)?;
    let view = der::provider_view(&bytes)?;
    let (remaining, parsed) = X509Certificate::from_der(&view).map_err(provider_error)?;
    if !remaining.is_empty() {
        return Err(invalid("x5c certificate has trailing bytes"));
    }
    parsed.extensions_map().map_err(provider_error)?;
    der::certificate(&bytes, &parsed)?;
    Ok(bytes)
}

fn validate_intended_use(
    certificate: &X509Certificate<'_>,
    policy: &CertificatePolicy,
) -> Result<(), HttpError> {
    // Apply the exact JWS minimum to x5c leaf keys as well as separately
    // configured keys. CA/path algorithm policy remains a separate contract.
    der::rsa_spki(certificate.public_key().raw)?;
    if certificate.public_key().algorithm.algorithm.to_id_string() != "1.2.840.113549.1.1.1" {
        return Err(invalid(
            "RS256/RS384/RS512 require an unrestricted rsaEncryption signer key",
        ));
    }
    if let Some(usage) = certificate.key_usage().map_err(provider_error)? {
        let permitted = match policy.intended_service {
            IntendedService::Integrity => usage.value.digital_signature(),
            IntendedService::NonRepudiation => usage.value.non_repudiation(),
            IntendedService::IntegrityOrNonRepudiation => {
                usage.value.digital_signature() || usage.value.non_repudiation()
            }
        };
        if !permitted {
            return Err(invalid(
                "x5c signer keyUsage does not permit the configured JWS service",
            ));
        }
    }
    if let Some(usage) = certificate.extended_key_usage().map_err(provider_error)? {
        let usage = &usage.value;
        let mut purposes = usage
            .other
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        for (present, oid) in [
            (usage.server_auth, "1.3.6.1.5.5.7.3.1"),
            (usage.client_auth, "1.3.6.1.5.5.7.3.2"),
            (usage.code_signing, "1.3.6.1.5.5.7.3.3"),
            (usage.email_protection, "1.3.6.1.5.5.7.3.4"),
            (usage.time_stamping, "1.3.6.1.5.5.7.3.8"),
            (usage.ocsp_signing, "1.3.6.1.5.5.7.3.9"),
        ] {
            if present {
                purposes.push(oid.to_owned());
            }
        }
        if !usage.any
            && !purposes
                .iter()
                .any(|oid| policy.allowed_extended_key_usages.contains(oid))
        {
            return Err(invalid(
                "x5c signer extendedKeyUsage does not permit a configured JWS purpose",
            ));
        }
    }
    Ok(())
}

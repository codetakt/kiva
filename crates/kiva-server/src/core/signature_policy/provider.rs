//! Pure-Rust path processing and ring verification of original signed octets.

use ::der::{Decode, Encode};
use pkix_path::{SignatureVerifier, TrustAnchor, ValidatedPath, ValidationPolicy};
use ring::signature::{
    RSA_PKCS1_2048_8192_SHA256, RSA_PKCS1_2048_8192_SHA384, RSA_PKCS1_2048_8192_SHA512,
    UnparsedPublicKey,
};
use spki::{AlgorithmIdentifierRef, SubjectPublicKeyInfoRef};
use x509_cert::Certificate;
use x509_parser::prelude::{FromDer, X509Certificate};

use super::{CertificatePolicy, HttpError, OffsetDateTime, invalid, provider_error};

struct OriginalSignature {
    provider_tbs: Vec<u8>,
    original_tbs: Vec<u8>,
    signature: Vec<u8>,
}

struct OriginalVerifier(Vec<OriginalSignature>);

impl SignatureVerifier for OriginalVerifier {
    fn verify_signature(
        &self,
        algorithm: AlgorithmIdentifierRef<'_>,
        issuer_spki: SubjectPublicKeyInfoRef<'_>,
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), signature::Error> {
        let original = self
            .0
            .iter()
            .find(|record| record.provider_tbs == message && record.signature == signature)
            .ok_or_else(signature::Error::new)?;
        if issuer_spki.algorithm.oid.to_string() != "1.2.840.113549.1.1.1" {
            return Err(signature::Error::new());
        }
        let algorithm = match algorithm.oid.to_string().as_str() {
            "1.2.840.113549.1.1.11" => &RSA_PKCS1_2048_8192_SHA256,
            "1.2.840.113549.1.1.12" => &RSA_PKCS1_2048_8192_SHA384,
            "1.2.840.113549.1.1.13" => &RSA_PKCS1_2048_8192_SHA512,
            _ => return Err(signature::Error::new()),
        };
        let key = issuer_spki
            .subject_public_key
            .as_bytes()
            .ok_or_else(signature::Error::new)?;
        UnparsedPublicKey::new(algorithm, key)
            .verify(&original.original_tbs, signature)
            .map_err(|_| signature::Error::new())
    }
}

pub(super) fn validate(
    encoded: &[Vec<u8>],
    anchors: &[Vec<u8>],
    config: &CertificatePolicy,
    now: OffsetDateTime,
) -> Result<ValidatedPath, HttpError> {
    let mut chain = encoded
        .iter()
        .map(|bytes| {
            Certificate::from_der(&super::der::provider_view(bytes)?).map_err(provider_error)
        })
        .collect::<Result<Vec<_>, _>>()?;
    // The optional final certificate may transport an already trusted anchor.
    // It contributes no new trust and is excluded from the prospective path.
    if encoded.len() > 1 && encoded.last().is_some_and(|last| anchors.contains(last)) {
        chain.pop();
    }
    let trust = anchors
        .iter()
        .map(|bytes| {
            let certificate = Certificate::from_der(&super::der::provider_view(bytes)?)
                .map_err(provider_error)?;
            Ok(TrustAnchor::new(
                certificate.tbs_certificate.subject,
                certificate.tbs_certificate.subject_public_key_info,
            ))
        })
        .collect::<Result<Vec<_>, HttpError>>()?;
    let mut originals: Vec<OriginalSignature> = Vec::new();
    for (certificate, bytes) in chain.iter().zip(encoded) {
        let view = super::der::provider_view(bytes)?;
        let (_, parsed) = X509Certificate::from_der(&view).map_err(provider_error)?;
        if !super::der::valid_at(bytes, now)? {
            return Err(invalid("x5c certificate is outside its validity interval"));
        }
        let provider_tbs = certificate
            .tbs_certificate
            .to_der()
            .map_err(provider_error)?;
        let original_tbs = super::der::original_tbs(bytes)?;
        if originals.iter().any(|original| {
            original.provider_tbs == provider_tbs
                && original.signature == parsed.signature_value.data.as_ref()
                && original.original_tbs != original_tbs
        }) {
            return Err(invalid("x5c contains ambiguous certificate views"));
        }
        originals.push(OriginalSignature {
            provider_tbs,
            original_tbs: original_tbs.to_vec(),
            signature: parsed.signature_value.data.to_vec(),
        });
    }
    // Only the provider time view uses this epoch. The caller's actual (possibly
    // historical or fractional) instant has already been checked above.
    let mut policy = ValidationPolicy::new(946_684_800);
    policy.max_path_len = u8::MAX;
    policy.initial_explicit_policy = config.explicit_policy;
    policy.initial_any_policy_inhibit = config.inhibit_any_policy;
    policy.initial_policy_mapping_inhibit = config.inhibit_policy_mapping;
    policy.initial_policy_set = config
        .initial_policy_set
        .iter()
        .map(|oid| ::der::asn1::ObjectIdentifier::new(oid).map_err(provider_error))
        .collect::<Result<_, _>>()?;
    let result = pkix_path::validate_path(&chain, &trust, &policy, &OriginalVerifier(originals))
        .map_err(|error| invalid(format!("x5c certification path is not trusted: {error}")))?;
    tracing::debug!(anchor_index = result.anchor_index, policy_tree = ?result.valid_policy_tree, "validated x5c path");
    Ok(result)
}

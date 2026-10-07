#![allow(clippy::wildcard_imports)]

use super::*;

/// Domain-separated digest for one PSI cohort identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "[u8; 32]", into = "[u8; 32]")]
pub struct PsiIdentifierDigest {
    pub(in crate::mechanism::core) bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES],
}

impl PsiIdentifierDigest {
    /// Creates a PSI identifier digest after rejecting the reserved empty value.
    pub fn new(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, SecureAggregateError> {
        if artifact_is_empty(&bytes) {
            return Err(SecureAggregateError::EmptyPsiIdentifierDigest);
        }
        Ok(Self { bytes })
    }

    /// Returns the canonical bytes for this digest.
    #[must_use]
    pub const fn bytes(self) -> [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
        self.bytes
    }
}

impl TryFrom<[u8; SECURE_AGGREGATE_ARTIFACT_BYTES]> for PsiIdentifierDigest {
    type Error = SecureAggregateError;

    fn try_from(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, Self::Error> {
        Self::new(bytes)
    }
}

impl From<PsiIdentifierDigest> for [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
    fn from(value: PsiIdentifierDigest) -> Self {
        value.bytes()
    }
}

pub(super) fn validate_psi_label(
    value: &str,
    empty_error: SecureAggregateError,
) -> Result<(), SecureAggregateError> {
    if value.is_empty() {
        return Err(empty_error);
    }
    if value.bytes().any(|byte| byte.is_ascii_control()) {
        return match empty_error {
            SecureAggregateError::PsiNamespaceEmpty => {
                Err(SecureAggregateError::PsiNamespaceContainsControl)
            }
            SecureAggregateError::PsiIdentifierEmpty => {
                Err(SecureAggregateError::PsiIdentifierContainsControl)
            }
            _ => Err(empty_error),
        };
    }
    Ok(())
}

/// Canonicalizes a bounded PSI cohort identifier before hashing.
///
/// The current CC4 profile is intentionally narrow: identifiers are trimmed,
/// ASCII-lowercased, and rejected if empty or if they contain ASCII control
/// characters. Deployments that require locale-specific identity matching must
/// canonicalize before using this helper and should keep the deployment rule in
/// the PSI job contract.
pub fn canonicalize_psi_cohort_identifier(
    identifier: &str,
) -> Result<String, SecureAggregateError> {
    let canonical = identifier.trim().to_ascii_lowercase();
    validate_psi_label(&canonical, SecureAggregateError::PsiIdentifierEmpty)?;
    Ok(canonical)
}

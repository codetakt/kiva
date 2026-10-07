#![allow(clippy::wildcard_imports)]

use super::*;

/// Opaque evaluated OPRF element used by the CC11 runtime boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "[u8; 32]", into = "[u8; 32]")]
pub struct PsiOprfElement {
    pub(in crate::mechanism::core) bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES],
}

impl PsiOprfElement {
    /// Creates an OPRF element after rejecting the reserved empty value.
    pub fn new(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, SecureAggregateError> {
        if artifact_is_empty(&bytes) {
            return Err(SecureAggregateError::EmptyPsiOprfElement);
        }
        Ok(Self { bytes })
    }

    /// Returns the opaque element bytes.
    #[must_use]
    pub const fn bytes(self) -> [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
        self.bytes
    }
}

impl TryFrom<[u8; SECURE_AGGREGATE_ARTIFACT_BYTES]> for PsiOprfElement {
    type Error = SecureAggregateError;

    fn try_from(bytes: [u8; SECURE_AGGREGATE_ARTIFACT_BYTES]) -> Result<Self, Self::Error> {
        Self::new(bytes)
    }
}

impl From<PsiOprfElement> for [u8; SECURE_AGGREGATE_ARTIFACT_BYTES] {
    fn from(value: PsiOprfElement) -> Self {
        value.bytes()
    }
}

/// Derives a deterministic fixture element for the CC11 DH-OPRF PSI boundary.
///
/// This helper is for reproducible vectors and boundary tests. Production OPRF
/// blinding, evaluation, and finalization are delegated to the isolated runtime
/// bound by `PsiOprfRuntimeBinding`.
pub fn derive_psi_oprf_fixture_element(
    job_id: SecureAggregateTranscriptId,
    namespace: &str,
    identifier: &str,
) -> Result<PsiOprfElement, SecureAggregateError> {
    let namespace = namespace.trim();
    validate_psi_label(namespace, SecureAggregateError::PsiNamespaceEmpty)?;
    let identifier = canonicalize_psi_cohort_identifier(identifier)?;

    let mut hasher = Sha256::new();
    hasher.update(PSI_OPRF_DH_VOPRF_ELEMENT_DOMAIN);
    update_len_prefixed(&mut hasher, PSI_OPRF_DH_VOPRF_PROFILE.as_bytes());
    update_len_prefixed(
        &mut hasher,
        PsiOprfSuite::Ristretto255Sha512Voprf.label().as_bytes(),
    );
    hasher.update(job_id.bytes());
    update_len_prefixed(&mut hasher, namespace.as_bytes());
    update_len_prefixed(&mut hasher, identifier.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; SECURE_AGGREGATE_ARTIFACT_BYTES];
    bytes.copy_from_slice(&digest);
    PsiOprfElement::new(bytes)
}

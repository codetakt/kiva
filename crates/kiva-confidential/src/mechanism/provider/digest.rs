//! Secret-compute provider binding digest derivation.

use sha2::{Digest, Sha256};

use super::binding::SecretComputeProviderBinding;
use crate::{
    mechanism::workflow::SECRET_COMPUTE_PROVIDER_BINDING_DOMAIN,
    primitive::SecureAggregateCommitment,
};

/// Derives a canonical digest for a provider binding.
#[must_use]
pub fn derive_secret_compute_provider_binding_digest(
    binding: SecretComputeProviderBinding,
) -> SecureAggregateCommitment {
    let mut digest = Sha256::new();
    digest.update(SECRET_COMPUTE_PROVIDER_BINDING_DOMAIN);
    update_len_prefixed(&mut digest, binding.profile().label().as_bytes());
    update_len_prefixed(&mut digest, binding.kind().label().as_bytes());
    update_len_prefixed(&mut digest, binding.capability().label().as_bytes());
    digest.update(binding.provider_id_digest().bytes());
    digest.update(binding.artifact_digest().bytes());
    match binding.parameter_set_digest() {
        Some(parameter_set_digest) => {
            digest.update([1]);
            digest.update(parameter_set_digest.bytes());
        }
        None => digest.update([0]),
    }
    match binding.review_evidence_digest() {
        Some(review_evidence_digest) => {
            digest.update([1]);
            digest.update(review_evidence_digest.bytes());
        }
        None => digest.update([0]),
    }
    digest.update([u8::from(binding.external_review_completed())]);
    digest.update([u8::from(binding.production_claim_requested())]);
    let bytes = digest.finalize().into();
    SecureAggregateCommitment { bytes }
}

fn update_len_prefixed(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u128).to_be_bytes());
    hasher.update(bytes);
}

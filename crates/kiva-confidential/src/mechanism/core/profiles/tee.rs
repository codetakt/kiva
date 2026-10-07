//! TEE attestation profile domains.

/// Profile identifier for Kiva's portable TEE attestation evidence gate.
pub const TEE_PORTABLE_ATTESTATION_PROFILE: &str = "kiva-tee-portable-attestation-v1";

/// Domain separator for portable TEE attestation evidence digests.
pub const TEE_ATTESTATION_EVIDENCE_DOMAIN: &[u8] = b"kiva-confidential:tee-attestation-evidence:v1";

/// Profile identifier for the CC12 review-gated AWS Nitro attestation boundary.
pub const TEE_AWS_NITRO_ATTESTATION_PROFILE: &str = "kiva-tee-aws-nitro-review-gated-v1";

/// Domain separator for verified AWS Nitro attestation document claim digests.
pub const TEE_AWS_NITRO_ATTESTATION_DOCUMENT_DOMAIN: &[u8] =
    b"kiva-confidential:tee-aws-nitro-attestation-document:v1";

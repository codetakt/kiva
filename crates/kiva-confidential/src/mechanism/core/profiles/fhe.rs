//! FHE feasibility and runtime profile domains.

/// Profile identifier for Kiva's narrow FHE sum feasibility boundary.
pub const FHE_FEASIBILITY_PROFILE: &str = "kiva-fhe-sum-bfv-feasibility-v1";

/// Domain separator for deterministic FHE feasibility ciphertext fixtures.
pub const FHE_FEASIBILITY_CIPHERTEXT_DOMAIN: &[u8] =
    b"kiva-confidential:fhe-sum-feasibility-ciphertext:v1";

/// Domain separator for deterministic FHE feasibility evaluated-output fixtures.
pub const FHE_FEASIBILITY_EVALUATION_DOMAIN: &[u8] =
    b"kiva-confidential:fhe-sum-feasibility-evaluation:v1";

/// Profile identifier for the CC14 review-gated FHE sum runtime boundary.
pub const FHE_RUNTIME_PROFILE: &str = "kiva-fhe-sum-runtime-review-gated-v1";

/// Domain separator for accepted FHE runtime ciphertext transcript metadata.
pub const FHE_RUNTIME_CIPHERTEXT_TRANSCRIPT_DOMAIN: &[u8] =
    b"kiva-confidential:fhe-runtime-ciphertext-transcript:v1";

/// Domain separator for accepted FHE runtime evaluation metadata.
pub const FHE_RUNTIME_EVALUATION_DOMAIN: &[u8] = b"kiva-confidential:fhe-runtime-evaluation:v1";

/// Polynomial modulus degree admitted by the current FHE feasibility profile.
pub const FHE_FEASIBILITY_POLYNOMIAL_MODULUS_DEGREE: u32 = 4_096;

/// Plaintext modulus used by the deterministic BFV-style feasibility profile.
pub const FHE_FEASIBILITY_PLAINTEXT_MODULUS: u64 = 1_048_583;

/// Ciphertext modulus bit budget documented for the feasibility profile.
pub const FHE_FEASIBILITY_CIPHERTEXT_MODULUS_BITS: u16 = 109;

/// Minimum security level documented for the feasibility profile.
pub const FHE_FEASIBILITY_SECURITY_LEVEL_BITS: u16 = 128;

/// Maximum participants admitted by the feasibility profile.
pub const FHE_FEASIBILITY_MAX_PARTICIPANTS: usize = 16;

/// Maximum plaintext value admitted for one participant in the feasibility profile.
pub const FHE_FEASIBILITY_MAX_PLAINTEXT_VALUE: u64 = 65_000;

/// Initial noise budget carried by deterministic ciphertext fixtures.
pub const FHE_FEASIBILITY_INITIAL_NOISE_BUDGET_BITS: u16 = 48;

/// Noise-budget cost assigned to one sum evaluation fixture.
pub const FHE_FEASIBILITY_SUM_NOISE_COST_BITS: u16 = 4;

//! Secret/confidential-compute provider profile domains.

pub use kiva_secret_provider_contract::{
    REFERENCE_PRIMITIVE_PROVIDER_PROFILE, SECRET_COMPUTE_PROVIDER_CONTRACT_PROFILE,
};

/// Domain separator for secret/confidential-compute provider bindings.
pub const SECRET_COMPUTE_PROVIDER_BINDING_DOMAIN: &[u8] =
    b"kiva-confidential:secret-compute-provider-binding:v1";

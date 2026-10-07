//! Secret-compute provider contract boundary.

mod admission;
mod binding;
mod contract;
mod digest;
mod pipeline;
mod runtime;

pub use crate::mechanism::workflow::SECRET_COMPUTE_PROVIDER_BINDING_DOMAIN;

pub use admission::{
    SecretComputeProviderAdmission, SecretComputeProviderEvidenceGate,
    SecretComputeProviderReleaseGate, admit_secret_compute_provider,
    validate_secret_compute_provider_for_mechanism,
};
pub use binding::{
    SecretComputeProviderBinding, SecretComputeProviderEvidence,
    SecretComputeProviderReviewEvidence, SecretComputeProviderUsageClaim,
    secret_compute_provider_kind_requires_parameter_set,
    secret_compute_required_capability_for_mechanism, validate_secret_compute_provider_binding,
};
pub use contract::{
    SecretComputeProviderCapabilityProfile, SecretComputeProviderContract,
    SecretComputeProviderContractProfile,
};
pub use digest::derive_secret_compute_provider_binding_digest;
pub use pipeline::{SecretComputeProviderContractPipeline, SecretComputeProviderPipelineAdmission};
pub use runtime::{
    SecretComputeProviderRuntimeBoundary, SecretComputeProviderRuntimeBoundaryClaim,
    SecretComputeProviderRuntimeClass, SecretComputeProviderRuntimeContract,
    admit_reference_primitive_runtime_boundary, admit_reviewed_production_runtime_boundary,
    admit_secret_compute_provider_runtime_boundary,
};

#[cfg(test)]
mod tests;

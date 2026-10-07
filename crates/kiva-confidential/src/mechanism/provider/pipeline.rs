//! Descriptor-to-runtime provider contract pipeline.

use super::{
    admission::SecretComputeProviderAdmission,
    binding::SecretComputeProviderBinding,
    contract::{
        SecretComputeProviderCapabilityProfile, SecretComputeProviderContract,
        SecretComputeProviderContractProfile,
    },
    runtime::SecretComputeProviderRuntimeBoundary,
};
use crate::{
    SecureAggregateError,
    mechanism::{
        registry::{ConfidentialMechanismDescriptor, descriptor_for_mechanism},
        workflow::ConfidentialMechanismKind,
    },
};
use kiva_secret_provider_contract::SecretComputeProviderCapability;

/// First-class provider pipeline from mechanism descriptor to runtime boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretComputeProviderContractPipeline {
    descriptor: ConfidentialMechanismDescriptor,
    provider_contract: SecretComputeProviderContract,
}

impl SecretComputeProviderContractPipeline {
    /// Creates a pipeline from a mechanism descriptor.
    #[must_use]
    pub const fn for_descriptor(descriptor: ConfidentialMechanismDescriptor) -> Self {
        Self {
            descriptor,
            provider_contract: SecretComputeProviderContract::for_mechanism(descriptor.kind()),
        }
    }

    /// Creates a pipeline for a concrete confidential mechanism.
    #[must_use]
    pub const fn for_mechanism(mechanism: ConfidentialMechanismKind) -> Self {
        Self::for_descriptor(descriptor_for_mechanism(mechanism))
    }

    /// Returns the source mechanism descriptor.
    #[must_use]
    pub const fn descriptor(self) -> ConfidentialMechanismDescriptor {
        self.descriptor
    }

    /// Returns the mechanism governed by this pipeline.
    #[must_use]
    pub const fn mechanism(self) -> ConfidentialMechanismKind {
        self.descriptor.kind()
    }

    /// Returns the provider admission contract derived from the descriptor.
    #[must_use]
    pub const fn provider_contract(self) -> SecretComputeProviderContract {
        self.provider_contract
    }

    /// Returns the required provider capability.
    #[must_use]
    pub const fn required_capability(self) -> SecretComputeProviderCapability {
        self.provider_contract.required_capability()
    }

    /// Returns whether production use requires completed external review.
    #[must_use]
    pub const fn production_provider_review_required(self) -> bool {
        self.provider_contract.production_provider_review_required()
    }

    /// Admits a binding and returns the normalized admission plus runtime boundary.
    pub fn admit_runtime_boundary(
        self,
        binding: SecretComputeProviderBinding,
    ) -> Result<SecretComputeProviderPipelineAdmission, SecureAggregateError> {
        let admission = self.provider_contract.admit(binding)?;
        let runtime_boundary = SecretComputeProviderRuntimeBoundary::new(admission)?;
        Ok(SecretComputeProviderPipelineAdmission {
            pipeline: self,
            admission,
            runtime_boundary,
        })
    }
}

impl SecretComputeProviderCapabilityProfile for SecretComputeProviderContractPipeline {
    fn required_capability(self) -> SecretComputeProviderCapability {
        self.required_capability()
    }

    fn production_provider_review_required(self) -> bool {
        self.production_provider_review_required()
    }
}

impl SecretComputeProviderContractProfile for SecretComputeProviderContractPipeline {
    fn mechanism(self) -> ConfidentialMechanismKind {
        self.mechanism()
    }

    fn provider_contract(self) -> SecretComputeProviderContract {
        self.provider_contract()
    }
}

/// Provider pipeline result after admission and runtime-boundary classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretComputeProviderPipelineAdmission {
    pipeline: SecretComputeProviderContractPipeline,
    admission: SecretComputeProviderAdmission,
    runtime_boundary: SecretComputeProviderRuntimeBoundary,
}

impl SecretComputeProviderPipelineAdmission {
    /// Returns the pipeline that produced this admission.
    #[must_use]
    pub const fn pipeline(self) -> SecretComputeProviderContractPipeline {
        self.pipeline
    }

    /// Returns the normalized provider admission.
    #[must_use]
    pub const fn admission(self) -> SecretComputeProviderAdmission {
        self.admission
    }

    /// Returns the admitted runtime boundary.
    #[must_use]
    pub const fn runtime_boundary(self) -> SecretComputeProviderRuntimeBoundary {
        self.runtime_boundary
    }
}

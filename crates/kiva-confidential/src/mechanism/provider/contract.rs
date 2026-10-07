//! Secret-compute provider contract object.

use super::{
    admission::{SecretComputeProviderAdmission, admit_secret_compute_provider},
    binding::SecretComputeProviderBinding,
};
use crate::{
    SecureAggregateError,
    mechanism::{
        registry::{ConfidentialMechanismDescriptor, descriptor_for_mechanism},
        workflow::ConfidentialMechanismKind,
    },
};
use kiva_secret_provider_contract::SecretComputeProviderCapability;

/// Mechanism or contract profile that fixes the required provider capability.
pub trait SecretComputeProviderCapabilityProfile: Copy {
    /// Returns the provider capability required by this profile.
    fn required_capability(self) -> SecretComputeProviderCapability;

    /// Returns whether production use requires completed external review evidence.
    fn production_provider_review_required(self) -> bool;
}

/// Mechanism or contract profile that can produce a provider admission contract.
pub trait SecretComputeProviderContractProfile: SecretComputeProviderCapabilityProfile {
    /// Returns the confidential mechanism governed by this profile.
    fn mechanism(self) -> ConfidentialMechanismKind;

    /// Returns the provider contract for this profile.
    fn provider_contract(self) -> SecretComputeProviderContract;
}

/// Stable contract boundary for admitting provider evidence for one mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretComputeProviderContract {
    mechanism: ConfidentialMechanismKind,
    required_capability: SecretComputeProviderCapability,
    production_provider_review_required: bool,
}

impl SecretComputeProviderContract {
    /// Creates the provider contract for a concrete confidential mechanism.
    #[must_use]
    pub const fn for_mechanism(mechanism: ConfidentialMechanismKind) -> Self {
        let descriptor = descriptor_for_mechanism(mechanism);
        Self {
            mechanism: descriptor.kind(),
            required_capability: descriptor.required_provider_capability(),
            production_provider_review_required: descriptor.production_provider_review_required(),
        }
    }

    /// Returns the mechanism governed by this provider contract.
    #[must_use]
    pub const fn mechanism(self) -> ConfidentialMechanismKind {
        self.mechanism
    }

    /// Returns the provider capability required for a non-reference binding.
    #[must_use]
    pub const fn required_capability(self) -> SecretComputeProviderCapability {
        self.required_capability
    }

    /// Returns whether production use requires external review evidence.
    #[must_use]
    pub const fn production_provider_review_required(self) -> bool {
        self.production_provider_review_required
    }

    /// Admits a metadata-only provider binding against this contract.
    pub fn admit(
        self,
        binding: SecretComputeProviderBinding,
    ) -> Result<SecretComputeProviderAdmission, SecureAggregateError> {
        admit_secret_compute_provider(binding, self.mechanism)
    }
}

impl SecretComputeProviderCapabilityProfile for ConfidentialMechanismKind {
    fn required_capability(self) -> SecretComputeProviderCapability {
        descriptor_for_mechanism(self).required_capability()
    }

    fn production_provider_review_required(self) -> bool {
        descriptor_for_mechanism(self).production_provider_review_required()
    }
}

impl SecretComputeProviderContractProfile for ConfidentialMechanismKind {
    fn mechanism(self) -> ConfidentialMechanismKind {
        descriptor_for_mechanism(self).kind()
    }

    fn provider_contract(self) -> SecretComputeProviderContract {
        descriptor_for_mechanism(self).provider_contract()
    }
}

impl SecretComputeProviderCapabilityProfile for ConfidentialMechanismDescriptor {
    fn required_capability(self) -> SecretComputeProviderCapability {
        self.required_provider_capability()
    }

    fn production_provider_review_required(self) -> bool {
        self.production_provider_review_required()
    }
}

impl SecretComputeProviderContractProfile for ConfidentialMechanismDescriptor {
    fn mechanism(self) -> ConfidentialMechanismKind {
        self.kind()
    }

    fn provider_contract(self) -> SecretComputeProviderContract {
        SecretComputeProviderContract::for_mechanism(self.kind())
    }
}

impl SecretComputeProviderCapabilityProfile for SecretComputeProviderContract {
    fn required_capability(self) -> SecretComputeProviderCapability {
        self.required_capability()
    }

    fn production_provider_review_required(self) -> bool {
        self.production_provider_review_required()
    }
}

impl SecretComputeProviderContractProfile for SecretComputeProviderContract {
    fn mechanism(self) -> ConfidentialMechanismKind {
        self.mechanism()
    }

    fn provider_contract(self) -> SecretComputeProviderContract {
        self
    }
}

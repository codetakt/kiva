//! Verification boundary markers for pure confidential mechanism kernels.
//!
//! The `mechanism::core` module keeps tightly coupled reference primitive
//! implementations together. This marker model names the subset treated as
//! pure kernels by the verification ledgers, separate from provider adapters,
//! HTTP surfaces, persistence, and external runtime evidence gates.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConfidentialKernelBoundary {
    AdditiveShareAdmissionAndRelease,
    DifferentialPrivacyBudgetRelease,
    ShamirVssReconstruction,
    PsiCohortAndOprfRelease,
    TeeAttestationGates,
    ThresholdFrostRuntime,
    FheFeasibilityAndRuntime,
    ConfidentialJobParticipantLifecycle,
    ProviderRuntimeBoundary,
}

impl ConfidentialKernelBoundary {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::AdditiveShareAdmissionAndRelease => "additive-share-admission-and-release",
            Self::DifferentialPrivacyBudgetRelease => "differential-privacy-budget-release",
            Self::ShamirVssReconstruction => "shamir-vss-reconstruction",
            Self::PsiCohortAndOprfRelease => "psi-cohort-and-oprf-release",
            Self::TeeAttestationGates => "tee-attestation-gates",
            Self::ThresholdFrostRuntime => "threshold-frost-runtime",
            Self::FheFeasibilityAndRuntime => "fhe-feasibility-and-runtime",
            Self::ConfidentialJobParticipantLifecycle => "confidential-job-participant-lifecycle",
            Self::ProviderRuntimeBoundary => "provider-runtime-boundary",
        }
    }
}

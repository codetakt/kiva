use super::*;
use crate::{
    mechanism::{registry::descriptor_for_mechanism, workflow::ConfidentialMechanismKind},
    primitive::SecureAggregateCommitment,
};
use kiva_secret_provider_contract::{
    ComputeMechanismProviderKind, ReferenceProviderKind, SECRET_COMPUTE_PROVIDER_KIND_REGISTRY,
    SecretComputeProviderCapability, SecretComputeProviderKind, SecretComputeProviderKindContract,
    SecretComputeProviderProfile, SecretCustodyProviderKind, registry_entry_for_provider_kind,
};

fn commitment(seed: u8) -> SecureAggregateCommitment {
    SecureAggregateCommitment::new([seed; 32]).expect("test commitment is non-empty")
}

fn assert_kind_contract<T: SecretComputeProviderKindContract>(
    typed_kind: T,
    shared_kind: SecretComputeProviderKind,
) {
    assert_eq!(typed_kind.as_secret_compute_provider_kind(), shared_kind);
    assert_eq!(typed_kind.label(), shared_kind.label());
}

fn provider_kinds() -> [SecretComputeProviderKind; 6] {
    [
        SecretComputeProviderKind::ReferencePrimitive,
        SecretComputeProviderKind::SecretCustody,
        SecretComputeProviderKind::TrustedExecutionEnvironment,
        SecretComputeProviderKind::FullyHomomorphicEncryption,
        SecretComputeProviderKind::PrivateSetIntersection,
        SecretComputeProviderKind::ThresholdCryptography,
    ]
}

#[test]
fn contract_admits_reference_primitive_binding_for_fixture_mechanism() {
    let contract =
        SecretComputeProviderContract::for_mechanism(ConfidentialMechanismKind::AdditiveNOfN);
    assert_eq!(
        contract.required_capability(),
        SecretComputeProviderCapability::ReferenceFixtures
    );
    assert!(!contract.production_provider_review_required());

    let binding = SecretComputeProviderBinding::new_with_evidence(
        SecretComputeProviderKind::ReferencePrimitive,
        SecretComputeProviderCapability::ReferenceFixtures,
        SecretComputeProviderProfile::KivaReferencePrimitivesV1,
        commitment(1),
        commitment(2),
        SecretComputeProviderEvidence::non_production(None),
    )
    .expect("reference primitive binding should be valid");
    let admission = contract
        .admit(binding)
        .expect("reference primitive binding should be admitted");

    assert_eq!(
        admission.evidence_gate(),
        SecretComputeProviderEvidenceGate::ReferenceFixture
    );
    assert_eq!(
        admission.release_gate(),
        SecretComputeProviderReleaseGate::FixtureOnly
    );

    let runtime_boundary = SecretComputeProviderRuntimeBoundary::new(admission)
        .expect("reference primitive admission should map to a fixture runtime boundary");
    assert_eq!(
        runtime_boundary.claim(),
        SecretComputeProviderRuntimeBoundaryClaim::Fixture
    );
    assert_eq!(
        runtime_boundary.runtime_class(),
        SecretComputeProviderRuntimeClass::Reference
    );
    assert_eq!(runtime_boundary.control_plane_label(), "kiva-control-plane");
    assert_eq!(runtime_boundary.data_plane_label(), "external-data-plane");
    assert!(runtime_boundary.reference_provider());
    assert!(!runtime_boundary.production_ready());

    let reference_boundary = admit_reference_primitive_runtime_boundary(
        commitment(1),
        commitment(2),
        ConfidentialMechanismKind::AdditiveNOfN,
    )
    .expect("reference helper should admit fixture boundary");
    assert_eq!(
        reference_boundary.runtime_class(),
        SecretComputeProviderRuntimeClass::Reference
    );
}

#[test]
fn runtime_boundary_records_reviewed_production_claims() {
    let binding = SecretComputeProviderBinding::new_with_evidence(
        SecretComputeProviderKind::FullyHomomorphicEncryption,
        SecretComputeProviderCapability::FheEvaluation,
        SecretComputeProviderProfile::ExternalReviewedProviderV1,
        commitment(11),
        commitment(12),
        SecretComputeProviderEvidence::reviewed_production(Some(commitment(13)), commitment(14)),
    )
    .expect("reviewed FHE provider binding should be valid");
    let runtime_boundary = admit_secret_compute_provider_runtime_boundary(
        binding,
        ConfidentialMechanismKind::FheSumRuntime,
    )
    .expect("reviewed FHE provider should admit a runtime boundary");

    assert_eq!(
        runtime_boundary.claim(),
        SecretComputeProviderRuntimeBoundaryClaim::ExternalReviewedProduction
    );
    assert_eq!(
        runtime_boundary.runtime_class(),
        SecretComputeProviderRuntimeClass::ExternalReviewedProduction
    );
    assert!(runtime_boundary.production_ready());

    let reviewed_boundary = admit_reviewed_production_runtime_boundary(
        ComputeMechanismProviderKind::FullyHomomorphicEncryption,
        SecretComputeProviderCapability::FheEvaluation,
        commitment(11),
        commitment(12),
        Some(commitment(13)),
        commitment(14),
        ConfidentialMechanismKind::FheSumRuntime,
    )
    .expect("reviewed production helper should admit a production boundary");
    assert!(reviewed_boundary.production_ready());
}

#[test]
fn provider_kind_labels_round_trip() {
    for kind in provider_kinds() {
        assert_eq!(
            SecretComputeProviderKind::from_label(kind.label()),
            Some(kind)
        );
    }
    assert_eq!(SecretComputeProviderKind::from_label("unknown"), None);
}

#[test]
fn provider_kind_category_conversions_are_explicit() {
    assert!(SecretComputeProviderKind::SecretCustody.is_secret_custody_provider());
    assert!(SecretComputeProviderKind::ReferencePrimitive.is_reference_primitive_provider());
    assert!(!SecretComputeProviderKind::SecretCustody.is_compute_mechanism_provider());
    assert_eq!(
        SecretComputeProviderKind::ReferencePrimitive.as_reference_provider_kind(),
        Some(ReferenceProviderKind::ReferencePrimitive)
    );
    assert_eq!(
        SecretComputeProviderKind::SecretCustody.as_secret_custody_provider_kind(),
        Some(SecretCustodyProviderKind::SecretCustody)
    );
    assert_eq!(
        SecretComputeProviderKind::FullyHomomorphicEncryption.as_compute_mechanism_provider_kind(),
        Some(ComputeMechanismProviderKind::FullyHomomorphicEncryption)
    );
    assert_eq!(
        ComputeMechanismProviderKind::from_label(
            ComputeMechanismProviderKind::FullyHomomorphicEncryption.label()
        ),
        Some(ComputeMechanismProviderKind::FullyHomomorphicEncryption)
    );
    assert_eq!(
        ComputeMechanismProviderKind::from_label("secret-custody"),
        None
    );
    assert!(SecretComputeProviderKind::FullyHomomorphicEncryption.is_compute_mechanism_provider());
    assert!(!SecretComputeProviderKind::ReferencePrimitive.is_compute_mechanism_provider());
}

#[test]
fn typed_provider_kind_contracts_project_to_shared_kind() {
    assert_kind_contract(
        SecretComputeProviderKind::ReferencePrimitive,
        SecretComputeProviderKind::ReferencePrimitive,
    );
    assert_kind_contract(
        ReferenceProviderKind::ReferencePrimitive,
        SecretComputeProviderKind::ReferencePrimitive,
    );
    assert_kind_contract(
        SecretCustodyProviderKind::SecretCustody,
        SecretComputeProviderKind::SecretCustody,
    );
    assert_kind_contract(
        ComputeMechanismProviderKind::FullyHomomorphicEncryption,
        SecretComputeProviderKind::FullyHomomorphicEncryption,
    );
}

#[test]
fn provider_capability_and_profile_labels_round_trip() {
    for capability in [
        SecretComputeProviderCapability::SecretCustody,
        SecretComputeProviderCapability::TeeAttestation,
        SecretComputeProviderCapability::FheEvaluation,
        SecretComputeProviderCapability::PsiEvaluation,
        SecretComputeProviderCapability::ThresholdSigning,
        SecretComputeProviderCapability::ReferenceFixtures,
    ] {
        assert_eq!(
            SecretComputeProviderCapability::from_label(capability.label()),
            Some(capability)
        );
    }
    for profile in [
        SecretComputeProviderProfile::KivaReferencePrimitivesV1,
        SecretComputeProviderProfile::ExternalReviewedProviderV1,
    ] {
        assert_eq!(
            SecretComputeProviderProfile::from_label(profile.label()),
            Some(profile)
        );
    }
    assert_eq!(SecretComputeProviderCapability::from_label("unknown"), None);
    assert_eq!(SecretComputeProviderProfile::from_label("unknown"), None);
}

#[test]
fn provider_kind_registry_matches_generated_labels() {
    for kind in provider_kinds() {
        let entry = registry_entry_for_provider_kind(kind)
            .expect("all provider kinds must be registry-backed");
        assert_eq!(entry.label, kind.label());
        assert_eq!(
            SecretComputeProviderKind::from_label(entry.label),
            Some(kind)
        );
    }
    assert_eq!(SECRET_COMPUTE_PROVIDER_KIND_REGISTRY.len(), 6);
}

#[test]
fn mechanism_profiles_project_provider_contracts() {
    for mechanism in [
        ConfidentialMechanismKind::AdditiveNOfN,
        ConfidentialMechanismKind::PrivateSetIntersection,
        ConfidentialMechanismKind::PrivateSetIntersectionOprfDh,
        ConfidentialMechanismKind::ShamirThreshold,
        ConfidentialMechanismKind::ThresholdFrostSigning,
        ConfidentialMechanismKind::TrustedExecutionEnvironment,
        ConfidentialMechanismKind::TrustedExecutionEnvironmentAwsNitro,
        ConfidentialMechanismKind::FheSumFeasibility,
        ConfidentialMechanismKind::FheSumRuntime,
    ] {
        let contract = mechanism.provider_contract();
        let descriptor = descriptor_for_mechanism(mechanism);
        assert_eq!(
            SecretComputeProviderContractProfile::provider_contract(descriptor),
            contract
        );
        assert_eq!(
            SecretComputeProviderCapabilityProfile::required_capability(descriptor),
            contract.required_capability()
        );
        assert_eq!(contract.mechanism(), mechanism);
        assert_eq!(
            contract.required_capability(),
            SecretComputeProviderCapabilityProfile::required_capability(mechanism)
        );
        assert_eq!(
            contract.production_provider_review_required(),
            SecretComputeProviderCapabilityProfile::production_provider_review_required(mechanism)
        );
        assert_eq!(
            SecretComputeProviderContractProfile::provider_contract(contract),
            contract
        );
        assert_eq!(
            SecretComputeProviderContractProfile::mechanism(contract),
            mechanism
        );
        let pipeline = SecretComputeProviderContractPipeline::for_descriptor(descriptor);
        assert_eq!(pipeline.descriptor(), descriptor);
        assert_eq!(pipeline.mechanism(), mechanism);
        assert_eq!(pipeline.provider_contract(), contract);
        assert_eq!(
            pipeline.required_capability(),
            contract.required_capability()
        );
        assert_eq!(
            pipeline.production_provider_review_required(),
            contract.production_provider_review_required()
        );
        assert_eq!(
            SecretComputeProviderContractProfile::provider_contract(pipeline),
            contract
        );
    }
}

#[test]
fn provider_pipeline_admits_runtime_boundary_through_contract() {
    let pipeline = SecretComputeProviderContractPipeline::for_mechanism(
        ConfidentialMechanismKind::TrustedExecutionEnvironmentAwsNitro,
    );
    let binding = SecretComputeProviderBinding::new_with_evidence(
        SecretComputeProviderKind::TrustedExecutionEnvironment,
        SecretComputeProviderCapability::TeeAttestation,
        SecretComputeProviderProfile::ExternalReviewedProviderV1,
        commitment(21),
        commitment(22),
        SecretComputeProviderEvidence::reviewed_production(None, commitment(23)),
    )
    .expect("reviewed Nitro provider binding should be valid");

    let admitted = pipeline
        .admit_runtime_boundary(binding)
        .expect("reviewed provider pipeline should admit runtime boundary");

    assert_eq!(admitted.pipeline(), pipeline);
    assert_eq!(
        admitted.admission().evidence_gate(),
        SecretComputeProviderEvidenceGate::ExternalReviewed
    );
    assert_eq!(
        admitted.runtime_boundary().runtime_class(),
        SecretComputeProviderRuntimeClass::ExternalReviewedProduction
    );
    assert!(admitted.runtime_boundary().production_ready());
    let runtime_contract = admitted.runtime_boundary().runtime_contract();
    assert_eq!(
        runtime_contract.mechanism(),
        ConfidentialMechanismKind::TrustedExecutionEnvironmentAwsNitro
    );
    assert_eq!(
        runtime_contract.required_capability(),
        SecretComputeProviderCapability::TeeAttestation
    );
    assert_eq!(
        runtime_contract.runtime_class(),
        SecretComputeProviderRuntimeClass::ExternalReviewedProduction
    );
    assert_eq!(
        runtime_contract.claim(),
        SecretComputeProviderRuntimeBoundaryClaim::ExternalReviewedProduction
    );
    assert!(runtime_contract.production_ready());
}

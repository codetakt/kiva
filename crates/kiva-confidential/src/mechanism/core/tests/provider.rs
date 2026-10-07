#![allow(clippy::wildcard_imports)]

use super::*;

#[test]
fn reference_primitive_provider_is_admitted_only_for_non_production_fixtures() {
    let binding = SecretComputeProviderBinding::new_with_evidence(
        SecretComputeProviderKind::ReferencePrimitive,
        SecretComputeProviderCapability::ReferenceFixtures,
        SecretComputeProviderProfile::KivaReferencePrimitivesV1,
        commitment(201),
        commitment(202),
        SecretComputeProviderEvidence::non_production(None),
    );
    assert!(binding.is_ok());
    let Ok(binding) = binding else {
        return;
    };
    assert!(
        validate_secret_compute_provider_for_mechanism(
            binding,
            ConfidentialMechanismKind::FheSumFeasibility,
        )
        .is_ok()
    );

    let production_reference = SecretComputeProviderBinding::new_with_evidence(
        SecretComputeProviderKind::ReferencePrimitive,
        SecretComputeProviderCapability::ReferenceFixtures,
        SecretComputeProviderProfile::KivaReferencePrimitivesV1,
        commitment(201),
        commitment(202),
        SecretComputeProviderEvidence::new(
            None,
            SecretComputeProviderReviewEvidence::Pending,
            SecretComputeProviderUsageClaim::Production,
        ),
    );
    assert_eq!(
        production_reference,
        Err(SecureAggregateError::ReferencePrimitiveProviderProductionClaimRejected)
    );
}

#[test]
fn external_provider_requires_review_evidence_for_production_claims() {
    let missing_parameter_set = SecretComputeProviderBinding::new_with_evidence(
        SecretComputeProviderKind::FullyHomomorphicEncryption,
        SecretComputeProviderCapability::FheEvaluation,
        SecretComputeProviderProfile::ExternalReviewedProviderV1,
        commitment(211),
        commitment(212),
        SecretComputeProviderEvidence::new(
            None,
            SecretComputeProviderReviewEvidence::Completed(commitment(214)),
            SecretComputeProviderUsageClaim::NonProduction,
        ),
    );
    assert_eq!(
        missing_parameter_set,
        Err(SecureAggregateError::SecretComputeProviderParameterSetRequired)
    );

    let missing_review = SecretComputeProviderBinding::new_with_evidence(
        SecretComputeProviderKind::FullyHomomorphicEncryption,
        SecretComputeProviderCapability::FheEvaluation,
        SecretComputeProviderProfile::ExternalReviewedProviderV1,
        commitment(211),
        commitment(212),
        SecretComputeProviderEvidence::new(
            Some(commitment(213)),
            SecretComputeProviderReviewEvidence::Pending,
            SecretComputeProviderUsageClaim::Production,
        ),
    );
    assert_eq!(
        missing_review,
        Err(SecureAggregateError::ExternalProviderReviewRequired)
    );

    let reviewed = SecretComputeProviderBinding::new_with_evidence(
        SecretComputeProviderKind::FullyHomomorphicEncryption,
        SecretComputeProviderCapability::FheEvaluation,
        SecretComputeProviderProfile::ExternalReviewedProviderV1,
        commitment(211),
        commitment(212),
        SecretComputeProviderEvidence::reviewed_production(Some(commitment(213)), commitment(214)),
    );
    assert!(reviewed.is_ok());
}

#[test]
fn provider_admission_records_capability_evidence_and_release_gates() {
    let reference = SecretComputeProviderBinding::new_with_evidence(
        SecretComputeProviderKind::ReferencePrimitive,
        SecretComputeProviderCapability::ReferenceFixtures,
        SecretComputeProviderProfile::KivaReferencePrimitivesV1,
        commitment(201),
        commitment(202),
        SecretComputeProviderEvidence::non_production(None),
    );
    assert!(reference.is_ok());
    let Ok(reference) = reference else {
        return;
    };
    let reference_admission =
        admit_secret_compute_provider(reference, ConfidentialMechanismKind::FheSumRuntime);
    assert!(reference_admission.is_ok());
    let Ok(reference_admission) = reference_admission else {
        return;
    };
    assert_eq!(
        reference_admission.required_capability(),
        SecretComputeProviderCapability::FheEvaluation
    );
    assert_eq!(
        reference_admission.evidence_gate(),
        SecretComputeProviderEvidenceGate::ReferenceFixture
    );
    assert_eq!(
        reference_admission.release_gate(),
        SecretComputeProviderReleaseGate::FixtureOnly
    );

    let reviewed = SecretComputeProviderBinding::new_with_evidence(
        SecretComputeProviderKind::FullyHomomorphicEncryption,
        SecretComputeProviderCapability::FheEvaluation,
        SecretComputeProviderProfile::ExternalReviewedProviderV1,
        commitment(211),
        commitment(212),
        SecretComputeProviderEvidence::reviewed_production(Some(commitment(213)), commitment(214)),
    );
    assert!(reviewed.is_ok());
    let Ok(reviewed) = reviewed else {
        return;
    };
    let reviewed_admission =
        admit_secret_compute_provider(reviewed, ConfidentialMechanismKind::FheSumRuntime);
    assert!(reviewed_admission.is_ok());
    let Ok(reviewed_admission) = reviewed_admission else {
        return;
    };
    assert_eq!(
        reviewed_admission.binding_digest(),
        derive_secret_compute_provider_binding_digest(reviewed)
    );
    assert_eq!(
        reviewed_admission.evidence_gate(),
        SecretComputeProviderEvidenceGate::ExternalReviewed
    );
    assert_eq!(
        reviewed_admission.release_gate(),
        SecretComputeProviderReleaseGate::Production
    );
    assert!(secret_compute_provider_kind_requires_parameter_set(
        SecretComputeProviderKind::FullyHomomorphicEncryption
    ));
    assert_eq!(
        secret_compute_required_capability_for_mechanism(ConfidentialMechanismKind::FheSumRuntime),
        SecretComputeProviderCapability::FheEvaluation
    );
}

#[test]
fn provider_capability_must_match_mechanism_family() {
    let binding = SecretComputeProviderBinding::new_with_evidence(
        SecretComputeProviderKind::SecretCustody,
        SecretComputeProviderCapability::SecretCustody,
        SecretComputeProviderProfile::ExternalReviewedProviderV1,
        commitment(221),
        commitment(222),
        SecretComputeProviderEvidence::reviewed_production(None, commitment(223)),
    );
    assert!(binding.is_ok());
    let Ok(binding) = binding else {
        return;
    };
    assert_eq!(
        validate_secret_compute_provider_for_mechanism(
            binding,
            ConfidentialMechanismKind::FheSumRuntime,
        ),
        Err(SecureAggregateError::SecretComputeProviderCapabilityMismatch)
    );
}

#[test]
fn provider_binding_digest_is_canonical_and_artifact_bound() {
    let first = SecretComputeProviderBinding::new_with_evidence(
        SecretComputeProviderKind::FullyHomomorphicEncryption,
        SecretComputeProviderCapability::FheEvaluation,
        SecretComputeProviderProfile::ExternalReviewedProviderV1,
        commitment(231),
        commitment(232),
        SecretComputeProviderEvidence::new(
            Some(commitment(233)),
            SecretComputeProviderReviewEvidence::Completed(commitment(234)),
            SecretComputeProviderUsageClaim::NonProduction,
        ),
    );
    let second = SecretComputeProviderBinding::new_with_evidence(
        SecretComputeProviderKind::FullyHomomorphicEncryption,
        SecretComputeProviderCapability::FheEvaluation,
        SecretComputeProviderProfile::ExternalReviewedProviderV1,
        commitment(231),
        commitment(235),
        SecretComputeProviderEvidence::new(
            Some(commitment(233)),
            SecretComputeProviderReviewEvidence::Completed(commitment(234)),
            SecretComputeProviderUsageClaim::NonProduction,
        ),
    );
    assert!(first.is_ok());
    assert!(second.is_ok());
    let (Ok(first), Ok(second)) = (first, second) else {
        return;
    };
    assert_ne!(
        derive_secret_compute_provider_binding_digest(first),
        derive_secret_compute_provider_binding_digest(second),
    );
    assert_eq!(
        derive_secret_compute_provider_binding_digest(first),
        derive_secret_compute_provider_binding_digest(first),
    );
}

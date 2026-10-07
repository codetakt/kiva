#![allow(clippy::wildcard_imports)]

use super::*;

#[kani::proof]
pub(super) fn reference_provider_rejects_production_claims() {
    let production_claim_requested: bool = kani::any();
    let binding = fixed_reference_provider_binding(production_claim_requested);
    let result = validate_secret_compute_provider_binding(binding);
    if production_claim_requested {
        assert!(matches!(
            result,
            Err(SecureAggregateError::ReferencePrimitiveProviderProductionClaimRejected)
        ));
    } else {
        assert!(result.is_ok());
    }
}

#[kani::proof]
pub(super) fn provider_capability_mismatch_fails_closed() {
    let binding = fixed_secret_custody_provider_binding();
    assert!(matches!(
        validate_secret_compute_provider_for_mechanism(
            binding,
            ConfidentialMechanismKind::FheSumRuntime,
        ),
        Err(SecureAggregateError::SecretComputeProviderCapabilityMismatch)
    ));
}

#[kani::proof]
pub(super) fn release_policy_validation_matches_structural_rules() {
    let participant_count = bounded_nonzero(kani::any());
    let release_threshold = bounded_nonzero(kani::any());

    let result = ReleasePolicy::new(participant_count, release_threshold);

    if participant_count.get() < 2 {
        assert!(matches!(result, Err(WorkflowError::TooFewParticipants)));
    } else if release_threshold.get() > participant_count.get() {
        assert!(matches!(
            result,
            Err(WorkflowError::ThresholdExceedsParticipants)
        ));
    } else {
        let Ok(policy) = result else {
            assert!(false);
            return;
        };
        assert!(policy.participant_count().get() >= 2);
        assert!(policy.release_threshold().get() <= policy.participant_count().get());
    }
}

#[kani::proof]
pub(super) fn confidential_mechanism_plan_enforces_additive_and_shamir_threshold_rules() {
    let participant_count = bounded_participant_count(kani::any());
    let release_threshold = bounded_threshold(kani::any(), participant_count);
    let Ok(policy) = ReleasePolicy::new(participant_count, release_threshold) else {
        assert!(false);
        return;
    };

    let additive = ConfidentialMechanismPlan::new(ConfidentialMechanismKind::AdditiveNOfN, policy);
    let psi =
        ConfidentialMechanismPlan::new(ConfidentialMechanismKind::PrivateSetIntersection, policy);
    let psi_oprf = ConfidentialMechanismPlan::new(
        ConfidentialMechanismKind::PrivateSetIntersectionOprfDh,
        policy,
    );
    let tee = ConfidentialMechanismPlan::new(
        ConfidentialMechanismKind::TrustedExecutionEnvironment,
        policy,
    );
    let tee_nitro = ConfidentialMechanismPlan::new(
        ConfidentialMechanismKind::TrustedExecutionEnvironmentAwsNitro,
        policy,
    );
    let fhe = ConfidentialMechanismPlan::new(ConfidentialMechanismKind::FheSumFeasibility, policy);
    let fhe_runtime =
        ConfidentialMechanismPlan::new(ConfidentialMechanismKind::FheSumRuntime, policy);
    let threshold_frost =
        ConfidentialMechanismPlan::new(ConfidentialMechanismKind::ThresholdFrostSigning, policy);
    if release_threshold.get() == participant_count.get() {
        assert!(additive.is_ok());
        assert!(psi.is_ok());
        assert!(psi_oprf.is_ok());
        assert!(tee.is_ok());
        assert!(tee_nitro.is_ok());
        assert!(fhe.is_ok());
        assert!(fhe_runtime.is_ok());
    } else {
        assert!(matches!(
            additive,
            Err(SecureAggregateError::ThresholdMustRequireAllParticipants)
        ));
        assert!(matches!(
            psi,
            Err(SecureAggregateError::PsiRequiresAllParticipants)
        ));
        assert!(matches!(
            psi_oprf,
            Err(SecureAggregateError::PsiRequiresAllParticipants)
        ));
        assert!(matches!(
            tee,
            Err(SecureAggregateError::TeeRequiresAllParticipants)
        ));
        assert!(matches!(
            tee_nitro,
            Err(SecureAggregateError::TeeRequiresAllParticipants)
        ));
        assert!(matches!(
            fhe,
            Err(SecureAggregateError::FheRequiresAllParticipants)
        ));
        assert!(matches!(
            fhe_runtime,
            Err(SecureAggregateError::FheRequiresAllParticipants)
        ));
    }

    let shamir = ConfidentialMechanismPlan::new(ConfidentialMechanismKind::ShamirThreshold, policy);
    if release_threshold.get() < 2 {
        assert!(matches!(
            shamir,
            Err(SecureAggregateError::ShamirThresholdTooSmall)
        ));
        assert!(matches!(
            threshold_frost,
            Err(SecureAggregateError::ShamirThresholdTooSmall)
        ));
    } else {
        assert!(shamir.is_ok());
        assert!(threshold_frost.is_ok());
    }
}

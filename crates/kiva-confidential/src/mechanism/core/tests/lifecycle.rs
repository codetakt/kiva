#![allow(clippy::wildcard_imports)]

use super::*;

#[test]
fn rejects_single_party_workflow() {
    let one = std::num::NonZeroUsize::MIN;

    assert_eq!(
        ReleasePolicy::new(one, one),
        Err(WorkflowError::TooFewParticipants)
    );
}

#[test]
fn validates_secure_aggregate_field_membership() {
    assert_eq!(
        SecureAggregateFieldElement::new(SECURE_AGGREGATE_FIELD_MODULUS),
        Err(SecureAggregateError::ValueOutsideField)
    );
}

#[test]
fn serde_rejects_invalid_secure_aggregate_field_membership() {
    let payload = serde_yaml::to_string(&SECURE_AGGREGATE_FIELD_MODULUS);
    assert!(payload.is_ok());
    let Ok(payload) = payload else {
        return;
    };

    let parsed: Result<SecureAggregateFieldElement, serde_yaml::Error> =
        serde_yaml::from_str(&payload);

    assert!(parsed.is_err());
}

#[test]
fn adds_secure_aggregate_field_elements_modulo_prime() {
    let last = field(SECURE_AGGREGATE_FIELD_MODULUS - 1);
    let two = field(2);

    assert_eq!(last.add_mod(two), field(1));
}

#[test]
fn accepts_additive_secure_aggregate_plan_only_for_all_participants() {
    let two = nonzero(2);
    let one = NonZeroUsize::MIN;
    let all_party = ReleasePolicy::new(two, two);
    let threshold = ReleasePolicy::new(two, one);

    assert!(matches!(
        all_party.map(AdditiveSecureAggregatePlan::new),
        Ok(Ok(_plan))
    ));
    assert_eq!(
        threshold.map(AdditiveSecureAggregatePlan::new),
        Ok(Err(
            SecureAggregateError::ThresholdMustRequireAllParticipants
        ))
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn mechanism_plan_enforces_additive_and_shamir_threshold_rules() {
    let three = nonzero(3);
    let one = NonZeroUsize::MIN;
    let additive_policy = policy(3, 3);
    let psi_policy = policy(3, 3);
    let psi_oprf_policy = policy(3, 3);
    let shamir_policy = policy(3, 2);
    let threshold_frost_policy = policy(3, 2);
    let tee_policy = policy(3, 3);
    let tee_nitro_policy = policy(3, 3);
    let fhe_policy = policy(3, 3);
    let fhe_runtime_policy = policy(3, 3);
    let invalid_additive_policy = policy(3, 2);
    let invalid_psi_policy = policy(3, 2);
    let invalid_psi_oprf_policy = policy(3, 2);
    let invalid_tee_policy = policy(3, 2);
    let invalid_tee_nitro_policy = policy(3, 2);
    let invalid_fhe_policy = policy(3, 2);
    let invalid_fhe_runtime_policy = policy(3, 2);
    let invalid_shamir_policy = ReleasePolicy::new(three, one);
    let invalid_threshold_frost_policy = ReleasePolicy::new(three, one);

    let additive =
        ConfidentialMechanismPlan::new(ConfidentialMechanismKind::AdditiveNOfN, additive_policy);
    let shamir =
        ConfidentialMechanismPlan::new(ConfidentialMechanismKind::ShamirThreshold, shamir_policy);
    let threshold_frost = ConfidentialMechanismPlan::new(
        ConfidentialMechanismKind::ThresholdFrostSigning,
        threshold_frost_policy,
    );
    let psi = ConfidentialMechanismPlan::new(
        ConfidentialMechanismKind::PrivateSetIntersection,
        psi_policy,
    );
    let psi_oprf = ConfidentialMechanismPlan::new(
        ConfidentialMechanismKind::PrivateSetIntersectionOprfDh,
        psi_oprf_policy,
    );
    let tee = ConfidentialMechanismPlan::new(
        ConfidentialMechanismKind::TrustedExecutionEnvironment,
        tee_policy,
    );
    let tee_nitro = ConfidentialMechanismPlan::new(
        ConfidentialMechanismKind::TrustedExecutionEnvironmentAwsNitro,
        tee_nitro_policy,
    );
    let fhe =
        ConfidentialMechanismPlan::new(ConfidentialMechanismKind::FheSumFeasibility, fhe_policy);
    let fhe_runtime = ConfidentialMechanismPlan::new(
        ConfidentialMechanismKind::FheSumRuntime,
        fhe_runtime_policy,
    );

    assert!(additive.is_ok());
    let Ok(additive) = additive else {
        return;
    };
    assert!(additive.requires_all_participants());
    assert_eq!(
        additive.technique(),
        super::Technique::AdditiveSecureAggregation
    );

    assert!(shamir.is_ok());
    let Ok(shamir) = shamir else {
        return;
    };
    assert!(!shamir.requires_all_participants());
    assert!(shamir.supports_partial_reconstruction());
    assert_eq!(shamir.technique(), super::Technique::ThresholdCryptography);

    assert!(threshold_frost.is_ok());
    let Ok(threshold_frost) = threshold_frost else {
        return;
    };
    assert!(!threshold_frost.requires_all_participants());
    assert!(threshold_frost.supports_partial_reconstruction());
    assert_eq!(
        threshold_frost.technique(),
        super::Technique::ThresholdCryptography
    );

    assert!(psi.is_ok());
    let Ok(psi) = psi else {
        return;
    };
    assert!(psi.requires_all_participants());
    assert_eq!(psi.technique(), super::Technique::PrivateSetIntersection);

    assert!(psi_oprf.is_ok());
    let Ok(psi_oprf) = psi_oprf else {
        return;
    };
    assert!(psi_oprf.requires_all_participants());
    assert_eq!(
        psi_oprf.technique(),
        super::Technique::PrivateSetIntersection
    );

    assert!(tee.is_ok());
    let Ok(tee) = tee else {
        return;
    };
    assert!(tee.requires_all_participants());
    assert_eq!(
        tee.technique(),
        super::Technique::TrustedExecutionEnvironment
    );

    assert!(tee_nitro.is_ok());
    let Ok(tee_nitro) = tee_nitro else {
        return;
    };
    assert!(tee_nitro.requires_all_participants());
    assert_eq!(
        tee_nitro.technique(),
        super::Technique::TrustedExecutionEnvironment
    );

    assert!(fhe.is_ok());
    let Ok(fhe) = fhe else {
        return;
    };
    assert!(fhe.requires_all_participants());
    assert_eq!(
        fhe.technique(),
        super::Technique::FullyHomomorphicEncryption
    );
    assert!(fhe_runtime.is_ok());
    let Ok(fhe_runtime) = fhe_runtime else {
        return;
    };
    assert!(fhe_runtime.requires_all_participants());
    assert_eq!(
        fhe_runtime.technique(),
        super::Technique::FullyHomomorphicEncryption
    );

    assert_eq!(
        ConfidentialMechanismPlan::new(
            ConfidentialMechanismKind::AdditiveNOfN,
            invalid_additive_policy,
        ),
        Err(SecureAggregateError::ThresholdMustRequireAllParticipants)
    );
    assert_eq!(
        ConfidentialMechanismPlan::new(
            ConfidentialMechanismKind::PrivateSetIntersection,
            invalid_psi_policy,
        ),
        Err(SecureAggregateError::PsiRequiresAllParticipants)
    );
    assert_eq!(
        ConfidentialMechanismPlan::new(
            ConfidentialMechanismKind::PrivateSetIntersectionOprfDh,
            invalid_psi_oprf_policy,
        ),
        Err(SecureAggregateError::PsiRequiresAllParticipants)
    );
    assert_eq!(
        ConfidentialMechanismPlan::new(
            ConfidentialMechanismKind::TrustedExecutionEnvironment,
            invalid_tee_policy,
        ),
        Err(SecureAggregateError::TeeRequiresAllParticipants)
    );
    assert_eq!(
        ConfidentialMechanismPlan::new(
            ConfidentialMechanismKind::TrustedExecutionEnvironmentAwsNitro,
            invalid_tee_nitro_policy,
        ),
        Err(SecureAggregateError::TeeRequiresAllParticipants)
    );
    assert_eq!(
        ConfidentialMechanismPlan::new(
            ConfidentialMechanismKind::FheSumFeasibility,
            invalid_fhe_policy,
        ),
        Err(SecureAggregateError::FheRequiresAllParticipants)
    );
    assert_eq!(
        ConfidentialMechanismPlan::new(
            ConfidentialMechanismKind::FheSumRuntime,
            invalid_fhe_runtime_policy,
        ),
        Err(SecureAggregateError::FheRequiresAllParticipants)
    );
    assert!(invalid_shamir_policy.is_ok());
    let Ok(invalid_shamir_policy) = invalid_shamir_policy else {
        return;
    };
    assert_eq!(
        ConfidentialMechanismPlan::new(
            ConfidentialMechanismKind::ShamirThreshold,
            invalid_shamir_policy,
        ),
        Err(SecureAggregateError::ShamirThresholdTooSmall)
    );
    assert!(invalid_threshold_frost_policy.is_ok());
    let Ok(invalid_threshold_frost_policy) = invalid_threshold_frost_policy else {
        return;
    };
    assert_eq!(
        ConfidentialMechanismPlan::new(
            ConfidentialMechanismKind::ThresholdFrostSigning,
            invalid_threshold_frost_policy,
        ),
        Err(SecureAggregateError::ShamirThresholdTooSmall)
    );
}

#[test]
fn confidential_job_lifecycle_allows_only_strict_transitions() {
    use ConfidentialJobStatus as Status;

    let allowed = [
        (Status::Draft, Status::Admitted),
        (Status::Admitted, Status::Running),
        (Status::Running, Status::AwaitingRelease),
        (Status::AwaitingRelease, Status::Released),
        (Status::Released, Status::Retired),
        (Status::Running, Status::Failed),
        (Status::Draft, Status::Expired),
    ];
    for (current, requested) in allowed {
        assert!(confidential_job_transition_is_allowed(current, requested));
        assert_eq!(
            transition_confidential_job_status(current, requested),
            Ok(requested)
        );
    }

    let rejected = [
        (Status::Draft, Status::Released),
        (Status::Admitted, Status::AwaitingRelease),
        (Status::Running, Status::Released),
        (Status::Released, Status::Running),
        (Status::Retired, Status::Draft),
    ];
    for (current, requested) in rejected {
        assert!(!confidential_job_transition_is_allowed(current, requested));
        assert_eq!(
            transition_confidential_job_status(current, requested),
            Err(ConfidentialJobTransitionError::InvalidTransition)
        );
    }
}

#[test]
fn confidential_participant_lifecycle_is_strict_and_fail_closed() {
    use ConfidentialParticipantStatus as Status;

    let cases = [
        (Status::Invited, Status::Active, true),
        (Status::Invited, Status::Suspended, true),
        (Status::Invited, Status::Removed, true),
        (Status::Active, Status::Suspended, true),
        (Status::Active, Status::Frozen, true),
        (Status::Active, Status::Removed, true),
        (Status::Suspended, Status::Active, true),
        (Status::Suspended, Status::Frozen, true),
        (Status::Suspended, Status::Removed, true),
        (Status::Frozen, Status::Active, true),
        (Status::Frozen, Status::Removed, true),
        (Status::Removed, Status::Active, false),
        (Status::Removed, Status::Suspended, false),
        (Status::Removed, Status::Frozen, false),
        (Status::Active, Status::Invited, false),
        (Status::Frozen, Status::Suspended, false),
    ];

    for (current, requested, allowed) in cases {
        assert_eq!(
            confidential_participant_transition_is_allowed(current, requested),
            allowed
        );
        if current == requested || allowed {
            assert_eq!(
                transition_confidential_participant_status(current, requested),
                Ok(requested)
            );
        } else {
            assert_eq!(
                transition_confidential_participant_status(current, requested),
                Err(ConfidentialParticipantTransitionError::InvalidTransition)
            );
        }
    }

    for status in [
        Status::Invited,
        Status::Suspended,
        Status::Frozen,
        Status::Removed,
    ] {
        assert!(!confidential_participant_accepts_jobs(status));
        assert!(!confidential_participant_accepts_new_keys(status));
    }
    assert!(confidential_participant_accepts_jobs(Status::Active));
    assert!(confidential_participant_accepts_new_keys(Status::Active));
}

#[test]
fn confidential_job_admission_fails_closed() {
    let accepted = ConfidentialJobAdmissionFacts::new(true, true, true, true, true, true, true);
    assert_eq!(validate_confidential_job_admission(accepted), Ok(()));

    assert_eq!(
        validate_confidential_job_admission(ConfidentialJobAdmissionFacts::new(
            false, true, true, true, true, true, true,
        )),
        Err(ConfidentialJobAdmissionError::TooFewParticipants)
    );
    assert_eq!(
        validate_confidential_job_admission(ConfidentialJobAdmissionFacts::new(
            true, false, true, true, true, true, true,
        )),
        Err(ConfidentialJobAdmissionError::MissingPolicyBinding)
    );
    assert_eq!(
        validate_confidential_job_admission(ConfidentialJobAdmissionFacts::new(
            true, true, false, true, true, true, true,
        )),
        Err(ConfidentialJobAdmissionError::DeadlineNotFuture)
    );
    assert_eq!(
        validate_confidential_job_admission(ConfidentialJobAdmissionFacts::new(
            true, true, true, false, true, true, true,
        )),
        Err(ConfidentialJobAdmissionError::AdmissionQuotaExceeded)
    );
    assert_eq!(
        validate_confidential_job_admission(ConfidentialJobAdmissionFacts::new(
            true, true, true, true, false, true, true,
        )),
        Err(ConfidentialJobAdmissionError::SchedulerBackpressure)
    );
    assert_eq!(
        validate_confidential_job_admission(ConfidentialJobAdmissionFacts::new(
            true, true, true, true, true, false, true,
        )),
        Err(ConfidentialJobAdmissionError::SchedulerBackpressure)
    );
    assert_eq!(
        validate_confidential_job_admission(ConfidentialJobAdmissionFacts::new(
            true, true, true, true, true, true, false,
        )),
        Err(ConfidentialJobAdmissionError::RetryBudgetExceeded)
    );
}

#[test]
fn serde_rejects_invalid_release_policy() {
    let payload = "participant_count: 1\nrelease_threshold: 1\n";
    let parsed: Result<ReleasePolicy, serde_yaml::Error> = serde_yaml::from_str(payload);

    assert!(parsed.is_err());
}

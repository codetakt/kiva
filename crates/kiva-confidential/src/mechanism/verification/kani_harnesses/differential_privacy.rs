#![allow(clippy::wildcard_imports)]

use super::*;

#[kani::proof]
pub(super) fn differential_privacy_policy_rejects_single_subject_minimum() {
    let minimum_population = bounded_nonzero(kani::any());
    let max_epsilon_microunits = bounded_nonzero_u64(kani::any());

    let result = DifferentialPrivacyReleasePolicy::new(minimum_population, max_epsilon_microunits);

    if minimum_population.get() < 2 {
        assert!(matches!(
            result,
            Err(DifferentialPrivacyPolicyError::MinimumPopulationTooSmall)
        ));
    } else {
        let Ok(policy) = result else {
            assert!(false);
            return;
        };
        assert!(policy.minimum_population().get() >= 2);
        assert_eq!(policy.max_epsilon_microunits(), max_epsilon_microunits);
    }
}

#[kani::proof]
pub(super) fn differential_privacy_budget_ledger_rejects_over_spent_state() {
    let total_epsilon_microunits = bounded_nonzero_u64(kani::any());
    let spent_epsilon_microunits = u64::from(kani::any::<u8>() % 18);

    let result =
        DifferentialPrivacyBudgetLedger::new(total_epsilon_microunits, spent_epsilon_microunits);

    if spent_epsilon_microunits > total_epsilon_microunits.get() {
        assert!(matches!(
            result,
            Err(DifferentialPrivacyBudgetError::SpentBudgetExceedsTotal)
        ));
    } else {
        let Ok(ledger) = result else {
            assert!(false);
            return;
        };
        assert_eq!(ledger.total_epsilon_microunits(), total_epsilon_microunits);
        assert_eq!(ledger.spent_epsilon_microunits(), spent_epsilon_microunits);
        assert!(ledger.spent_epsilon_microunits() <= ledger.total_epsilon_microunits().get());
    }
}

#[kani::proof]
pub(super) fn differential_privacy_budget_spend_fails_closed_and_is_monotonic() {
    let total_epsilon_microunits = bounded_nonzero_u64(kani::any());
    let spent_epsilon_microunits = u64::from(kani::any::<u8>()) % total_epsilon_microunits.get();
    let Ok(ledger) =
        DifferentialPrivacyBudgetLedger::new(total_epsilon_microunits, spent_epsilon_microunits)
    else {
        assert!(false);
        return;
    };
    let requested_epsilon_microunits = u64::from(kani::any::<u8>() % 18);

    let result = spend_differential_privacy_budget(ledger, requested_epsilon_microunits);

    if requested_epsilon_microunits == 0 {
        assert!(matches!(
            result,
            Err(DifferentialPrivacyBudgetError::SpendRequestMustBeNonZero)
        ));
    } else if requested_epsilon_microunits > ledger.remaining_epsilon_microunits() {
        assert!(matches!(
            result,
            Err(DifferentialPrivacyBudgetError::SpendRequestExceedsRemaining)
        ));
    } else {
        let Ok(spent) = result else {
            assert!(false);
            return;
        };
        assert_eq!(
            spent.total_epsilon_microunits(),
            ledger.total_epsilon_microunits()
        );
        assert_eq!(
            spent.spent_epsilon_microunits(),
            ledger.spent_epsilon_microunits() + requested_epsilon_microunits
        );
        assert!(spent.spent_epsilon_microunits() >= ledger.spent_epsilon_microunits());
        assert!(spent.remaining_epsilon_microunits() <= ledger.remaining_epsilon_microunits());
        assert!(spent.spent_epsilon_microunits() <= spent.total_epsilon_microunits().get());
    }
}

#[kani::proof]
pub(super) fn differential_privacy_composition_summary_fails_closed() {
    let total_epsilon_microunits = bounded_nonzero_u64(kani::any());
    let spent_epsilon_microunits = u64::from(kani::any::<u8>()) % total_epsilon_microunits.get();
    let Ok(ledger) =
        DifferentialPrivacyBudgetLedger::new(total_epsilon_microunits, spent_epsilon_microunits)
    else {
        assert!(false);
        return;
    };
    let release_epsilon = [
        u64::from(kani::any::<u8>() % 10),
        u64::from(kani::any::<u8>() % 10),
    ];
    let release_sum = release_epsilon[0] + release_epsilon[1];

    let result = summarize_differential_privacy_composition(ledger, &release_epsilon);

    if release_sum > ledger.spent_epsilon_microunits() {
        assert!(matches!(
            result,
            Err(DifferentialPrivacyCompositionError::ReleaseSpendExceedsBudgetSpend)
        ));
    } else {
        let Ok(summary) = result else {
            assert!(false);
            return;
        };
        assert_eq!(summary.ledger(), ledger);
        assert_eq!(summary.release_count(), 2);
        assert_eq!(summary.release_spent_epsilon_microunits(), release_sum);
        assert_eq!(
            summary.unattributed_spent_epsilon_microunits(),
            ledger.spent_epsilon_microunits() - release_sum
        );
        assert!(summary.max_release_epsilon_microunits() <= 9);
    }
}

#[kani::proof]
pub(super) fn differential_privacy_advanced_composition_delta_fails_closed() {
    let delta_microunits = if kani::any::<bool>() {
        NonZeroU64::MIN
    } else {
        let Some(delta_one) = NonZeroU64::new(super::DIFFERENTIAL_PRIVACY_MICROUNITS_PER_UNIT)
        else {
            return;
        };
        delta_one
    };
    let release_epsilon = [
        u64::from(kani::any::<u8>() % 10),
        u64::from(kani::any::<u8>() % 10),
    ];

    let result =
        bound_differential_privacy_advanced_composition(&release_epsilon, delta_microunits);

    if delta_microunits.get() >= super::DIFFERENTIAL_PRIVACY_MICROUNITS_PER_UNIT {
        assert!(matches!(
            result,
            Err(DifferentialPrivacyCompositionError::AdvancedCompositionDeltaOutOfRange)
        ));
    } else {
        let Ok(bound) = result else {
            assert!(false);
            return;
        };
        assert_eq!(bound.release_count(), 2);
        assert_eq!(bound.delta_microunits(), delta_microunits);
        assert_eq!(
            bound.basic_epsilon_microunits(),
            release_epsilon[0] + release_epsilon[1]
        );
    }
}

#[kani::proof]
pub(super) fn differential_privacy_budget_policy_lifecycle_fails_closed() {
    let current = any_differential_privacy_budget_policy_status();
    let requested = any_differential_privacy_budget_policy_status();

    let result = transition_differential_privacy_budget_policy(current, requested);

    match (current, requested) {
        (
            DifferentialPrivacyBudgetPolicyStatus::Draft,
            DifferentialPrivacyBudgetPolicyStatus::Draft,
        )
        | (
            DifferentialPrivacyBudgetPolicyStatus::Draft,
            DifferentialPrivacyBudgetPolicyStatus::Active,
        )
        | (
            DifferentialPrivacyBudgetPolicyStatus::Draft,
            DifferentialPrivacyBudgetPolicyStatus::Retired,
        )
        | (
            DifferentialPrivacyBudgetPolicyStatus::Active,
            DifferentialPrivacyBudgetPolicyStatus::Active,
        )
        | (
            DifferentialPrivacyBudgetPolicyStatus::Active,
            DifferentialPrivacyBudgetPolicyStatus::Superseded,
        )
        | (
            DifferentialPrivacyBudgetPolicyStatus::Active,
            DifferentialPrivacyBudgetPolicyStatus::Retired,
        )
        | (
            DifferentialPrivacyBudgetPolicyStatus::Superseded,
            DifferentialPrivacyBudgetPolicyStatus::Superseded,
        )
        | (
            DifferentialPrivacyBudgetPolicyStatus::Superseded,
            DifferentialPrivacyBudgetPolicyStatus::Retired,
        )
        | (
            DifferentialPrivacyBudgetPolicyStatus::Retired,
            DifferentialPrivacyBudgetPolicyStatus::Retired,
        ) => assert_eq!(result, Ok(requested)),
        _other => assert!(matches!(
            result,
            Err(DifferentialPrivacyBudgetPolicyLifecycleError::InvalidTransition)
        )),
    }
}

#[kani::proof]
pub(super) fn differential_privacy_noise_application_matches_checked_arithmetic() {
    let parameters = DifferentialPrivacyNoiseParameters::new(
        DifferentialPrivacyNoiseMechanism::DiscreteLaplace,
        super::DifferentialPrivacyAccountingScope::SubjectBudget,
        bounded_nonzero_u64(kani::any()),
        bounded_nonzero_u64(kani::any()),
    );
    let aggregate_value: i64 = kani::any();
    let noise_value: i64 = kani::any();

    let result = apply_differential_privacy_integer_noise(parameters, aggregate_value, noise_value);

    match aggregate_value.checked_add(noise_value) {
        Some(expected) => {
            let Ok(noised) = result else {
                assert!(false);
                return;
            };
            assert_eq!(noised.parameters(), parameters);
            assert_eq!(noised.aggregate_value(), aggregate_value);
            assert_eq!(noised.noise_value(), noise_value);
            assert_eq!(noised.noised_value(), expected);
            assert!(noised.noise_applied());
        }
        None => {
            assert!(matches!(
                result,
                Err(DifferentialPrivacyNoiseError::NoisedValueOverflow)
            ));
        }
    }
}

#[kani::proof]
pub(super) fn differential_privacy_release_gate_fails_closed() {
    let minimum_population = bounded_participant_count(kani::any());
    let max_epsilon_microunits = bounded_nonzero_u64(kani::any());
    let Ok(policy) =
        DifferentialPrivacyReleasePolicy::new(minimum_population, max_epsilon_microunits)
    else {
        assert!(false);
        return;
    };
    let aggregate_computed: bool = kani::any();
    let data_subject_count = usize::from(kani::any::<u8>() % 16);
    let epsilon_microunits = u64::from(kani::any::<u8>() % 18);
    let noise_applied: bool = kani::any();
    let context = DifferentialPrivacyReleaseContext::new(
        aggregate_computed,
        data_subject_count,
        epsilon_microunits,
        noise_applied,
    );

    let result = validate_differential_privacy_release(policy, context);

    if !aggregate_computed {
        assert!(matches!(
            result,
            Err(DifferentialPrivacyReleaseError::MissingComputedAggregate)
        ));
    } else if data_subject_count < minimum_population.get() {
        assert!(matches!(
            result,
            Err(DifferentialPrivacyReleaseError::PopulationBelowMinimum)
        ));
    } else if epsilon_microunits == 0 {
        assert!(matches!(
            result,
            Err(DifferentialPrivacyReleaseError::EpsilonBudgetMustBeNonZero)
        ));
    } else if epsilon_microunits > max_epsilon_microunits.get() {
        assert!(matches!(
            result,
            Err(DifferentialPrivacyReleaseError::EpsilonBudgetExceeded)
        ));
    } else if !noise_applied {
        assert!(matches!(
            result,
            Err(DifferentialPrivacyReleaseError::NoiseNotApplied)
        ));
    } else {
        assert!(result.is_ok());
    }
}

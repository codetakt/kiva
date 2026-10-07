#![allow(clippy::wildcard_imports)]

use super::*;

#[test]
fn rejects_single_subject_differential_privacy_policy() {
    let policy = DifferentialPrivacyReleasePolicy::new(NonZeroUsize::MIN, nonzero_u64(1));

    assert_eq!(
        policy,
        Err(DifferentialPrivacyPolicyError::MinimumPopulationTooSmall)
    );
}

#[test]
fn serde_rejects_single_subject_differential_privacy_policy() {
    let payload = "minimum_population: 1\nmax_epsilon_microunits: 1000000\n";
    let parsed: Result<DifferentialPrivacyReleasePolicy, serde_yaml::Error> =
        serde_yaml::from_str(payload);

    assert!(parsed.is_err());
}

#[test]
fn rejects_over_spent_differential_privacy_budget_ledger() {
    let ledger = DifferentialPrivacyBudgetLedger::new(nonzero_u64(1_000_000), 1_000_001);

    assert_eq!(
        ledger,
        Err(DifferentialPrivacyBudgetError::SpentBudgetExceedsTotal)
    );
}

#[test]
fn serde_rejects_over_spent_differential_privacy_budget_ledger() {
    let payload = "total_epsilon_microunits: 1000000\nspent_epsilon_microunits: 1000001\n";
    let parsed: Result<DifferentialPrivacyBudgetLedger, serde_yaml::Error> =
        serde_yaml::from_str(payload);

    assert!(parsed.is_err());
}

#[test]
fn differential_privacy_budget_spend_rejects_zero_request() {
    let ledger = differential_privacy_budget_ledger(1_000_000, 0);

    assert_eq!(
        spend_differential_privacy_budget(ledger, 0),
        Err(DifferentialPrivacyBudgetError::SpendRequestMustBeNonZero)
    );
}

#[test]
fn differential_privacy_budget_spend_rejects_remaining_budget_overflow() {
    let ledger = differential_privacy_budget_ledger(1_000_000, 750_000);

    assert_eq!(
        spend_differential_privacy_budget(ledger, 250_001),
        Err(DifferentialPrivacyBudgetError::SpendRequestExceedsRemaining)
    );
}

#[test]
fn differential_privacy_budget_spend_is_monotonic() {
    let ledger = differential_privacy_budget_ledger(1_000_000, 250_000);
    let spent = spend_differential_privacy_budget(ledger, 250_000);

    assert!(spent.is_ok());
    let Ok(spent) = spent else {
        return;
    };

    assert_eq!(spent.total_epsilon_microunits(), nonzero_u64(1_000_000));
    assert_eq!(spent.spent_epsilon_microunits(), 500_000);
    assert_eq!(spent.remaining_epsilon_microunits(), 500_000);
}

#[test]
fn differential_privacy_budget_spend_allows_exact_exhaustion() {
    let ledger = differential_privacy_budget_ledger(1_000_000, 750_000);
    let spent = spend_differential_privacy_budget(ledger, 250_000);

    assert!(spent.is_ok());
    let Ok(spent) = spent else {
        return;
    };

    assert_eq!(spent.remaining_epsilon_microunits(), 0);
    assert_eq!(
        spend_differential_privacy_budget(spent, 1),
        Err(DifferentialPrivacyBudgetError::SpendRequestExceedsRemaining)
    );
}

#[test]
fn differential_privacy_composition_summarizes_release_and_unattributed_spend() {
    let ledger = differential_privacy_budget_ledger(1_000_000, 600_000);

    let summary = summarize_differential_privacy_composition(ledger, &[250_000, 100_000, 200_000]);

    let Ok(summary) = summary else {
        panic!("composition summary should be valid");
    };
    assert_eq!(summary.ledger(), ledger);
    assert_eq!(summary.release_count(), 3);
    assert_eq!(summary.release_spent_epsilon_microunits(), 550_000);
    assert_eq!(
        summary.release_epsilon_square_sum_microunits2(),
        112_500_000_000
    );
    assert_eq!(summary.unattributed_spent_epsilon_microunits(), 50_000);
    assert_eq!(summary.max_release_epsilon_microunits(), 250_000);
}

#[test]
fn differential_privacy_composition_rejects_release_spend_above_ledger() {
    let ledger = differential_privacy_budget_ledger(1_000_000, 300_000);

    let summary = summarize_differential_privacy_composition(ledger, &[250_000, 100_000]);

    assert_eq!(
        summary,
        Err(DifferentialPrivacyCompositionError::ReleaseSpendExceedsBudgetSpend)
    );
}

#[test]
fn differential_privacy_advanced_composition_reports_conservative_bound() {
    let bound = bound_differential_privacy_advanced_composition(
        &[250_000, 100_000, 200_000],
        NonZeroU64::MIN,
    );

    assert!(bound.is_ok());
    let Ok(bound) = bound else {
        return;
    };

    assert_eq!(bound.release_count(), 3);
    assert_eq!(bound.delta_microunits(), NonZeroU64::MIN);
    assert_eq!(bound.basic_epsilon_microunits(), 550_000);
    assert!(bound.epsilon_upper_bound_microunits() >= bound.basic_epsilon_microunits());
}

#[test]
fn differential_privacy_advanced_composition_rejects_delta_one() {
    let delta_one = nonzero_u64(1_000_000);

    assert_eq!(
        bound_differential_privacy_advanced_composition(&[250_000], delta_one),
        Err(DifferentialPrivacyCompositionError::AdvancedCompositionDeltaOutOfRange)
    );
}

#[test]
fn differential_privacy_budget_policy_lifecycle_is_monotonic() {
    assert_eq!(
        transition_differential_privacy_budget_policy(
            DifferentialPrivacyBudgetPolicyStatus::Draft,
            DifferentialPrivacyBudgetPolicyStatus::Active,
        ),
        Ok(DifferentialPrivacyBudgetPolicyStatus::Active)
    );
    assert_eq!(
        transition_differential_privacy_budget_policy(
            DifferentialPrivacyBudgetPolicyStatus::Active,
            DifferentialPrivacyBudgetPolicyStatus::Superseded,
        ),
        Ok(DifferentialPrivacyBudgetPolicyStatus::Superseded)
    );
    assert_eq!(
        transition_differential_privacy_budget_policy(
            DifferentialPrivacyBudgetPolicyStatus::Superseded,
            DifferentialPrivacyBudgetPolicyStatus::Retired,
        ),
        Ok(DifferentialPrivacyBudgetPolicyStatus::Retired)
    );
    assert_eq!(
        transition_differential_privacy_budget_policy(
            DifferentialPrivacyBudgetPolicyStatus::Retired,
            DifferentialPrivacyBudgetPolicyStatus::Active,
        ),
        Err(DifferentialPrivacyBudgetPolicyLifecycleError::InvalidTransition)
    );
}

#[test]
fn differential_privacy_release_requires_computed_aggregate() {
    let policy = differential_privacy_policy(3, 1_000_000);
    let context = DifferentialPrivacyReleaseContext::new(false, 3, 1_000_000, true);

    assert_eq!(
        validate_differential_privacy_release(policy, context),
        Err(DifferentialPrivacyReleaseError::MissingComputedAggregate)
    );
}

#[test]
fn differential_privacy_release_rejects_small_population() {
    let policy = differential_privacy_policy(5, 1_000_000);
    let context = DifferentialPrivacyReleaseContext::new(true, 4, 1_000_000, true);

    assert_eq!(
        validate_differential_privacy_release(policy, context),
        Err(DifferentialPrivacyReleaseError::PopulationBelowMinimum)
    );
}

#[test]
fn differential_privacy_release_rejects_invalid_epsilon_budget() {
    let policy = differential_privacy_policy(3, 1_000_000);
    let zero_budget = DifferentialPrivacyReleaseContext::new(true, 3, 0, true);
    let over_budget = DifferentialPrivacyReleaseContext::new(true, 3, 1_000_001, true);

    assert_eq!(
        validate_differential_privacy_release(policy, zero_budget),
        Err(DifferentialPrivacyReleaseError::EpsilonBudgetMustBeNonZero)
    );
    assert_eq!(
        validate_differential_privacy_release(policy, over_budget),
        Err(DifferentialPrivacyReleaseError::EpsilonBudgetExceeded)
    );
}

#[test]
fn differential_privacy_release_requires_applied_noise() {
    let policy = differential_privacy_policy(3, 1_000_000);
    let context = DifferentialPrivacyReleaseContext::new(true, 3, 1_000_000, false);

    assert_eq!(
        validate_differential_privacy_release(policy, context),
        Err(DifferentialPrivacyReleaseError::NoiseNotApplied)
    );
}

#[test]
fn differential_privacy_release_accepts_complete_context() {
    let policy = differential_privacy_policy(3, 1_000_000);
    let context = DifferentialPrivacyReleaseContext::new(true, 3, 1_000_000, true);

    assert_eq!(
        validate_differential_privacy_release(policy, context),
        Ok(())
    );
}

#[test]
fn differential_privacy_noise_application_records_evidence() {
    let parameters = differential_privacy_noise_parameters(250_000, 1);
    let noised = apply_differential_privacy_integer_noise(parameters, 42, -7);

    assert!(noised.is_ok());
    let Ok(noised) = noised else {
        return;
    };

    assert_eq!(noised.parameters(), parameters);
    assert_eq!(noised.aggregate_value(), 42);
    assert_eq!(noised.noise_value(), -7);
    assert_eq!(noised.noised_value(), 35);
    assert!(noised.noise_applied());
    assert_eq!(
        noised.parameters().mechanism(),
        DifferentialPrivacyNoiseMechanism::DiscreteLaplace
    );
}

#[test]
fn differential_privacy_noise_application_rejects_overflow() {
    let parameters = differential_privacy_noise_parameters(250_000, 1);

    assert_eq!(
        apply_differential_privacy_integer_noise(parameters, i64::MAX, 1),
        Err(DifferentialPrivacyNoiseError::NoisedValueOverflow)
    );
    assert_eq!(
        apply_differential_privacy_integer_noise(parameters, i64::MIN, -1),
        Err(DifferentialPrivacyNoiseError::NoisedValueOverflow)
    );
}

#[test]
fn differential_privacy_discrete_laplace_sampler_is_deterministic() {
    let parameters = differential_privacy_noise_parameters(250_000, 1);

    let noise = sample_differential_privacy_discrete_laplace_noise_from_uniforms(
        parameters,
        u64::MAX,
        u64::MAX,
    );

    assert_eq!(noise, Ok(0));
}

#[test]
fn differential_privacy_discrete_laplace_sampler_rejects_unstable_parameters() {
    let parameters = differential_privacy_noise_parameters(1, u64::MAX);

    let noise = sample_differential_privacy_discrete_laplace_noise_from_uniforms(
        parameters,
        u64::MAX / 2,
        u64::MAX / 2,
    );

    assert_eq!(
        noise,
        Err(DifferentialPrivacyNoiseError::NoiseParameterOutOfRange)
    );
}

#[test]
#[allow(clippy::cast_precision_loss)]
fn differential_privacy_discrete_laplace_sampler_matches_distribution_smoke() {
    const SAMPLE_COUNT: u64 = 8_192;
    let parameters = differential_privacy_noise_parameters(1_000_000, 1);
    let alpha = std::f64::consts::E.recip();
    let expected_zero_probability = (1.0 - alpha) / (1.0 + alpha);
    let expected_mean_absolute = 2.0 * alpha / alpha.mul_add(-alpha, 1.0);
    let mut zero_count = 0_u64;
    let mut absolute_sum = 0_u64;

    for index in 0..SAMPLE_COUNT {
        let left = splitmix64(index);
        let right = splitmix64(index ^ 0x9e37_79b9_7f4a_7c15);
        let noise = sample_differential_privacy_discrete_laplace_noise_from_uniforms(
            parameters, left, right,
        );
        assert!(noise.is_ok());
        let Ok(noise) = noise else {
            return;
        };
        if noise == 0 {
            zero_count += 1;
        }
        absolute_sum += noise.unsigned_abs();
    }

    let observed_zero_probability = zero_count as f64 / SAMPLE_COUNT as f64;
    let observed_mean_absolute = absolute_sum as f64 / SAMPLE_COUNT as f64;
    assert!((observed_zero_probability - expected_zero_probability).abs() < 0.025);
    assert!((observed_mean_absolute - expected_mean_absolute).abs() < 0.08);
}

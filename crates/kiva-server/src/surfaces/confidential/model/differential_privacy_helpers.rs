#![allow(clippy::wildcard_imports)]

use super::*;

pub(crate) fn existing_differential_privacy_release_outcome(
    existing: ConfidentialDifferentialPrivacyReleaseState,
    principal: &AuthenticatedPrincipal,
    request: &ConfidentialDifferentialPrivacyReleaseRequest,
) -> Result<ConfidentialDifferentialPrivacyReleaseState, ConfidentialDifferentialPrivacyReleaseError>
{
    existing.ensure_owner(principal)?;
    if existing.same_request(principal, request)? {
        Ok(existing)
    } else {
        Err(ConfidentialDifferentialPrivacyReleaseError::Conflict)
    }
}

pub(crate) fn existing_differential_privacy_sampled_release_outcome(
    existing: ConfidentialDifferentialPrivacyReleaseState,
    principal: &AuthenticatedPrincipal,
    request: &ConfidentialDifferentialPrivacySampledReleaseRequest,
) -> Result<ConfidentialDifferentialPrivacyReleaseState, ConfidentialDifferentialPrivacyReleaseError>
{
    existing.ensure_owner(principal)?;
    if existing.same_sampled_request(principal, request) {
        Ok(existing)
    } else {
        Err(ConfidentialDifferentialPrivacyReleaseError::Conflict)
    }
}

pub(crate) fn sample_differential_privacy_noise(
    request: ConfidentialDifferentialPrivacySampledReleaseRequest,
) -> Result<i64, ConfidentialDifferentialPrivacyReleaseError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        tracing::error!(
            error = ?error,
            "failed to read OS randomness for differential privacy release"
        );
        ConfidentialDifferentialPrivacyReleaseError::RandomnessUnavailable
    })?;

    let mut left = [0_u8; 8];
    let mut right = [0_u8; 8];
    left.copy_from_slice(&bytes[..8]);
    right.copy_from_slice(&bytes[8..]);
    let parameters = DifferentialPrivacyNoiseParameters::new(
        DifferentialPrivacyNoiseMechanism::DiscreteLaplace,
        DifferentialPrivacyAccountingScope::SubjectBudget,
        request.epsilon_microunits,
        request.l1_sensitivity,
    );
    sample_differential_privacy_discrete_laplace_noise_from_uniforms(
        parameters,
        u64::from_le_bytes(left),
        u64::from_le_bytes(right),
    )
    .map_err(ConfidentialDifferentialPrivacyReleaseError::Noise)
}

pub(crate) fn differential_privacy_composition_summaries_from_metadata(
    budgets: Vec<ConfidentialDifferentialPrivacyBudgetState>,
    releases: &[ConfidentialDifferentialPrivacyReleaseState],
) -> Result<
    Vec<ConfidentialDifferentialPrivacyCompositionSummaryState>,
    ConfidentialDifferentialPrivacyBudgetError,
> {
    budgets
        .into_iter()
        .map(|budget| {
            let releases_for_budget = releases
                .iter()
                .filter(|release| release.budget_id == budget.budget_id)
                .cloned()
                .collect::<Vec<_>>();
            ConfidentialDifferentialPrivacyCompositionSummaryState::from_budget_and_releases(
                budget,
                &releases_for_budget,
            )
        })
        .collect()
}

pub(crate) fn differential_privacy_release_error_for_composition(
    error: ConfidentialDifferentialPrivacyReleaseError,
) -> ConfidentialDifferentialPrivacyBudgetError {
    match error {
        ConfidentialDifferentialPrivacyReleaseError::ApplicationAdmission(message) => {
            ConfidentialDifferentialPrivacyBudgetError::ApplicationAdmission(message)
        }
        ConfidentialDifferentialPrivacyReleaseError::Store(source) => {
            ConfidentialDifferentialPrivacyBudgetError::Store(source)
        }
        ConfidentialDifferentialPrivacyReleaseError::NotFound => {
            ConfidentialDifferentialPrivacyBudgetError::NotFound
        }
        ConfidentialDifferentialPrivacyReleaseError::Forbidden => {
            ConfidentialDifferentialPrivacyBudgetError::Forbidden
        }
        ConfidentialDifferentialPrivacyReleaseError::Budget(source) => {
            ConfidentialDifferentialPrivacyBudgetError::Budget(source)
        }
        ConfidentialDifferentialPrivacyReleaseError::Noise(_)
        | ConfidentialDifferentialPrivacyReleaseError::Policy(_)
        | ConfidentialDifferentialPrivacyReleaseError::Release(_)
        | ConfidentialDifferentialPrivacyReleaseError::Secure(_)
        | ConfidentialDifferentialPrivacyReleaseError::MissingComputedAggregate
        | ConfidentialDifferentialPrivacyReleaseError::DataSubjectCountMismatch
        | ConfidentialDifferentialPrivacyReleaseError::RandomnessUnavailable
        | ConfidentialDifferentialPrivacyReleaseError::Conflict => {
            ConfidentialDifferentialPrivacyBudgetError::Store(
                ConfidentialAggregateStoreError::InvalidStoredDifferentialPrivacyRelease,
            )
        }
    }
}

pub(crate) fn differential_privacy_budget_error_for_release(
    error: ConfidentialDifferentialPrivacyBudgetError,
) -> ConfidentialDifferentialPrivacyReleaseError {
    match error {
        ConfidentialDifferentialPrivacyBudgetError::ApplicationAdmission(message) => {
            ConfidentialDifferentialPrivacyReleaseError::ApplicationAdmission(message)
        }
        ConfidentialDifferentialPrivacyBudgetError::Budget(source) => {
            ConfidentialDifferentialPrivacyReleaseError::Budget(source)
        }
        ConfidentialDifferentialPrivacyBudgetError::Store(source) => {
            ConfidentialDifferentialPrivacyReleaseError::Store(source)
        }
        ConfidentialDifferentialPrivacyBudgetError::Composition(_source) => {
            ConfidentialDifferentialPrivacyReleaseError::Store(
                ConfidentialAggregateStoreError::InvalidStoredDifferentialPrivacyRelease,
            )
        }
        ConfidentialDifferentialPrivacyBudgetError::NotFound => {
            ConfidentialDifferentialPrivacyReleaseError::NotFound
        }
        ConfidentialDifferentialPrivacyBudgetError::Forbidden => {
            ConfidentialDifferentialPrivacyReleaseError::Forbidden
        }
    }
}

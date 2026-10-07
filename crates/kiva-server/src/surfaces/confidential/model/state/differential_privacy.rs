//! Confidential differential-privacy server-local state.

#![allow(clippy::wildcard_imports)]

use super::super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfidentialDifferentialPrivacyReleaseState {
    pub(crate) release_id: SecureAggregateTranscriptId,
    pub(crate) budget_id: SecureAggregateTranscriptId,
    pub(crate) transcript_id: SecureAggregateTranscriptId,
    pub(crate) owner_subject: String,
    pub(crate) noised_aggregate: DifferentialPrivacyNoisedAggregate,
    pub(crate) data_subject_count: NonZeroUsize,
    pub(crate) minimum_population: NonZeroUsize,
    pub(crate) created_at: OffsetDateTime,
}

impl ConfidentialDifferentialPrivacyReleaseState {
    pub(crate) fn from_request(
        owner_subject: String,
        request: ConfidentialDifferentialPrivacyReleaseRequest,
        aggregate_result: &ConfidentialAdditiveAggregateResult,
    ) -> Result<Self, ConfidentialDifferentialPrivacyReleaseError> {
        ensure_matching_differential_privacy_aggregate_result(Some(aggregate_result), &request)?;
        if request.data_subject_count != aggregate_result.participant_count {
            return Err(ConfidentialDifferentialPrivacyReleaseError::DataSubjectCountMismatch);
        }

        let parameters = DifferentialPrivacyNoiseParameters::new(
            DifferentialPrivacyNoiseMechanism::DiscreteLaplace,
            DifferentialPrivacyAccountingScope::SubjectBudget,
            request.epsilon_microunits,
            request.l1_sensitivity,
        );
        let noised_aggregate = apply_differential_privacy_integer_noise(
            parameters,
            field_element_i64(request.aggregate)
                .map_err(ConfidentialDifferentialPrivacyReleaseError::Secure)?,
            request.noise_value,
        )
        .map_err(ConfidentialDifferentialPrivacyReleaseError::Noise)?;
        let policy = DifferentialPrivacyReleasePolicy::new(
            request.minimum_population,
            request.epsilon_microunits,
        )
        .map_err(ConfidentialDifferentialPrivacyReleaseError::Policy)?;
        let context = DifferentialPrivacyReleaseContext::new(
            true,
            request.data_subject_count.get(),
            request.epsilon_microunits.get(),
            noised_aggregate.noise_applied(),
        );
        validate_differential_privacy_release(policy, context)
            .map_err(ConfidentialDifferentialPrivacyReleaseError::Release)?;

        Ok(Self {
            release_id: request.release_id,
            budget_id: request.budget_id,
            transcript_id: request.transcript_id,
            owner_subject,
            noised_aggregate,
            data_subject_count: request.data_subject_count,
            minimum_population: request.minimum_population,
            created_at: OffsetDateTime::now_utc(),
        })
    }

    pub(crate) fn from_sampled_request(
        owner_subject: String,
        request: ConfidentialDifferentialPrivacySampledReleaseRequest,
        aggregate_result: &ConfidentialAdditiveAggregateResult,
        noise_value: i64,
    ) -> Result<Self, ConfidentialDifferentialPrivacyReleaseError> {
        Self::from_request(
            owner_subject,
            request.into_release_request(aggregate_result.aggregate, noise_value),
            aggregate_result,
        )
    }

    pub(crate) fn ensure_owner(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<(), ConfidentialDifferentialPrivacyReleaseError> {
        if self.owner_subject == principal.subject {
            Ok(())
        } else {
            Err(ConfidentialDifferentialPrivacyReleaseError::Forbidden)
        }
    }

    pub(crate) fn same_request(
        &self,
        principal: &AuthenticatedPrincipal,
        request: &ConfidentialDifferentialPrivacyReleaseRequest,
    ) -> Result<bool, ConfidentialDifferentialPrivacyReleaseError> {
        let parameters = self.noised_aggregate.parameters();
        let request_aggregate = field_element_i64(request.aggregate)
            .map_err(ConfidentialDifferentialPrivacyReleaseError::Secure)?;
        let same_owner = self.owner_subject == principal.subject;
        let same_ids = self.release_id == request.release_id
            && self.budget_id == request.budget_id
            && self.transcript_id == request.transcript_id;
        let same_noise_evidence = self.noised_aggregate.aggregate_value() == request_aggregate
            && self.noised_aggregate.noise_value() == request.noise_value;
        let same_parameters = parameters.mechanism()
            == DifferentialPrivacyNoiseMechanism::DiscreteLaplace
            && parameters.accounting_scope() == DifferentialPrivacyAccountingScope::SubjectBudget
            && parameters.epsilon_microunits() == request.epsilon_microunits
            && parameters.l1_sensitivity() == request.l1_sensitivity;
        let same_population = self.data_subject_count == request.data_subject_count
            && self.minimum_population == request.minimum_population;
        Ok(same_owner && same_ids && same_noise_evidence && same_parameters && same_population)
    }

    pub(crate) fn same_sampled_request(
        &self,
        principal: &AuthenticatedPrincipal,
        request: &ConfidentialDifferentialPrivacySampledReleaseRequest,
    ) -> bool {
        let parameters = self.noised_aggregate.parameters();
        let same_owner = self.owner_subject == principal.subject;
        let same_ids = self.release_id == request.release_id
            && self.budget_id == request.budget_id
            && self.transcript_id == request.transcript_id;
        let same_parameters = matches!(
            parameters.mechanism(),
            DifferentialPrivacyNoiseMechanism::DiscreteLaplace
        ) && matches!(
            parameters.accounting_scope(),
            DifferentialPrivacyAccountingScope::SubjectBudget
        ) && parameters.epsilon_microunits().get()
            == request.epsilon_microunits.get()
            && parameters.l1_sensitivity().get() == request.l1_sensitivity.get();
        let same_population = self.data_subject_count.get() == request.data_subject_count.get()
            && self.minimum_population.get() == request.minimum_population.get();
        same_owner && same_ids && same_parameters && same_population
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfidentialDifferentialPrivacyBudgetState {
    pub(crate) budget_id: SecureAggregateTranscriptId,
    pub(crate) owner_subject: String,
    pub(crate) ledger: DifferentialPrivacyBudgetLedger,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) updated_at: OffsetDateTime,
}

impl ConfidentialDifferentialPrivacyBudgetState {
    pub(crate) fn from_request(
        principal: &AuthenticatedPrincipal,
        request: ConfidentialDifferentialPrivacyBudgetRequest,
    ) -> Result<Self, ConfidentialDifferentialPrivacyBudgetError> {
        let ledger = DifferentialPrivacyBudgetLedger::new(request.total_epsilon_microunits, 0)
            .map_err(ConfidentialDifferentialPrivacyBudgetError::Budget)?;
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            budget_id: request.budget_id,
            owner_subject: principal.subject.clone(),
            ledger,
            created_at: now,
            updated_at: now,
        })
    }

    pub(crate) fn same_registration(&self, other: &Self) -> bool {
        self.budget_id == other.budget_id
            && self.owner_subject == other.owner_subject
            && self.ledger.total_epsilon_microunits() == other.ledger.total_epsilon_microunits()
    }

    pub(crate) fn ensure_owner(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<(), ConfidentialDifferentialPrivacyBudgetError> {
        if self.owner_subject == principal.subject {
            Ok(())
        } else {
            Err(ConfidentialDifferentialPrivacyBudgetError::Forbidden)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfidentialDifferentialPrivacyCompositionSummaryState {
    pub(crate) budget_id: SecureAggregateTranscriptId,
    pub(crate) owner_subject: String,
    pub(crate) summary: DifferentialPrivacyCompositionSummary,
    pub(crate) advanced: DifferentialPrivacyAdvancedCompositionBound,
}

impl ConfidentialDifferentialPrivacyCompositionSummaryState {
    pub(crate) fn from_budget_and_releases(
        budget: ConfidentialDifferentialPrivacyBudgetState,
        releases: &[ConfidentialDifferentialPrivacyReleaseState],
    ) -> Result<Self, ConfidentialDifferentialPrivacyBudgetError> {
        let release_epsilon_microunits = releases
            .iter()
            .map(|release| {
                release
                    .noised_aggregate
                    .parameters()
                    .epsilon_microunits()
                    .get()
            })
            .collect::<Vec<_>>();
        let summary =
            summarize_differential_privacy_composition(budget.ledger, &release_epsilon_microunits)
                .map_err(ConfidentialDifferentialPrivacyBudgetError::Composition)?;
        let advanced = bound_differential_privacy_advanced_composition(
            &release_epsilon_microunits,
            CONFIDENTIAL_DIFFERENTIAL_PRIVACY_ADVANCED_DELTA_MICROUNITS,
        )
        .map_err(ConfidentialDifferentialPrivacyBudgetError::Composition)?;
        Ok(Self {
            budget_id: budget.budget_id,
            owner_subject: budget.owner_subject,
            summary,
            advanced,
        })
    }
}

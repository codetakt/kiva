#![allow(clippy::wildcard_imports)]

use super::*;

impl MemoryConfidentialRepository {
    pub(crate) async fn register_differential_privacy_budget(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialDifferentialPrivacyBudgetRequest,
    ) -> Result<StoreOutcome, ConfidentialDifferentialPrivacyBudgetError> {
        let budget = ConfidentialDifferentialPrivacyBudgetState::from_request(principal, request)?;
        let key = ConfidentialDifferentialPrivacyBudgetKey::from_budget_id(budget.budget_id);
        let mut budgets = self.differential_privacy.budgets.write().await;
        let outcome = match budgets.get(&key) {
            Some(existing) if existing.same_registration(&budget) => StoreOutcome::Unchanged,
            Some(_existing) => StoreOutcome::Conflict,
            None => {
                budgets.insert(key, budget);
                StoreOutcome::Stored
            }
        };
        drop(budgets);
        Ok(outcome)
    }

    pub(crate) async fn spend_differential_privacy_budget(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialDifferentialPrivacyBudgetSpendRequest,
    ) -> Result<
        ConfidentialDifferentialPrivacyBudgetState,
        ConfidentialDifferentialPrivacyBudgetError,
    > {
        let key = ConfidentialDifferentialPrivacyBudgetKey::from_budget_id(request.budget_id);
        let mut budgets = self.differential_privacy.budgets.write().await;
        let Some(budget) = budgets.get_mut(&key) else {
            return Err(ConfidentialDifferentialPrivacyBudgetError::NotFound);
        };
        budget.ensure_owner(principal)?;
        budget.ledger = spend_differential_privacy_budget_ledger(
            budget.ledger,
            request.epsilon_microunits.get(),
        )
        .map_err(ConfidentialDifferentialPrivacyBudgetError::Budget)?;
        budget.updated_at = OffsetDateTime::now_utc();
        let budget = budget.clone();
        drop(budgets);
        Ok(budget)
    }

    pub(crate) async fn list_differential_privacy_budgets(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialDifferentialPrivacyBudgetFilter,
    ) -> Result<
        Vec<ConfidentialDifferentialPrivacyBudgetState>,
        ConfidentialDifferentialPrivacyBudgetError,
    > {
        let mut budgets = {
            let budgets = self.differential_privacy.budgets.read().await;
            budgets
                .values()
                .filter(|budget| {
                    budget.owner_subject == principal.subject && filter.matches(budget)
                })
                .cloned()
                .collect::<Vec<_>>()
        };
        budgets.sort_by_key(|budget| (budget.created_at, budget.budget_id.bytes()));
        Ok(budgets)
    }

    pub(crate) async fn list_differential_privacy_composition_summaries(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialDifferentialPrivacyBudgetFilter,
    ) -> Result<
        Vec<ConfidentialDifferentialPrivacyCompositionSummaryState>,
        ConfidentialDifferentialPrivacyBudgetError,
    > {
        let budgets = self
            .list_differential_privacy_budgets(principal, filter)
            .await?;
        let releases = {
            let releases = self.differential_privacy.releases.read().await;
            releases
                .values()
                .filter(|release| release.owner_subject == principal.subject)
                .cloned()
                .collect::<Vec<_>>()
        };
        differential_privacy_composition_summaries_from_metadata(budgets, &releases)
    }

    pub(crate) async fn release_differential_privacy_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialDifferentialPrivacyReleaseRequest,
    ) -> Result<
        ConfidentialDifferentialPrivacyReleaseState,
        ConfidentialDifferentialPrivacyReleaseError,
    > {
        let release_key =
            ConfidentialDifferentialPrivacyReleaseKey::from_release_id(request.release_id);
        let mut releases = self.differential_privacy.releases.write().await;
        if let Some(existing) = releases.get(&release_key) {
            existing.ensure_owner(principal)?;
            return if existing.same_request(principal, &request)? {
                Ok(existing.clone())
            } else {
                Err(ConfidentialDifferentialPrivacyReleaseError::Conflict)
            };
        }

        let aggregate_result = {
            let aggregate_results = self.additive.aggregate_results.read().await;
            let result = aggregate_results
                .get(&ConfidentialAggregateResultKey::from_transcript_id(
                    request.transcript_id,
                ))
                .cloned();
            drop(aggregate_results);
            result.ok_or(ConfidentialDifferentialPrivacyReleaseError::MissingComputedAggregate)?
        };

        let mut budgets = self.differential_privacy.budgets.write().await;
        let budget_key =
            ConfidentialDifferentialPrivacyBudgetKey::from_budget_id(request.budget_id);
        let Some(budget) = budgets.get_mut(&budget_key) else {
            return Err(ConfidentialDifferentialPrivacyReleaseError::NotFound);
        };
        if budget.owner_subject != principal.subject {
            return Err(ConfidentialDifferentialPrivacyReleaseError::Forbidden);
        }

        let release = ConfidentialDifferentialPrivacyReleaseState::from_request(
            budget.owner_subject.clone(),
            request,
            &aggregate_result,
        )?;
        let spent_ledger = spend_differential_privacy_budget_ledger(
            budget.ledger,
            request.epsilon_microunits.get(),
        )
        .map_err(ConfidentialDifferentialPrivacyReleaseError::Budget)?;
        budget.ledger = spent_ledger;
        budget.updated_at = OffsetDateTime::now_utc();
        releases.insert(release_key, release.clone());
        drop(budgets);
        drop(releases);
        Ok(release)
    }

    pub(crate) async fn release_sampled_differential_privacy_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialDifferentialPrivacySampledReleaseRequest,
    ) -> Result<
        ConfidentialDifferentialPrivacyReleaseState,
        ConfidentialDifferentialPrivacyReleaseError,
    > {
        let release_key =
            ConfidentialDifferentialPrivacyReleaseKey::from_release_id(request.release_id);
        let mut releases = self.differential_privacy.releases.write().await;
        if let Some(existing) = releases.get(&release_key) {
            existing.ensure_owner(principal)?;
            return if existing.same_sampled_request(principal, &request) {
                Ok(existing.clone())
            } else {
                Err(ConfidentialDifferentialPrivacyReleaseError::Conflict)
            };
        }

        let aggregate_result = {
            let aggregate_results = self.additive.aggregate_results.read().await;
            let result = aggregate_results
                .get(&ConfidentialAggregateResultKey::from_transcript_id(
                    request.transcript_id,
                ))
                .cloned();
            drop(aggregate_results);
            result.ok_or(ConfidentialDifferentialPrivacyReleaseError::MissingComputedAggregate)?
        };

        let mut budgets = self.differential_privacy.budgets.write().await;
        let budget_key =
            ConfidentialDifferentialPrivacyBudgetKey::from_budget_id(request.budget_id);
        let Some(budget) = budgets.get_mut(&budget_key) else {
            return Err(ConfidentialDifferentialPrivacyReleaseError::NotFound);
        };
        if budget.owner_subject != principal.subject {
            return Err(ConfidentialDifferentialPrivacyReleaseError::Forbidden);
        }

        let noise_value = sample_differential_privacy_noise(request)?;
        let release = ConfidentialDifferentialPrivacyReleaseState::from_sampled_request(
            budget.owner_subject.clone(),
            request,
            &aggregate_result,
            noise_value,
        )?;
        let spent_ledger = spend_differential_privacy_budget_ledger(
            budget.ledger,
            request.epsilon_microunits.get(),
        )
        .map_err(ConfidentialDifferentialPrivacyReleaseError::Budget)?;
        budget.ledger = spent_ledger;
        budget.updated_at = OffsetDateTime::now_utc();
        releases.insert(release_key, release.clone());
        drop(budgets);
        drop(releases);
        Ok(release)
    }

    pub(crate) async fn list_differential_privacy_releases(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialDifferentialPrivacyReleaseFilter,
    ) -> Result<
        Vec<ConfidentialDifferentialPrivacyReleaseState>,
        ConfidentialDifferentialPrivacyReleaseError,
    > {
        let mut releases = {
            let releases = self.differential_privacy.releases.read().await;
            releases
                .values()
                .filter(|release| {
                    release.owner_subject == principal.subject && filter.matches(release)
                })
                .cloned()
                .collect::<Vec<_>>()
        };
        releases.sort_by_key(|release| (release.created_at, release.release_id.bytes()));
        Ok(releases)
    }
}

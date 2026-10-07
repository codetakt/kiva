#![allow(clippy::wildcard_imports)]

use super::*;

impl PostgresConfidentialRepository {
    pub(crate) async fn register_differential_privacy_budget(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialDifferentialPrivacyBudgetRequest,
    ) -> Result<StoreOutcome, ConfidentialDifferentialPrivacyBudgetError> {
        let budget = ConfidentialDifferentialPrivacyBudgetState::from_request(principal, request)?;
        let result = sqlx::query(
            "INSERT INTO kiva.confidential_differential_privacy_budgets \
             (tenant_id, budget_id, owner_subject, total_epsilon_microunits, spent_epsilon_microunits) \
             VALUES ($1, $2, $3, ($4)::numeric, ($5)::numeric) \
             ON CONFLICT (tenant_id, budget_id) DO NOTHING",
        )
        .bind(&self.tenant_id)
        .bind(budget.budget_id.bytes().to_vec())
        .bind(&budget.owner_subject)
        .bind(budget.ledger.total_epsilon_microunits().get().to_string())
        .bind(budget.ledger.spent_epsilon_microunits().to_string())
        .execute(&self.pool)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialDifferentialPrivacyBudgetError::Store)?;

        if InsertOutcome::from_rows_affected(result.rows_affected()) == InsertOutcome::Inserted {
            return Ok(StoreOutcome::Stored);
        }

        let existing =
            select_differential_privacy_budget(&self.pool, &self.tenant_id, budget.budget_id)
                .await?
                .ok_or(ConfidentialDifferentialPrivacyBudgetError::NotFound)?;
        if existing.same_registration(&budget) {
            Ok(StoreOutcome::Unchanged)
        } else {
            Ok(StoreOutcome::Conflict)
        }
    }

    pub(crate) async fn spend_differential_privacy_budget(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialDifferentialPrivacyBudgetSpendRequest,
    ) -> Result<
        ConfidentialDifferentialPrivacyBudgetState,
        ConfidentialDifferentialPrivacyBudgetError,
    > {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialDifferentialPrivacyBudgetError::Store)?;
        let budget = select_differential_privacy_budget_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.budget_id,
        )
        .await?
        .ok_or(ConfidentialDifferentialPrivacyBudgetError::NotFound)?;
        budget.ensure_owner(principal)?;
        let spent_ledger = spend_differential_privacy_budget_ledger(
            budget.ledger,
            request.epsilon_microunits.get(),
        )
        .map_err(ConfidentialDifferentialPrivacyBudgetError::Budget)?;
        let row = sqlx::query_as::<_, ConfidentialDifferentialPrivacyBudgetRow>(
            "UPDATE kiva.confidential_differential_privacy_budgets \
             SET spent_epsilon_microunits = ($3)::numeric, updated_at = now() \
             WHERE tenant_id = $1 AND budget_id = $2 \
             RETURNING budget_id, owner_subject, total_epsilon_microunits::text AS total_epsilon_microunits, \
              spent_epsilon_microunits::text AS spent_epsilon_microunits, created_at, updated_at",
        )
        .bind(&self.tenant_id)
        .bind(request.budget_id.bytes().to_vec())
        .bind(spent_ledger.spent_epsilon_microunits().to_string())
        .fetch_one(&mut *transaction)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialDifferentialPrivacyBudgetError::Store)?;
        transaction
            .commit()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialDifferentialPrivacyBudgetError::Store)?;
        differential_privacy_budget_from_row(row)
            .map_err(ConfidentialDifferentialPrivacyBudgetError::Store)
    }

    pub(crate) async fn list_differential_privacy_budgets(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialDifferentialPrivacyBudgetFilter,
    ) -> Result<
        Vec<ConfidentialDifferentialPrivacyBudgetState>,
        ConfidentialDifferentialPrivacyBudgetError,
    > {
        let rows = if let Some(budget_id) = filter.budget_id {
            sqlx::query_as::<_, ConfidentialDifferentialPrivacyBudgetRow>(
                "SELECT budget_id, owner_subject, total_epsilon_microunits::text AS total_epsilon_microunits, \
                 spent_epsilon_microunits::text AS spent_epsilon_microunits, created_at, updated_at \
                 FROM kiva.confidential_differential_privacy_budgets \
                 WHERE tenant_id = $1 AND owner_subject = $2 AND budget_id = $3 \
                 ORDER BY created_at ASC, budget_id ASC",
            )
            .bind(&self.tenant_id)
            .bind(&principal.subject)
            .bind(budget_id.bytes().to_vec())
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, ConfidentialDifferentialPrivacyBudgetRow>(
                "SELECT budget_id, owner_subject, total_epsilon_microunits::text AS total_epsilon_microunits, \
                 spent_epsilon_microunits::text AS spent_epsilon_microunits, created_at, updated_at \
                 FROM kiva.confidential_differential_privacy_budgets \
                 WHERE tenant_id = $1 AND owner_subject = $2 \
                 ORDER BY created_at ASC, budget_id ASC",
            )
            .bind(&self.tenant_id)
            .bind(&principal.subject)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(confidential_store)
        .map_err(ConfidentialDifferentialPrivacyBudgetError::Store)?;

        rows.into_iter()
            .map(differential_privacy_budget_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ConfidentialDifferentialPrivacyBudgetError::Store)
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
        let releases = self
            .list_differential_privacy_releases(
                principal,
                ConfidentialDifferentialPrivacyReleaseFilter {
                    release: None,
                    budget: filter.budget_id,
                    transcript: None,
                },
            )
            .await
            .map_err(differential_privacy_release_error_for_composition)?;
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
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialDifferentialPrivacyReleaseError::Store)?;

        if let Some(existing) = select_differential_privacy_release_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.release_id,
        )
        .await?
        {
            return existing_differential_privacy_release_outcome(existing, principal, &request);
        }

        let budget = select_differential_privacy_budget_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.budget_id,
        )
        .await
        .map_err(differential_privacy_budget_error_for_release)?
        .ok_or(ConfidentialDifferentialPrivacyReleaseError::NotFound)?;
        if budget.owner_subject != principal.subject {
            return Err(ConfidentialDifferentialPrivacyReleaseError::Forbidden);
        }

        if let Some(existing) = select_differential_privacy_release_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.release_id,
        )
        .await?
        {
            return existing_differential_privacy_release_outcome(existing, principal, &request);
        }

        let aggregate_result = select_confidential_additive_aggregate_result_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.transcript_id,
        )
        .await
        .map_err(ConfidentialDifferentialPrivacyReleaseError::Store)?
        .ok_or(ConfidentialDifferentialPrivacyReleaseError::MissingComputedAggregate)?;
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
        update_differential_privacy_budget_spend_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.budget_id,
            spent_ledger,
        )
        .await?;
        let release = insert_differential_privacy_release_in_transaction(
            &mut transaction,
            &self.tenant_id,
            &release,
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialDifferentialPrivacyReleaseError::Store)?;
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
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialDifferentialPrivacyReleaseError::Store)?;

        if let Some(existing) = select_differential_privacy_release_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.release_id,
        )
        .await?
        {
            return existing_differential_privacy_sampled_release_outcome(
                existing, principal, &request,
            );
        }

        let budget = select_differential_privacy_budget_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.budget_id,
        )
        .await
        .map_err(differential_privacy_budget_error_for_release)?
        .ok_or(ConfidentialDifferentialPrivacyReleaseError::NotFound)?;
        if budget.owner_subject != principal.subject {
            return Err(ConfidentialDifferentialPrivacyReleaseError::Forbidden);
        }

        if let Some(existing) = select_differential_privacy_release_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.release_id,
        )
        .await?
        {
            return existing_differential_privacy_sampled_release_outcome(
                existing, principal, &request,
            );
        }

        let aggregate_result = select_confidential_additive_aggregate_result_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.transcript_id,
        )
        .await
        .map_err(ConfidentialDifferentialPrivacyReleaseError::Store)?
        .ok_or(ConfidentialDifferentialPrivacyReleaseError::MissingComputedAggregate)?;
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
        update_differential_privacy_budget_spend_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.budget_id,
            spent_ledger,
        )
        .await?;
        let release = insert_differential_privacy_release_in_transaction(
            &mut transaction,
            &self.tenant_id,
            &release,
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialDifferentialPrivacyReleaseError::Store)?;
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
        let rows = sqlx::query_as::<_, ConfidentialDifferentialPrivacyReleaseRow>(
            "SELECT release_id, budget_id, transcript_id, owner_subject, aggregate_value, \
             noise_value, noised_value, mechanism, accounting_scope, epsilon_microunits::text AS epsilon_microunits, \
             l1_sensitivity::text AS l1_sensitivity, data_subject_count, minimum_population, created_at \
             FROM kiva.confidential_differential_privacy_releases \
             WHERE tenant_id = $1 AND owner_subject = $2 \
              AND ($3::bytea IS NULL OR release_id = $3) \
              AND ($4::bytea IS NULL OR budget_id = $4) \
              AND ($5::bytea IS NULL OR transcript_id = $5) \
             ORDER BY created_at ASC, release_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(&principal.subject)
        .bind(filter.release.map(|release_id| release_id.bytes().to_vec()))
        .bind(filter.budget.map(|budget_id| budget_id.bytes().to_vec()))
        .bind(
            filter
                .transcript
                .map(|transcript_id| transcript_id.bytes().to_vec()),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialDifferentialPrivacyReleaseError::Store)?;

        rows.into_iter()
            .map(differential_privacy_release_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ConfidentialDifferentialPrivacyReleaseError::Store)
    }
}

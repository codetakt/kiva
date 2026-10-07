#![allow(clippy::wildcard_imports)]

use super::*;

pub(crate) async fn update_differential_privacy_budget_spend_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    budget_id: SecureAggregateTranscriptId,
    spent_ledger: DifferentialPrivacyBudgetLedger,
) -> Result<(), ConfidentialDifferentialPrivacyReleaseError> {
    sqlx::query(
        "UPDATE kiva.confidential_differential_privacy_budgets \
         SET spent_epsilon_microunits = ($3)::numeric, updated_at = now() \
         WHERE tenant_id = $1 AND budget_id = $2",
    )
    .bind(tenant_id)
    .bind(budget_id.bytes().to_vec())
    .bind(spent_ledger.spent_epsilon_microunits().to_string())
    .execute(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialDifferentialPrivacyReleaseError::Store)?;
    Ok(())
}

pub(crate) async fn insert_differential_privacy_release_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    release: &ConfidentialDifferentialPrivacyReleaseState,
) -> Result<ConfidentialDifferentialPrivacyReleaseState, ConfidentialDifferentialPrivacyReleaseError>
{
    let parameters = release.noised_aggregate.parameters();
    let row = sqlx::query_as::<_, ConfidentialDifferentialPrivacyReleaseRow>(
        "INSERT INTO kiva.confidential_differential_privacy_releases \
         (tenant_id, release_id, budget_id, transcript_id, owner_subject, \
          aggregate_value, noise_value, noised_value, mechanism, accounting_scope, \
          epsilon_microunits, l1_sensitivity, data_subject_count, minimum_population) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'discrete-laplace', 'subject-budget', \
          ($9)::numeric, ($10)::numeric, $11, $12) \
         ON CONFLICT (tenant_id, release_id) DO NOTHING \
         RETURNING release_id, budget_id, transcript_id, owner_subject, aggregate_value, \
          noise_value, noised_value, mechanism, accounting_scope, \
          epsilon_microunits::text AS epsilon_microunits, \
          l1_sensitivity::text AS l1_sensitivity, data_subject_count, minimum_population, created_at",
    )
    .bind(tenant_id)
    .bind(release.release_id.bytes().to_vec())
    .bind(release.budget_id.bytes().to_vec())
    .bind(release.transcript_id.bytes().to_vec())
    .bind(&release.owner_subject)
    .bind(release.noised_aggregate.aggregate_value())
    .bind(release.noised_aggregate.noise_value())
    .bind(release.noised_aggregate.noised_value())
    .bind(parameters.epsilon_microunits().get().to_string())
    .bind(parameters.l1_sensitivity().get().to_string())
    .bind(
        nonzero_usize_i64(release.data_subject_count)
            .map_err(ConfidentialDifferentialPrivacyReleaseError::Store)?,
    )
    .bind(
        nonzero_usize_i64(release.minimum_population)
            .map_err(ConfidentialDifferentialPrivacyReleaseError::Store)?,
    )
    .fetch_optional(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialDifferentialPrivacyReleaseError::Store)?;

    let row = row.ok_or(ConfidentialDifferentialPrivacyReleaseError::Conflict)?;
    differential_privacy_release_from_row(row)
        .map_err(ConfidentialDifferentialPrivacyReleaseError::Store)
}

#![allow(clippy::wildcard_imports)]

use super::super::{mapping::*, postgres_queries::*, *};

impl PostgresPdsRepository {
    pub(crate) async fn register_governance_profile(
        &self,
        projection: PdsGovernanceProfileProjection,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(PdsRepositoryError::store)?;
        let profile = projection.state();
        let result = sqlx::query(
            "INSERT INTO kiva.pds_governance_profiles \
             (tenant_id, profile_id, profile_owner_subject, profile_kind, tenant_partition, \
              subject_partition, data_custodian_subject, platform_operator_subject, \
              policy_authority_subject, analyst_subject, auditor_subject, \
              identity_authority_subject, emergency_access_policy, runbook_refs) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) \
             ON CONFLICT (tenant_id, profile_id) DO NOTHING",
        )
        .bind(&self.tenant_id)
        .bind(profile.profile_id)
        .bind(&profile.profile_owner_subject)
        .bind(profile.profile_kind.label())
        .bind(&profile.tenant_partition)
        .bind(&profile.subject_partition)
        .bind(&profile.data_custodian_subject)
        .bind(&profile.platform_operator_subject)
        .bind(&profile.policy_authority_subject)
        .bind(&profile.analyst_subject)
        .bind(&profile.auditor_subject)
        .bind(&profile.identity_authority_subject)
        .bind(&profile.emergency_access_policy)
        .bind(&profile.runbook_refs)
        .execute(&mut *transaction)
        .await
        .map_err(PdsRepositoryError::store)?;

        let outcome = if InsertOutcome::from_rows_affected(result.rows_affected())
            == InsertOutcome::Inserted
        {
            let ledger_event = projection.ledger_event();
            insert_pds_ledger_event_in_transaction(
                &mut transaction,
                &self.tenant_id,
                &ledger_event,
            )
            .await?;
            StoreOutcome::Stored
        } else {
            let existing = select_pds_governance_profile_in_transaction(
                &mut transaction,
                &self.tenant_id,
                profile.profile_id,
            )
            .await?
            .ok_or_else(|| {
                PdsRepositoryError::InvalidRequest(
                    "PDS governance profile was not found".to_owned(),
                )
            })?;
            if existing.same_registration(profile) {
                StoreOutcome::Unchanged
            } else {
                StoreOutcome::Conflict
            }
        };
        transaction
            .commit()
            .await
            .map_err(PdsRepositoryError::store)?;
        Ok(outcome)
    }

    pub(crate) async fn list_governance_profiles(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsGovernanceProfileFilter,
    ) -> Result<Vec<PdsGovernanceProfileState>, PdsRepositoryError> {
        let rows = sqlx::query_as::<_, PdsGovernanceProfileRow>(
            "SELECT profile_id, profile_owner_subject, profile_kind, tenant_partition, \
              subject_partition, data_custodian_subject, platform_operator_subject, \
              policy_authority_subject, analyst_subject, auditor_subject, \
              identity_authority_subject, emergency_access_policy, runbook_refs, created_at \
             FROM kiva.pds_governance_profiles \
             WHERE tenant_id = $1 AND profile_owner_subject = $2 \
              AND ($3::uuid IS NULL OR profile_id = $3) \
             ORDER BY created_at ASC, profile_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(&principal.subject)
        .bind(filter.profile_id)
        .fetch_all(&self.pool)
        .await
        .map_err(PdsRepositoryError::store)?;

        rows.into_iter()
            .map(pds_governance_profile_from_row)
            .collect()
    }
}

#![allow(clippy::wildcard_imports)]

use super::super::{mapping::*, postgres_queries::*, *};

mod cases;

impl PostgresPdsRepository {
    pub(crate) async fn register_lifecycle_event(
        &self,
        projection: PdsLifecycleEventProjection,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(PdsRepositoryError::store)?;
        let event = projection.state();
        let result = sqlx::query(
            "INSERT INTO kiva.pds_lifecycle_events \
             (tenant_id, event_id, data_subject, event_type, resource_kind, resource_id, reason) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (tenant_id, event_id) DO NOTHING",
        )
        .bind(&self.tenant_id)
        .bind(event.event_id)
        .bind(&event.data_subject)
        .bind(event.event_type.label())
        .bind(event.resource_kind.label())
        .bind(&event.resource_id)
        .bind(&event.reason)
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
            let existing = select_pds_lifecycle_event_in_transaction(
                &mut transaction,
                &self.tenant_id,
                event.event_id,
            )
            .await?;
            let Some(existing) = existing else {
                return Err(PdsRepositoryError::InvalidRequest(
                    "PDS lifecycle event was not found".to_owned(),
                ));
            };
            if existing.same_registration(event) {
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

    pub(crate) async fn list_lifecycle_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsLifecycleEventFilter,
    ) -> Result<Vec<PdsLifecycleEventState>, PdsRepositoryError> {
        let rows = sqlx::query_as::<_, PdsLifecycleEventRow>(
            "SELECT event_id, data_subject, event_type, resource_kind, resource_id, reason, \
              created_at \
             FROM kiva.pds_lifecycle_events \
             WHERE tenant_id = $1 AND data_subject = $2 \
              AND ($3::uuid IS NULL OR event_id = $3) \
             ORDER BY created_at DESC, event_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(&principal.subject)
        .bind(filter.event_id)
        .fetch_all(&self.pool)
        .await
        .map_err(PdsRepositoryError::store)?;

        rows.into_iter().map(pds_lifecycle_event_from_row).collect()
    }

    pub(crate) async fn register_retention_policy(
        &self,
        projection: PdsRetentionPolicyProjection,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(PdsRepositoryError::store)?;
        let policy = projection.state();
        let result = sqlx::query(
            "INSERT INTO kiva.pds_retention_policies \
             (tenant_id, policy_id, owner_subject, resource_kind, data_class, \
              retention_period_days, review_interval_days, tombstone_after_days, \
              tombstone_publication, legal_basis, description) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
             ON CONFLICT (tenant_id, policy_id) DO NOTHING",
        )
        .bind(&self.tenant_id)
        .bind(policy.policy_id)
        .bind(&policy.owner_subject)
        .bind(policy.resource_kind.label())
        .bind(policy.data_class.label())
        .bind(pds_days_to_i32(policy.retention_period_days)?)
        .bind(pds_days_to_i32(policy.review_interval_days)?)
        .bind(
            policy
                .tombstone_after_days
                .map(pds_days_to_i32)
                .transpose()?,
        )
        .bind(policy.tombstone_publication.label())
        .bind(policy.legal_basis.label())
        .bind(&policy.description)
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
            let existing = select_pds_retention_policy_in_transaction(
                &mut transaction,
                &self.tenant_id,
                policy.policy_id,
            )
            .await?;
            let Some(existing) = existing else {
                return Err(PdsRepositoryError::InvalidRequest(
                    "PDS retention policy was not found".to_owned(),
                ));
            };
            if existing.same_registration(policy) {
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

    pub(crate) async fn list_retention_policies(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsRetentionPolicyFilter,
    ) -> Result<Vec<PdsRetentionPolicyState>, PdsRepositoryError> {
        let rows = sqlx::query_as::<_, PdsRetentionPolicyRow>(
            "SELECT policy_id, owner_subject, resource_kind, data_class, \
              retention_period_days, review_interval_days, tombstone_after_days, \
              tombstone_publication, legal_basis, description, created_at \
             FROM kiva.pds_retention_policies \
             WHERE tenant_id = $1 AND owner_subject = $2 \
              AND ($3::uuid IS NULL OR policy_id = $3) \
             ORDER BY created_at ASC, policy_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(&principal.subject)
        .bind(filter.policy_id)
        .fetch_all(&self.pool)
        .await
        .map_err(PdsRepositoryError::store)?;

        rows.into_iter()
            .map(pds_retention_policy_from_row)
            .collect()
    }

    pub(crate) async fn list_lifecycle_cases(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsLifecycleCaseFilter,
    ) -> Result<Vec<PdsLifecycleCaseState>, PdsRepositoryError> {
        let rows = sqlx::query_as::<_, PdsLifecycleCaseRow>(
            "SELECT case_id, data_subject, case_type, resource_kind, resource_id, status, \
              retention_policy_id, summary, resolution, tombstone_publication, created_at, updated_at \
             FROM kiva.pds_lifecycle_cases \
             WHERE tenant_id = $1 AND data_subject = $2 \
              AND ($3::uuid IS NULL OR case_id = $3) \
             ORDER BY updated_at DESC, case_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(&principal.subject)
        .bind(filter.case_id)
        .fetch_all(&self.pool)
        .await
        .map_err(PdsRepositoryError::store)?;

        rows.into_iter().map(pds_lifecycle_case_from_row).collect()
    }
}

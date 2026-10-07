#![allow(clippy::wildcard_imports)]

use super::super::super::{postgres_queries::*, *};

impl PostgresPdsRepository {
    pub(crate) async fn register_lifecycle_case(
        &self,
        projection: PdsLifecycleCaseProjection,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(PdsRepositoryError::store)?;
        let requested = projection.state();
        self.ensure_lifecycle_case_retention_policy(&mut transaction, requested)
            .await?;

        let existing = select_pds_lifecycle_case_in_transaction(
            &mut transaction,
            &self.tenant_id,
            requested.case_id,
        )
        .await?;
        let outcome = if let Some(existing) = existing {
            self.transition_lifecycle_case_in_transaction(
                &mut transaction,
                &projection,
                requested,
                &existing,
            )
            .await?
        } else {
            self.insert_lifecycle_case_in_transaction(&mut transaction, &projection, requested)
                .await?
        };
        transaction
            .commit()
            .await
            .map_err(PdsRepositoryError::store)?;
        Ok(outcome)
    }

    async fn ensure_lifecycle_case_retention_policy(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        requested: &PdsLifecycleCaseState,
    ) -> Result<(), PdsRepositoryError> {
        let Some(policy_id) = requested.retention_policy_id else {
            return Ok(());
        };
        let Some(policy) =
            select_pds_retention_policy_in_transaction(transaction, &self.tenant_id, policy_id)
                .await?
        else {
            return Err(PdsRepositoryError::InvalidRequest(
                "PDS lifecycle case retention_policy_id must reference an existing retention policy"
                    .to_owned(),
            ));
        };
        if policy.owner_subject != requested.data_subject {
            return Err(PdsRepositoryError::InvalidRequest(
                "PDS lifecycle case retention policy belongs to a different subject".to_owned(),
            ));
        }
        Ok(())
    }

    async fn insert_lifecycle_case_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        projection: &PdsLifecycleCaseProjection,
        requested: &PdsLifecycleCaseState,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        if !pds_lifecycle_case_initial_status_is_allowed(requested.status) {
            return Err(PdsRepositoryError::InvalidRequest(
                "PDS lifecycle case must start in open status".to_owned(),
            ));
        }
        sqlx::query(
            "INSERT INTO kiva.pds_lifecycle_cases \
             (tenant_id, case_id, data_subject, case_type, resource_kind, resource_id, \
              status, retention_policy_id, summary, resolution, tombstone_publication) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(&self.tenant_id)
        .bind(requested.case_id)
        .bind(&requested.data_subject)
        .bind(requested.case_type.label())
        .bind(requested.resource_kind.label())
        .bind(&requested.resource_id)
        .bind(requested.status.label())
        .bind(requested.retention_policy_id)
        .bind(&requested.summary)
        .bind(&requested.resolution)
        .bind(
            requested
                .tombstone_publication
                .map(|rule| rule.label().to_owned()),
        )
        .execute(&mut **transaction)
        .await
        .map_err(PdsRepositoryError::store)?;
        let ledger_event = projection.opened_ledger_event();
        insert_pds_ledger_event_in_transaction(transaction, &self.tenant_id, &ledger_event).await?;
        Ok(StoreOutcome::Stored)
    }

    async fn transition_lifecycle_case_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        projection: &PdsLifecycleCaseProjection,
        requested: &PdsLifecycleCaseState,
        existing: &PdsLifecycleCaseState,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        match existing.transition_from(requested)? {
            StoreOutcome::Stored => {
                sqlx::query(
                    "UPDATE kiva.pds_lifecycle_cases \
                     SET status = $3, resolution = $4, updated_at = $5 \
                     WHERE tenant_id = $1 AND case_id = $2",
                )
                .bind(&self.tenant_id)
                .bind(requested.case_id)
                .bind(requested.status.label())
                .bind(&requested.resolution)
                .bind(requested.updated_at)
                .execute(&mut **transaction)
                .await
                .map_err(PdsRepositoryError::store)?;
                let ledger_event = projection.transitioned_ledger_event();
                insert_pds_ledger_event_in_transaction(transaction, &self.tenant_id, &ledger_event)
                    .await?;
                Ok(StoreOutcome::Stored)
            }
            outcome => Ok(outcome),
        }
    }
}

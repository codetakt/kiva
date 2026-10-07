#![allow(clippy::wildcard_imports)]

use super::super::{mapping::*, postgres_queries::*, *};

impl PostgresPdsRepository {
    pub(crate) async fn list_consent_grants(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsConsentGrantFilter,
    ) -> Result<Vec<PdsConsentGrantState>, PdsRepositoryError> {
        let rows = sqlx::query_as::<_, PdsConsentGrantRow>(
            "SELECT grant_id, data_subject, grantee_subject, scope, state, purpose, legal_basis, \
              created_at, expires_at, revoked_at, renewed_from_grant_id \
             FROM kiva.pds_consent_grants \
             WHERE tenant_id = $1 \
              AND (data_subject = $2 OR grantee_subject = $2) \
              AND ($3::uuid IS NULL OR grant_id = $3) \
             ORDER BY created_at ASC, grant_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(&principal.subject)
        .bind(filter.grant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(PdsRepositoryError::store)?;

        rows.into_iter().map(pds_consent_grant_from_row).collect()
    }

    pub(crate) async fn register_consent_grant(
        &self,
        projection: PdsConsentGrantProjection,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(PdsRepositoryError::store)?;
        let grant = projection.state();
        let result = sqlx::query(
            "INSERT INTO kiva.pds_consent_grants \
             (tenant_id, grant_id, data_subject, grantee_subject, scope, state, purpose, \
              legal_basis, expires_at, renewed_from_grant_id) \
             VALUES ($1, $2, $3, $4, $5, 'active', $6, $7, $8, $9) \
             ON CONFLICT (tenant_id, grant_id) DO NOTHING",
        )
        .bind(&self.tenant_id)
        .bind(grant.grant_id)
        .bind(&grant.data_subject)
        .bind(&grant.grantee_subject)
        .bind(grant.scope.label())
        .bind(&grant.purpose)
        .bind(grant.legal_basis.label())
        .bind(grant.expires_at)
        .bind(grant.renewed_from_grant_id)
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
            let existing = select_pds_consent_grant_in_transaction(
                &mut transaction,
                &self.tenant_id,
                grant.grant_id,
            )
            .await?
            .ok_or_else(|| {
                PdsRepositoryError::InvalidRequest("PDS consent grant was not found".to_owned())
            })?;
            if existing.same_active_registration(grant) {
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

    pub(crate) async fn revoke_consent_grant(
        &self,
        event: PdsConsentGrantRevocationRequested,
    ) -> Result<PdsRevocationOutcome, PdsRepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(PdsRepositoryError::store)?;
        let existing = select_pds_consent_grant_in_transaction(
            &mut transaction,
            &self.tenant_id,
            event.grant_id(),
        )
        .await?;
        let Some(existing) = existing else {
            return Ok(PdsRevocationOutcome::NotFound);
        };
        if existing.data_subject != event.data_subject() {
            return Ok(PdsRevocationOutcome::Forbidden);
        }
        if existing.state == PdsConsentState::Revoked {
            return Ok(PdsRevocationOutcome::AlreadyRevoked);
        }

        let result = sqlx::query(
            "UPDATE kiva.pds_consent_grants \
             SET state = 'revoked', revoked_at = now() \
             WHERE tenant_id = $1 AND grant_id = $2 AND state = 'active'",
        )
        .bind(&self.tenant_id)
        .bind(event.grant_id())
        .execute(&mut *transaction)
        .await
        .map_err(PdsRepositoryError::store)?;

        if result.rows_affected() == 0 {
            Ok(PdsRevocationOutcome::AlreadyRevoked)
        } else {
            insert_pds_ledger_event_in_transaction(
                &mut transaction,
                &self.tenant_id,
                &PdsLedgerEventState::consent_grant_revoked(&existing, event.revoked_at()),
            )
            .await?;
            transaction
                .commit()
                .await
                .map_err(PdsRepositoryError::store)?;
            Ok(PdsRevocationOutcome::Revoked)
        }
    }
}

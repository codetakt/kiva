#![allow(clippy::wildcard_imports)]

use super::super::{mapping::*, postgres_queries::*, *};

impl PostgresPdsRepository {
    pub(crate) async fn list_data_products(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsDataProductFilter,
    ) -> Result<Vec<PdsDataProductState>, PdsRepositoryError> {
        let rows = sqlx::query_as::<_, PdsDataProductRow>(
            "SELECT data_product_id, owner_subject, source_kind, release_id, title, purpose, \
              legal_basis, data_classes, created_at \
             FROM kiva.pds_data_products \
             WHERE tenant_id = $1 AND owner_subject = $2 \
              AND ($3::uuid IS NULL OR data_product_id = $3) \
             ORDER BY created_at ASC, data_product_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(&principal.subject)
        .bind(filter.data_product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(PdsRepositoryError::store)?;

        rows.into_iter().map(pds_data_product_from_row).collect()
    }

    pub(crate) async fn register_data_product(
        &self,
        projection: PdsDataProductProjection,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(PdsRepositoryError::store)?;
        let product = projection.state();
        let result = sqlx::query(
            "INSERT INTO kiva.pds_data_products \
             (tenant_id, data_product_id, owner_subject, source_kind, release_id, title, purpose, \
              legal_basis, data_classes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             ON CONFLICT (tenant_id, data_product_id) DO NOTHING",
        )
        .bind(&self.tenant_id)
        .bind(product.data_product_id)
        .bind(&product.owner_subject)
        .bind(product.source_kind().label())
        .bind(product.release_artifact().artifact_id().to_vec())
        .bind(&product.title)
        .bind(&product.purpose)
        .bind(product.legal_basis.label())
        .bind(SqlJson(pds_data_class_labels(&product.data_classes)))
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
            let existing = select_pds_data_product_in_transaction(
                &mut transaction,
                &self.tenant_id,
                product.data_product_id,
            )
            .await?
            .ok_or_else(|| {
                PdsRepositoryError::InvalidRequest("PDS data product was not found".to_owned())
            })?;
            if existing.same_registration(product) {
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

    pub(crate) async fn read_data_product(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsDataProductReadIntent,
    ) -> Result<PdsDataProductReadOutcome, PdsRepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(PdsRepositoryError::store)?;
        let data_product = select_pds_data_product_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.data_product_id,
        )
        .await?;
        let grant = select_pds_consent_grant_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.grant_id,
        )
        .await?;
        let event = pds_access_audit_event_from_decision(
            principal,
            pds_data_product_read_access_check(request),
            grant.as_ref(),
            data_product.as_ref(),
        );
        insert_pds_access_audit_event_in_transaction(&mut transaction, &self.tenant_id, &event)
            .await?;
        insert_pds_ledger_event_in_transaction(
            &mut transaction,
            &self.tenant_id,
            &PdsLedgerEventState::access_decision_recorded(&event),
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(PdsRepositoryError::store)?;
        Ok(PdsDataProductReadOutcome::from_event_and_product(
            event,
            data_product,
        ))
    }
}

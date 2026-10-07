#![allow(clippy::wildcard_imports)]

use super::super::{mapping::*, postgres_queries::*, *};

impl PostgresPdsRepository {
    pub(crate) async fn record_analytics_read(
        &self,
        event: PdsAccessAuditEventState,
    ) -> Result<(), PdsRepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(PdsRepositoryError::store)?;
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
            .map_err(PdsRepositoryError::store)
    }

    pub(crate) async fn evaluate_access_check(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsAccessCheckIntent,
    ) -> Result<PdsAccessAuditEventState, PdsRepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(PdsRepositoryError::store)?;
        let request = PdsParsedAccessCheckRequest::from_request(&request)?;
        let data_product = if let Some(data_product_id) = request.data_product_id {
            select_pds_data_product_in_transaction(
                &mut transaction,
                &self.tenant_id,
                data_product_id,
            )
            .await?
        } else {
            None
        };
        let grant = if let Some(grant_id) = request.grant_id {
            select_pds_consent_grant_in_transaction(&mut transaction, &self.tenant_id, grant_id)
                .await?
        } else {
            None
        };
        let event = pds_access_audit_event_from_decision(
            principal,
            request,
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
        Ok(event)
    }

    pub(crate) async fn list_access_audit_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsAccessAuditEventFilter,
    ) -> Result<Vec<PdsAccessAuditEventState>, PdsRepositoryError> {
        let rows = sqlx::query_as::<_, PdsAccessAuditEventRow>(
            "SELECT event_id, requester_subject, role, resource, action, grant_id, data_product_id, \
              decision, deny_reason, created_at \
             FROM kiva.pds_access_audit_events \
             WHERE tenant_id = $1 AND requester_subject = $2 \
              AND ($3::uuid IS NULL OR event_id = $3) \
             ORDER BY created_at DESC, event_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(&principal.subject)
        .bind(filter.event_id)
        .fetch_all(&self.pool)
        .await
        .map_err(PdsRepositoryError::store)?;

        rows.into_iter()
            .map(pds_access_audit_event_from_row)
            .collect()
    }
}

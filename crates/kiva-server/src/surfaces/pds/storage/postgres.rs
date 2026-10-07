#![allow(clippy::wildcard_imports)]

use super::postgres_queries::insert_pds_ledger_event_in_transaction;
use super::*;

mod access;
mod consent;
mod data_products;
mod governance;
mod lifecycle;
mod provider_bindings;

impl PostgresPdsRepository {
    pub(crate) fn new(pool: PgPool, tenant_id: String) -> Result<Self, io::Error> {
        if tenant_id.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{KIVA_TENANT_ID_ENV} must not be empty"),
            ));
        }
        Ok(Self { pool, tenant_id })
    }

    pub(crate) async fn record_portability_export_audit(
        &self,
        event: &kiva_pds::PortabilityExportAuditRecorded,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(PdsRepositoryError::store)?;
        insert_pds_ledger_event_in_transaction(
            &mut transaction,
            &self.tenant_id,
            &PdsLedgerEventState::portability_export_audited(event),
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(PdsRepositoryError::store)?;
        Ok(StoreOutcome::Stored)
    }

    #[cfg(test)]
    pub(crate) async fn ledger_event_matches_for_test(
        &self,
        event_kind: &str,
        owner_subject: &str,
        resource_kind: &str,
        resource_id: &str,
    ) -> Result<bool, String> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM kiva.pds_ledger_events \
             WHERE tenant_id = $1 AND event_kind = $2 AND owner_subject = $3 \
               AND resource_kind = $4 AND resource_id = $5)",
        )
        .bind(&self.tenant_id)
        .bind(event_kind)
        .bind(owner_subject)
        .bind(resource_kind)
        .bind(resource_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| format!("PDS ledger test query should succeed: {error:?}"))
    }

    #[cfg(test)]
    pub(crate) async fn ledger_event_kind_count_for_test(
        &self,
        event_kind: &str,
        owner_subject: &str,
    ) -> Result<usize, String> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM kiva.pds_ledger_events \
             WHERE tenant_id = $1 AND event_kind = $2 AND owner_subject = $3",
        )
        .bind(&self.tenant_id)
        .bind(event_kind)
        .bind(owner_subject)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| format!("PDS ledger count query should succeed: {error:?}"))?;
        usize::try_from(count)
            .map_err(|error| format!("PDS ledger count should fit usize: {error}"))
    }
}

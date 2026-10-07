#![allow(clippy::wildcard_imports)]

use super::super::*;

pub(in crate::surfaces::pds::storage) async fn insert_pds_ledger_event_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    event: &PdsLedgerEventState,
) -> Result<(), PdsRepositoryError> {
    insert_pds_ledger_event_with_executor(&mut **transaction, tenant_id, event).await
}

async fn insert_pds_ledger_event_with_executor<'e, E>(
    executor: E,
    tenant_id: &str,
    event: &PdsLedgerEventState,
) -> Result<(), PdsRepositoryError>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        "INSERT INTO kiva.pds_ledger_events \
         (tenant_id, event_kind, owner_subject, resource_kind, resource_id, occurred_at) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (tenant_id, event_kind, resource_id) DO NOTHING",
    )
    .bind(tenant_id)
    .bind(event.event_kind.label())
    .bind(&event.owner_subject)
    .bind(event.resource_kind)
    .bind(&event.resource_id)
    .bind(event.occurred_at)
    .execute(executor)
    .await
    .map_err(PdsRepositoryError::store)?;
    Ok(())
}

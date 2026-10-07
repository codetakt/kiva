#![allow(clippy::wildcard_imports)]

use super::super::*;

pub(in crate::surfaces::pds::storage) async fn insert_pds_access_audit_event_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    event: &PdsAccessAuditEventState,
) -> Result<(), PdsRepositoryError> {
    insert_pds_access_audit_event_with_executor(&mut **transaction, tenant_id, event).await
}

async fn insert_pds_access_audit_event_with_executor<'e, E>(
    executor: E,
    tenant_id: &str,
    event: &PdsAccessAuditEventState,
) -> Result<(), PdsRepositoryError>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        "INSERT INTO kiva.pds_access_audit_events \
         (tenant_id, event_id, requester_subject, role, resource, action, grant_id, \
          data_product_id, decision, deny_reason, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(tenant_id)
    .bind(event.event_id)
    .bind(&event.requester_subject)
    .bind(pds_principal_role_label(event.role))
    .bind(pds_resource_class_label(event.resource))
    .bind(pds_action_label(event.action))
    .bind(event.grant_id)
    .bind(event.data_product_id)
    .bind(pds_decision_label(event.decision))
    .bind(pds_deny_reason(event.decision).map(pds_deny_reason_label))
    .bind(event.created_at)
    .execute(executor)
    .await
    .map_err(PdsRepositoryError::store)?;
    Ok(())
}

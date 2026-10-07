#![allow(clippy::wildcard_imports)]

use super::super::{mapping::*, *};

pub(in crate::surfaces::pds::storage) async fn select_pds_lifecycle_event_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    event_id: Uuid,
) -> Result<Option<PdsLifecycleEventState>, PdsRepositoryError> {
    select_pds_lifecycle_event_with_executor(&mut **transaction, tenant_id, event_id).await
}

async fn select_pds_lifecycle_event_with_executor<'e, E>(
    executor: E,
    tenant_id: &str,
    event_id: Uuid,
) -> Result<Option<PdsLifecycleEventState>, PdsRepositoryError>
where
    E: Executor<'e, Database = Postgres>,
{
    let row = sqlx::query_as::<_, PdsLifecycleEventRow>(
        "SELECT event_id, data_subject, event_type, resource_kind, resource_id, reason, created_at \
         FROM kiva.pds_lifecycle_events \
         WHERE tenant_id = $1 AND event_id = $2",
    )
    .bind(tenant_id)
    .bind(event_id)
    .fetch_optional(executor)
    .await
    .map_err(PdsRepositoryError::store)?;

    row.map(pds_lifecycle_event_from_row).transpose()
}

pub(in crate::surfaces::pds::storage) async fn select_pds_retention_policy_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    policy_id: Uuid,
) -> Result<Option<PdsRetentionPolicyState>, PdsRepositoryError> {
    select_pds_retention_policy_with_executor(&mut **transaction, tenant_id, policy_id).await
}

async fn select_pds_retention_policy_with_executor<'e, E>(
    executor: E,
    tenant_id: &str,
    policy_id: Uuid,
) -> Result<Option<PdsRetentionPolicyState>, PdsRepositoryError>
where
    E: Executor<'e, Database = Postgres>,
{
    let row = sqlx::query_as::<_, PdsRetentionPolicyRow>(
        "SELECT policy_id, owner_subject, resource_kind, data_class, \
          retention_period_days, review_interval_days, tombstone_after_days, \
          tombstone_publication, legal_basis, description, created_at \
         FROM kiva.pds_retention_policies \
         WHERE tenant_id = $1 AND policy_id = $2",
    )
    .bind(tenant_id)
    .bind(policy_id)
    .fetch_optional(executor)
    .await
    .map_err(PdsRepositoryError::store)?;

    row.map(pds_retention_policy_from_row).transpose()
}

pub(in crate::surfaces::pds::storage) async fn select_pds_lifecycle_case_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    case_id: Uuid,
) -> Result<Option<PdsLifecycleCaseState>, PdsRepositoryError> {
    select_pds_lifecycle_case_with_executor(&mut **transaction, tenant_id, case_id).await
}

async fn select_pds_lifecycle_case_with_executor<'e, E>(
    executor: E,
    tenant_id: &str,
    case_id: Uuid,
) -> Result<Option<PdsLifecycleCaseState>, PdsRepositoryError>
where
    E: Executor<'e, Database = Postgres>,
{
    let row = sqlx::query_as::<_, PdsLifecycleCaseRow>(
        "SELECT case_id, data_subject, case_type, resource_kind, resource_id, status, \
          retention_policy_id, summary, resolution, tombstone_publication, created_at, updated_at \
         FROM kiva.pds_lifecycle_cases \
         WHERE tenant_id = $1 AND case_id = $2",
    )
    .bind(tenant_id)
    .bind(case_id)
    .fetch_optional(executor)
    .await
    .map_err(PdsRepositoryError::store)?;

    row.map(pds_lifecycle_case_from_row).transpose()
}

#![allow(clippy::wildcard_imports)]

use super::super::{mapping::*, *};

pub(in crate::surfaces::pds::storage) async fn select_pds_governance_profile_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    profile_id: Uuid,
) -> Result<Option<PdsGovernanceProfileState>, PdsRepositoryError> {
    select_pds_governance_profile_with_executor(&mut **transaction, tenant_id, profile_id).await
}

async fn select_pds_governance_profile_with_executor<'e, E>(
    executor: E,
    tenant_id: &str,
    profile_id: Uuid,
) -> Result<Option<PdsGovernanceProfileState>, PdsRepositoryError>
where
    E: Executor<'e, Database = Postgres>,
{
    let row = sqlx::query_as::<_, PdsGovernanceProfileRow>(
        "SELECT profile_id, profile_owner_subject, profile_kind, tenant_partition, \
          subject_partition, data_custodian_subject, platform_operator_subject, \
          policy_authority_subject, analyst_subject, auditor_subject, \
          identity_authority_subject, emergency_access_policy, runbook_refs, created_at \
         FROM kiva.pds_governance_profiles \
         WHERE tenant_id = $1 AND profile_id = $2",
    )
    .bind(tenant_id)
    .bind(profile_id)
    .fetch_optional(executor)
    .await
    .map_err(PdsRepositoryError::store)?;

    row.map(pds_governance_profile_from_row).transpose()
}

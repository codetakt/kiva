#![allow(clippy::wildcard_imports)]

use super::super::{mapping::*, *};

pub(in crate::surfaces::pds::storage) async fn select_pds_consent_grant_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    grant_id: Uuid,
) -> Result<Option<PdsConsentGrantState>, PdsRepositoryError> {
    select_pds_consent_grant_with_executor(&mut **transaction, tenant_id, grant_id).await
}

async fn select_pds_consent_grant_with_executor<'e, E>(
    executor: E,
    tenant_id: &str,
    grant_id: Uuid,
) -> Result<Option<PdsConsentGrantState>, PdsRepositoryError>
where
    E: Executor<'e, Database = Postgres>,
{
    let row = sqlx::query_as::<_, PdsConsentGrantRow>(
        "SELECT grant_id, data_subject, grantee_subject, scope, state, purpose, legal_basis, \
          created_at, expires_at, revoked_at, renewed_from_grant_id \
         FROM kiva.pds_consent_grants \
         WHERE tenant_id = $1 AND grant_id = $2",
    )
    .bind(tenant_id)
    .bind(grant_id)
    .fetch_optional(executor)
    .await
    .map_err(PdsRepositoryError::store)?;

    row.map(pds_consent_grant_from_row).transpose()
}

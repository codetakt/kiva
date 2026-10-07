#![allow(clippy::wildcard_imports)]

use super::super::{mapping::*, *};

pub(in crate::surfaces::pds::storage) async fn select_pds_provider_binding_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    binding_id: Uuid,
) -> Result<Option<PdsProviderBindingState>, PdsRepositoryError> {
    select_pds_provider_binding_with_executor(&mut **transaction, tenant_id, binding_id).await
}

async fn select_pds_provider_binding_with_executor<'e, E>(
    executor: E,
    tenant_id: &str,
    binding_id: Uuid,
) -> Result<Option<PdsProviderBindingState>, PdsRepositoryError>
where
    E: Executor<'e, Database = Postgres>,
{
    let row = sqlx::query_as::<_, PdsProviderBindingRow>(
        "SELECT binding_id, owner_subject, mechanism, provider_kind, capability, profile, \
          provider_id_digest, artifact_digest, parameter_set_digest, review_evidence_digest, \
          production_claim_requested, binding_digest, evidence_gate, release_gate, runtime_class, \
          production_ready, created_at \
         FROM kiva.pds_provider_bindings \
         WHERE tenant_id = $1 AND binding_id = $2",
    )
    .bind(tenant_id)
    .bind(binding_id)
    .fetch_optional(executor)
    .await
    .map_err(PdsRepositoryError::store)?;

    row.map(pds_provider_binding_from_row).transpose()
}

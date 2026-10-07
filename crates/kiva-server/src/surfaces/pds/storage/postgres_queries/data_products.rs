#![allow(clippy::wildcard_imports)]

use super::super::{mapping::*, *};

pub(in crate::surfaces::pds::storage) async fn select_pds_data_product_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    data_product_id: Uuid,
) -> Result<Option<PdsDataProductState>, PdsRepositoryError> {
    select_pds_data_product_with_executor(&mut **transaction, tenant_id, data_product_id).await
}

async fn select_pds_data_product_with_executor<'e, E>(
    executor: E,
    tenant_id: &str,
    data_product_id: Uuid,
) -> Result<Option<PdsDataProductState>, PdsRepositoryError>
where
    E: Executor<'e, Database = Postgres>,
{
    let row = sqlx::query_as::<_, PdsDataProductRow>(
        "SELECT data_product_id, owner_subject, source_kind, release_id, title, purpose, \
          legal_basis, data_classes, created_at \
         FROM kiva.pds_data_products \
         WHERE tenant_id = $1 AND data_product_id = $2",
    )
    .bind(tenant_id)
    .bind(data_product_id)
    .fetch_optional(executor)
    .await
    .map_err(PdsRepositoryError::store)?;

    row.map(pds_data_product_from_row).transpose()
}

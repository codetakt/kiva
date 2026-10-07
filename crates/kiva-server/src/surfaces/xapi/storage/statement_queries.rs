#![allow(clippy::wildcard_imports)]

use super::*;

pub(super) async fn select_statement_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    statement_id: Uuid,
) -> Result<Option<StoredStatement>, StatementStoreError> {
    let row = sqlx::query_as::<_, (SqlJson<Value>, OffsetDateTime)>(
        "SELECT statement, stored_at FROM kiva.xapi_statements \
         WHERE tenant_id = $1 AND statement_id = $2",
    )
    .bind(tenant_id)
    .bind(statement_id)
    .fetch_optional(&mut **transaction)
    .await?;

    let Some((statement, stored_at)) = row else {
        return Ok(None);
    };
    let attachments =
        select_statement_attachments_in_transaction(transaction, tenant_id, statement_id).await?;
    Ok(Some(StoredStatement {
        statement: statement.0,
        stored_at,
        attachments,
    }))
}

pub(super) async fn insert_statement_attachments_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    statement_id: Uuid,
    attachments: &BTreeMap<String, AttachmentRecord>,
) -> Result<(), StatementStoreError> {
    for (sha2, attachment) in attachments {
        sqlx::query(
            "INSERT INTO kiva.xapi_statement_attachments \
             (tenant_id, statement_id, sha2, content_type, body) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(tenant_id)
        .bind(statement_id)
        .bind(sha2)
        .bind(&attachment.content_type)
        .bind(&attachment.body)
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

pub(super) async fn select_statement_attachments(
    pool: &PgPool,
    tenant_id: &str,
    statement_id: Uuid,
) -> Result<BTreeMap<String, AttachmentRecord>, StatementStoreError> {
    let rows = sqlx::query_as::<_, (String, String, Vec<u8>)>(
        "SELECT sha2, content_type, body FROM kiva.xapi_statement_attachments \
         WHERE tenant_id = $1 AND statement_id = $2 \
         ORDER BY sha2 ASC",
    )
    .bind(tenant_id)
    .bind(statement_id)
    .fetch_all(pool)
    .await?;

    Ok(attachment_records_from_rows(rows))
}

pub(super) async fn select_statement_attachments_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    statement_id: Uuid,
) -> Result<BTreeMap<String, AttachmentRecord>, StatementStoreError> {
    let rows = sqlx::query_as::<_, (String, String, Vec<u8>)>(
        "SELECT sha2, content_type, body FROM kiva.xapi_statement_attachments \
         WHERE tenant_id = $1 AND statement_id = $2 \
         ORDER BY sha2 ASC",
    )
    .bind(tenant_id)
    .bind(statement_id)
    .fetch_all(&mut **transaction)
    .await?;

    Ok(attachment_records_from_rows(rows))
}

pub(super) fn attachment_records_from_rows(
    rows: Vec<(String, String, Vec<u8>)>,
) -> BTreeMap<String, AttachmentRecord> {
    rows.into_iter()
        .map(|(sha2, content_type, body)| (sha2, AttachmentRecord { content_type, body }))
        .collect()
}

pub(super) fn sql_limit(limit: Option<usize>) -> i64 {
    limit
        .and_then(|limit| i64::try_from(limit).ok())
        .unwrap_or(i64::MAX)
}

pub(super) fn stored_statements_equivalent(
    left: &StoredStatement,
    right: &StoredStatement,
) -> bool {
    statements_equivalent(&left.statement, &right.statement)
        && left.attachments == right.attachments
}

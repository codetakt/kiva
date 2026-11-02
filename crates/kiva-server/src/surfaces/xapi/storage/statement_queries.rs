#![allow(clippy::wildcard_imports)]

use super::*;

#[cfg(test)]
std::thread_local! {
    // Native admission controls use a current-thread Tokio runtime. Recording
    // the actual call is advisory evidence, never a replacement comparator.
    static REPLAY_OBSERVATIONS: std::cell::RefCell<Vec<Value>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
pub(crate) fn take_replay_observations() -> Vec<Value> {
    REPLAY_OBSERVATIONS.with(|values| std::mem::take(&mut *values.borrow_mut()))
}

#[cfg(test)]
pub(super) fn snapshot_record(record: &StoredStatement) -> Value {
    let attachments = record.attachments.iter().map(|(hash, attachment)| serde_json::json!({"sha2":hash,"media_type":attachment.content_type,"body_hex":crate::core::response::hex_lower(&attachment.body)})).collect::<Vec<_>>();
    serde_json::json!({"statement":record.statement,"opaque_values":record.opaque_values,"stored_at":record.stored_at.unix_timestamp_nanos().to_string(),"attachments":attachments})
}

pub(super) async fn select_statement_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    statement_id: Uuid,
) -> Result<Option<StoredStatement>, StatementStoreError> {
    let row = sqlx::query_as::<
        _,
        (
            SqlJson<crate::surfaces::xapi::lossless::StoredJson>,
            OffsetDateTime,
            SqlJson<crate::surfaces::xapi::lossless::OpaqueValues>,
        ),
    >(
        "SELECT statement, stored_at, opaque_values FROM kiva.xapi_statements \
         WHERE tenant_id = $1 AND statement_id = $2",
    )
    .bind(tenant_id)
    .bind(statement_id)
    .fetch_optional(&mut **transaction)
    .await?;

    let Some((statement, stored_at, opaque_values)) = row else {
        return Ok(None);
    };
    let attachments =
        select_statement_attachments_in_transaction(transaction, tenant_id, statement_id).await?;
    let mut record = StoredStatement::new(statement.0.0, stored_at, attachments);
    record.opaque_values = opaque_values.0;
    Ok(Some(record))
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
        .bind(attachment.content_type.as_bytes())
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
    let rows = sqlx::query_as::<_, (String, Vec<u8>, Vec<u8>)>(
        "SELECT sha2, content_type, body FROM kiva.xapi_statement_attachments \
         WHERE tenant_id = $1 AND statement_id = $2 \
         ORDER BY sha2 ASC",
    )
    .bind(tenant_id)
    .bind(statement_id)
    .fetch_all(pool)
    .await?;

    attachment_records_from_rows(rows)
}

pub(super) async fn select_statement_attachments_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    statement_id: Uuid,
) -> Result<BTreeMap<String, AttachmentRecord>, StatementStoreError> {
    let rows = sqlx::query_as::<_, (String, Vec<u8>, Vec<u8>)>(
        "SELECT sha2, content_type, body FROM kiva.xapi_statement_attachments \
         WHERE tenant_id = $1 AND statement_id = $2 \
         ORDER BY sha2 ASC",
    )
    .bind(tenant_id)
    .bind(statement_id)
    .fetch_all(&mut **transaction)
    .await?;

    attachment_records_from_rows(rows)
}

pub(super) fn attachment_records_from_rows(
    rows: Vec<(String, Vec<u8>, Vec<u8>)>,
) -> Result<BTreeMap<String, AttachmentRecord>, StatementStoreError> {
    rows.into_iter()
        .map(|(sha2, content_type, body)| {
            let content_type = String::from_utf8(content_type)
                .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
            Ok((sha2, AttachmentRecord { content_type, body }))
        })
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
    let mut incoming = right.statement.clone();
    if right.defaulted_substatement_timestamp
        && let Some(object) = incoming.get_mut("object").and_then(Value::as_object_mut)
    {
        // Only the incoming omission authorizes reuse. An explicitly supplied
        // nested timestamp still participates in immutable-content comparison.
        if let Some(timestamp) = left.statement.pointer("/object/timestamp") {
            object.insert("timestamp".to_owned(), timestamp.clone());
        } else {
            // Older records may predate nested timestamp defaulting. Preserve
            // that absence instead of treating the generated value as an edit.
            object.remove("timestamp");
        }
    }
    let mut existing = left.statement.clone();
    normalize_nested_timestamp_for_comparison(&mut existing);
    normalize_nested_timestamp_for_comparison(&mut incoming);
    let equivalent = crate::surfaces::xapi::lossless::mask_equivalent_values(
        &mut existing,
        &left.opaque_values,
        &mut incoming,
        &right.opaque_values,
    ) && statements_equivalent(&existing, &incoming)
        && attachment_records_equivalent(&left.attachments, &right.attachments);
    #[cfg(test)]
    REPLAY_OBSERVATIONS.with(|values| values.borrow_mut().push(serde_json::json!({"existing_id":existing["id"],"incoming_id":incoming["id"],"equivalent":equivalent})));
    equivalent
}

fn attachment_records_equivalent(
    left: &BTreeMap<String, AttachmentRecord>,
    right: &BTreeMap<String, AttachmentRecord>,
) -> bool {
    // Existing records may predate canonical digest keys. A spelling change
    // at the MIME boundary must not turn an identical retry into a conflict.
    let left = left
        .iter()
        .map(|(hash, record)| (hash.to_ascii_lowercase(), record))
        .collect::<BTreeMap<_, _>>();
    let right = right
        .iter()
        .map(|(hash, record)| (hash.to_ascii_lowercase(), record))
        .collect::<BTreeMap<_, _>>();
    left == right
}

fn normalize_nested_timestamp_for_comparison(statement: &mut Value) {
    let Some(object) = statement.get_mut("object").and_then(Value::as_object_mut) else {
        return;
    };
    if object.get("objectType").and_then(Value::as_str) != Some("SubStatement") {
        return;
    }
    let Some(timestamp) = object
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(crate::surfaces::xapi::timestamp::normalize)
    else {
        return;
    };
    // Normalize comparison copies only; legacy JSON retains its stored spelling.
    object.insert("timestamp".to_owned(), Value::String(timestamp));
}

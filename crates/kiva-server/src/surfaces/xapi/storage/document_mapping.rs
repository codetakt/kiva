#![allow(clippy::wildcard_imports)]

use super::*;

pub(super) async fn select_document(
    pool: &PgPool,
    tenant_id: &str,
    address: &DocumentAddress,
) -> Result<Option<DocumentRecord>, DocumentStoreError> {
    let row = sqlx::query_as::<_, DocumentRow>(
        "SELECT document_id, content_type, content_json, content_bytes, etag, updated_at \
         FROM kiva.xapi_documents \
         WHERE tenant_id = $1 AND resource_kind = $2 AND document_key = $3",
    )
    .bind(tenant_id)
    .bind(address.kind.as_resource_kind())
    .bind(&address.document_key)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(document_record_from_row))
}

pub(super) fn document_record_from_row(
    (document_id, content_type, content_json, content_bytes, etag, updated_at): DocumentRow,
) -> DocumentRecord {
    let content = content_json.map_or_else(
        || DocumentContent::Bytes {
            content_type: content_type.clone(),
            body: content_bytes.unwrap_or_default(),
        },
        |content| DocumentContent::Json(content.0),
    );
    DocumentRecord {
        document_id,
        content,
        content_type,
        etag,
        updated_at,
    }
}

pub(super) fn document_json_content(content: &DocumentContent) -> Option<SqlJson<Value>> {
    match content {
        DocumentContent::Json(value) => Some(SqlJson(value.clone())),
        DocumentContent::Bytes { .. } => None,
    }
}

pub(super) fn document_bytes_content(content: &DocumentContent) -> Option<Vec<u8>> {
    match content {
        DocumentContent::Json(_value) => None,
        DocumentContent::Bytes { body, .. } => Some(body.clone()),
    }
}

pub(super) fn put_document_decision(
    existing: Option<&DocumentRecord>,
    _content: &DocumentContent,
    preconditions: &Preconditions,
) -> DocumentWriteDecision {
    document_put_decision(
        preconditions_pass(existing, preconditions),
        preconditions.has_any(),
        existing.is_some(),
    )
}

pub(super) fn preconditions_pass(
    existing: Option<&DocumentRecord>,
    preconditions: &Preconditions,
) -> bool {
    document_preconditions_pass(
        existing.map(|record| record.etag.as_str()),
        preconditions.if_match.as_deref(),
        preconditions.if_none_match.as_deref(),
    )
}

pub(super) fn merged_json_document(
    existing: Option<&DocumentRecord>,
    content: &DocumentContent,
) -> Option<DocumentContent> {
    if document_json_merge_decision(
        existing.map(|record| record.content.shape()),
        content.shape(),
    ) != DocumentJsonMergeDecision::Merge
    {
        return None;
    }

    let patch = content.as_json_object()?;
    let mut merged = existing.map_or_else(Map::new, |record| {
        record
            .content
            .as_json_object()
            .cloned()
            .unwrap_or_else(Map::new)
    });
    for (key, value) in patch {
        merged.insert(key.clone(), value.clone());
    }
    Some(DocumentContent::Json(Value::Object(merged)))
}

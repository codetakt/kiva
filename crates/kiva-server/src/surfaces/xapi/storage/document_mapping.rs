#![allow(clippy::wildcard_imports)]

use super::*;

pub(super) async fn select_document<'a>(
    executor: impl sqlx::Executor<'a, Database = sqlx::Postgres>,
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
    .bind(address.document_key.as_bytes())
    .fetch_optional(executor)
    .await?;

    row.map(document_record_from_row).transpose()
}

pub(super) fn document_record_from_row(
    (document_id, content_type, content_json, content_bytes, etag, updated_at): DocumentRow,
) -> Result<DocumentRecord, DocumentStoreError> {
    let document_id =
        String::from_utf8(document_id).map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
    let content = content_json.map_or_else(
        || DocumentContent::Bytes {
            content_type: content_type.clone(),
            body: content_bytes.unwrap_or_default(),
        },
        |content| DocumentContent::Json(content.0.0),
    );
    Ok(DocumentRecord {
        document_id,
        content,
        content_type,
        etag,
        updated_at,
    })
}

pub(super) fn document_json_content(content: &DocumentContent) -> Option<String> {
    match content {
        DocumentContent::Json(value) => Some(value.to_string()),
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
    if let (Some(record), Some(instant)) = (existing, preconditions.if_unmodified_since)
        && record.updated_at.unix_timestamp() > instant.unix_timestamp()
    {
        return false;
    }
    // RFC2616 14.26: a matching If-None-Match does not suppress a method
    // when its accompanying valid If-Modified-Since requires the newer entity.
    let if_none_match = if existing.is_some_and(|record| {
        preconditions
            .if_modified_since
            .is_some_and(|date| record.updated_at.unix_timestamp() > date.unix_timestamp())
    }) {
        None
    } else {
        preconditions.if_none_match.as_deref()
    };
    document_preconditions_pass(
        existing.map(|record| record.etag.as_str()),
        preconditions.if_match.as_deref(),
        if_none_match,
    )
}

pub(super) fn scope_preconditions_pass<'a>(
    ids: impl Iterator<Item = &'a str>,
    preconditions: &Preconditions,
) -> bool {
    let list = crate::surfaces::xapi::model::DocumentList::new(ids);
    // Conditions apply to the representation selected by this Request-URI,
    // including an empty list, not independently to each member document.
    // Lists have no modification history, so date conditions are unknown.
    document_preconditions_pass(
        Some(&list.etag),
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
    let Some(existing) = existing else {
        return Some(content.clone());
    };
    let mut merged = existing.content.as_json_object()?;
    // Replace complete top-level values. Never parse/re-serialize nested owner
    // values or discard their repeated member names and exact number tokens.
    let replaced = patch
        .0
        .iter()
        .map(|(key, _)| key)
        .collect::<std::collections::BTreeSet<_>>();
    merged.0.retain(|(key, _)| !replaced.contains(key));
    merged.0.extend(patch.0);
    Some(DocumentContent::Bytes {
        content_type: content.content_type().to_owned(),
        body: serde_json::to_vec(&merged).ok()?,
    })
}

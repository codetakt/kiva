#![allow(clippy::wildcard_imports)]

use super::document_mapping::*;
use super::*;

impl PostgresDocumentRepository {
    pub(crate) fn new(pool: PgPool, tenant_id: String) -> Result<Self, io::Error> {
        if tenant_id.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{KIVA_TENANT_ID_ENV} must not be empty"),
            ));
        }
        Ok(Self { pool, tenant_id })
    }

    pub(crate) async fn get(
        &self,
        address: &DocumentAddress,
    ) -> Result<Option<DocumentRecord>, DocumentStoreError> {
        select_document(&self.pool, &self.tenant_id, address).await
    }

    pub(crate) async fn list_ids(
        &self,
        scope: &DocumentScope,
    ) -> Result<Vec<String>, DocumentStoreError> {
        let rows = sqlx::query_as::<_, (String,)>(
            "SELECT DISTINCT document_id FROM kiva.xapi_documents \
             WHERE tenant_id = $1 AND resource_kind = $2 AND starts_with(document_key, $3) \
             AND ($4::timestamptz IS NULL OR updated_at > $4) \
             ORDER BY document_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(scope.kind.as_resource_kind())
        .bind(&scope.document_key_prefix)
        .bind(scope.since)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(document_id,)| document_id).collect())
    }

    pub(crate) async fn put(
        &self,
        address: DocumentAddress,
        content: DocumentContent,
        preconditions: Preconditions,
    ) -> Result<DocumentWriteOutcome, DocumentStoreError> {
        let existing = self.get(&address).await?;
        let decision = put_document_decision(existing.as_ref(), &content, &preconditions);
        if decision != DocumentWriteDecision::Write {
            return Ok(decision.into());
        }

        self.upsert(&address, content).await?;
        Ok(DocumentWriteOutcome::Stored)
    }

    pub(crate) async fn post_json_merge(
        &self,
        address: DocumentAddress,
        content: DocumentContent,
        preconditions: Preconditions,
    ) -> Result<DocumentWriteOutcome, DocumentStoreError> {
        let existing = self.get(&address).await?;
        if !preconditions_pass(existing.as_ref(), &preconditions) {
            return Ok(DocumentWriteOutcome::PreconditionFailed);
        }
        let Some(merged) = merged_json_document(existing.as_ref(), &content) else {
            return Ok(DocumentWriteOutcome::InvalidMerge);
        };

        self.upsert(&address, merged).await?;
        Ok(DocumentWriteOutcome::Stored)
    }

    pub(crate) async fn delete(
        &self,
        selection: &DocumentSelection,
        preconditions: &Preconditions,
    ) -> Result<DocumentDeleteOutcome, DocumentStoreError> {
        match selection {
            DocumentSelection::Single(address) => {
                let Some(existing) = self.get(address).await? else {
                    if !preconditions_pass(None, preconditions) {
                        return Ok(DocumentDeleteOutcome::PreconditionFailed);
                    }
                    return Ok(DocumentDeleteOutcome::NotFound);
                };
                if !preconditions_pass(Some(&existing), preconditions) {
                    return Ok(DocumentDeleteOutcome::PreconditionFailed);
                }
                sqlx::query(
                    "DELETE FROM kiva.xapi_documents \
                     WHERE tenant_id = $1 AND resource_kind = $2 AND document_key = $3",
                )
                .bind(&self.tenant_id)
                .bind(address.kind.as_resource_kind())
                .bind(&address.document_key)
                .execute(&self.pool)
                .await?;
                Ok(DocumentDeleteOutcome::Deleted)
            }
            DocumentSelection::Scope(scope) => {
                let records = self.records_for_scope(scope).await?;
                if records.is_empty() {
                    return Ok(DocumentDeleteOutcome::NotFound);
                }
                if records
                    .iter()
                    .any(|record| !preconditions_pass(Some(record), preconditions))
                {
                    return Ok(DocumentDeleteOutcome::PreconditionFailed);
                }
                sqlx::query(
                    "DELETE FROM kiva.xapi_documents \
                     WHERE tenant_id = $1 AND resource_kind = $2 AND starts_with(document_key, $3)",
                )
                .bind(&self.tenant_id)
                .bind(scope.kind.as_resource_kind())
                .bind(&scope.document_key_prefix)
                .execute(&self.pool)
                .await?;
                Ok(DocumentDeleteOutcome::Deleted)
            }
        }
    }

    async fn records_for_scope(
        &self,
        scope: &DocumentScope,
    ) -> Result<Vec<DocumentRecord>, DocumentStoreError> {
        let rows = sqlx::query_as::<_, DocumentRow>(
            "SELECT document_id, content_type, content_json, content_bytes, etag, updated_at \
             FROM kiva.xapi_documents \
             WHERE tenant_id = $1 AND resource_kind = $2 AND starts_with(document_key, $3) \
             ORDER BY document_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(scope.kind.as_resource_kind())
        .bind(&scope.document_key_prefix)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(document_record_from_row).collect())
    }

    async fn upsert(
        &self,
        address: &DocumentAddress,
        content: DocumentContent,
    ) -> Result<(), DocumentStoreError> {
        let etag = new_etag();
        sqlx::query(
            "INSERT INTO kiva.xapi_documents \
             (tenant_id, resource_kind, document_key, document_id, activity_id, agent, registration, \
              content_type, content_json, content_bytes, etag, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now()) \
             ON CONFLICT (tenant_id, resource_kind, document_key) DO UPDATE SET \
              document_id = EXCLUDED.document_id, \
              activity_id = EXCLUDED.activity_id, \
              agent = EXCLUDED.agent, \
              registration = EXCLUDED.registration, \
              content_type = EXCLUDED.content_type, \
              content_json = EXCLUDED.content_json, \
              content_bytes = EXCLUDED.content_bytes, \
              etag = EXCLUDED.etag, \
              updated_at = now()",
        )
        .bind(&self.tenant_id)
        .bind(address.kind.as_resource_kind())
        .bind(&address.document_key)
        .bind(&address.document_id)
        .bind(address.activity_id.as_deref())
        .bind(address.agent.clone().map(SqlJson))
        .bind(address.registration)
        .bind(content.content_type())
        .bind(document_json_content(&content))
        .bind(document_bytes_content(&content))
        .bind(etag)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

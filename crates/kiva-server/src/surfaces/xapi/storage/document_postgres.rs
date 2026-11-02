#![allow(clippy::wildcard_imports)]

use super::document_mapping::*;
use super::*;

/// Bind an instant without `SQLx`'s unchecked conversion to a UTC calendar date.
/// A legal local RFC3339 year can cross that conversion's supported year range.
/// Stored times have microsecond precision, so flooring (also before the epoch)
/// preserves the exclusive `updated_at > since` comparison exactly.
fn since_interval(
    since: time::OffsetDateTime,
) -> Result<sqlx::postgres::types::PgInterval, sqlx::Error> {
    const DAY_MICROS: i128 = 86_400_000_000;
    let micros = since.unix_timestamp_nanos().div_euclid(1_000);
    Ok(sqlx::postgres::types::PgInterval {
        months: 0,
        days: i32::try_from(micros.div_euclid(DAY_MICROS))
            .map_err(|error| sqlx::Error::Encode(Box::new(error)))?,
        microseconds: i64::try_from(micros.rem_euclid(DAY_MICROS))
            .map_err(|error| sqlx::Error::Encode(Box::new(error)))?,
    })
}

impl PostgresDocumentRepository {
    pub(crate) fn new(pool: PgPool, tenant_id: String) -> Result<Self, io::Error> {
        if tenant_id.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{KIVA_TENANT_ID_ENV} must not be empty"),
            ));
        }
        Ok(Self {
            pool,
            tenant_id,
            clock: RecordingClock::default(),
        })
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
        let since = scope.since.map(since_interval).transpose()?;
        // Apply integer days/microseconds to a timestamp without time zone,
        // then interpret it as UTC: no floating-point or session-DST rounding.
        let rows = sqlx::query_as::<_, (Vec<u8>,)>(
            "SELECT DISTINCT document_id FROM kiva.xapi_documents \
             WHERE tenant_id = $1 AND resource_kind = $2 \
             AND substring(document_key FROM 1 FOR octet_length($3::bytea)) = $3 \
             AND ($4::interval IS NULL OR updated_at > \
                  ((TIMESTAMP '1970-01-01' + $4::interval) AT TIME ZONE 'UTC')) \
             ORDER BY document_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(scope.kind.as_resource_kind())
        .bind(scope.document_key_prefix.as_bytes())
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|(document_id,)| {
                String::from_utf8(document_id)
                    .map_err(|error| sqlx::Error::Decode(Box::new(error)).into())
            })
            .collect()
    }

    pub(crate) async fn put(
        &self,
        address: DocumentAddress,
        content: DocumentContent,
        preconditions: Preconditions,
    ) -> Result<DocumentWriteOutcome, DocumentStoreError> {
        let mut transaction = self.begin_mutation(address.kind).await?;
        let existing = select_document(&mut *transaction, &self.tenant_id, &address).await?;
        let decision = put_document_decision(existing.as_ref(), &preconditions);
        if decision != DocumentWriteDecision::Write {
            return Ok(decision.into());
        }

        self.upsert(&mut transaction, &address, content).await?;
        transaction.commit().await?;
        Ok(DocumentWriteOutcome::Stored)
    }

    pub(crate) async fn post_json_merge(
        &self,
        address: DocumentAddress,
        content: DocumentContent,
        preconditions: Preconditions,
    ) -> Result<DocumentWriteOutcome, DocumentStoreError> {
        let mut transaction = self.begin_mutation(address.kind).await?;
        let existing = select_document(&mut *transaction, &self.tenant_id, &address).await?;
        let Some(merged) = merged_json_document(existing.as_ref(), &content) else {
            return Ok(DocumentWriteOutcome::InvalidMerge);
        };
        if preconditions.undefined_combination {
            return Ok(DocumentWriteOutcome::UndefinedPreconditions);
        }
        if !preconditions_pass(existing.as_ref(), &preconditions) {
            return Ok(DocumentWriteOutcome::PreconditionFailed);
        }

        self.upsert(&mut transaction, &address, merged).await?;
        transaction.commit().await?;
        Ok(DocumentWriteOutcome::Stored)
    }

    pub(crate) async fn delete(
        &self,
        selection: &DocumentSelection,
        preconditions: &Preconditions,
    ) -> Result<DocumentDeleteOutcome, DocumentStoreError> {
        let kind = match selection {
            DocumentSelection::Single(address) => address.kind,
            DocumentSelection::Scope(scope) => scope.kind,
        };
        let mut transaction = self.begin_mutation(kind).await?;
        let outcome = match selection {
            DocumentSelection::Single(address) => {
                let Some(existing) =
                    select_document(&mut *transaction, &self.tenant_id, address).await?
                else {
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
                .bind(address.document_key.as_bytes())
                .execute(&mut *transaction)
                .await?;
                DocumentDeleteOutcome::Deleted
            }
            DocumentSelection::Scope(scope) => {
                let records = self.records_for_scope(&mut transaction, scope).await?;
                if !scope_preconditions_pass(
                    records.iter().map(|record| record.document_id.as_str()),
                    preconditions,
                ) {
                    return Ok(DocumentDeleteOutcome::PreconditionFailed);
                }
                if records.is_empty() {
                    return Ok(DocumentDeleteOutcome::NotFound);
                }
                sqlx::query(
                    "DELETE FROM kiva.xapi_documents \
                     WHERE tenant_id = $1 AND resource_kind = $2 \
                     AND substring(document_key FROM 1 FOR octet_length($3::bytea)) = $3",
                )
                .bind(&self.tenant_id)
                .bind(scope.kind.as_resource_kind())
                .bind(scope.document_key_prefix.as_bytes())
                .execute(&mut *transaction)
                .await?;
                DocumentDeleteOutcome::Deleted
            }
        };
        transaction.commit().await?;
        Ok(outcome)
    }

    async fn begin_mutation(
        &self,
        kind: crate::surfaces::xapi::model::DocumentKind,
    ) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, DocumentStoreError> {
        let mut transaction = self.pool.begin().await?;
        // Serialize the precondition/read/modify/write operation across all
        // connections, including missing rows and State scope deletion. A row
        // lock alone cannot protect a not-yet-existing document or scope.
        let namespace = format!(
            "xapi-document:{}:{}:{}",
            self.tenant_id.len(),
            self.tenant_id,
            kind.as_resource_kind()
        );
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(namespace)
            .execute(&mut *transaction)
            .await?;
        Ok(transaction)
    }

    async fn records_for_scope(
        &self,
        connection: &mut sqlx::PgConnection,
        scope: &DocumentScope,
    ) -> Result<Vec<DocumentRecord>, DocumentStoreError> {
        let rows = sqlx::query_as::<_, DocumentRow>(
            "SELECT document_id, content_type, content_json, content_bytes, etag, updated_at \
             FROM kiva.xapi_documents \
             WHERE tenant_id = $1 AND resource_kind = $2 \
             AND substring(document_key FROM 1 FOR octet_length($3::bytea)) = $3 \
             ORDER BY document_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(scope.kind.as_resource_kind())
        .bind(scope.document_key_prefix.as_bytes())
        .fetch_all(connection)
        .await?;

        rows.into_iter().map(document_record_from_row).collect()
    }

    async fn upsert(
        &self,
        connection: &mut sqlx::PgConnection,
        address: &DocumentAddress,
        content: DocumentContent,
    ) -> Result<(), DocumentStoreError> {
        let etag = new_etag();
        sqlx::query(
            "INSERT INTO kiva.xapi_documents \
             (tenant_id, resource_kind, document_key, document_id, activity_id, agent, registration, \
              content_type, content_json, content_bytes, etag, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6::json, $7, $8, $9::json, $10, $11, COALESCE($12, clock_timestamp())) \
             ON CONFLICT (tenant_id, resource_kind, document_key) DO UPDATE SET \
              document_id = EXCLUDED.document_id, \
              activity_id = EXCLUDED.activity_id, \
              agent = EXCLUDED.agent, \
              registration = EXCLUDED.registration, \
              content_type = EXCLUDED.content_type, \
              content_json = EXCLUDED.content_json, \
              content_bytes = EXCLUDED.content_bytes, \
              etag = EXCLUDED.etag, \
              updated_at = EXCLUDED.updated_at",
        )
        .bind(&self.tenant_id)
        .bind(address.kind.as_resource_kind())
        .bind(address.document_key.as_bytes())
        .bind(address.document_id.as_bytes())
        .bind(address.activity_id.as_deref())
        .bind(address.agent.as_ref().map(Value::to_string))
        .bind(address.registration)
        .bind(content.content_type())
        .bind(document_json_content(&content))
        .bind(document_bytes_content(&content))
        .bind(etag)
        .bind(self.clock.override_time())
        .execute(connection)
        .await?;

        Ok(())
    }
}

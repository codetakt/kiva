#![allow(clippy::wildcard_imports)]

use super::statement_queries::*;
use super::*;

impl PostgresStatementRepository {
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
        statement_id: Uuid,
    ) -> Result<Option<StoredStatement>, StatementStoreError> {
        let row = sqlx::query_as::<_, (SqlJson<Value>, OffsetDateTime)>(
            "SELECT statement, stored_at FROM kiva.xapi_statements \
             WHERE tenant_id = $1 AND statement_id = $2",
        )
        .bind(&self.tenant_id)
        .bind(statement_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some((statement, stored_at)) = row else {
            return Ok(None);
        };
        let attachments =
            select_statement_attachments(&self.pool, &self.tenant_id, statement_id).await?;
        Ok(Some(StoredStatement::new(
            statement.0,
            stored_at,
            attachments,
        )))
    }

    pub(crate) async fn list(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<Value>, StatementStoreError> {
        let rows = sqlx::query_as::<_, (SqlJson<Value>,)>(
            "SELECT statement FROM kiva.xapi_statements \
             WHERE tenant_id = $1 \
             ORDER BY stored_at ASC, statement_id ASC \
             LIMIT $2",
        )
        .bind(&self.tenant_id)
        .bind(sql_limit(limit))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(statement,)| statement.0)
            .collect::<Vec<_>>())
    }

    pub(crate) async fn list_metadata_by_actor_ifi(
        &self,
        actor_ifi: &str,
    ) -> Result<Vec<StoredStatementMetadata>, StatementStoreError> {
        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                Option<String>,
                Option<String>,
                OffsetDateTime,
                String,
            ),
        >(
            "SELECT statement_id, actor_ifi, verb_id, object_id, stored_at, version \
             FROM kiva.xapi_statements \
             WHERE tenant_id = $1 AND actor_ifi = $2 \
             ORDER BY stored_at ASC, statement_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(actor_ifi)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(statement_id, actor_ifi, verb_id, object_id, stored_at, version)| {
                    StoredStatementMetadata {
                        statement_id,
                        actor_ifi,
                        verb_id,
                        object_id,
                        stored_at,
                        version,
                    }
                },
            )
            .collect())
    }

    pub(crate) async fn list_portability_page_by_actor_ifi(
        &self,
        actor_ifi: &str,
        cursor: Option<Uuid>,
        limit: usize,
    ) -> Result<Option<Vec<StoredPortabilityStatement>>, StatementStoreError> {
        let cursor_position = if let Some(cursor) = cursor {
            let position = sqlx::query_as::<_, (OffsetDateTime, Uuid)>(
                "SELECT stored_at, statement_id FROM kiva.xapi_statements \
                 WHERE tenant_id = $1 AND actor_ifi = $2 AND statement_id = $3",
            )
            .bind(&self.tenant_id)
            .bind(actor_ifi)
            .bind(cursor)
            .fetch_optional(&self.pool)
            .await?;
            let Some(position) = position else {
                return Ok(None);
            };
            Some(position)
        } else {
            None
        };
        let (cursor_stored_at, cursor_id) = cursor_position
            .map_or((OffsetDateTime::UNIX_EPOCH, Uuid::nil()), |position| {
                position
            });
        let rows = sqlx::query_as::<_, (Uuid, SqlJson<Value>, OffsetDateTime)>(
            "SELECT statement_id, statement, stored_at FROM kiva.xapi_statements \
             WHERE tenant_id = $1 AND actor_ifi = $2 \
               AND (stored_at, statement_id) > ($3, $4) \
             ORDER BY stored_at ASC, statement_id ASC LIMIT $5",
        )
        .bind(&self.tenant_id)
        .bind(actor_ifi)
        .bind(cursor_stored_at)
        .bind(cursor_id)
        .bind(sql_limit(Some(limit)))
        .fetch_all(&self.pool)
        .await?;
        let all = self.list(None).await?;
        Ok(Some(
            rows.into_iter()
                .map(
                    |(statement_id, statement, stored_at)| StoredPortabilityStatement {
                        statement_id,
                        statement: statement.0,
                        stored_at,
                        voided: kiva_xapi::statement_is_voided(&all, statement_id),
                    },
                )
                .collect(),
        ))
    }

    pub(crate) async fn put(
        &self,
        statement_id: Uuid,
        statement: StoredStatement,
    ) -> Result<StoreOutcome, StatementStoreError> {
        let outcome = self.insert_statement(statement_id, &statement).await?;
        if outcome == InsertOutcome::Inserted {
            return Ok(StoreOutcome::Stored);
        }

        let existing = self.get(statement_id).await?;
        Ok(
            match statement_put_decision(
                existing
                    .as_ref()
                    .map(|existing| stored_statements_equivalent(existing, &statement)),
            ) {
                StatementPutDecision::Store | StatementPutDecision::Conflict => {
                    StoreOutcome::Conflict
                }
                StatementPutDecision::Unchanged => StoreOutcome::Unchanged,
            },
        )
    }

    pub(crate) async fn put_batch(
        &self,
        batch: Vec<(Uuid, StoredStatement)>,
    ) -> Result<(), PutBatchError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(StatementStoreError::from)
            .map_err(PutBatchError::Store)?;
        for (statement_id, statement) in batch {
            let outcome = self
                .insert_statement_in_transaction(&mut transaction, statement_id, &statement)
                .await
                .map_err(PutBatchError::Store)?;
            if outcome == InsertOutcome::Inserted {
                continue;
            }

            let existing =
                select_statement_in_transaction(&mut transaction, &self.tenant_id, statement_id)
                    .await
                    .map_err(PutBatchError::Store)?;
            match existing {
                Some(existing) if stored_statements_equivalent(&existing, &statement) => {}
                Some(_) | None => return Err(PutBatchError::Conflict(statement_id)),
            }
        }

        transaction
            .commit()
            .await
            .map_err(StatementStoreError::from)
            .map_err(PutBatchError::Store)?;
        Ok(())
    }

    async fn insert_statement(
        &self,
        statement_id: Uuid,
        statement: &StoredStatement,
    ) -> Result<InsertOutcome, StatementStoreError> {
        let mut transaction = self.pool.begin().await?;
        let outcome = self
            .insert_statement_in_transaction(&mut transaction, statement_id, statement)
            .await?;
        transaction.commit().await?;
        Ok(outcome)
    }

    async fn insert_statement_in_transaction(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        statement_id: Uuid,
        statement: &StoredStatement,
    ) -> Result<InsertOutcome, StatementStoreError> {
        let metadata = StatementMetadata::from_statement(&statement.statement, XAPI_VERSION);
        let result = sqlx::query(
            "INSERT INTO kiva.xapi_statements \
             (tenant_id, statement_id, actor_ifi, verb_id, object_id, authority, statement, stored_at, version) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             ON CONFLICT (tenant_id, statement_id) DO NOTHING",
        )
        .bind(&self.tenant_id)
        .bind(statement_id)
        .bind(metadata.actor_ifi)
        .bind(metadata.verb_id)
        .bind(metadata.object_id)
        .bind(metadata.authority.map(SqlJson))
        .bind(SqlJson(statement.statement.clone()))
        .bind(statement.stored_at)
        .bind(metadata.version)
        .execute(&mut **transaction)
        .await?;

        let outcome = InsertOutcome::from_rows_affected(result.rows_affected());
        if outcome == InsertOutcome::Inserted {
            insert_statement_attachments_in_transaction(
                transaction,
                &self.tenant_id,
                statement_id,
                &statement.attachments,
            )
            .await?;
        }
        Ok(outcome)
    }
}

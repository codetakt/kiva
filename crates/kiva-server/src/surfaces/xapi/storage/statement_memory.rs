#![allow(clippy::wildcard_imports)]

use super::statement_queries::stored_statements_equivalent;
use super::*;

impl MemoryStatementRepository {
    pub(crate) async fn opaque_values(
        &self,
        ids: &[Uuid],
    ) -> BTreeMap<Uuid, crate::surfaces::xapi::lossless::OpaqueValues> {
        let stored = self.statements.read().await;
        ids.iter()
            .filter_map(|id| {
                stored
                    .get(id)
                    .filter(|record| !record.opaque_values.is_empty())
                    .map(|record| (*id, record.opaque_values.clone()))
            })
            .collect()
    }

    pub(crate) async fn consistent_through(&self) -> OffsetDateTime {
        self.statements
            .read()
            .await
            .values()
            .map(|record| record.stored_at)
            .max()
            .unwrap_or(EMPTY_WATERMARK)
    }

    pub(crate) async fn get(&self, statement_id: Uuid) -> Option<StoredStatement> {
        self.statements.read().await.get(&statement_id).cloned()
    }

    pub(crate) async fn list(&self, limit: Option<usize>) -> Vec<Value> {
        let limit = limit.unwrap_or(usize::MAX);
        let statements = self.statements.read().await;
        // Match the persistent repository's ordering before applying a limit.
        // Activity metadata uses this sequence to retain first-known values;
        // the map's UUID order cannot stand in for recording order.
        let mut records = statements.iter().collect::<Vec<_>>();
        records.sort_by(|(left_id, left), (right_id, right)| {
            left.stored_at
                .cmp(&right.stored_at)
                .then_with(|| left_id.cmp(right_id))
        });
        let selected = records
            .into_iter()
            .take(limit)
            .map(|(_, record)| record.statement.clone())
            .collect();
        drop(statements);
        selected
    }

    pub(crate) async fn list_metadata_by_actor_ifi(
        &self,
        actor_ifi: &str,
    ) -> Vec<StoredStatementMetadata> {
        let mut records = self
            .statements
            .read()
            .await
            .iter()
            .filter_map(|(statement_id, record)| {
                stored_statement_metadata(*statement_id, record)
                    .filter(|metadata| metadata.actor_ifi == actor_ifi)
            })
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            left.stored_at
                .cmp(&right.stored_at)
                .then_with(|| left.statement_id.cmp(&right.statement_id))
        });
        records
    }

    pub(crate) async fn list_portability_page_by_actor_ifi(
        &self,
        actor_ifi: &str,
        cursor: Option<Uuid>,
        limit: usize,
    ) -> Option<Vec<StoredPortabilityStatement>> {
        let statements = self.statements.read().await;
        let all = statements
            .values()
            .map(|record| record.statement.clone())
            .collect::<Vec<_>>();
        let mut owned = statements
            .iter()
            .filter_map(|(statement_id, record)| {
                stored_statement_metadata(*statement_id, record)
                    .filter(|metadata| metadata.actor_ifi == actor_ifi)
                    .map(|metadata| StoredPortabilityStatement {
                        statement_id: metadata.statement_id,
                        statement: record.statement.clone(),
                        opaque_values: record.opaque_values.clone(),
                        stored_at: record.stored_at,
                        voided: kiva_xapi::statement_is_voided(&all, metadata.statement_id),
                    })
            })
            .collect::<Vec<_>>();
        drop(statements);
        owned.sort_by(|left, right| {
            left.stored_at
                .cmp(&right.stored_at)
                .then_with(|| left.statement_id.cmp(&right.statement_id))
        });
        let start = match cursor {
            Some(cursor) => {
                owned
                    .iter()
                    .position(|record| record.statement_id == cursor)?
                    + 1
            }
            None => 0,
        };
        Some(owned.into_iter().skip(start).take(limit).collect())
    }

    pub(crate) async fn put(
        &self,
        statement_id: Uuid,
        mut statement: StoredStatement,
    ) -> StoreOutcome {
        let _admission = self.admission.lock().await;
        let existing_matches = self
            .statements
            .read()
            .await
            .get(&statement_id)
            .map(|existing| stored_statements_equivalent(existing, &statement));
        match statement_put_decision(existing_matches) {
            StatementPutDecision::Store => {
                let instant = self.clock.next(self.consistent_through().await);
                statement.record_at(instant);
                self.clock.recorded(instant).await;
                self.statements
                    .write()
                    .await
                    .insert(statement_id, statement);
                StoreOutcome::Stored
            }
            StatementPutDecision::Unchanged => StoreOutcome::Unchanged,
            StatementPutDecision::Conflict => StoreOutcome::Conflict,
        }
    }

    #[allow(clippy::significant_drop_tightening)]
    pub(crate) async fn put_batch(&self, batch: Vec<(Uuid, StoredStatement)>) -> Result<(), Uuid> {
        let _admission = self.admission.lock().await;
        let statements = self.statements.read().await;
        for (statement_id, statement) in &batch {
            if statements
                .get(statement_id)
                .is_some_and(|existing| !stored_statements_equivalent(existing, statement))
            {
                return Err(*statement_id);
            }
        }
        let instant = self.clock.next(
            statements
                .values()
                .map(|record| record.stored_at)
                .max()
                .unwrap_or(EMPTY_WATERMARK),
        );
        drop(statements);
        self.clock.recorded(instant).await;
        let mut statements = self.statements.write().await;
        for (statement_id, mut statement) in batch {
            statement.record_at(instant);
            statements.entry(statement_id).or_insert(statement);
        }
        Ok(())
    }
}

fn stored_statement_metadata(
    statement_id: Uuid,
    record: &StoredStatement,
) -> Option<StoredStatementMetadata> {
    let metadata = StatementMetadata::from_statement(&record.statement, XAPI_VERSION);
    Some(StoredStatementMetadata {
        statement_id,
        actor_ifi: metadata.actor_ifi?,
        verb_id: metadata.verb_id,
        object_id: metadata.object_id,
        stored_at: record.stored_at,
        version: metadata.version,
    })
}

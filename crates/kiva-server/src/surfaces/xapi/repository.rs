//! xAPI repository facades.

use std::io;

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::outcome::StoreOutcome;
use crate::surfaces::xapi::{
    model::{
        DocumentAddress, DocumentContent, DocumentRecord, DocumentScope, DocumentSelection,
        Preconditions, StoredPortabilityStatement, StoredStatement, StoredStatementMetadata,
    },
    outcome::{
        DocumentDeleteOutcome, DocumentStoreError, DocumentWriteOutcome, PutBatchError,
        StatementStoreError,
    },
    storage::{
        MemoryDocumentRepository, MemoryStatementRepository, PostgresDocumentRepository,
        PostgresStatementRepository,
    },
};

#[derive(Debug, Clone)]
pub(crate) enum StatementRepository {
    Memory(MemoryStatementRepository),
    Postgres(PostgresStatementRepository),
}

impl Default for StatementRepository {
    fn default() -> Self {
        Self::Memory(MemoryStatementRepository::default())
    }
}

#[derive(Debug, Clone)]
pub(crate) enum DocumentRepository {
    Memory(MemoryDocumentRepository),
    Postgres(PostgresDocumentRepository),
}

impl Default for DocumentRepository {
    fn default() -> Self {
        Self::Memory(MemoryDocumentRepository::default())
    }
}

impl StatementRepository {
    pub(crate) fn postgres(pool: PgPool, tenant_id: String) -> Result<Self, io::Error> {
        Ok(Self::Postgres(PostgresStatementRepository::new(
            pool, tenant_id,
        )?))
    }

    pub(crate) async fn get(
        &self,
        statement_id: Uuid,
    ) -> Result<Option<StoredStatement>, StatementStoreError> {
        dispatch_repository!(self, |repository| {
            memory => Ok(repository.get(statement_id).await),
            postgres => repository.get(statement_id).await,
        })
    }

    pub(crate) async fn list(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<Value>, StatementStoreError> {
        dispatch_repository!(self, |repository| {
            memory => Ok(repository.list(limit).await),
            postgres => repository.list(limit).await,
        })
    }

    pub(crate) async fn list_metadata_by_actor_ifi(
        &self,
        actor_ifi: &str,
    ) -> Result<Vec<StoredStatementMetadata>, StatementStoreError> {
        dispatch_repository!(self, |repository| {
            memory => Ok(repository.list_metadata_by_actor_ifi(actor_ifi).await),
            postgres => repository.list_metadata_by_actor_ifi(actor_ifi).await,
        })
    }

    pub(crate) async fn list_portability_page_by_actor_ifi(
        &self,
        actor_ifi: &str,
        cursor: Option<Uuid>,
        limit: usize,
    ) -> Result<Option<Vec<StoredPortabilityStatement>>, StatementStoreError> {
        dispatch_repository!(self, |repository| {
            memory => Ok(repository.list_portability_page_by_actor_ifi(actor_ifi, cursor, limit).await),
            postgres => repository.list_portability_page_by_actor_ifi(actor_ifi, cursor, limit).await,
        })
    }

    pub(crate) async fn put(
        &self,
        statement_id: Uuid,
        statement: StoredStatement,
    ) -> Result<StoreOutcome, StatementStoreError> {
        dispatch_repository!(self, |repository| {
            memory => Ok(repository.put(statement_id, statement).await),
            postgres => repository.put(statement_id, statement).await,
        })
    }

    pub(crate) async fn put_batch(
        &self,
        batch: Vec<(Uuid, StoredStatement)>,
    ) -> Result<(), PutBatchError> {
        dispatch_repository!(self, |repository| {
            memory => repository
                .put_batch(batch)
                .await
                .map_err(PutBatchError::Conflict),
            postgres => repository.put_batch(batch).await,
        })
    }
}

impl DocumentRepository {
    pub(crate) fn postgres(pool: PgPool, tenant_id: String) -> Result<Self, io::Error> {
        Ok(Self::Postgres(PostgresDocumentRepository::new(
            pool, tenant_id,
        )?))
    }

    pub(crate) async fn get(
        &self,
        address: &DocumentAddress,
    ) -> Result<Option<DocumentRecord>, DocumentStoreError> {
        dispatch_repository!(self, |repository| {
            memory => Ok(repository.get(address).await),
            postgres => repository.get(address).await,
        })
    }

    pub(crate) async fn list_ids(
        &self,
        scope: &DocumentScope,
    ) -> Result<Vec<String>, DocumentStoreError> {
        dispatch_repository!(self, |repository| {
            memory => Ok(repository.list_ids(scope).await),
            postgres => repository.list_ids(scope).await,
        })
    }

    pub(crate) async fn put(
        &self,
        address: DocumentAddress,
        content: DocumentContent,
        preconditions: Preconditions,
    ) -> Result<DocumentWriteOutcome, DocumentStoreError> {
        dispatch_repository!(self, |repository| {
            memory => Ok(repository.put(address, content, preconditions).await),
            postgres => repository.put(address, content, preconditions).await,
        })
    }

    pub(crate) async fn post_json_merge(
        &self,
        address: DocumentAddress,
        content: DocumentContent,
        preconditions: Preconditions,
    ) -> Result<DocumentWriteOutcome, DocumentStoreError> {
        dispatch_repository!(self, |repository| {
            memory => Ok(repository
                .post_json_merge(address, content, preconditions)
                .await),
            postgres => repository
                .post_json_merge(address, content, preconditions)
                .await,
        })
    }

    pub(crate) async fn delete(
        &self,
        selection: DocumentSelection,
        preconditions: Preconditions,
    ) -> Result<DocumentDeleteOutcome, DocumentStoreError> {
        dispatch_repository!(self, |repository| {
            memory => Ok(repository.delete(&selection, &preconditions).await),
            postgres => repository.delete(&selection, &preconditions).await,
        })
    }
}

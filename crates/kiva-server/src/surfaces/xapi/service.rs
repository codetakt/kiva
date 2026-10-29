//! xAPI application use cases.

use crate::{
    core::outcome::StoreOutcome,
    surfaces::xapi::{
        model::{
            DocumentAddress, DocumentContent, DocumentRecord, DocumentScope, DocumentSelection,
            Preconditions, StoredStatement,
        },
        outcome::{
            DocumentDeleteOutcome, DocumentStoreError, DocumentWriteOutcome, PutBatchError,
            StatementStoreError,
        },
        repository::{DocumentRepository, StatementRepository},
    },
};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub(crate) struct XapiServices<'a> {
    statements: &'a StatementRepository,
    documents: &'a DocumentRepository,
}

impl<'a> XapiServices<'a> {
    pub(crate) const fn new(
        statements: &'a StatementRepository,
        documents: &'a DocumentRepository,
    ) -> Self {
        Self {
            statements,
            documents,
        }
    }
}

pub(crate) async fn get_statement(
    services: XapiServices<'_>,
    statement_id: Uuid,
) -> Result<Option<StoredStatement>, StatementStoreError> {
    services.statements.get(statement_id).await
}

pub(crate) async fn list_statements(
    services: XapiServices<'_>,
    limit: Option<usize>,
) -> Result<Vec<Value>, StatementStoreError> {
    services.statements.list(limit).await
}

pub(crate) async fn statement_opaque_values(
    services: XapiServices<'_>,
    ids: &[Uuid],
) -> Result<std::collections::BTreeMap<Uuid, super::lossless::OpaqueValues>, StatementStoreError> {
    services.statements.opaque_values(ids).await
}

pub(crate) async fn put_statement(
    services: XapiServices<'_>,
    statement_id: Uuid,
    statement: StoredStatement,
) -> Result<StoreOutcome, StatementStoreError> {
    services.statements.put(statement_id, statement).await
}

pub(crate) async fn put_statement_batch(
    services: XapiServices<'_>,
    batch: Vec<(Uuid, StoredStatement)>,
) -> Result<(), PutBatchError> {
    services.statements.put_batch(batch).await
}

pub(crate) async fn get_document(
    services: XapiServices<'_>,
    address: &DocumentAddress,
) -> Result<Option<DocumentRecord>, DocumentStoreError> {
    services.documents.get(address).await
}

pub(crate) async fn list_document_ids(
    services: XapiServices<'_>,
    scope: &DocumentScope,
) -> Result<Vec<String>, DocumentStoreError> {
    services.documents.list_ids(scope).await
}

pub(crate) async fn put_document(
    services: XapiServices<'_>,
    address: DocumentAddress,
    content: DocumentContent,
    preconditions: Preconditions,
) -> Result<DocumentWriteOutcome, DocumentStoreError> {
    services
        .documents
        .put(address, content, preconditions)
        .await
}

pub(crate) async fn post_json_merge(
    services: XapiServices<'_>,
    address: DocumentAddress,
    content: DocumentContent,
    preconditions: Preconditions,
) -> Result<DocumentWriteOutcome, DocumentStoreError> {
    services
        .documents
        .post_json_merge(address, content, preconditions)
        .await
}

pub(crate) async fn delete_document(
    services: XapiServices<'_>,
    selection: DocumentSelection,
    preconditions: Preconditions,
) -> Result<DocumentDeleteOutcome, DocumentStoreError> {
    services.documents.delete(selection, preconditions).await
}

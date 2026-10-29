//! xAPI memory and `PostgreSQL` storage implementations.

use std::{collections::BTreeMap, io, sync::Arc};

#[cfg(test)]
pub(crate) use statement_queries::take_replay_observations;

use kiva_http_core::{
    DocumentJsonMergeDecision, DocumentWriteDecision, StatementPutDecision,
    document_json_merge_decision, document_preconditions_pass, document_put_decision,
    statement_put_decision,
};
use kiva_xapi::StatementMetadata;
use serde_json::Value;
use sqlx::{PgPool, types::Json as SqlJson};
use time::OffsetDateTime;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::core::{
    config::{KIVA_TENANT_ID_ENV, XAPI_VERSION},
    outcome::{InsertOutcome, StoreOutcome},
    recording_clock::{EMPTY_WATERMARK, RecordingClock},
    response::new_etag,
};
use crate::surfaces::xapi::{
    model::{
        AttachmentRecord, DocumentAddress, DocumentContent, DocumentRecord, DocumentScope,
        DocumentSelection, Preconditions, StoredPortabilityStatement, StoredStatement,
        StoredStatementMetadata,
    },
    outcome::{
        DocumentDeleteOutcome, DocumentStoreError, DocumentWriteOutcome, PutBatchError,
        StatementStoreError,
    },
    request::statements_equivalent,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct MemoryStatementRepository {
    statements: Arc<RwLock<BTreeMap<Uuid, StoredStatement>>>,
    admission: Arc<tokio::sync::Mutex<()>>,
    pub(crate) clock: RecordingClock,
    #[cfg(test)]
    pub(crate) read_failure: read_failure::ReadFailure,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MemoryDocumentRepository {
    documents: Arc<RwLock<BTreeMap<String, DocumentRecord>>>,
    pub(crate) clock: RecordingClock,
}

#[derive(Debug, Clone)]
pub(crate) struct PostgresStatementRepository {
    pool: PgPool,
    tenant_id: String,
    pub(crate) clock: RecordingClock,
    #[cfg(test)]
    pub(crate) read_failure: read_failure::ReadFailure,
}

#[derive(Debug, Clone)]
pub(crate) struct PostgresDocumentRepository {
    pool: PgPool,
    tenant_id: String,
    pub(crate) clock: RecordingClock,
}

type DocumentRow = (
    Vec<u8>,
    String,
    Option<SqlJson<super::lossless::StoredJson>>,
    Option<Vec<u8>>,
    String,
    OffsetDateTime,
);

mod document_mapping;
mod document_memory;
mod document_postgres;
#[cfg(test)]
pub(crate) mod read_failure;
mod statement_memory;
mod statement_postgres;
mod statement_queries;

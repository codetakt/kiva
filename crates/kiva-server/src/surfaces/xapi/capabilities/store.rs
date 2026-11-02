//! Tenant-local persistent extension records, with serialized conditional mutation.

use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::Mutex;

use crate::core::response::HttpError;
use axum::http::StatusCode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Record {
    pub(crate) body: Vec<u8>,
    pub(crate) etag: String,
}

#[derive(Debug, Clone)]
pub(crate) enum CapabilityStore {
    Memory(Arc<Mutex<BTreeMap<String, Record>>>),
    Postgres { pool: PgPool, tenant: String },
}

impl Default for CapabilityStore {
    fn default() -> Self {
        Self::Memory(Arc::default())
    }
}

fn failure(error: impl std::fmt::Display) -> HttpError {
    tracing::error!(error = %error, "xAPI capability storage failed");
    HttpError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        "xAPI capability storage failed",
    )
}

impl CapabilityStore {
    pub(crate) async fn get(&self, key: &str) -> Result<Option<Record>, HttpError> {
        match self {
            Self::Memory(records) => Ok(records.lock().await.get(key).cloned()),
            Self::Postgres { pool, tenant } => sqlx::query_as::<_, (Vec<u8>, String)>(
                "SELECT body, etag FROM kiva.xapi_capability_records WHERE tenant_id=$1 AND record_key=$2"
            ).bind(tenant).bind(key.as_bytes()).fetch_optional(pool).await.map_err(failure)
                .map(|row| row.map(|(body,etag)| Record { body,etag })),
        }
    }

    pub(crate) async fn list(&self, prefix: &str) -> Result<BTreeMap<String, Record>, HttpError> {
        match self {
            Self::Memory(records) => Ok(records
                .lock()
                .await
                .iter()
                .filter(|(key, _)| key.starts_with(prefix))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()),
            Self::Postgres { pool, tenant } => {
                let rows=sqlx::query_as::<_, (Vec<u8>, Vec<u8>, String)>(
                    "SELECT record_key, body, etag FROM kiva.xapi_capability_records WHERE tenant_id=$1 AND substring(record_key from 1 for octet_length($2::bytea))=$2 ORDER BY record_key"
                ).bind(tenant).bind(prefix.as_bytes()).fetch_all(pool).await.map_err(failure)?;
                rows.into_iter()
                    .map(|(key, body, etag)| {
                        String::from_utf8(key)
                            .map(|key| (key, Record { body, etag }))
                            .map_err(failure)
                    })
                    .filter(|row| {
                        row.as_ref()
                            .map_or(true, |(key, _)| key.starts_with(prefix))
                    })
                    .collect()
            }
        }
    }

    /// The callback observes and changes one namespace snapshot under the
    /// same database transaction lock on every process. It cannot do network I/O.
    pub(crate) async fn mutate<T>(
        &self,
        prefix: &str,
        apply: impl FnOnce(&mut BTreeMap<String, Record>) -> Result<T, HttpError> + Send,
    ) -> Result<T, HttpError> {
        match self {
            Self::Memory(records) => {
                let mut current = records.lock().await;
                let mut candidate = current
                    .iter()
                    .filter(|(k, _)| k.starts_with(prefix))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                let result = apply(&mut candidate)?;
                current.retain(|key, _| !key.starts_with(prefix));
                current.extend(candidate);
                drop(current);
                Ok(result)
            }
            Self::Postgres { pool, tenant } => {
                let mut transaction = pool.begin().await.map_err(failure)?;
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 72927412))")
                    .bind(format!("{tenant}:{}:{prefix}", tenant.len()))
                    .execute(&mut *transaction)
                    .await
                    .map_err(failure)?;
                let rows=sqlx::query_as::<_,(Vec<u8>,Vec<u8>,String)>(
                    "SELECT record_key, body, etag FROM kiva.xapi_capability_records WHERE tenant_id=$1 AND substring(record_key from 1 for octet_length($2::bytea))=$2"
                ).bind(tenant).bind(prefix.as_bytes()).fetch_all(&mut *transaction).await.map_err(failure)?;
                let mut before = BTreeMap::new();
                for (key, body, etag) in rows {
                    before.insert(
                        String::from_utf8(key).map_err(failure)?,
                        Record { body, etag },
                    );
                }
                let mut after = before.clone();
                let result = apply(&mut after)?;
                for key in before.keys().filter(|key| !after.contains_key(*key)) {
                    sqlx::query("DELETE FROM kiva.xapi_capability_records WHERE tenant_id=$1 AND record_key=$2")
                        .bind(tenant).bind(key.as_bytes()).execute(&mut *transaction).await.map_err(failure)?;
                }
                for (key, record) in after {
                    if before
                        .get(&key)
                        .is_some_and(|old| old.etag == record.etag && old.body == record.body)
                    {
                        continue;
                    }
                    sqlx::query("INSERT INTO kiva.xapi_capability_records (tenant_id,record_key,body,etag) VALUES ($1,$2,$3,$4) ON CONFLICT (tenant_id,record_key) DO UPDATE SET body=EXCLUDED.body,etag=EXCLUDED.etag")
                        .bind(tenant).bind(key.as_bytes()).bind(record.body).bind(record.etag)
                        .execute(&mut *transaction).await.map_err(failure)?;
                }
                transaction.commit().await.map_err(failure)?;
                Ok(result)
            }
        }
    }
}

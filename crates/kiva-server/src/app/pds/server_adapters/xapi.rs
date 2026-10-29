//! xAPI server adapter for PDS application reads.

use kiva_application::pds::{ApplicationFuture, XapiStatementReader};
use uuid::Uuid;

use crate::surfaces::xapi::repository::StatementRepository;

use super::{
    super::{
        PortabilityStatementPage, PortabilityStatementRecord, StatementReadError, StatementRecord,
    },
    records::{statement_error, statement_record},
};

#[derive(Debug, Clone)]
pub(crate) struct XapiStatementReadPort {
    repository: StatementRepository,
}

impl XapiStatementReadPort {
    pub(super) fn from_server_adapter(repository: &StatementRepository) -> Self {
        Self {
            repository: repository.clone(),
        }
    }

    async fn list_metadata_by_actor_ifi_impl(
        &self,
        actor_ifi: &str,
    ) -> Result<Vec<StatementRecord>, StatementReadError> {
        self.repository
            .list_metadata_by_actor_ifi(actor_ifi)
            .await
            .map(|metadata| metadata.into_iter().map(statement_record).collect())
            .map_err(statement_error)
    }

    async fn list_portability_page_by_actor_ifi_impl(
        &self,
        actor_ifi: &str,
        cursor: Option<&str>,
        page_size: usize,
    ) -> Result<PortabilityStatementPage, StatementReadError> {
        let cursor = cursor
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|_error| StatementReadError::InvalidCursor)?;
        let mut records = self
            .repository
            .list_portability_page_by_actor_ifi(actor_ifi, cursor, page_size.saturating_add(1))
            .await
            .map_err(|_error| StatementReadError::Store)?
            .ok_or(StatementReadError::InvalidCursor)?;
        let has_more = records.len() > page_size;
        if has_more {
            records.truncate(page_size);
        }
        let next_cursor = has_more
            .then(|| records.last().map(|record| record.statement_id.to_string()))
            .flatten();
        Ok(PortabilityStatementPage {
            statements: records
                .into_iter()
                .map(|record| PortabilityStatementRecord {
                    statement_id: record.statement_id,
                    statement: record.statement,
                    opaque_extension_values: record.opaque_values,
                    stored_at: record.stored_at,
                    voided: record.voided,
                })
                .collect(),
            next_cursor,
        })
    }
}

impl XapiStatementReader for XapiStatementReadPort {
    fn list_metadata_by_actor_ifi<'a>(
        &'a self,
        actor_ifi: &'a str,
    ) -> ApplicationFuture<'a, Result<Vec<StatementRecord>, StatementReadError>> {
        Box::pin(async move { self.list_metadata_by_actor_ifi_impl(actor_ifi).await })
    }

    fn list_portability_page_by_actor_ifi<'a>(
        &'a self,
        actor_ifi: &'a str,
        cursor: Option<&'a str>,
        page_size: usize,
    ) -> ApplicationFuture<'a, Result<PortabilityStatementPage, StatementReadError>> {
        Box::pin(async move {
            self.list_portability_page_by_actor_ifi_impl(actor_ifi, cursor, page_size)
                .await
        })
    }
}

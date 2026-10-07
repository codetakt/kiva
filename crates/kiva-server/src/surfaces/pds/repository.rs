//! PDS repository facade.

use std::io;

use sqlx::PgPool;

use crate::surfaces::pds::storage::{MemoryPdsRepository, PostgresPdsRepository};

mod access;
mod consent;
mod data_products;
mod governance;
mod lifecycle;
mod portability;
mod provider_bindings;

#[derive(Debug, Clone)]
pub(crate) enum PdsRepository {
    Memory(MemoryPdsRepository),
    Postgres(PostgresPdsRepository),
}

impl Default for PdsRepository {
    fn default() -> Self {
        Self::Memory(MemoryPdsRepository::default())
    }
}

impl PdsRepository {
    pub(crate) fn postgres(pool: PgPool, tenant_id: String) -> Result<Self, io::Error> {
        Ok(Self::Postgres(PostgresPdsRepository::new(pool, tenant_id)?))
    }

    #[cfg(test)]
    pub(crate) async fn ledger_event_matches_for_test(
        &self,
        event_kind: &str,
        owner_subject: &str,
        resource_kind: &str,
        resource_id: &str,
    ) -> Result<bool, String> {
        match self {
            Self::Memory(repository) => Ok(repository
                .ledger_event_for_test(event_kind, resource_id)
                .await
                .is_some_and(|event| {
                    event.owner_subject == owner_subject && event.resource_kind == resource_kind
                })),
            Self::Postgres(repository) => {
                repository
                    .ledger_event_matches_for_test(
                        event_kind,
                        owner_subject,
                        resource_kind,
                        resource_id,
                    )
                    .await
            }
        }
    }

    #[cfg(test)]
    pub(crate) async fn ledger_event_kind_count_for_test(
        &self,
        event_kind: &str,
        owner_subject: &str,
    ) -> Result<usize, String> {
        match self {
            Self::Memory(repository) => Ok(repository
                .ledger_events_for_test(event_kind, owner_subject)
                .await),
            Self::Postgres(repository) => {
                repository
                    .ledger_event_kind_count_for_test(event_kind, owner_subject)
                    .await
            }
        }
    }
}

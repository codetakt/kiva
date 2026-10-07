#![allow(clippy::wildcard_imports)]

use super::*;

impl ConfidentialRepository {
    pub(crate) fn postgres(pool: PgPool, tenant_id: String) -> Result<Self, io::Error> {
        Ok(Self::Postgres(PostgresConfidentialRepository::new(
            pool, tenant_id,
        )?))
    }
}

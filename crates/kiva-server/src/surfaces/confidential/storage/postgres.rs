#![allow(clippy::wildcard_imports)]

use super::*;

mod additive;
mod commands;
pub(super) mod differential_privacy;
mod differential_privacy_api;
mod jobs;
pub(super) mod lifecycle;
mod participants;
pub(super) mod queries;
mod release_artifacts;
mod shamir;
pub(super) mod shares_threshold;
mod threshold;

use differential_privacy::*;
use lifecycle::*;
use queries::*;
use shares_threshold::*;

impl PostgresConfidentialRepository {
    pub(crate) fn new(pool: PgPool, tenant_id: String) -> Result<Self, io::Error> {
        if tenant_id.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{KIVA_TENANT_ID_ENV} must not be empty"),
            ));
        }
        Ok(Self { pool, tenant_id })
    }
}

//! Environment-backed server bootstrap.

use std::io;

use kiva_idp_adapter::BearerTokenVerifier;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

use crate::{
    core::{
        auth::AuthConfig,
        config::{DATABASE_URL_ENV, DEFAULT_TENANT_ID, KIVA_TENANT_ID_ENV},
        runtime::{RuntimeLimits, env_or_default},
        state::AppState,
    },
    surfaces::{
        confidential::repository::ConfidentialRepository,
        pds::repository::PdsRepository,
        xapi::repository::{DocumentRepository, StatementRepository},
    },
};

impl AppState {
    pub(crate) async fn from_env(
        token_verifier: Option<Arc<dyn BearerTokenVerifier>>,
    ) -> Result<Self, io::Error> {
        let auth = AuthConfig::from_env(token_verifier)?;
        let limits = RuntimeLimits::from_env()?;
        let signature_policy = crate::core::signature_policy::SignaturePolicy::from_env()?;
        let mut state = match std::env::var(DATABASE_URL_ENV) {
            Ok(database_url) if database_url.is_empty() => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{DATABASE_URL_ENV} must not be empty when set"),
            )),
            Ok(database_url) => {
                let tenant_id = env_or_default(KIVA_TENANT_ID_ENV, DEFAULT_TENANT_ID)?;
                let pool = PgPoolOptions::new()
                    .max_connections(5)
                    .connect(&database_url)
                    .await
                    .map_err(|error| {
                        io::Error::other(format!(
                            "failed to connect to PostgreSQL from {DATABASE_URL_ENV}: {error}"
                        ))
                    })?;
                Ok(Self::from_repositories(
                    StatementRepository::postgres(pool.clone(), tenant_id.clone())?,
                    DocumentRepository::postgres(pool.clone(), tenant_id.clone())?,
                    PdsRepository::postgres(pool.clone(), tenant_id.clone())?,
                    ConfidentialRepository::postgres(pool, tenant_id)?,
                    limits,
                    auth,
                ))
            }
            Err(std::env::VarError::NotPresent) => Ok(Self {
                limits,
                auth,
                ..Self::default()
            }),
            Err(error) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("failed to read {DATABASE_URL_ENV}: {error}"),
            )),
        }?;
        state.xapi.signature_policy = signature_policy;
        Ok(state)
    }
}

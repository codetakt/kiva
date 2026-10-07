#![allow(clippy::wildcard_imports)]

use super::*;
use crate::core::state::{
    AnalyticsHttpContext, ConfidentialHttpContext, PdsApplicationHttpContext, PdsHttpContext,
};

pub(in crate::tests) fn analytics_state(state: &super::AppState) -> State<AnalyticsHttpContext> {
    State(state.analytics_http_context())
}

pub(in crate::tests) fn pds_state(state: &super::AppState) -> State<PdsHttpContext> {
    State(state.pds_http_context())
}

pub(in crate::tests) fn pds_application_state(
    state: &super::AppState,
) -> State<PdsApplicationHttpContext> {
    State(state.pds_application_http_context())
}

pub(in crate::tests) fn confidential_state(
    state: &super::AppState,
) -> State<ConfidentialHttpContext> {
    State(state.confidential_http_context())
}

pub(in crate::tests) fn memory_test_state_with_auth(auth: super::AuthConfig) -> super::AppState {
    super::AppState::from_repositories(
        super::StatementRepository::default(),
        super::DocumentRepository::default(),
        super::PdsRepository::default(),
        super::ConfidentialRepository::default(),
        super::RuntimeLimits::default(),
        auth,
    )
}

pub(in crate::tests) async fn postgres_test_state() -> Result<Option<super::AppState>, String> {
    postgres_test_state_with_auth(super::AuthConfig::default()).await
}

pub(in crate::tests) async fn postgres_test_state_with_auth(
    auth: super::AuthConfig,
) -> Result<Option<super::AppState>, String> {
    let database_url = match std::env::var("KIVA_POSTGRES_TEST_DATABASE_URL") {
        Ok(value) if value.is_empty() => {
            return Err("KIVA_POSTGRES_TEST_DATABASE_URL must not be empty".to_owned());
        }
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(error) => {
            return Err(format!(
                "KIVA_POSTGRES_TEST_DATABASE_URL should be readable: {error}"
            ));
        }
    };
    let tenant_id = format!("pg-threshold-{}", Uuid::new_v4());
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(8)
        .connect(&database_url)
        .await
        .map_err(|error| format!("PostgreSQL test database should be reachable: {error}"))?;

    let confidential = super::ConfidentialRepository::postgres(pool.clone(), tenant_id.clone())
        .map_err(|error| format!("confidential repository should initialize: {error}"))?;

    Ok(Some(super::AppState::from_repositories(
        super::StatementRepository::postgres(pool.clone(), tenant_id.clone())
            .map_err(|error| format!("statement repository should initialize: {error}"))?,
        super::DocumentRepository::postgres(pool.clone(), tenant_id.clone())
            .map_err(|error| format!("document repository should initialize: {error}"))?,
        super::PdsRepository::postgres(pool, tenant_id)
            .map_err(|error| format!("PDS repository should initialize: {error}"))?,
        confidential,
        super::RuntimeLimits::default(),
        auth,
    )))
}

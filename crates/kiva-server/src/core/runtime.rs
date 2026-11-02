//! Runtime environment and request-limit configuration.

use std::io;
use std::sync::Arc;

use time::{Duration, OffsetDateTime};
use tokio::sync::RwLock;

pub(crate) mod xapi_limits;

use crate::core::config::{
    KIVA_MAX_STATEMENT_BODY_BYTES_ENV, KIVA_RATE_LIMIT_PATH_ENV, KIVA_RATE_LIMIT_REQUEST_COUNT_ENV,
    KIVA_RATE_LIMIT_WINDOW_SECONDS_ENV,
};

/// Runtime limits applied by the HTTP server.
#[derive(Debug, Clone, Default)]
pub(crate) struct RuntimeLimits {
    max_statement_body_bytes: Option<usize>,
    pub(crate) max_statement_page_size: Option<std::num::NonZeroUsize>,
    rate_limiter: Option<RateLimiter>,
    pub(crate) xapi: xapi_limits::XapiLimits,
}

impl RuntimeLimits {
    /// Builds runtime limits from process environment variables.
    pub(crate) fn from_env() -> Result<Self, io::Error> {
        Ok(Self {
            max_statement_body_bytes: optional_env_usize(KIVA_MAX_STATEMENT_BODY_BYTES_ENV)?,
            max_statement_page_size: optional_env_usize("KIVA_XAPI_STATEMENT_PAGE_SIZE")?
                .and_then(std::num::NonZeroUsize::new),
            rate_limiter: RateLimiter::from_env()?,
            xapi: optional_env_string("KIVA_XAPI_LIMITS_JSON")?
                .map(|json| xapi_limits::XapiLimits::from_json(&json))
                .transpose()?
                .unwrap_or_default(),
        })
    }

    /// Returns the configured maximum xAPI Statement body size.
    pub(crate) const fn max_statement_body_bytes(&self) -> Option<usize> {
        self.max_statement_body_bytes
    }

    /// Returns whether a request for `path` is currently admitted.
    pub(crate) async fn allows_path(&self, path: &str) -> bool {
        match self.rate_limiter.as_ref() {
            Some(limiter) => limiter.allows(path).await,
            None => true,
        }
    }
}

#[derive(Debug, Clone)]
struct RateLimiter {
    path: String,
    request_count: usize,
    window: Duration,
    state: Arc<RwLock<RateLimitState>>,
}

impl RateLimiter {
    fn from_env() -> Result<Option<Self>, io::Error> {
        let Some(path) = optional_env_string(KIVA_RATE_LIMIT_PATH_ENV)? else {
            return Ok(None);
        };
        let request_count = required_env_usize(KIVA_RATE_LIMIT_REQUEST_COUNT_ENV)?;
        let window_seconds = required_env_i64(KIVA_RATE_LIMIT_WINDOW_SECONDS_ENV)?;
        if !path.starts_with('/') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{KIVA_RATE_LIMIT_PATH_ENV} must start with /"),
            ));
        }
        Ok(Some(Self {
            path,
            request_count,
            window: Duration::seconds(window_seconds),
            state: Arc::new(RwLock::new(RateLimitState::default())),
        }))
    }

    async fn allows(&self, path: &str) -> bool {
        if path != self.path {
            return true;
        }
        let now = OffsetDateTime::now_utc();
        let mut state = self.state.write().await;
        if now - state.window_started_at >= self.window {
            state.window_started_at = now;
            state.count = 0;
        }
        state.count += 1;
        state.count < self.request_count
    }
}

#[derive(Debug, Clone)]
struct RateLimitState {
    window_started_at: OffsetDateTime,
    count: usize,
}

impl Default for RateLimitState {
    fn default() -> Self {
        Self {
            window_started_at: OffsetDateTime::now_utc(),
            count: 0,
        }
    }
}

/// Reads a non-empty environment variable or returns a default.
pub(crate) fn env_or_default(name: &'static str, default: &str) -> Result<String, io::Error> {
    match std::env::var(name) {
        Ok(value) if value.is_empty() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} must not be empty when set"),
        )),
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotPresent) => Ok(default.to_owned()),
        Err(error) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("failed to read {name}: {error}"),
        )),
    }
}

/// Reads an optional non-empty environment variable.
pub(crate) fn optional_env_string(name: &'static str) -> Result<Option<String>, io::Error> {
    match std::env::var(name) {
        Ok(value) if value.is_empty() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} must not be empty when set"),
        )),
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("failed to read {name}: {error}"),
        )),
    }
}

/// Reads an optional positive `usize` environment variable.
pub(crate) fn optional_env_usize(name: &'static str) -> Result<Option<usize>, io::Error> {
    optional_env_string(name)?
        .map(|value| parse_positive_usize(name, &value))
        .transpose()
}

/// Reads an optional boolean environment variable.
pub(crate) fn optional_env_bool(name: &'static str) -> Result<Option<bool>, io::Error> {
    optional_env_string(name)?
        .map(|value| parse_bool(name, &value))
        .transpose()
}

fn required_env_usize(name: &'static str) -> Result<usize, io::Error> {
    let value = optional_env_string(name)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} is required when {KIVA_RATE_LIMIT_PATH_ENV} is set"),
        )
    })?;
    parse_positive_usize(name, &value)
}

fn required_env_i64(name: &'static str) -> Result<i64, io::Error> {
    let value = optional_env_string(name)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} is required when {KIVA_RATE_LIMIT_PATH_ENV} is set"),
        )
    })?;
    let value = value.parse::<i64>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} must be a positive integer: {error}"),
        )
    })?;
    if value <= 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} must be positive"),
        ));
    }
    Ok(value)
}

fn parse_bool(name: &'static str, value: &str) -> Result<bool, io::Error> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} must be true or false"),
        )),
    }
}

fn parse_positive_usize(name: &'static str, value: &str) -> Result<usize, io::Error> {
    let value = value.parse::<usize>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} must be a positive integer: {error}"),
        )
    })?;
    if value == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} must be positive"),
        ));
    }
    Ok(value)
}

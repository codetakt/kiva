//! Deployment choices for the LRS's additional capabilities.

use std::{collections::BTreeSet, io};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PaginationMode {
    #[default]
    Live,
    Snapshot,
}

// Independent deployment switches, not mutually exclusive lifecycle states.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct CapabilityConfig {
    pub(crate) mailbox_hash_queries: bool,
    pub(crate) detailed_interactions: bool,
    pub(crate) canonical_inventory: bool,
    pub(crate) activity_hosting: bool,
    pub(crate) person_enrichment: bool,
    pub(crate) rollback_voided_metadata: bool,
    pub(crate) canonical_writers: BTreeSet<String>,
    pub(crate) directory_writers: BTreeSet<String>,
    pub(crate) trusted_authority_subjects: BTreeSet<String>,
    pub(crate) metadata_origins: BTreeSet<String>,
    pub(crate) metadata_html: bool,
    pub(crate) metadata_timeout_ms: u64,
    pub(crate) metadata_max_bytes: usize,
    pub(crate) metadata_retry_seconds: u32,
    pub(crate) public_base_url: Option<String>,
    pub(crate) pagination: PaginationMode,
    pub(crate) cursor_ttl_seconds: u32,
    pub(crate) max_cursors: usize,
    pub(crate) cursor_storage_bytes: usize,
    pub(crate) self_contained_live_cursors: bool,
    pub(crate) legacy_extension_aliases: bool,
}

impl Default for CapabilityConfig {
    fn default() -> Self {
        Self {
            mailbox_hash_queries: true,
            detailed_interactions: false,
            canonical_inventory: false,
            activity_hosting: false,
            person_enrichment: false,
            rollback_voided_metadata: true,
            canonical_writers: BTreeSet::new(),
            directory_writers: BTreeSet::new(),
            trusted_authority_subjects: BTreeSet::new(),
            metadata_origins: BTreeSet::new(),
            metadata_html: true,
            metadata_timeout_ms: 2_000,
            metadata_max_bytes: 1_048_576,
            metadata_retry_seconds: 60,
            public_base_url: None,
            pagination: PaginationMode::Live,
            cursor_ttl_seconds: 3600,
            max_cursors: 10_000,
            cursor_storage_bytes: 67_108_864,
            self_contained_live_cursors: false,
            legacy_extension_aliases: true,
        }
    }
}

impl CapabilityConfig {
    pub(crate) fn from_env() -> Result<Self, io::Error> {
        match std::env::var("KIVA_XAPI_CAPABILITIES_JSON") {
            Ok(json) => Self::from_json(&json),
            Err(std::env::VarError::NotPresent) => Ok(Self::default()),
            Err(error) => Err(io::Error::new(io::ErrorKind::InvalidInput, error)),
        }
    }

    pub(crate) fn from_json(json: &str) -> Result<Self, io::Error> {
        let config: Self = serde_json::from_str(json).map_err(io::Error::other)?;
        if config.metadata_timeout_ms == 0
            || config.metadata_timeout_ms > 30_000
            || config.metadata_max_bytes == 0
            || config.metadata_max_bytes > 16_777_216
            || config.metadata_retry_seconds == 0
            || config.cursor_ttl_seconds == 0
            || config.max_cursors == 0
            || config.cursor_storage_bytes == 0
            || config.max_cursors > 1_000_000
            || config
                .canonical_writers
                .iter()
                .chain(&config.directory_writers)
                .chain(&config.trusted_authority_subjects)
                .any(String::is_empty)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid xAPI capability limits or subject identifiers",
            ));
        }
        for origin in &config.metadata_origins {
            let url = url::Url::parse(origin).map_err(io::Error::other)?;
            if !matches!(url.scheme(), "http" | "https")
                || url.origin().ascii_serialization() != *origin
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "metadata_origins must contain exact HTTP(S) origins",
                ));
            }
        }
        if let Some(base) = &config.public_base_url {
            let url = url::Url::parse(base).map_err(io::Error::other)?;
            if !matches!(url.scheme(), "http" | "https")
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || base.ends_with('/')
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "public_base_url must be an HTTP(S) base without credentials, query, fragment or trailing slash",
                ));
            }
        }
        if !config.metadata_origins.is_empty() && !config.canonical_inventory {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "metadata fetching requires canonical_inventory",
            ));
        }
        if config.activity_hosting
            && (!config.canonical_inventory || config.public_base_url.is_none())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "activity_hosting requires canonical_inventory and public_base_url",
            ));
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::CapabilityConfig;

    #[test]
    fn hosting_requires_canonical_projection_but_proxy_prefix_does_not() {
        for config in [
            r#"{"activity_hosting":true}"#,
            r#"{"activity_hosting":true,"public_base_url":"https://lrs.example.org"}"#,
            r#"{"activity_hosting":true,"canonical_inventory":true}"#,
        ] {
            assert!(CapabilityConfig::from_json(config).is_err());
        }
        assert!(CapabilityConfig::from_json(
            r#"{"activity_hosting":true,"canonical_inventory":true,"public_base_url":"https://lrs.example.org"}"#
        ).is_ok());
        assert!(
            CapabilityConfig::from_json(r#"{"public_base_url":"https://lrs.example.org/xapi"}"#)
                .is_ok()
        );
    }
}

//! Configured product capabilities layered over the adopted xAPI receiver contract.

pub(crate) mod canonical;
pub(crate) mod config;
pub(crate) mod fetch;
pub(crate) mod host;
pub(crate) mod interaction;
pub(crate) mod people;
pub(crate) mod store;

use std::sync::Arc;

pub(crate) use config::CapabilityConfig;
pub(crate) use store::CapabilityStore;

#[derive(Debug, Clone)]
pub(crate) struct CanonicalSource {
    pub(crate) statement: serde_json::Value,
    pub(crate) opaque_values: super::lossless::OpaqueValues,
    pub(crate) may_update: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Capabilities {
    pub(crate) config: Arc<CapabilityConfig>,
    pub(crate) store: CapabilityStore,
    fetch_worker: Arc<tokio::sync::Mutex<()>>,
    fetch_pending: Arc<std::sync::atomic::AtomicBool>,
}

impl Capabilities {
    pub(crate) fn public_profile(&self) -> serde_json::Value {
        serde_json::json!({
            "requirementsBaseline":"IEEE Std 9274.1.1-2023",
            "errors":["application/json","text/plain","text/html"],
            "canonicalInventory":self.config.canonical_inventory,
            "resolvableMetadata":!self.config.metadata_origins.is_empty(),
            "metadataFormats":if self.config.metadata_html {vec!["application/json","text/html"]}else{vec!["application/json"]},
            "activityHosting":self.config.activity_hosting,
            "mailboxHashQueries":self.config.mailbox_hash_queries,
            "trustedAuthority":!self.config.trusted_authority_subjects.is_empty(),
            "detailedInteractions":self.config.detailed_interactions,
            "personEnrichment":self.config.person_enrichment,
            "voidMetadataRollback":self.config.rollback_voided_metadata,
            "pagination":{"selected":self.config.pagination,"supported":["live","snapshot"],"cursorLifetimeSeconds":self.config.cursor_ttl_seconds,"selfContainedLiveCursors":self.config.self_contained_live_cursors},
            "extensionsBase":"/extensions","legacyExtensionAliases":self.config.legacy_extension_aliases
        })
    }

    pub(crate) fn new(store: CapabilityStore) -> Self {
        Self {
            config: Arc::default(),
            store,
            fetch_worker: Arc::default(),
            fetch_pending: Arc::default(),
        }
    }

    pub(crate) fn configure(&mut self, config: CapabilityConfig) {
        self.config = Arc::new(config);
    }

    /// Called only after ordinary admission and signature validation succeed.
    pub(crate) fn finish_admission(
        &self,
        record: &mut super::model::StoredStatement,
        subject: Option<&str>,
        supplied_authority: Option<serde_json::Value>,
    ) {
        record.canonical_update_permitted =
            subject.is_some_and(|subject| self.config.canonical_writers.contains(subject));
        if subject.is_some_and(|subject| self.config.trusted_authority_subjects.contains(subject))
            && let Some(authority) = supplied_authority
        {
            record.statement["authority"] = authority;
        }
    }
}

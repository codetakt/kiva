//! PDS memory and `PostgreSQL` storage implementations.
#![allow(clippy::wildcard_imports)]

use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    sync::Arc,
};

use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::{
    AccessCheckIntent as PdsAccessCheckIntent,
    ConsentGrantRevocationRequested as PdsConsentGrantRevocationRequested,
    ConsentScope as PdsConsentScope, ConsentState as PdsConsentState, DataClass as PdsDataClass,
    DataProductReadIntent as PdsDataProductReadIntent,
    DataProductSourceKind as PdsDataProductSourceKind,
    GovernanceProfileKind as PdsGovernanceProfileKind, LegalBasis as PdsLegalBasis,
    LifecycleCaseStatus as PdsLifecycleCaseStatus, LifecycleCaseType as PdsLifecycleCaseType,
    LifecycleEventType as PdsLifecycleEventType, LifecycleResourceKind as PdsLifecycleResourceKind,
    TombstonePublicationRule as PdsTombstonePublicationRule,
};
use sqlx::{Executor, PgPool, Postgres, Transaction, types::Json as SqlJson};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::core::{
    config::KIVA_TENANT_ID_ENV,
    outcome::{InsertOutcome, StoreOutcome},
};
use crate::surfaces::pds::{model::*, outcome::*};

#[derive(Debug, Clone, Default)]
pub(crate) struct MemoryPdsRepository {
    consent: MemoryPdsConsentStore,
    data_products: MemoryPdsDataProductStore,
    provider_bindings: MemoryPdsProviderBindingStore,
    access: MemoryPdsAccessStore,
    lifecycle: MemoryPdsLifecycleStore,
    governance: MemoryPdsGovernanceStore,
}

#[derive(Debug, Clone, Default)]
struct MemoryPdsConsentStore {
    grants: Arc<RwLock<BTreeMap<PdsConsentGrantKey, PdsConsentGrantState>>>,
}

#[derive(Debug, Clone, Default)]
struct MemoryPdsDataProductStore {
    products: Arc<RwLock<BTreeMap<PdsDataProductKey, PdsDataProductState>>>,
}

#[derive(Debug, Clone, Default)]
struct MemoryPdsProviderBindingStore {
    bindings: Arc<RwLock<BTreeMap<PdsProviderBindingKey, PdsProviderBindingState>>>,
}

#[derive(Debug, Clone, Default)]
struct MemoryPdsAccessStore {
    audit_events: Arc<RwLock<BTreeMap<PdsAccessAuditEventKey, PdsAccessAuditEventState>>>,
}

#[derive(Debug, Clone, Default)]
struct MemoryPdsLifecycleStore {
    ledger_events: Arc<RwLock<BTreeMap<PdsLedgerEventKey, PdsLedgerEventState>>>,
    events: Arc<RwLock<BTreeMap<PdsLifecycleEventKey, PdsLifecycleEventState>>>,
    retention_policies: Arc<RwLock<BTreeMap<PdsRetentionPolicyKey, PdsRetentionPolicyState>>>,
    cases: Arc<RwLock<BTreeMap<PdsLifecycleCaseKey, PdsLifecycleCaseState>>>,
}

#[derive(Debug, Clone, Default)]
struct MemoryPdsGovernanceStore {
    profiles: Arc<RwLock<BTreeMap<PdsGovernanceProfileKey, PdsGovernanceProfileState>>>,
}

#[derive(Debug, Clone)]
pub(crate) struct PostgresPdsRepository {
    pool: PgPool,
    tenant_id: String,
}

mod mapping;
mod memory;
mod postgres;
mod postgres_queries;
mod rows;

use rows::*;

#[cfg(test)]
pub(crate) use mapping::pds_data_class_labels;

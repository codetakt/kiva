//! PDS application subject snapshots and read results.

use super::records::{
    ConsentGrantRecord, DataProductRecord, GovernanceProfileRecord, LifecycleCaseRecord,
    LifecycleEventRecord, PortabilityStatementPage, PortabilityStatementRecord,
    RetentionPolicyRecord, StatementRecord,
};
use time::OffsetDateTime;
use uuid::Uuid;

/// PDS application subject identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdsSubjectId(String);

impl PdsSubjectId {
    /// Creates a subject identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the subject identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Subject-facing inventory of accepted statement and data-product records.
pub struct RecordInventory {
    /// Subject who owns the inventory.
    pub data_subject: PdsSubjectId,
    /// xAPI statement metadata records.
    pub statements: Vec<StatementRecord>,
    /// Approved PDS data products.
    pub data_products: Vec<DataProductRecord>,
}

/// Subject-facing PDS metadata export.
pub struct MetadataExport {
    /// Subject who owns the export.
    pub data_subject: PdsSubjectId,
    /// xAPI statement metadata records.
    pub statements: Vec<StatementRecord>,
    /// Approved PDS data products.
    pub data_products: Vec<DataProductRecord>,
    /// Consent and delegation grants.
    pub consent_grants: Vec<ConsentGrantRecord>,
    /// Lifecycle events.
    pub lifecycle_events: Vec<LifecycleEventRecord>,
    /// Retention policies.
    pub retention_policies: Vec<RetentionPolicyRecord>,
    /// Lifecycle cases.
    pub lifecycle_cases: Vec<LifecycleCaseRecord>,
    /// Governance profiles.
    pub governance_profiles: Vec<GovernanceProfileRecord>,
}

/// Bounded learner-initiated export of preserved Statement bodies and PDS metadata.
pub struct PortabilityExport {
    /// Subject who owns every exported Statement partition.
    pub data_subject: PdsSubjectId,
    /// Time at which this export page was generated.
    pub generated_at: OffsetDateTime,
    /// Durable PDS audit reference for this export page.
    pub audit_reference: Uuid,
    /// Preserved subject-owned Statement bodies.
    pub statements: Vec<PortabilityStatementRecord>,
    /// Opaque continuation cursor, if another page exists.
    pub next_cursor: Option<String>,
    /// Approved PDS data products.
    pub data_products: Vec<DataProductRecord>,
    /// Consent and delegation grants.
    pub consent_grants: Vec<ConsentGrantRecord>,
    /// Lifecycle events.
    pub lifecycle_events: Vec<LifecycleEventRecord>,
    /// Retention policies.
    pub retention_policies: Vec<RetentionPolicyRecord>,
    /// Lifecycle cases.
    pub lifecycle_cases: Vec<LifecycleCaseRecord>,
    /// Governance profiles.
    pub governance_profiles: Vec<GovernanceProfileRecord>,
}

/// Grouped PDS metadata records used by subject snapshots.
pub struct PdsMetadataRecords {
    /// Consent and delegation grants.
    pub consent_grants: Vec<ConsentGrantRecord>,
    /// Lifecycle events.
    pub lifecycle_events: Vec<LifecycleEventRecord>,
    /// Retention policies.
    pub retention_policies: Vec<RetentionPolicyRecord>,
    /// Lifecycle cases.
    pub lifecycle_cases: Vec<LifecycleCaseRecord>,
    /// Governance profiles.
    pub governance_profiles: Vec<GovernanceProfileRecord>,
}

/// Complete subject snapshot before projection into inventory or export views.
pub struct PdsSubjectSnapshot {
    /// Subject who owns the snapshot.
    pub data_subject: PdsSubjectId,
    /// xAPI statement metadata records.
    pub statements: Vec<StatementRecord>,
    /// Approved PDS data products.
    pub data_products: Vec<DataProductRecord>,
    /// Consent and delegation grants.
    pub consent_grants: Vec<ConsentGrantRecord>,
    /// Lifecycle events.
    pub lifecycle_events: Vec<LifecycleEventRecord>,
    /// Retention policies.
    pub retention_policies: Vec<RetentionPolicyRecord>,
    /// Lifecycle cases.
    pub lifecycle_cases: Vec<LifecycleCaseRecord>,
    /// Governance profiles.
    pub governance_profiles: Vec<GovernanceProfileRecord>,
}

/// Subject-scoped portability data before audit admission.
pub struct PdsPortabilitySnapshot {
    pub data_subject: PdsSubjectId,
    pub statement_page: PortabilityStatementPage,
    pub data_products: Vec<DataProductRecord>,
    pub consent_grants: Vec<ConsentGrantRecord>,
    pub lifecycle_events: Vec<LifecycleEventRecord>,
    pub retention_policies: Vec<RetentionPolicyRecord>,
    pub lifecycle_cases: Vec<LifecycleCaseRecord>,
    pub governance_profiles: Vec<GovernanceProfileRecord>,
}

impl PdsPortabilitySnapshot {
    pub fn into_export(
        self,
        generated_at: OffsetDateTime,
        audit_reference: Uuid,
    ) -> PortabilityExport {
        PortabilityExport {
            data_subject: self.data_subject,
            generated_at,
            audit_reference,
            statements: self.statement_page.statements,
            next_cursor: self.statement_page.next_cursor,
            data_products: self.data_products,
            consent_grants: self.consent_grants,
            lifecycle_events: self.lifecycle_events,
            retention_policies: self.retention_policies,
            lifecycle_cases: self.lifecycle_cases,
            governance_profiles: self.governance_profiles,
        }
    }
}

impl PdsSubjectSnapshot {
    /// Projects the snapshot into record inventory.
    pub fn into_record_inventory(self) -> RecordInventory {
        let Self {
            data_subject,
            statements,
            data_products,
            ..
        } = self;
        RecordInventory {
            data_subject,
            statements,
            data_products,
        }
    }

    /// Projects the snapshot into a metadata export.
    pub fn into_metadata_export(self) -> MetadataExport {
        let Self {
            data_subject,
            statements,
            data_products,
            consent_grants,
            lifecycle_events,
            retention_policies,
            lifecycle_cases,
            governance_profiles,
            ..
        } = self;
        MetadataExport {
            data_subject,
            statements,
            data_products,
            consent_grants,
            lifecycle_events,
            retention_policies,
            lifecycle_cases,
            governance_profiles,
        }
    }
}

//! PDS application read records.

use kiva_pds::{
    ConsentScope, ConsentState, DataClass, GovernanceProfileKind, LegalBasis, LifecycleCaseStatus,
    LifecycleCaseType, LifecycleEventType, LifecycleResourceKind, ReleaseArtifactRef,
    TombstonePublicationRule,
};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

/// xAPI statement metadata record exposed to PDS read models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementRecord {
    /// Statement identifier.
    pub statement_id: String,
    /// Canonical actor IFI used for subject indexing.
    pub actor_ifi: String,
    /// Optional verb IRI.
    pub verb_id: Option<String>,
    /// Optional object IRI or account identifier.
    pub object_id: Option<String>,
    /// Statement stored timestamp.
    pub stored_at: OffsetDateTime,
    /// xAPI version associated with the stored statement.
    pub version: String,
}

/// Preserved xAPI Statement body exposed only by the subject-bound portability workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortabilityStatementRecord {
    /// Statement identifier used for stable pagination.
    pub statement_id: Uuid,
    /// Stored Statement JSON, read without typed DTO reprojection.
    pub statement: Value,
    /// Statement stored timestamp.
    pub stored_at: OffsetDateTime,
    /// Whether the Statement is the target of an accepted voiding Statement.
    pub voided: bool,
}

/// One bounded page of subject-owned Statement bodies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortabilityStatementPage {
    /// Subject-owned Statement bodies in stable storage order.
    pub statements: Vec<PortabilityStatementRecord>,
    /// Opaque continuation cursor, if another page exists.
    pub next_cursor: Option<String>,
}

/// PDS consent or delegation grant record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentGrantRecord {
    /// Grant identifier.
    pub grant_id: Uuid,
    /// Subject who owns the data.
    pub data_subject: String,
    /// Subject receiving the delegated access.
    pub grantee_subject: String,
    /// Delegated consent scope.
    pub scope: ConsentScope,
    /// Current consent state.
    pub state: ConsentState,
    /// Declared purpose for the grant.
    pub purpose: String,
    /// Legal basis for the grant.
    pub legal_basis: LegalBasis,
    /// Grant creation timestamp.
    pub created_at: OffsetDateTime,
    /// Optional grant expiration timestamp.
    pub expires_at: Option<OffsetDateTime>,
    /// Optional grant revocation timestamp.
    pub revoked_at: Option<OffsetDateTime>,
    /// Optional predecessor grant identifier.
    pub renewed_from_grant_id: Option<Uuid>,
}

/// Approved PDS data-product record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataProductRecord {
    /// Data-product identifier.
    pub data_product_id: Uuid,
    /// Subject that owns the data product.
    pub owner_subject: String,
    /// Normalized confidential release-artifact reference.
    pub release_artifact: ReleaseArtifactRef,
    /// Human-readable title.
    pub title: String,
    /// Declared purpose for the data product.
    pub purpose: String,
    /// Legal basis for the data product.
    pub legal_basis: LegalBasis,
    /// Data classes used by the product.
    pub data_classes: Vec<DataClass>,
    /// Creation timestamp.
    pub created_at: OffsetDateTime,
}

/// PDS lifecycle ledger event record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleEventRecord {
    /// Lifecycle event identifier.
    pub event_id: Uuid,
    /// Subject who owns the affected data.
    pub data_subject: String,
    /// Lifecycle event type.
    pub event_type: LifecycleEventType,
    /// Affected resource family.
    pub resource_kind: LifecycleResourceKind,
    /// Affected resource identifier.
    pub resource_id: String,
    /// Human-readable reason.
    pub reason: String,
    /// Event creation timestamp.
    pub created_at: OffsetDateTime,
}

/// PDS retention policy record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionPolicyRecord {
    /// Retention policy identifier.
    pub policy_id: Uuid,
    /// Subject that owns the policy.
    pub owner_subject: String,
    /// Resource family covered by the policy.
    pub resource_kind: LifecycleResourceKind,
    /// Data class covered by the policy.
    pub data_class: DataClass,
    /// Retention period in days.
    pub retention_period_days: u32,
    /// Review interval in days.
    pub review_interval_days: u32,
    /// Optional tombstone deadline in days.
    pub tombstone_after_days: Option<u32>,
    /// Tombstone publication rule.
    pub tombstone_publication: TombstonePublicationRule,
    /// Legal basis for the retention policy.
    pub legal_basis: LegalBasis,
    /// Human-readable policy description.
    pub description: String,
    /// Policy creation timestamp.
    pub created_at: OffsetDateTime,
}

/// PDS correction, dispute, or tombstone lifecycle case record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleCaseRecord {
    /// Lifecycle case identifier.
    pub case_id: Uuid,
    /// Subject who owns the affected data.
    pub data_subject: String,
    /// Lifecycle case type.
    pub case_type: LifecycleCaseType,
    /// Affected resource family.
    pub resource_kind: LifecycleResourceKind,
    /// Affected resource identifier.
    pub resource_id: String,
    /// Current case status.
    pub status: LifecycleCaseStatus,
    /// Optional linked retention policy.
    pub retention_policy_id: Option<Uuid>,
    /// Case summary.
    pub summary: String,
    /// Optional case resolution.
    pub resolution: Option<String>,
    /// Optional tombstone publication rule.
    pub tombstone_publication: Option<TombstonePublicationRule>,
    /// Case creation timestamp.
    pub created_at: OffsetDateTime,
    /// Last update timestamp.
    pub updated_at: OffsetDateTime,
}

/// Public-sector PDS governance profile record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernanceProfileRecord {
    /// Governance profile identifier.
    pub profile_id: Uuid,
    /// Subject that owns the governance profile.
    pub profile_owner_subject: String,
    /// Governance profile kind.
    pub profile_kind: GovernanceProfileKind,
    /// Tenant partition label.
    pub tenant_partition: String,
    /// Subject partition label.
    pub subject_partition: String,
    /// Data custodian subject.
    pub data_custodian_subject: String,
    /// Platform operator subject.
    pub platform_operator_subject: String,
    /// Policy authority subject.
    pub policy_authority_subject: String,
    /// Analyst subject.
    pub analyst_subject: String,
    /// Auditor subject.
    pub auditor_subject: String,
    /// Optional identity authority subject.
    pub identity_authority_subject: Option<String>,
    /// Emergency access policy reference.
    pub emergency_access_policy: String,
    /// Operator runbook references.
    pub runbook_refs: Vec<String>,
    /// Profile creation timestamp.
    pub created_at: OffsetDateTime,
}

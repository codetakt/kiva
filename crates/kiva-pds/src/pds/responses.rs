#![allow(clippy::wildcard_imports)]

use super::*;

/// xAPI Agent account object used by PDS subject responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdsSubjectAccount {
    /// Account homepage IRI.
    pub home_page: String,
    /// Account name.
    pub name: String,
}

/// xAPI Agent object used by PDS subject responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdsSubjectAgent {
    /// xAPI object type label.
    pub object_type: String,
    /// Subject account identifier.
    pub account: PdsSubjectAccount,
}

/// Consent-grant response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentGrant {
    /// Stable consent grant identifier.
    pub grant_id: String,
    /// Data subject owning the consent grant.
    pub data_subject: String,
    /// Subject receiving delegated access.
    pub grantee_subject: String,
    /// Delegated consent scope.
    pub scope: ConsentScope,
    /// Persisted consent state.
    pub state: ConsentState,
    /// Declared access purpose.
    pub purpose: String,
    /// Legal basis for the delegation.
    pub legal_basis: LegalBasis,
    /// Creation timestamp.
    pub created_at: String,
    /// Optional expiration timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// Optional revocation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
    /// Optional predecessor grant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewed_from_grant_id: Option<String>,
}

/// Consent-grant list response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentGrantList {
    /// Consent grants visible to the requester.
    pub grants: Vec<ConsentGrant>,
}

/// Approved data-product response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataProduct {
    /// Stable data-product identifier.
    pub data_product_id: String,
    /// Subject owning the product.
    pub owner_subject: String,
    /// Source artifact kind retained for wire compatibility.
    pub source_kind: DataProductSourceKind,
    /// Hex-encoded source artifact identifier retained for wire compatibility.
    pub release_id: String,
    /// Normalized release artifact reference.
    pub release_artifact: ReleaseArtifactRef,
    /// Human-readable data-product title.
    pub title: String,
    /// Declared data-product purpose.
    pub purpose: String,
    /// Legal basis for the data product.
    pub legal_basis: LegalBasis,
    /// Metadata classes included in the data product.
    pub data_classes: Vec<DataClass>,
    /// Creation timestamp.
    pub created_at: String,
}

/// Approved data-product list response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataProductList {
    /// Approved data products visible to the requester.
    pub data_products: Vec<DataProduct>,
}

/// Governed data-product read response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataProductReadResponse {
    /// Access-audit event identifier.
    pub event_id: String,
    /// Access decision label.
    pub decision: String,
    /// Optional denial reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_reason: Option<String>,
    /// Optional approved data product when access is allowed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_product: Option<DataProduct>,
}

/// Secret/confidential-compute provider binding metadata exposed by the PDS registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderBinding {
    /// Stable provider-binding identifier.
    pub binding_id: String,
    /// Subject that owns the registry entry.
    pub owner_subject: String,
    /// Confidential mechanism label admitted for this binding.
    pub mechanism: String,
    /// Provider family label.
    pub provider_kind: String,
    /// Provider capability label.
    pub capability: String,
    /// Provider profile identifier.
    pub profile: String,
    /// Hex-encoded provider identifier digest.
    pub provider_id_digest: String,
    /// Hex-encoded provider artifact digest.
    pub artifact_digest: String,
    /// Optional hex-encoded parameter-set digest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_set_digest: Option<String>,
    /// Optional hex-encoded external-review evidence digest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_evidence_digest: Option<String>,
    /// Whether the binding requested a production security claim.
    pub production_claim_requested: bool,
    /// Canonical binding digest derived by the confidential provider contract.
    pub binding_digest: String,
    /// Evidence gate admitted by the confidential provider contract.
    pub evidence_gate: String,
    /// Release gate admitted by the confidential provider contract.
    pub release_gate: String,
    /// Runtime boundary class admitted for the binding.
    pub runtime_class: String,
    /// Whether the binding is admitted for production runtime use.
    pub production_ready: bool,
    /// Creation timestamp.
    pub created_at: String,
}

/// PDS provider-binding registry list response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderBindingList {
    /// Provider bindings visible to the requester.
    pub provider_bindings: Vec<ProviderBinding>,
}

/// Subject-scoped statement inventory record response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatementInventoryRecord {
    /// Statement identifier.
    pub statement_id: String,
    /// Canonical actor IFI.
    pub actor_ifi: String,
    /// Verb IRI.
    pub verb_id: String,
    /// Object IRI.
    pub object_id: String,
    /// Stored timestamp.
    pub stored_at: String,
    /// xAPI version.
    pub version: String,
}

/// PDS record-inventory response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordInventory {
    /// Data subject.
    pub data_subject: String,
    /// Subject actor.
    pub actor: PdsSubjectAgent,
    /// Subject actor IFI.
    pub actor_ifi: String,
    /// Statement metadata records.
    pub statement_records: Vec<StatementInventoryRecord>,
    /// Approved data products.
    pub data_products: Vec<DataProduct>,
}

/// Lifecycle-event response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleEvent {
    /// Stable lifecycle event identifier.
    pub event_id: String,
    /// Data subject.
    pub data_subject: String,
    /// Lifecycle event type.
    pub event_type: LifecycleEventType,
    /// Resource kind affected by the event.
    pub resource_kind: LifecycleResourceKind,
    /// Resource identifier.
    pub resource_id: String,
    /// Human-readable reason.
    pub reason: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// Lifecycle-event list response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleEventList {
    /// Lifecycle events.
    pub events: Vec<LifecycleEvent>,
}

/// Retention-policy response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPolicy {
    /// Stable retention policy identifier.
    pub policy_id: String,
    /// Subject owning the policy.
    pub owner_subject: String,
    /// Resource kind covered by the policy.
    pub resource_kind: LifecycleResourceKind,
    /// Metadata class covered by the policy.
    pub data_class: DataClass,
    /// Retention period in days.
    pub retention_period_days: u32,
    /// Review interval in days.
    pub review_interval_days: u32,
    /// Optional tombstone threshold in days.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tombstone_after_days: Option<u32>,
    /// Tombstone publication rule.
    pub tombstone_publication: TombstonePublicationRule,
    /// Legal basis for the policy.
    pub legal_basis: LegalBasis,
    /// Human-readable policy description.
    pub description: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// Retention-policy list response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPolicyList {
    /// Retention policies.
    pub policies: Vec<RetentionPolicy>,
}

/// Lifecycle-case response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleCase {
    /// Stable lifecycle case identifier.
    pub case_id: String,
    /// Data subject.
    pub data_subject: String,
    /// Lifecycle case type.
    pub case_type: LifecycleCaseType,
    /// Resource kind affected by the case.
    pub resource_kind: LifecycleResourceKind,
    /// Resource identifier.
    pub resource_id: String,
    /// Case status.
    pub status: LifecycleCaseStatus,
    /// Optional linked retention policy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_policy_id: Option<String>,
    /// Case summary.
    pub summary: String,
    /// Optional resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    /// Optional tombstone publication rule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tombstone_publication: Option<TombstonePublicationRule>,
    /// Creation timestamp.
    pub created_at: String,
    /// Update timestamp.
    pub updated_at: String,
}

/// Lifecycle-case list response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleCaseList {
    /// Lifecycle cases.
    pub cases: Vec<LifecycleCase>,
}

/// Governance-profile response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceProfile {
    /// Stable governance profile identifier.
    pub profile_id: String,
    /// Subject owning the governance profile.
    pub profile_owner_subject: String,
    /// Governance profile kind.
    pub profile_kind: GovernanceProfileKind,
    /// Tenant partition declaration.
    pub tenant_partition: String,
    /// Subject partition declaration.
    pub subject_partition: String,
    /// Data-custodian subject.
    pub data_custodian_subject: String,
    /// Platform-operator subject.
    pub platform_operator_subject: String,
    /// Policy-authority subject.
    pub policy_authority_subject: String,
    /// Analyst subject.
    pub analyst_subject: String,
    /// Auditor subject.
    pub auditor_subject: String,
    /// Optional identity-authority subject.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_authority_subject: Option<String>,
    /// Emergency access policy reference or declaration.
    pub emergency_access_policy: String,
    /// Runbook references supporting the profile.
    pub runbook_refs: Vec<String>,
    /// Creation timestamp.
    pub created_at: String,
}

/// Governance-profile list response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceProfileList {
    /// Governance profiles.
    pub profiles: Vec<GovernanceProfile>,
}

/// Metadata export response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataExport {
    /// Data subject.
    pub data_subject: String,
    /// Subject actor.
    pub actor: PdsSubjectAgent,
    /// Subject actor IFI.
    pub actor_ifi: String,
    /// Export generation timestamp.
    pub generated_at: String,
    /// Subject record inventory.
    pub record_inventory: RecordInventory,
    /// Consent grants.
    pub consent_grants: Vec<ConsentGrant>,
    /// Approved data products.
    pub data_products: Vec<DataProduct>,
    /// Lifecycle events.
    pub lifecycle_events: Vec<LifecycleEvent>,
    /// Retention policies.
    pub retention_policies: Vec<RetentionPolicy>,
    /// Lifecycle cases.
    pub lifecycle_cases: Vec<LifecycleCase>,
    /// Governance profiles.
    pub governance_profiles: Vec<GovernanceProfile>,
}

/// Preserved Statement entry in a learner-initiated portability export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortabilityStatement {
    /// Stable Statement identifier.
    pub statement_id: String,
    /// Statement stored timestamp.
    pub stored_at: String,
    /// Whether an accepted voiding Statement targets this Statement.
    pub voided: bool,
    /// Stored Statement JSON without typed DTO reprojection.
    pub statement: serde_json::Value,
}

/// One bounded learner-initiated portability export page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortabilityExport {
    /// Data subject controlling the export.
    pub data_subject: String,
    /// Deterministically mapped subject actor.
    pub actor: PdsSubjectAgent,
    /// Canonical subject actor IFI.
    pub actor_ifi: String,
    /// Export page generation timestamp.
    pub generated_at: String,
    /// Durable PDS audit event reference.
    pub audit_reference: String,
    /// Preserved subject-owned Statement entries.
    pub statements: Vec<PortabilityStatement>,
    /// Opaque continuation cursor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Consent and delegation grants.
    pub consent_grants: Vec<ConsentGrant>,
    /// Approved data products.
    pub data_products: Vec<DataProduct>,
    /// Lifecycle events.
    pub lifecycle_events: Vec<LifecycleEvent>,
    /// Retention policies.
    pub retention_policies: Vec<RetentionPolicy>,
    /// Lifecycle cases.
    pub lifecycle_cases: Vec<LifecycleCase>,
    /// Governance profiles.
    pub governance_profiles: Vec<GovernanceProfile>,
}

/// Access-check response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessCheckResponse {
    /// Access-audit event identifier.
    pub event_id: String,
    /// Access decision label.
    pub decision: String,
    /// Optional denial reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_reason: Option<String>,
}

/// Access-audit event response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessAuditEvent {
    /// Access-audit event identifier.
    pub event_id: String,
    /// Requesting subject.
    pub requester_subject: String,
    /// Principal role label.
    pub role: String,
    /// Resource class label.
    pub resource: String,
    /// Policy action label.
    pub action: String,
    /// Optional consent grant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_id: Option<String>,
    /// Optional data-product identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_product_id: Option<String>,
    /// Access decision label.
    pub decision: String,
    /// Optional denial reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_reason: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
}

/// Access-audit event list response DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessAuditEventList {
    /// Access-audit events.
    pub events: Vec<AccessAuditEvent>,
}

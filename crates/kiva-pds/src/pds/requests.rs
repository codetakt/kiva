#![allow(clippy::wildcard_imports)]

use super::*;

/// Domain command input for registering a PDS consent grant.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConsentGrantCommandInput {
    /// Stable consent grant identifier.
    pub grant_id: Uuid,
    /// Subject receiving delegated access.
    pub grantee_subject: String,
    /// Delegated consent scope.
    pub scope: ConsentScope,
    /// Declared access purpose.
    pub purpose: String,
    /// Legal basis for the delegation.
    pub legal_basis: LegalBasis,
    /// Optional expiration timestamp.
    pub expires_at: Option<OffsetDateTime>,
    /// Optional predecessor grant identifier.
    pub renewed_from_grant_id: Option<Uuid>,
}

/// Domain command input for revoking a PDS consent grant.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct ConsentRevocationCommandInput {
    /// Consent grant identifier to revoke.
    pub grant_id: Uuid,
}

/// Domain command input for registering an approved PDS data product.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataProductRegistrationCommandInput {
    /// Stable data-product identifier.
    pub data_product_id: Uuid,
    /// Source release artifact kind.
    pub source_kind: DataProductSourceKind,
    /// Hex-encoded source release artifact identifier.
    pub release_id: String,
    /// Human-readable data-product title.
    pub title: String,
    /// Declared data-product purpose.
    pub purpose: String,
    /// Legal basis for the data-product registration.
    pub legal_basis: LegalBasis,
    /// Metadata classes included in the data product.
    pub data_classes: Vec<DataClass>,
}

/// Domain intent for reading an approved PDS data product.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct DataProductReadIntent {
    /// Consent grant used for delegated read authorization.
    pub grant_id: Uuid,
    /// Data-product identifier being read.
    pub data_product_id: Uuid,
}

/// Domain command input for registering a secret/confidential-compute provider binding.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderBindingRegistrationCommandInput {
    /// Stable provider-binding identifier in the PDS registry.
    pub binding_id: Uuid,
    /// Confidential mechanism label governed by the binding.
    pub mechanism: String,
    /// Provider family label from the Kiva secret-compute provider contract.
    pub provider_kind: String,
    /// Provider capability label from the Kiva secret-compute provider contract.
    pub capability: String,
    /// Provider profile identifier.
    pub profile: String,
    /// Hex-encoded provider identifier digest.
    pub provider_id_digest: String,
    /// Hex-encoded provider artifact digest.
    pub artifact_digest: String,
    /// Optional hex-encoded parameter-set digest.
    pub parameter_set_digest: Option<String>,
    /// Optional hex-encoded external-review evidence digest.
    pub review_evidence_digest: Option<String>,
    /// Whether the binding requests a production security claim.
    pub production_claim_requested: bool,
}

/// Domain intent for checking PDS access policy.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccessCheckIntent {
    /// Principal role label.
    pub role: String,
    /// Resource class label.
    pub resource: String,
    /// Policy action label.
    pub action: String,
    /// Optional consent grant identifier.
    pub grant_id: Option<Uuid>,
    /// Optional data-product identifier.
    pub data_product_id: Option<Uuid>,
}

/// Domain command input for recording a PDS lifecycle event.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LifecycleEventCommandInput {
    /// Stable lifecycle event identifier.
    pub event_id: Uuid,
    /// Lifecycle event type.
    pub event_type: LifecycleEventType,
    /// Resource kind affected by the event.
    pub resource_kind: LifecycleResourceKind,
    /// Resource identifier.
    pub resource_id: String,
    /// Human-readable reason.
    pub reason: String,
}

/// Domain command input for registering a PDS retention policy.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetentionPolicyCommandInput {
    /// Stable retention policy identifier.
    pub policy_id: Uuid,
    /// Resource kind covered by the policy.
    pub resource_kind: LifecycleResourceKind,
    /// Metadata class covered by the policy.
    pub data_class: DataClass,
    /// Retention period in days.
    pub retention_period_days: u32,
    /// Review interval in days.
    pub review_interval_days: u32,
    /// Optional tombstone threshold in days.
    pub tombstone_after_days: Option<u32>,
    /// Tombstone publication rule.
    pub tombstone_publication: TombstonePublicationRule,
    /// Legal basis for the policy.
    pub legal_basis: LegalBasis,
    /// Human-readable policy description.
    pub description: String,
}

/// Domain command input for creating or transitioning a PDS lifecycle case.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LifecycleCaseCommandInput {
    /// Stable lifecycle case identifier.
    pub case_id: Uuid,
    /// Lifecycle case type.
    pub case_type: LifecycleCaseType,
    /// Resource kind affected by the case.
    pub resource_kind: LifecycleResourceKind,
    /// Resource identifier.
    pub resource_id: String,
    /// Requested case status.
    pub status: LifecycleCaseStatus,
    /// Optional linked retention policy identifier.
    pub retention_policy_id: Option<Uuid>,
    /// Case summary.
    pub summary: String,
    /// Optional case resolution for terminal states.
    pub resolution: Option<String>,
    /// Optional tombstone publication rule.
    pub tombstone_publication: Option<TombstonePublicationRule>,
}

/// Domain command input for registering a PDS governance profile.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GovernanceProfileCommandInput {
    /// Stable governance profile identifier.
    pub profile_id: Uuid,
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
    pub identity_authority_subject: Option<String>,
    /// Emergency access policy reference or declaration.
    pub emergency_access_policy: String,
    /// Runbook references supporting the profile.
    pub runbook_refs: Vec<String>,
}

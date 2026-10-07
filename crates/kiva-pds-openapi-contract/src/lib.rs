//! Generated PDS `OpenAPI` contract DTO slice.
//!
//! This crate is the first generated-contract boundary for Kiva PDS. It is
//! intentionally wire-only and has no dependency on the server runtime,
//! persistence adapters, or application workflows.

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ConsentGrantRequest {
    pub grant_id: Uuid,
    pub grantee_subject: String,
    pub scope: ConsentScope,
    pub purpose: String,
    pub legal_basis: LegalBasis,
    pub expires_at: Option<OffsetDateTime>,
    pub renewed_from_grant_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct ConsentRevocationRequest {
    pub grant_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct DataProductRequest {
    pub data_product_id: Uuid,
    pub source_kind: DataProductSourceKind,
    pub release_id: String,
    pub title: String,
    pub purpose: String,
    pub legal_basis: LegalBasis,
    pub data_classes: Vec<DataClass>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct DataProductReadRequest {
    pub grant_id: Uuid,
    pub data_product_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProviderBindingRequest {
    pub binding_id: Uuid,
    pub mechanism: String,
    pub provider_kind: String,
    pub capability: String,
    pub profile: String,
    pub provider_id_digest: String,
    pub artifact_digest: String,
    pub parameter_set_digest: Option<String>,
    pub review_evidence_digest: Option<String>,
    pub production_claim_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct LifecycleEventRequest {
    pub event_id: Uuid,
    pub event_type: LifecycleEventType,
    pub resource_kind: LifecycleResourceKind,
    pub resource_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RetentionPolicyRequest {
    pub policy_id: Uuid,
    pub resource_kind: LifecycleResourceKind,
    pub data_class: DataClass,
    pub retention_period_days: u32,
    pub review_interval_days: u32,
    pub tombstone_after_days: Option<u32>,
    pub tombstone_publication: TombstonePublicationRule,
    pub legal_basis: LegalBasis,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct LifecycleCaseRequest {
    pub case_id: Uuid,
    pub case_type: LifecycleCaseType,
    pub resource_kind: LifecycleResourceKind,
    pub resource_id: String,
    pub status: LifecycleCaseStatus,
    pub retention_policy_id: Option<Uuid>,
    pub summary: String,
    pub resolution: Option<String>,
    pub tombstone_publication: Option<TombstonePublicationRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct GovernanceProfileRequest {
    pub profile_id: Uuid,
    pub profile_kind: GovernanceProfileKind,
    pub tenant_partition: String,
    pub subject_partition: String,
    pub data_custodian_subject: String,
    pub platform_operator_subject: String,
    pub policy_authority_subject: String,
    pub analyst_subject: String,
    pub auditor_subject: String,
    pub identity_authority_subject: Option<String>,
    pub emergency_access_policy: String,
    pub runbook_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AccessCheckRequest {
    pub role: PrincipalRole,
    pub resource: ResourceClass,
    pub action: PolicyAction,
    pub grant_id: Option<Uuid>,
    pub data_product_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdsSubjectAccount {
    pub home_page: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdsSubjectAgent {
    pub object_type: String,
    pub account: PdsSubjectAccount,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentGrant {
    pub grant_id: String,
    pub data_subject: String,
    pub grantee_subject: String,
    pub scope: ConsentScope,
    pub state: ConsentState,
    pub purpose: String,
    pub legal_basis: LegalBasis,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
    pub renewed_from_grant_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentGrantList {
    pub grants: Vec<ConsentGrant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseArtifactRef {
    pub kind: ReleaseArtifactKind,
    pub artifact_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataProduct {
    pub data_product_id: String,
    pub owner_subject: String,
    pub source_kind: DataProductSourceKind,
    pub release_id: String,
    pub release_artifact: ReleaseArtifactRef,
    pub title: String,
    pub purpose: String,
    pub legal_basis: LegalBasis,
    pub data_classes: Vec<DataClass>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataProductList {
    pub data_products: Vec<DataProduct>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataProductReadResponse {
    pub event_id: String,
    pub decision: PolicyDecision,
    pub deny_reason: Option<String>,
    pub data_product: Option<DataProduct>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderBinding {
    pub binding_id: String,
    pub owner_subject: String,
    pub mechanism: ConfidentialMechanismKind,
    pub provider_kind: SecretComputeProviderKind,
    pub capability: SecretComputeProviderCapability,
    pub profile: SecretComputeProviderProfile,
    pub provider_id_digest: String,
    pub artifact_digest: String,
    pub parameter_set_digest: Option<String>,
    pub review_evidence_digest: Option<String>,
    pub production_claim_requested: bool,
    pub binding_digest: String,
    pub evidence_gate: SecretComputeProviderEvidenceGate,
    pub release_gate: SecretComputeProviderReleaseGate,
    pub runtime_class: SecretComputeProviderRuntimeClass,
    pub production_ready: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderBindingList {
    pub provider_bindings: Vec<ProviderBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatementInventoryRecord {
    pub statement_id: String,
    pub actor_ifi: String,
    pub verb_id: String,
    pub object_id: String,
    pub stored_at: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordInventory {
    pub data_subject: String,
    pub actor: PdsSubjectAgent,
    pub actor_ifi: String,
    pub statement_records: Vec<StatementInventoryRecord>,
    pub data_products: Vec<DataProduct>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleEvent {
    pub event_id: String,
    pub data_subject: String,
    pub event_type: LifecycleEventType,
    pub resource_kind: LifecycleResourceKind,
    pub resource_id: String,
    pub reason: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleEventList {
    pub events: Vec<LifecycleEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPolicy {
    pub policy_id: String,
    pub owner_subject: String,
    pub resource_kind: LifecycleResourceKind,
    pub data_class: DataClass,
    pub retention_period_days: u32,
    pub review_interval_days: u32,
    pub tombstone_after_days: Option<u32>,
    pub tombstone_publication: TombstonePublicationRule,
    pub legal_basis: LegalBasis,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPolicyList {
    pub policies: Vec<RetentionPolicy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleCase {
    pub case_id: String,
    pub data_subject: String,
    pub case_type: LifecycleCaseType,
    pub resource_kind: LifecycleResourceKind,
    pub resource_id: String,
    pub status: LifecycleCaseStatus,
    pub retention_policy_id: Option<String>,
    pub summary: String,
    pub resolution: Option<String>,
    pub tombstone_publication: Option<TombstonePublicationRule>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleCaseList {
    pub cases: Vec<LifecycleCase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceProfile {
    pub profile_id: String,
    pub profile_owner_subject: String,
    pub profile_kind: GovernanceProfileKind,
    pub tenant_partition: String,
    pub subject_partition: String,
    pub data_custodian_subject: String,
    pub platform_operator_subject: String,
    pub policy_authority_subject: String,
    pub analyst_subject: String,
    pub auditor_subject: String,
    pub identity_authority_subject: Option<String>,
    pub emergency_access_policy: String,
    pub runbook_refs: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceProfileList {
    pub profiles: Vec<GovernanceProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataExport {
    pub data_subject: String,
    pub actor: PdsSubjectAgent,
    pub actor_ifi: String,
    pub generated_at: String,
    pub record_inventory: RecordInventory,
    pub consent_grants: Vec<ConsentGrant>,
    pub data_products: Vec<DataProduct>,
    pub lifecycle_events: Vec<LifecycleEvent>,
    pub retention_policies: Vec<RetentionPolicy>,
    pub lifecycle_cases: Vec<LifecycleCase>,
    pub governance_profiles: Vec<GovernanceProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortabilityStatement {
    pub statement_id: String,
    pub stored_at: String,
    pub voided: bool,
    pub statement: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortabilityExport {
    pub data_subject: String,
    pub actor: PdsSubjectAgent,
    pub actor_ifi: String,
    pub generated_at: String,
    pub audit_reference: String,
    pub statements: Vec<PortabilityStatement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub consent_grants: Vec<ConsentGrant>,
    pub data_products: Vec<DataProduct>,
    pub lifecycle_events: Vec<LifecycleEvent>,
    pub retention_policies: Vec<RetentionPolicy>,
    pub lifecycle_cases: Vec<LifecycleCase>,
    pub governance_profiles: Vec<GovernanceProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessCheckResponse {
    pub event_id: String,
    pub decision: PolicyDecision,
    pub deny_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessAuditEvent {
    pub event_id: String,
    pub requester_subject: String,
    pub role: PrincipalRole,
    pub resource: ResourceClass,
    pub action: PolicyAction,
    pub grant_id: Option<String>,
    pub data_product_id: Option<String>,
    pub decision: PolicyDecision,
    pub deny_reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessAuditEventList {
    pub events: Vec<AccessAuditEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum ConsentScope {
    #[serde(rename = "raw-records")]
    RawRecords,
    #[serde(rename = "personal-data")]
    PersonalData,
    #[serde(rename = "aggregate-insights")]
    AggregateInsights,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum DataProductSourceKind {
    #[serde(rename = "threshold-release")]
    ThresholdRelease,
    #[serde(rename = "differential-privacy-release")]
    DifferentialPrivacyRelease,
    #[serde(rename = "shamir-threshold-reconstruction")]
    ShamirThresholdReconstruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum LegalBasis {
    #[serde(rename = "consent")]
    Consent,
    #[serde(rename = "public-task")]
    PublicTask,
    #[serde(rename = "legitimate-interest")]
    LegitimateInterest,
    #[serde(rename = "contract")]
    Contract,
    #[serde(rename = "legal-obligation")]
    LegalObligation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum DataClass {
    #[serde(rename = "statement-metadata")]
    Statement,
    #[serde(rename = "consent-metadata")]
    Consent,
    #[serde(rename = "data-product-metadata")]
    DataProduct,
    #[serde(rename = "access-audit-metadata")]
    AccessAudit,
    #[serde(rename = "lifecycle-metadata")]
    Lifecycle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum PrincipalRole {
    #[serde(rename = "data-subject")]
    DataSubject,
    #[serde(rename = "data-custodian")]
    DataCustodian,
    #[serde(rename = "policy-authority")]
    PolicyAuthority,
    #[serde(rename = "analyst")]
    Analyst,
    #[serde(rename = "auditor")]
    Auditor,
    #[serde(rename = "computation-participant")]
    ComputationParticipant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum ResourceClass {
    #[serde(rename = "raw-learning-record")]
    RawLearningRecord,
    #[serde(rename = "personal-data-store")]
    PersonalDataStore,
    #[serde(rename = "aggregate-insight")]
    AggregateInsight,
    #[serde(rename = "audit-evidence")]
    AuditEvidence,
    #[serde(rename = "confidential-computation")]
    ConfidentialComputation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum PolicyAction {
    #[serde(rename = "write")]
    Write,
    #[serde(rename = "read")]
    Read,
    #[serde(rename = "delegate")]
    Delegate,
    #[serde(rename = "revoke")]
    Revoke,
    #[serde(rename = "compute")]
    Compute,
    #[serde(rename = "release")]
    Release,
    #[serde(rename = "audit")]
    Audit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum LifecycleEventType {
    #[serde(rename = "exported")]
    Exported,
    #[serde(rename = "retention-reviewed")]
    RetentionReviewed,
    #[serde(rename = "correction-requested")]
    CorrectionRequested,
    #[serde(rename = "dispute-opened")]
    DisputeOpened,
    #[serde(rename = "tombstone-recorded")]
    TombstoneRecorded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum LifecycleResourceKind {
    #[serde(rename = "xapi-statement")]
    XapiStatement,
    #[serde(rename = "consent-grant")]
    ConsentGrant,
    #[serde(rename = "data-product")]
    DataProduct,
    #[serde(rename = "metadata-export")]
    MetadataExport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum TombstonePublicationRule {
    #[serde(rename = "metadata-only")]
    MetadataOnly,
    #[serde(rename = "subject-visible")]
    SubjectVisible,
    #[serde(rename = "public-redacted")]
    PublicRedacted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum LifecycleCaseType {
    #[serde(rename = "correction")]
    Correction,
    #[serde(rename = "dispute")]
    Dispute,
    #[serde(rename = "tombstone")]
    Tombstone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum LifecycleCaseStatus {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "under-review")]
    UnderReview,
    #[serde(rename = "resolved")]
    Resolved,
    #[serde(rename = "rejected")]
    Rejected,
    #[serde(rename = "tombstoned")]
    Tombstoned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum GovernanceProfileKind {
    #[serde(rename = "single-operator")]
    SingleOperator,
    #[serde(rename = "school-district")]
    SchoolDistrict,
    #[serde(rename = "multi-agency")]
    MultiAgency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum ConsentState {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "revoked")]
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum PolicyDecision {
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "deny")]
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum ReleaseArtifactKind {
    #[serde(rename = "threshold-release")]
    ThresholdRelease,
    #[serde(rename = "differential-privacy-release")]
    DifferentialPrivacyRelease,
    #[serde(rename = "shamir-threshold-reconstruction")]
    ShamirThresholdReconstruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum ConfidentialMechanismKind {
    #[serde(rename = "additive-n-of-n")]
    AdditiveNOfN,
    #[serde(rename = "private-set-intersection")]
    PrivateSetIntersection,
    #[serde(rename = "private-set-intersection-oprf-dh")]
    PrivateSetIntersectionOprfDh,
    #[serde(rename = "shamir-threshold")]
    ShamirThreshold,
    #[serde(rename = "threshold-frost-signing")]
    ThresholdFrostSigning,
    #[serde(rename = "trusted-execution-environment")]
    TrustedExecutionEnvironment,
    #[serde(rename = "trusted-execution-environment-aws-nitro")]
    TrustedExecutionEnvironmentAwsNitro,
    #[serde(rename = "fhe-sum-feasibility")]
    FheSumFeasibility,
    #[serde(rename = "fhe-sum-runtime")]
    FheSumRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum SecretComputeProviderKind {
    #[serde(rename = "reference-primitive")]
    ReferencePrimitive,
    #[serde(rename = "secret-custody")]
    SecretCustody,
    #[serde(rename = "trusted-execution-environment")]
    TrustedExecutionEnvironment,
    #[serde(rename = "fully-homomorphic-encryption")]
    FullyHomomorphicEncryption,
    #[serde(rename = "private-set-intersection")]
    PrivateSetIntersection,
    #[serde(rename = "threshold-cryptography")]
    ThresholdCryptography,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum SecretComputeProviderCapability {
    #[serde(rename = "secret-custody")]
    SecretCustody,
    #[serde(rename = "tee-attestation")]
    TeeAttestation,
    #[serde(rename = "fhe-evaluation")]
    FheEvaluation,
    #[serde(rename = "psi-evaluation")]
    PsiEvaluation,
    #[serde(rename = "threshold-signing")]
    ThresholdSigning,
    #[serde(rename = "reference-fixtures")]
    ReferenceFixtures,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum SecretComputeProviderProfile {
    #[serde(rename = "kiva-reference-primitives-v1")]
    KivaReferencePrimitivesV1,
    #[serde(rename = "kiva-secret-compute-provider-contract-v1")]
    KivaSecretComputeProviderContractV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum SecretComputeProviderEvidenceGate {
    #[serde(rename = "reference-fixture")]
    ReferenceFixture,
    #[serde(rename = "external-non-production")]
    ExternalNonProduction,
    #[serde(rename = "external-reviewed")]
    ExternalReviewed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum SecretComputeProviderReleaseGate {
    #[serde(rename = "fixture-only")]
    FixtureOnly,
    #[serde(rename = "non-production")]
    NonProduction,
    #[serde(rename = "production")]
    Production,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum SecretComputeProviderRuntimeClass {
    #[serde(rename = "reference")]
    Reference,
    #[serde(rename = "external-non-production")]
    ExternalNonProduction,
    #[serde(rename = "external-reviewed-production")]
    ExternalReviewedProduction,
}

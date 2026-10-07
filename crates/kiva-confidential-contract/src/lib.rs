//! Confidential-computation HTTP contract DTOs.

#![allow(missing_docs)]

use std::num::{NonZeroU64, NonZeroUsize};

use serde::{Deserialize, Serialize};

pub type RequiredNullableString = Option<String>;
pub type ByteArray32 = Vec<u8>;
pub type ByteArray64 = Vec<u8>;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfidentialJobMechanism {
    AdditiveSecureAggregation,
    DifferentialPrivacyRelease,
    ShamirVssAdmission,
    ThresholdFrostSigning,
    PsiCohortHashToken,
    PsiOprfDhVoprf,
    TeePortableAttestation,
    TeeAwsNitroAttestation,
    FheSumFeasibility,
    FheSumRuntime,
}

impl ConfidentialJobMechanism {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::AdditiveSecureAggregation => "additive-secure-aggregation",
            Self::DifferentialPrivacyRelease => "differential-privacy-release",
            Self::ShamirVssAdmission => "shamir-vss-admission",
            Self::ThresholdFrostSigning => "threshold-frost-signing",
            Self::PsiCohortHashToken => "psi-cohort-hash-token",
            Self::PsiOprfDhVoprf => "psi-oprf-dh-voprf",
            Self::TeePortableAttestation => "tee-portable-attestation",
            Self::TeeAwsNitroAttestation => "tee-aws-nitro-attestation",
            Self::FheSumFeasibility => "fhe-sum-feasibility",
            Self::FheSumRuntime => "fhe-sum-runtime",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfidentialJobStatus {
    Draft,
    Admitted,
    Running,
    AwaitingRelease,
    Released,
    Failed,
    Cancelled,
    Expired,
    Retired,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfidentialParticipantStatus {
    Invited,
    Active,
    Suspended,
    Frozen,
    Removed,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfidentialParticipantRole {
    AdditiveShareContributor,
    DataCustodian,
    PolicyAuthority,
    PsiParticipant,
    TeeOperator,
    FheKeyAuthority,
    Auditor,
}

impl ConfidentialParticipantRole {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::AdditiveShareContributor => "additive-share-contributor",
            Self::DataCustodian => "data-custodian",
            Self::PolicyAuthority => "policy-authority",
            Self::PsiParticipant => "psi-participant",
            Self::TeeOperator => "tee-operator",
            Self::FheKeyAuthority => "fhe-key-authority",
            Self::Auditor => "auditor",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdditiveShareReplayWindow {
    pub first_sequence: NonZeroU64,
    pub last_sequence: NonZeroU64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdditiveShareCommitmentSlot {
    pub transcript_id: ByteArray32,
    pub participant_index: NonZeroUsize,
    pub commitment: ByteArray32,
    pub signing_key_id: ByteArray32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecureAggregateShare {
    pub participant_index: NonZeroUsize,
    pub value: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthenticatedAdditiveShareEnvelope {
    pub transcript_id: ByteArray32,
    pub share: SecureAggregateShare,
    pub commitment: ByteArray32,
    pub signing_key_id: ByteArray32,
    pub sequence: NonZeroU64,
    pub proof: ByteArray32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SignedAuthenticatedAdditiveShareEnvelope {
    pub envelope: AuthenticatedAdditiveShareEnvelope,
    pub signature: ByteArray64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdditiveAggregateRequest {
    pub participant_count: NonZeroUsize,
    pub transcript_id: ByteArray32,
    pub slots: Vec<AdditiveShareCommitmentSlot>,
    pub replay_window: AdditiveShareReplayWindow,
    #[serde(default)]
    pub signed_envelopes: Vec<SignedAuthenticatedAdditiveShareEnvelope>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShareSigningKeyRegistration {
    pub signing_key_id: ByteArray32,
    pub participant_index: NonZeroUsize,
    pub public_key: ByteArray32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShareSigningKeyRotation {
    pub old_signing_key_id: ByteArray32,
    pub new_signing_key_id: ByteArray32,
    pub participant_index: NonZeroUsize,
    pub public_key: ByteArray32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShareSigningKeyRevocation {
    pub signing_key_id: ByteArray32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShareSubmission {
    pub signed_envelope: SignedAuthenticatedAdditiveShareEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThresholdReleaseRequest {
    pub release_id: ByteArray32,
    pub transcript_id: ByteArray32,
    pub aggregate: u64,
    pub left_authority_subject: String,
    pub right_authority_subject: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThresholdReleaseApproval {
    pub release_id: ByteArray32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShamirShare {
    pub participant_index: NonZeroUsize,
    pub value: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VerifiableShamirShare {
    pub share: ShamirShare,
    pub commitment: ByteArray32,
    pub opening_nonce: ByteArray32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShamirThresholdReconstruction {
    pub reconstruction_id: ByteArray32,
    pub transcript_id: ByteArray32,
    pub participant_count: NonZeroUsize,
    pub threshold: NonZeroUsize,
    pub share_openings: Vec<VerifiableShamirShare>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DifferentialPrivacyBudget {
    pub budget_id: ByteArray32,
    pub total_epsilon_microunits: NonZeroU64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DifferentialPrivacyBudgetSpend {
    pub budget_id: ByteArray32,
    pub epsilon_microunits: NonZeroU64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DifferentialPrivacyRelease {
    pub release_id: ByteArray32,
    pub budget_id: ByteArray32,
    pub transcript_id: ByteArray32,
    pub aggregate: u64,
    pub data_subject_count: NonZeroUsize,
    pub minimum_population: NonZeroUsize,
    pub epsilon_microunits: NonZeroU64,
    pub l1_sensitivity: NonZeroU64,
    pub noise_value: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DifferentialPrivacySampledRelease {
    pub release_id: ByteArray32,
    pub budget_id: ByteArray32,
    pub transcript_id: ByteArray32,
    pub data_subject_count: NonZeroUsize,
    pub minimum_population: NonZeroUsize,
    pub epsilon_microunits: NonZeroU64,
    pub l1_sensitivity: NonZeroU64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfidentialJobRequest {
    pub job_id: ByteArray32,
    pub mechanism: ConfidentialJobMechanism,
    pub mechanism_profile: String,
    pub workload_id: String,
    pub participant_subjects: Vec<String>,
    pub policy_binding_id: String,
    pub deadline_at: String,
    pub admission_quota_key: String,
    #[serde(default = "default_confidential_job_max_retry_count")]
    pub max_retry_count: i64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfidentialJobTransitionRequest {
    pub job_id: ByteArray32,
    pub target_status: ConfidentialJobStatus,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[must_use]
pub const fn default_confidential_job_max_retry_count() -> i64 {
    3
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfidentialParticipantRequest {
    pub participant_subject: String,
    pub participant_index: NonZeroUsize,
    pub mechanism_roles: Vec<ConfidentialParticipantRole>,
    pub authorization_scope: String,
    pub key_ceremony_ref: String,
    pub recovery_contact_ref: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfidentialParticipantTransitionRequest {
    pub participant_subject: String,
    pub participant_index: NonZeroUsize,
    pub target_status: ConfidentialParticipantStatus,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditiveAggregateResponse {
    pub aggregate: u64,
    pub accepted_records: Vec<ReplayRecordMetadata>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareSigningKeyMetadataList {
    pub signing_keys: Vec<ShareSigningKeyMetadata>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareSigningKeyMetadata {
    pub signing_key_id: String,
    pub participant_index: usize,
    pub public_key: String,
    pub oauth_subject: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmittedShareMetadataList {
    pub submissions: Vec<SubmittedShareMetadata>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmittedShareMetadata {
    pub transcript_id: String,
    pub participant_index: usize,
    pub sequence: u64,
    pub signing_key_id: String,
    pub oauth_subject: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayRecordMetadataList {
    pub records: Vec<ReplayRecordMetadata>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayRecordMetadata {
    pub transcript_id: String,
    pub participant_index: usize,
    pub sequence: u64,
    pub signing_key_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThresholdReleaseMetadataList {
    pub releases: Vec<ThresholdReleaseMetadata>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThresholdReleaseMetadata {
    pub release_id: String,
    pub transcript_id: String,
    pub requester_subject: String,
    pub authority_subjects: Vec<String>,
    pub approved_authority_subjects: Vec<String>,
    pub status: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub released_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThresholdReleaseResponse {
    pub release: ThresholdReleaseMetadata,
    pub aggregate: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShamirThresholdReconstructionMetadataList {
    pub reconstructions: Vec<ShamirThresholdReconstructionMetadata>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShamirThresholdReconstructionMetadata {
    pub reconstruction_id: String,
    pub transcript_id: String,
    pub owner_subject: String,
    pub participant_count: usize,
    pub threshold: usize,
    pub share_count: usize,
    pub verification_profile: String,
    pub verified_share_count: usize,
    pub complaint_count: usize,
    pub mechanism: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShamirThresholdReconstructionResponse {
    pub reconstruction: ShamirThresholdReconstructionMetadata,
    pub reconstructed_value: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DifferentialPrivacyBudgetMetadataList {
    pub budgets: Vec<DifferentialPrivacyBudgetMetadata>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DifferentialPrivacyBudgetMetadata {
    pub budget_id: String,
    pub owner_subject: String,
    pub total_epsilon_microunits: u64,
    pub spent_epsilon_microunits: u64,
    pub remaining_epsilon_microunits: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DifferentialPrivacyCompositionSummaryList {
    pub summaries: Vec<DifferentialPrivacyCompositionSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DifferentialPrivacyCompositionSummary {
    pub budget_id: String,
    pub owner_subject: String,
    pub accounting_scope: String,
    pub composition_rule: String,
    pub advanced_composition_rule: String,
    pub advanced_composition_delta_microunits: u64,
    pub advanced_composition_epsilon_microunits: u64,
    pub total_epsilon_microunits: u64,
    pub budget_spent_epsilon_microunits: u64,
    pub release_spent_epsilon_microunits: u64,
    pub release_epsilon_square_sum_microunits2: String,
    pub unattributed_spent_epsilon_microunits: u64,
    pub remaining_epsilon_microunits: u64,
    pub release_count: usize,
    pub max_release_epsilon_microunits: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DifferentialPrivacyReleaseMetadataList {
    pub releases: Vec<DifferentialPrivacyReleaseMetadata>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DifferentialPrivacyReleaseMetadata {
    pub release_id: String,
    pub budget_id: String,
    pub transcript_id: String,
    pub owner_subject: String,
    pub noised_value: i64,
    pub mechanism: String,
    pub accounting_scope: String,
    pub adjacency_model: String,
    pub epsilon_microunits: u64,
    pub l1_sensitivity: u64,
    pub data_subject_count: usize,
    pub minimum_population: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidentialJobMetadataList {
    pub jobs: Vec<ConfidentialJobMetadata>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidentialJobMetadata {
    pub job_id: String,
    pub owner_subject: String,
    pub mechanism: String,
    pub mechanism_profile: String,
    pub workload_id: String,
    pub participant_subjects: Vec<String>,
    pub policy_binding_id: String,
    pub status: String,
    pub retry_count: i64,
    pub max_retry_count: i64,
    pub deadline_at: String,
    pub admission_quota_key: String,
    pub evidence_refs: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidentialJobAuditEventList {
    pub events: Vec<ConfidentialJobAuditEvent>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidentialJobAuditEvent {
    pub event_id: String,
    pub job_id: String,
    pub previous_status: RequiredNullableString,
    pub new_status: String,
    pub actor_subject: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidentialParticipantMetadataList {
    pub participants: Vec<ConfidentialParticipantMetadata>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidentialParticipantMetadata {
    pub owner_subject: String,
    pub participant_subject: String,
    pub participant_index: usize,
    pub status: String,
    pub mechanism_roles: Vec<String>,
    pub authorization_scope: String,
    pub key_ceremony_ref: String,
    pub recovery_contact_ref: String,
    pub evidence_refs: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidentialParticipantAuditEventList {
    pub events: Vec<ConfidentialParticipantAuditEvent>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidentialParticipantAuditEvent {
    pub event_id: String,
    pub participant_subject: String,
    pub participant_index: usize,
    pub previous_status: RequiredNullableString,
    pub new_status: String,
    pub actor_subject: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
    pub created_at: String,
}

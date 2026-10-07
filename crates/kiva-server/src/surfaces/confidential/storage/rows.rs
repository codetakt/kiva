//! `PostgreSQL` row shapes for confidential-computation storage.

use kiva_confidential::mechanism::additive::SignedAuthenticatedAdditiveShareEnvelope;
use sqlx::types::Json as SqlJson;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct ConfidentialReplayRecordRow {
    pub(super) transcript_id: Vec<u8>,
    pub(super) participant_index: i64,
    pub(super) sequence: String,
    pub(super) signing_key_id: Vec<u8>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct ConfidentialSigningKeyRow {
    pub(super) signing_key_id: Vec<u8>,
    pub(super) participant_index: i64,
    pub(super) public_key: Vec<u8>,
    pub(super) oauth_subject: String,
    pub(super) revoked_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct ConfidentialSubmittedShareRow {
    pub(super) signed_envelope: SqlJson<SignedAuthenticatedAdditiveShareEnvelope>,
    pub(super) oauth_subject: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct ConfidentialAggregateResultRow {
    pub(super) transcript_id: Vec<u8>,
    pub(super) aggregate_value: i64,
    pub(super) participant_count: i64,
    pub(super) computed_at: OffsetDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct ConfidentialShamirThresholdReconstructionRow {
    pub(super) reconstruction_id: Vec<u8>,
    pub(super) transcript_id: Vec<u8>,
    pub(super) owner_subject: String,
    pub(super) participant_count: i64,
    pub(super) threshold: i64,
    pub(super) share_count: i64,
    pub(super) reconstructed_value: i64,
    pub(super) verification_profile: String,
    pub(super) verified_share_count: i64,
    pub(super) complaint_count: i64,
    pub(super) created_at: OffsetDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct ConfidentialDifferentialPrivacyBudgetRow {
    pub(super) budget_id: Vec<u8>,
    pub(super) owner_subject: String,
    pub(super) total_epsilon_microunits: String,
    pub(super) spent_epsilon_microunits: String,
    pub(super) created_at: OffsetDateTime,
    pub(super) updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct ConfidentialDifferentialPrivacyReleaseRow {
    pub(super) release_id: Vec<u8>,
    pub(super) budget_id: Vec<u8>,
    pub(super) transcript_id: Vec<u8>,
    pub(super) owner_subject: String,
    pub(super) aggregate_value: i64,
    pub(super) noise_value: i64,
    pub(super) noised_value: i64,
    pub(super) mechanism: String,
    pub(super) accounting_scope: String,
    pub(super) epsilon_microunits: String,
    pub(super) l1_sensitivity: String,
    pub(super) data_subject_count: i64,
    pub(super) minimum_population: i64,
    pub(super) created_at: OffsetDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct ConfidentialJobRow {
    pub(super) job_id: Vec<u8>,
    pub(super) owner_subject: String,
    pub(super) mechanism: String,
    pub(super) mechanism_profile: String,
    pub(super) workload_id: String,
    pub(super) participant_subjects: Vec<String>,
    pub(super) policy_binding_id: String,
    pub(super) status: String,
    pub(super) retry_count: i64,
    pub(super) max_retry_count: i64,
    pub(super) deadline_at: OffsetDateTime,
    pub(super) admission_quota_key: String,
    pub(super) evidence_refs: Vec<String>,
    pub(super) created_at: OffsetDateTime,
    pub(super) updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct ConfidentialJobAuditEventRow {
    pub(super) event_id: Uuid,
    pub(super) job_id: Vec<u8>,
    pub(super) previous_status: Option<String>,
    pub(super) new_status: String,
    pub(super) actor_subject: String,
    pub(super) reason: String,
    pub(super) evidence_refs: Vec<String>,
    pub(super) created_at: OffsetDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct ConfidentialParticipantRow {
    pub(super) owner_subject: String,
    pub(super) participant_subject: String,
    pub(super) participant_index: i64,
    pub(super) status: String,
    pub(super) mechanism_roles: Vec<String>,
    pub(super) authorization_scope: String,
    pub(super) key_ceremony_ref: String,
    pub(super) recovery_contact_ref: String,
    pub(super) evidence_refs: Vec<String>,
    pub(super) created_at: OffsetDateTime,
    pub(super) updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct ConfidentialParticipantAuditEventRow {
    pub(super) event_id: Uuid,
    pub(super) participant_subject: String,
    pub(super) participant_index: i64,
    pub(super) previous_status: Option<String>,
    pub(super) new_status: String,
    pub(super) actor_subject: String,
    pub(super) reason: String,
    pub(super) evidence_refs: Vec<String>,
    pub(super) created_at: OffsetDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct ConfidentialThresholdReleaseRequestRow {
    pub(super) release_id: Vec<u8>,
    pub(super) transcript_id: Vec<u8>,
    pub(super) aggregate_value: i64,
    pub(super) requester_subject: String,
    pub(super) left_authority_subject: String,
    pub(super) right_authority_subject: String,
    pub(super) created_at: OffsetDateTime,
    pub(super) released_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct ConfidentialThresholdReleaseApprovalRow {
    pub(super) authority_subject: String,
    pub(super) approved_at: OffsetDateTime,
}

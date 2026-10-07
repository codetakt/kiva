//! `PostgreSQL` row tuple shapes for PDS persistence.

use sqlx::types::Json as SqlJson;
use time::OffsetDateTime;
use uuid::Uuid;

pub(super) type PdsConsentGrantRow = (
    Uuid,
    String,
    String,
    String,
    String,
    String,
    String,
    OffsetDateTime,
    Option<OffsetDateTime>,
    Option<OffsetDateTime>,
    Option<Uuid>,
);

pub(super) type PdsDataProductRow = (
    Uuid,
    String,
    String,
    Vec<u8>,
    String,
    String,
    String,
    SqlJson<Vec<String>>,
    OffsetDateTime,
);

#[derive(Debug, sqlx::FromRow)]
pub(super) struct PdsProviderBindingRow {
    pub(super) binding_id: Uuid,
    pub(super) owner_subject: String,
    pub(super) mechanism: String,
    pub(super) provider_kind: String,
    pub(super) capability: String,
    pub(super) profile: String,
    pub(super) provider_id_digest: String,
    pub(super) artifact_digest: String,
    pub(super) parameter_set_digest: Option<String>,
    pub(super) review_evidence_digest: Option<String>,
    pub(super) production_claim_requested: bool,
    pub(super) binding_digest: String,
    pub(super) evidence_gate: String,
    pub(super) release_gate: String,
    pub(super) runtime_class: String,
    pub(super) production_ready: bool,
    pub(super) created_at: OffsetDateTime,
}

pub(super) type PdsAccessAuditEventRow = (
    Uuid,
    String,
    String,
    String,
    String,
    Option<Uuid>,
    Option<Uuid>,
    String,
    Option<String>,
    OffsetDateTime,
);

pub(super) type PdsLifecycleEventRow =
    (Uuid, String, String, String, String, String, OffsetDateTime);

pub(super) type PdsRetentionPolicyRow = (
    Uuid,
    String,
    String,
    String,
    i32,
    i32,
    Option<i32>,
    String,
    String,
    String,
    OffsetDateTime,
);

pub(super) type PdsLifecycleCaseRow = (
    Uuid,
    String,
    String,
    String,
    String,
    String,
    Option<Uuid>,
    String,
    Option<String>,
    Option<String>,
    OffsetDateTime,
    OffsetDateTime,
);

pub(super) type PdsGovernanceProfileRow = (
    Uuid,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    Vec<String>,
    OffsetDateTime,
);

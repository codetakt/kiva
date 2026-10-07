//! PDS secret/confidential-compute provider-binding registry state.

use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdsProviderBindingState {
    pub(crate) binding_id: Uuid,
    pub(crate) owner_subject: String,
    pub(crate) mechanism: String,
    pub(crate) provider_kind: String,
    pub(crate) capability: String,
    pub(crate) profile: String,
    pub(crate) provider_id_digest: String,
    pub(crate) artifact_digest: String,
    pub(crate) parameter_set_digest: Option<String>,
    pub(crate) review_evidence_digest: Option<String>,
    pub(crate) production_claim_requested: bool,
    pub(crate) binding_digest: String,
    pub(crate) evidence_gate: String,
    pub(crate) release_gate: String,
    pub(crate) runtime_class: String,
    pub(crate) production_ready: bool,
    pub(crate) created_at: OffsetDateTime,
}

impl PdsProviderBindingState {
    pub(crate) fn same_registration(&self, other: &Self) -> bool {
        self.binding_id == other.binding_id
            && self.owner_subject == other.owner_subject
            && self.mechanism == other.mechanism
            && self.provider_kind == other.provider_kind
            && self.capability == other.capability
            && self.profile == other.profile
            && self.provider_id_digest == other.provider_id_digest
            && self.artifact_digest == other.artifact_digest
            && self.parameter_set_digest == other.parameter_set_digest
            && self.review_evidence_digest == other.review_evidence_digest
            && self.production_claim_requested == other.production_claim_requested
            && self.binding_digest == other.binding_digest
            && self.evidence_gate == other.evidence_gate
            && self.release_gate == other.release_gate
            && self.runtime_class == other.runtime_class
            && self.production_ready == other.production_ready
    }
}

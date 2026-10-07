//! PDS governance-profile server-local state.

use kiva_pds::GovernanceProfileKind as PdsGovernanceProfileKind;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdsGovernanceProfileState {
    pub(crate) profile_id: Uuid,
    pub(crate) profile_owner_subject: String,
    pub(crate) profile_kind: PdsGovernanceProfileKind,
    pub(crate) tenant_partition: String,
    pub(crate) subject_partition: String,
    pub(crate) data_custodian_subject: String,
    pub(crate) platform_operator_subject: String,
    pub(crate) policy_authority_subject: String,
    pub(crate) analyst_subject: String,
    pub(crate) auditor_subject: String,
    pub(crate) identity_authority_subject: Option<String>,
    pub(crate) emergency_access_policy: String,
    pub(crate) runbook_refs: Vec<String>,
    pub(crate) created_at: OffsetDateTime,
}

impl PdsGovernanceProfileState {
    pub(crate) fn same_registration(&self, other: &Self) -> bool {
        self.profile_id == other.profile_id
            && self.profile_owner_subject == other.profile_owner_subject
            && self.profile_kind == other.profile_kind
            && self.tenant_partition == other.tenant_partition
            && self.subject_partition == other.subject_partition
            && self.data_custodian_subject == other.data_custodian_subject
            && self.platform_operator_subject == other.platform_operator_subject
            && self.policy_authority_subject == other.policy_authority_subject
            && self.analyst_subject == other.analyst_subject
            && self.auditor_subject == other.auditor_subject
            && self.identity_authority_subject == other.identity_authority_subject
            && self.emergency_access_policy == other.emergency_access_policy
            && self.runbook_refs == other.runbook_refs
    }
}

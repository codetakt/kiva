//! Governance-profile PDS wire projection.

use kiva_pds::{
    GovernanceProfile as GovernanceProfileDto, GovernanceProfileKind, GovernanceProfileList,
};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    app::pds as pds_app, core::response::rfc3339, surfaces::pds::model::PdsGovernanceProfileState,
};

use super::common::dto_value;

pub(crate) fn governance_profile_list_response(
    profiles: &[PdsGovernanceProfileState],
) -> Result<Value, serde_json::Error> {
    dto_value(&GovernanceProfileList {
        profiles: profiles.iter().map(governance_profile_dto).collect(),
    })
}

pub(super) trait GovernanceProfileProjection {
    fn profile_id(&self) -> Uuid;
    fn profile_owner_subject(&self) -> &str;
    fn profile_kind(&self) -> GovernanceProfileKind;
    fn tenant_partition(&self) -> &str;
    fn subject_partition(&self) -> &str;
    fn data_custodian_subject(&self) -> &str;
    fn platform_operator_subject(&self) -> &str;
    fn policy_authority_subject(&self) -> &str;
    fn analyst_subject(&self) -> &str;
    fn auditor_subject(&self) -> &str;
    fn identity_authority_subject(&self) -> Option<&str>;
    fn emergency_access_policy(&self) -> &str;
    fn runbook_refs(&self) -> &[String];
    fn created_at(&self) -> OffsetDateTime;
}

impl GovernanceProfileProjection for PdsGovernanceProfileState {
    fn profile_id(&self) -> Uuid {
        self.profile_id
    }

    fn profile_owner_subject(&self) -> &str {
        &self.profile_owner_subject
    }

    fn profile_kind(&self) -> GovernanceProfileKind {
        self.profile_kind
    }

    fn tenant_partition(&self) -> &str {
        &self.tenant_partition
    }

    fn subject_partition(&self) -> &str {
        &self.subject_partition
    }

    fn data_custodian_subject(&self) -> &str {
        &self.data_custodian_subject
    }

    fn platform_operator_subject(&self) -> &str {
        &self.platform_operator_subject
    }

    fn policy_authority_subject(&self) -> &str {
        &self.policy_authority_subject
    }

    fn analyst_subject(&self) -> &str {
        &self.analyst_subject
    }

    fn auditor_subject(&self) -> &str {
        &self.auditor_subject
    }

    fn identity_authority_subject(&self) -> Option<&str> {
        self.identity_authority_subject.as_deref()
    }

    fn emergency_access_policy(&self) -> &str {
        &self.emergency_access_policy
    }

    fn runbook_refs(&self) -> &[String] {
        &self.runbook_refs
    }

    fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

impl GovernanceProfileProjection for pds_app::GovernanceProfileRecord {
    fn profile_id(&self) -> Uuid {
        self.profile_id
    }

    fn profile_owner_subject(&self) -> &str {
        &self.profile_owner_subject
    }

    fn profile_kind(&self) -> GovernanceProfileKind {
        self.profile_kind
    }

    fn tenant_partition(&self) -> &str {
        &self.tenant_partition
    }

    fn subject_partition(&self) -> &str {
        &self.subject_partition
    }

    fn data_custodian_subject(&self) -> &str {
        &self.data_custodian_subject
    }

    fn platform_operator_subject(&self) -> &str {
        &self.platform_operator_subject
    }

    fn policy_authority_subject(&self) -> &str {
        &self.policy_authority_subject
    }

    fn analyst_subject(&self) -> &str {
        &self.analyst_subject
    }

    fn auditor_subject(&self) -> &str {
        &self.auditor_subject
    }

    fn identity_authority_subject(&self) -> Option<&str> {
        self.identity_authority_subject.as_deref()
    }

    fn emergency_access_policy(&self) -> &str {
        &self.emergency_access_policy
    }

    fn runbook_refs(&self) -> &[String] {
        &self.runbook_refs
    }

    fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

pub(super) fn governance_profile_dto<T: GovernanceProfileProjection>(
    profile: &T,
) -> GovernanceProfileDto {
    GovernanceProfileDto {
        profile_id: profile.profile_id().to_string(),
        profile_owner_subject: profile.profile_owner_subject().to_owned(),
        profile_kind: profile.profile_kind(),
        tenant_partition: profile.tenant_partition().to_owned(),
        subject_partition: profile.subject_partition().to_owned(),
        data_custodian_subject: profile.data_custodian_subject().to_owned(),
        platform_operator_subject: profile.platform_operator_subject().to_owned(),
        policy_authority_subject: profile.policy_authority_subject().to_owned(),
        analyst_subject: profile.analyst_subject().to_owned(),
        auditor_subject: profile.auditor_subject().to_owned(),
        identity_authority_subject: profile.identity_authority_subject().map(str::to_owned),
        emergency_access_policy: profile.emergency_access_policy().to_owned(),
        runbook_refs: profile.runbook_refs().to_vec(),
        created_at: rfc3339(profile.created_at()),
    }
}

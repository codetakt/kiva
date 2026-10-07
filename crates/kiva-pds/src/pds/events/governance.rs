#![allow(clippy::wildcard_imports)]

use super::super::*;

/// Domain event produced when a governance-profile registration command is admitted.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GovernanceProfileRegistrationRequested {
    profile_owner_subject: String,
    profile_id: Uuid,
    profile_kind: GovernanceProfileKind,
    tenant_partition: String,
    subject_partition: String,
    data_custodian_subject: String,
    platform_operator_subject: String,
    policy_authority_subject: String,
    analyst_subject: String,
    auditor_subject: String,
    identity_authority_subject: Option<String>,
    emergency_access_policy: String,
    runbook_refs: Vec<String>,
    distinct_role_subjects: bool,
    identity_authority_absent_or_distinct: bool,
    created_at: OffsetDateTime,
}

impl GovernanceProfileRegistrationRequested {
    /// Projects a governance-profile command into a validated event.
    pub fn from_command(
        command: &RegisterGovernanceProfileCommand,
    ) -> Result<Self, ValidationError> {
        let normalized = normalize_governance_profile_request(command.request())?;
        let request = command.request();
        Ok(Self {
            profile_owner_subject: command.profile_owner_subject().to_owned(),
            profile_id: request.profile_id,
            profile_kind: request.profile_kind,
            tenant_partition: normalized.tenant_partition,
            subject_partition: normalized.subject_partition,
            data_custodian_subject: normalized.data_custodian_subject,
            platform_operator_subject: normalized.platform_operator_subject,
            policy_authority_subject: normalized.policy_authority_subject,
            analyst_subject: normalized.analyst_subject,
            auditor_subject: normalized.auditor_subject,
            identity_authority_subject: normalized.identity_authority_subject,
            emergency_access_policy: normalized.emergency_access_policy,
            runbook_refs: normalized.runbook_refs,
            distinct_role_subjects: normalized.distinct_role_subjects,
            identity_authority_absent_or_distinct: normalized.identity_authority_absent_or_distinct,
            created_at: command.created_at(),
        })
    }

    /// Returns the subject that owns the governance profile.
    #[must_use]
    pub fn profile_owner_subject(&self) -> &str {
        &self.profile_owner_subject
    }

    /// Returns the stable governance profile identifier.
    #[must_use]
    pub const fn profile_id(&self) -> Uuid {
        self.profile_id
    }

    /// Returns the governance profile kind.
    #[must_use]
    pub const fn profile_kind(&self) -> GovernanceProfileKind {
        self.profile_kind
    }

    /// Returns the normalized tenant partition.
    #[must_use]
    pub fn tenant_partition(&self) -> &str {
        &self.tenant_partition
    }

    /// Returns the normalized subject partition.
    #[must_use]
    pub fn subject_partition(&self) -> &str {
        &self.subject_partition
    }

    /// Returns the data-custodian subject.
    #[must_use]
    pub fn data_custodian_subject(&self) -> &str {
        &self.data_custodian_subject
    }

    /// Returns the platform-operator subject.
    #[must_use]
    pub fn platform_operator_subject(&self) -> &str {
        &self.platform_operator_subject
    }

    /// Returns the policy-authority subject.
    #[must_use]
    pub fn policy_authority_subject(&self) -> &str {
        &self.policy_authority_subject
    }

    /// Returns the analyst subject.
    #[must_use]
    pub fn analyst_subject(&self) -> &str {
        &self.analyst_subject
    }

    /// Returns the auditor subject.
    #[must_use]
    pub fn auditor_subject(&self) -> &str {
        &self.auditor_subject
    }

    /// Returns the optional identity-authority subject.
    #[must_use]
    pub fn identity_authority_subject(&self) -> Option<&str> {
        self.identity_authority_subject.as_deref()
    }

    /// Returns the emergency access policy.
    #[must_use]
    pub fn emergency_access_policy(&self) -> &str {
        &self.emergency_access_policy
    }

    /// Returns the runbook references.
    #[must_use]
    pub fn runbook_refs(&self) -> &[String] {
        &self.runbook_refs
    }

    /// Returns whether required governance roles are separated.
    #[must_use]
    pub const fn distinct_role_subjects(&self) -> bool {
        self.distinct_role_subjects
    }

    /// Returns whether the identity authority is absent or distinct.
    #[must_use]
    pub const fn identity_authority_absent_or_distinct(&self) -> bool {
        self.identity_authority_absent_or_distinct
    }

    /// Returns the registration timestamp.
    #[must_use]
    pub const fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

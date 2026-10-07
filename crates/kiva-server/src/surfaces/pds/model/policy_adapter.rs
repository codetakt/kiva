//! Boundary between PDS vocabulary and `kiva-policy` concepts.

use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::{
    AccessCheckIntent as PdsAccessCheckIntent, ConsentScope as PdsConsentScope,
    ConsentState as PdsConsentState, DataProductReadIntent as PdsDataProductReadIntent,
    LifecycleCaseStatus as PdsLifecycleCaseStatus,
};
use kiva_policy::policy::{
    AccessRequest, Action, DataProductAccessAuthorization, Decision, DelegationScope,
    DelegationState, DenyReason, LifecycleCaseStatus as PolicyLifecycleCaseStatus, PrincipalRole,
    PublicSectorGovernanceProfile, ResourceClass, decide_personal_data_request,
    pds_data_product_access_is_scoped as policy_pds_data_product_access_is_scoped,
    pds_lifecycle_case_transition_is_allowed, pds_public_sector_governance_profile_is_admissible,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::surfaces::pds::{
    model::state::{PdsAccessAuditEventState, PdsConsentGrantState, PdsDataProductState},
    outcome::PdsRepositoryError,
};

pub(super) const fn pds_consent_scope_delegation_scope(scope: PdsConsentScope) -> DelegationScope {
    match scope {
        PdsConsentScope::RawRecords => DelegationScope::RawRecords,
        PdsConsentScope::PersonalData => DelegationScope::PersonalData,
        PdsConsentScope::AggregateInsights => DelegationScope::AggregateInsights,
    }
}

pub(super) const fn pds_consent_state_delegation_state(state: PdsConsentState) -> DelegationState {
    match state {
        PdsConsentState::Active => DelegationState::Active,
        PdsConsentState::Revoked => DelegationState::Revoked,
    }
}

pub(crate) const fn pds_lifecycle_case_policy_status(
    status: PdsLifecycleCaseStatus,
) -> PolicyLifecycleCaseStatus {
    match status {
        PdsLifecycleCaseStatus::Open => PolicyLifecycleCaseStatus::Open,
        PdsLifecycleCaseStatus::UnderReview => PolicyLifecycleCaseStatus::UnderReview,
        PdsLifecycleCaseStatus::Resolved => PolicyLifecycleCaseStatus::Resolved,
        PdsLifecycleCaseStatus::Rejected => PolicyLifecycleCaseStatus::Rejected,
        PdsLifecycleCaseStatus::Tombstoned => PolicyLifecycleCaseStatus::Tombstoned,
    }
}

pub(crate) const fn pds_lifecycle_case_initial_status_is_allowed(
    status: PdsLifecycleCaseStatus,
) -> bool {
    pds_lifecycle_case_transition_is_allowed(None, pds_lifecycle_case_policy_status(status))
}

pub(crate) const fn pds_data_product_read_access_check(
    request: PdsDataProductReadIntent,
) -> PdsParsedAccessCheckRequest {
    PdsParsedAccessCheckRequest {
        role: PrincipalRole::Analyst,
        resource: ResourceClass::AggregateInsight,
        action: Action::Read,
        grant_id: Some(request.grant_id),
        data_product_id: Some(request.data_product_id),
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PdsParsedAccessCheckRequest {
    pub(crate) role: PrincipalRole,
    pub(crate) resource: ResourceClass,
    pub(crate) action: Action,
    pub(crate) grant_id: Option<Uuid>,
    pub(crate) data_product_id: Option<Uuid>,
}

impl PdsParsedAccessCheckRequest {
    pub(crate) fn from_request(request: &PdsAccessCheckIntent) -> Result<Self, PdsRepositoryError> {
        Ok(Self {
            role: pds_principal_role_from_request(&request.role)?,
            resource: pds_resource_class_from_request(&request.resource)?,
            action: pds_action_from_request(&request.action)?,
            grant_id: request.grant_id,
            data_product_id: request.data_product_id,
        })
    }

    pub(crate) const fn access_request(self) -> AccessRequest {
        AccessRequest::new(self.role, self.resource, self.action)
    }
}

pub(crate) fn pds_access_audit_event_from_decision(
    principal: &AuthenticatedPrincipal,
    request: PdsParsedAccessCheckRequest,
    grant: Option<&PdsConsentGrantState>,
    data_product: Option<&PdsDataProductState>,
) -> PdsAccessAuditEventState {
    let access_request = request.access_request();
    let product_owner_matches_delegation_subject = match (grant, data_product) {
        (Some(grant), Some(data_product)) => data_product.owner_subject == grant.data_subject,
        _ => false,
    };
    let now = OffsetDateTime::now_utc();
    let delegation = grant.and_then(|grant| {
        (grant.grantee_subject == principal.subject).then(|| grant.consent_delegation_at(now))
    });
    let data_product_authorization = DataProductAccessAuthorization::new(
        request.data_product_id.is_some(),
        data_product.is_some(),
        product_owner_matches_delegation_subject,
    );
    let decision =
        if policy_pds_data_product_access_is_scoped(access_request, data_product_authorization) {
            decide_personal_data_request(access_request, delegation)
        } else {
            Decision::Deny(DenyReason::DataProductUnavailable)
        };
    PdsAccessAuditEventState {
        event_id: Uuid::new_v4(),
        requester_subject: principal.subject.clone(),
        role: request.role,
        resource: request.resource,
        action: request.action,
        grant_id: request.grant_id,
        data_product_id: request.data_product_id,
        decision,
        created_at: now,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PdsEvidenceFlag {
    Missing,
    Present,
}

impl PdsEvidenceFlag {
    pub(crate) const fn from_bool(value: bool) -> Self {
        if value { Self::Present } else { Self::Missing }
    }

    pub(crate) const fn is_present(self) -> bool {
        matches!(self, Self::Present)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PdsGovernanceProfileMetadataFacts<'a> {
    pub(crate) tenant_partition: &'a str,
    pub(crate) subject_partition: &'a str,
    pub(crate) required_role_subjects_declared: PdsEvidenceFlag,
    pub(crate) distinct_role_subjects: PdsEvidenceFlag,
    pub(crate) identity_authority_absent_or_distinct: PdsEvidenceFlag,
    pub(crate) emergency_access_policy: &'a str,
    pub(crate) runbook_refs_present: PdsEvidenceFlag,
}

pub(crate) const fn pds_governance_profile_metadata_is_admissible(
    facts: PdsGovernanceProfileMetadataFacts<'_>,
) -> bool {
    let mut evidence_bits = 0_u8;
    if !facts.tenant_partition.is_empty() {
        evidence_bits |= PublicSectorGovernanceProfile::TENANT_PARTITION_DECLARED;
    }
    if !facts.subject_partition.is_empty() {
        evidence_bits |= PublicSectorGovernanceProfile::SUBJECT_PARTITION_DECLARED;
    }
    if facts.required_role_subjects_declared.is_present() {
        evidence_bits |= PublicSectorGovernanceProfile::REQUIRED_ROLE_SUBJECTS_DECLARED;
    }
    if facts.distinct_role_subjects.is_present() {
        evidence_bits |= PublicSectorGovernanceProfile::REQUIRED_ROLE_SUBJECTS_DISTINCT;
    }
    if facts.identity_authority_absent_or_distinct.is_present() {
        evidence_bits |= PublicSectorGovernanceProfile::IDENTITY_AUTHORITY_ABSENT_OR_DISTINCT;
    }
    if !facts.emergency_access_policy.is_empty() {
        evidence_bits |= PublicSectorGovernanceProfile::EMERGENCY_ACCESS_POLICY_DECLARED;
    }
    if facts.runbook_refs_present.is_present() {
        evidence_bits |= PublicSectorGovernanceProfile::RUNBOOKS_DECLARED;
    }
    pds_public_sector_governance_profile_is_admissible(
        PublicSectorGovernanceProfile::from_evidence_bits(evidence_bits),
    )
}

pub(crate) fn pds_principal_role_from_label(value: &str) -> Option<PrincipalRole> {
    match value {
        "data-subject" => Some(PrincipalRole::DataSubject),
        "data-custodian" => Some(PrincipalRole::DataCustodian),
        "policy-authority" => Some(PrincipalRole::PolicyAuthority),
        "analyst" => Some(PrincipalRole::Analyst),
        "auditor" => Some(PrincipalRole::Auditor),
        "computation-participant" => Some(PrincipalRole::ComputationParticipant),
        _other => None,
    }
}

pub(crate) fn pds_resource_class_from_label(value: &str) -> Option<ResourceClass> {
    match value {
        "raw-learning-record" => Some(ResourceClass::RawLearningRecord),
        "personal-data-store" => Some(ResourceClass::PersonalDataStore),
        "aggregate-insight" => Some(ResourceClass::AggregateInsight),
        "audit-evidence" => Some(ResourceClass::AuditEvidence),
        "confidential-computation" => Some(ResourceClass::ConfidentialComputation),
        _other => None,
    }
}

pub(crate) fn pds_action_from_label(value: &str) -> Option<Action> {
    match value {
        "write" => Some(Action::Write),
        "read" => Some(Action::Read),
        "delegate" => Some(Action::Delegate),
        "revoke" => Some(Action::Revoke),
        "compute" => Some(Action::Compute),
        "release" => Some(Action::Release),
        "audit" => Some(Action::Audit),
        _other => None,
    }
}

fn pds_principal_role_from_request(value: &str) -> Result<PrincipalRole, PdsRepositoryError> {
    pds_principal_role_from_label(value).ok_or_else(|| {
        PdsRepositoryError::InvalidRequest("PDS access check role is not supported".to_owned())
    })
}

fn pds_resource_class_from_request(value: &str) -> Result<ResourceClass, PdsRepositoryError> {
    pds_resource_class_from_label(value).ok_or_else(|| {
        PdsRepositoryError::InvalidRequest("PDS access check resource is not supported".to_owned())
    })
}

fn pds_action_from_request(value: &str) -> Result<Action, PdsRepositoryError> {
    pds_action_from_label(value).ok_or_else(|| {
        PdsRepositoryError::InvalidRequest("PDS access check action is not supported".to_owned())
    })
}

pub(crate) const fn pds_principal_role_label(role: PrincipalRole) -> &'static str {
    match role {
        PrincipalRole::DataSubject => "data-subject",
        PrincipalRole::DataCustodian => "data-custodian",
        PrincipalRole::PolicyAuthority => "policy-authority",
        PrincipalRole::Analyst => "analyst",
        PrincipalRole::Auditor => "auditor",
        PrincipalRole::ComputationParticipant => "computation-participant",
    }
}

pub(crate) const fn pds_resource_class_label(resource: ResourceClass) -> &'static str {
    match resource {
        ResourceClass::RawLearningRecord => "raw-learning-record",
        ResourceClass::PersonalDataStore => "personal-data-store",
        ResourceClass::AggregateInsight => "aggregate-insight",
        ResourceClass::AuditEvidence => "audit-evidence",
        ResourceClass::ConfidentialComputation => "confidential-computation",
    }
}

pub(crate) const fn pds_action_label(action: Action) -> &'static str {
    match action {
        Action::Write => "write",
        Action::Read => "read",
        Action::Delegate => "delegate",
        Action::Revoke => "revoke",
        Action::Compute => "compute",
        Action::Release => "release",
        Action::Audit => "audit",
    }
}

pub(crate) const fn pds_decision_label(decision: Decision) -> &'static str {
    match decision {
        Decision::Allow => "allow",
        Decision::Deny(_reason) => "deny",
    }
}

pub(crate) const fn pds_deny_reason(decision: Decision) -> Option<DenyReason> {
    match decision {
        Decision::Allow => None,
        Decision::Deny(reason) => Some(reason),
    }
}

pub(crate) const fn pds_deny_reason_label(reason: DenyReason) -> &'static str {
    match reason {
        DenyReason::RawRecordBoundary => "rawRecordBoundary",
        DenyReason::ReleaseRequiresAuthority => "releaseRequiresAuthority",
        DenyReason::ConsentRequired => "consentRequired",
        DenyReason::ConsentRevoked => "consentRevoked",
        DenyReason::ConsentExpired => "consentExpired",
        DenyReason::DelegationScopeMismatch => "delegationScopeMismatch",
        DenyReason::DataProductUnavailable => "dataProductUnavailable",
        DenyReason::UnsupportedRoleAction => "unsupportedRoleAction",
    }
}

fn pds_deny_reason_from_label(value: &str) -> Option<DenyReason> {
    match value {
        "rawRecordBoundary" => Some(DenyReason::RawRecordBoundary),
        "releaseRequiresAuthority" => Some(DenyReason::ReleaseRequiresAuthority),
        "consentRequired" => Some(DenyReason::ConsentRequired),
        "consentRevoked" => Some(DenyReason::ConsentRevoked),
        "consentExpired" => Some(DenyReason::ConsentExpired),
        "delegationScopeMismatch" => Some(DenyReason::DelegationScopeMismatch),
        "dataProductUnavailable" => Some(DenyReason::DataProductUnavailable),
        "unsupportedRoleAction" => Some(DenyReason::UnsupportedRoleAction),
        _other => None,
    }
}

pub(crate) fn pds_decision_from_labels(
    decision: &str,
    deny_reason: Option<&str>,
) -> Result<Decision, PdsRepositoryError> {
    match (decision, deny_reason) {
        ("allow", None) => Ok(Decision::Allow),
        ("deny", Some(reason)) => pds_deny_reason_from_label(reason)
            .map(Decision::Deny)
            .ok_or(PdsRepositoryError::InvalidStoredData),
        _other => Err(PdsRepositoryError::InvalidStoredData),
    }
}

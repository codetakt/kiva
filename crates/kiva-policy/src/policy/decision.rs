#![allow(clippy::wildcard_imports)]

use super::*;

/// Reason a request is denied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DenyReason {
    /// Raw learning records are not available to the requested role and action.
    RawRecordBoundary,
    /// Result release requires an explicit policy authority.
    ReleaseRequiresAuthority,
    /// The stricter PDS policy requires active subject consent/delegation.
    ConsentRequired,
    /// The subject consent/delegation was revoked.
    ConsentRevoked,
    /// The subject consent/delegation expired.
    ConsentExpired,
    /// The active delegation scope does not cover the requested operation.
    DelegationScopeMismatch,
    /// The referenced PDS data product is missing or outside the delegated subject boundary.
    DataProductUnavailable,
    /// The requested role/action pair is not part of the baseline policy.
    UnsupportedRoleAction,
}

/// Policy decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    /// The request is allowed.
    Allow,
    /// The request is denied with a reason.
    Deny(DenyReason),
}

/// Evaluates the baseline Kiva policy.
#[must_use]
pub const fn decide(request: AccessRequest) -> Decision {
    match (request.role, request.resource, request.action) {
        (
            PrincipalRole::DataSubject,
            ResourceClass::PersonalDataStore,
            Action::Read | Action::Delegate | Action::Revoke,
        )
        | (
            PrincipalRole::DataCustodian,
            ResourceClass::RawLearningRecord,
            Action::Write | Action::Read,
        )
        | (PrincipalRole::Auditor, ResourceClass::AuditEvidence, Action::Audit)
        | (PrincipalRole::Analyst, ResourceClass::AggregateInsight, Action::Read)
        | (
            PrincipalRole::ComputationParticipant,
            ResourceClass::ConfidentialComputation,
            Action::Compute,
        ) => Decision::Allow,
        (_, ResourceClass::ConfidentialComputation, Action::Release)
            if matches!(request.role, PrincipalRole::PolicyAuthority) =>
        {
            Decision::Allow
        }
        (_, ResourceClass::RawLearningRecord, Action::Read) => {
            Decision::Deny(DenyReason::RawRecordBoundary)
        }
        (_, ResourceClass::ConfidentialComputation, Action::Release) => {
            Decision::Deny(DenyReason::ReleaseRequiresAuthority)
        }
        _ => Decision::Deny(DenyReason::UnsupportedRoleAction),
    }
}

/// Returns whether a delegation scope covers a specific read request.
#[must_use]
pub const fn delegation_scope_matches(scope: DelegationScope, request: AccessRequest) -> bool {
    matches!(
        (scope, request.resource, request.action),
        (
            DelegationScope::RawRecords,
            ResourceClass::RawLearningRecord,
            Action::Read,
        ) | (
            DelegationScope::PersonalData,
            ResourceClass::PersonalDataStore,
            Action::Read,
        ) | (
            DelegationScope::AggregateInsights,
            ResourceClass::AggregateInsight,
            Action::Read,
        )
    )
}

const fn decide_delegated_read(
    request: AccessRequest,
    delegation: Option<ConsentDelegation>,
) -> Decision {
    let Some(delegation) = delegation else {
        return Decision::Deny(DenyReason::ConsentRequired);
    };
    if matches!(delegation.state(), DelegationState::Revoked) {
        return Decision::Deny(DenyReason::ConsentRevoked);
    }
    if matches!(delegation.state(), DelegationState::Expired) {
        return Decision::Deny(DenyReason::ConsentExpired);
    }
    if !delegation_scope_matches(delegation.scope(), request) {
        return Decision::Deny(DenyReason::DelegationScopeMismatch);
    }
    Decision::Allow
}

/// Evaluates the stricter PDS-oriented policy boundary with subject delegation.
///
/// This model is intentionally narrower than deployment policy. It captures the
/// fail-closed behavior Kiva needs before raw or derived personal data leaves a
/// subject-managed boundary.
#[must_use]
pub const fn decide_personal_data_request(
    request: AccessRequest,
    delegation: Option<ConsentDelegation>,
) -> Decision {
    match (request.role, request.resource, request.action) {
        (
            PrincipalRole::DataSubject,
            ResourceClass::PersonalDataStore,
            Action::Read | Action::Delegate | Action::Revoke,
        )
        | (PrincipalRole::DataCustodian, ResourceClass::RawLearningRecord, Action::Write)
        | (PrincipalRole::Auditor, ResourceClass::AuditEvidence, Action::Audit)
        | (
            PrincipalRole::ComputationParticipant,
            ResourceClass::ConfidentialComputation,
            Action::Compute,
        ) => Decision::Allow,
        (PrincipalRole::DataCustodian, ResourceClass::RawLearningRecord, Action::Read)
        | (PrincipalRole::Analyst, ResourceClass::AggregateInsight, Action::Read) => {
            decide_delegated_read(request, delegation)
        }
        (_, ResourceClass::ConfidentialComputation, Action::Release)
            if matches!(request.role, PrincipalRole::PolicyAuthority) =>
        {
            Decision::Allow
        }
        (_, ResourceClass::RawLearningRecord, Action::Read) => {
            Decision::Deny(DenyReason::RawRecordBoundary)
        }
        (_, ResourceClass::ConfidentialComputation, Action::Release) => {
            Decision::Deny(DenyReason::ReleaseRequiresAuthority)
        }
        _ => Decision::Deny(DenyReason::UnsupportedRoleAction),
    }
}

/// Returns whether PDS may register a data product for release metadata.
///
/// Differential-privacy and Shamir reconstruction sources are available by
/// construction once persisted. Threshold artifacts require an explicit
/// completed release state. All artifact kinds require the artifact to exist
/// and to be owned by the authenticated principal.
#[must_use]
pub const fn release_artifact_is_available_for_pds_data_product(
    release: ReleaseArtifactAuthorization,
) -> bool {
    match release.artifact_kind() {
        ReleaseArtifactKind::ThresholdRelease => {
            release.artifact_exists()
                && release.principal_matches_artifact_owner()
                && release.artifact_released()
        }
        ReleaseArtifactKind::DifferentialPrivacyRelease
        | ReleaseArtifactKind::ShamirThresholdReconstruction => {
            release.artifact_exists() && release.principal_matches_artifact_owner()
        }
    }
}

/// Returns whether PDS may register a data product for legacy release metadata.
#[must_use]
pub const fn pds_data_product_release_is_available(
    release: DataProductReleaseAuthorization,
) -> bool {
    release_artifact_is_available_for_pds_data_product(release.release_artifact())
}

/// Returns whether an optional data-product reference is valid for access.
///
/// A request without a data-product reference is left to the ordinary PDS
/// policy. A request with a data-product reference must be an aggregate read,
/// must reference an existing product, and must bind that product's owner to
/// the same data subject that delegated access.
#[must_use]
pub const fn pds_data_product_access_is_scoped(
    request: AccessRequest,
    authorization: DataProductAccessAuthorization,
) -> bool {
    !authorization.data_product_requested()
        || (authorization.data_product_exists()
            && authorization.product_owner_matches_delegation_subject()
            && matches!(
                (request.resource(), request.action()),
                (ResourceClass::AggregateInsight, Action::Read)
            ))
}

/// Returns whether a public-sector PDS governance profile is admissible.
///
/// This is intentionally structural: deployment-specific law, procurement,
/// and identity-provider policy remain outside this crate, but Kiva's PDS
/// metadata must fail closed unless partitioning, separated duties, emergency
/// policy, and operational runbook evidence are all declared.
#[must_use]
pub const fn pds_public_sector_governance_profile_is_admissible(
    profile: PublicSectorGovernanceProfile,
) -> bool {
    profile.evidence_bits() & PublicSectorGovernanceProfile::STRICT_EVIDENCE_BITS
        == PublicSectorGovernanceProfile::STRICT_EVIDENCE_BITS
}

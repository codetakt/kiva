use super::{
    AccessRequest, Action, ConsentDelegation, DataProductAccessAuthorization,
    DataProductReleaseAuthorization, DataProductSourceKind, Decision, DelegationScope,
    DelegationState, DenyReason, LifecycleCaseStatus, OAuthScope, PrincipalRole,
    PublicSectorGovernanceProfile, ResourceClass, ResourceServerOperation, decide,
    decide_personal_data_request, oauth_scope_allows_operation, pds_data_product_access_is_scoped,
    pds_data_product_release_is_available, pds_lifecycle_case_transition_is_allowed,
    pds_public_sector_governance_profile_is_admissible,
};

#[test]
fn analysts_read_aggregate_insights_only() {
    let aggregate = AccessRequest::new(
        PrincipalRole::Analyst,
        ResourceClass::AggregateInsight,
        Action::Read,
    );
    let raw = AccessRequest::new(
        PrincipalRole::Analyst,
        ResourceClass::RawLearningRecord,
        Action::Read,
    );

    assert_eq!(decide(aggregate), Decision::Allow);
    assert_ne!(decide(raw), Decision::Allow);
}

#[test]
fn pds_policy_requires_active_matching_delegation_for_sensitive_reads() {
    let aggregate = AccessRequest::new(
        PrincipalRole::Analyst,
        ResourceClass::AggregateInsight,
        Action::Read,
    );
    let raw = AccessRequest::new(
        PrincipalRole::DataCustodian,
        ResourceClass::RawLearningRecord,
        Action::Read,
    );

    assert_eq!(
        decide_personal_data_request(
            aggregate,
            Some(ConsentDelegation::active(
                DelegationScope::AggregateInsights
            )),
        ),
        Decision::Allow
    );
    assert_eq!(
        decide_personal_data_request(
            aggregate,
            Some(ConsentDelegation::active(DelegationScope::RawRecords)),
        ),
        Decision::Deny(DenyReason::DelegationScopeMismatch)
    );
    assert_eq!(
        decide_personal_data_request(
            raw,
            Some(ConsentDelegation::revoked(DelegationScope::RawRecords)),
        ),
        Decision::Deny(DenyReason::ConsentRevoked)
    );
    assert_eq!(
        decide_personal_data_request(
            raw,
            Some(ConsentDelegation::new(
                DelegationState::Expired,
                DelegationScope::RawRecords,
            )),
        ),
        Decision::Deny(DenyReason::ConsentExpired)
    );
}

#[test]
fn pds_data_product_release_admission_fails_closed() {
    assert!(!pds_data_product_release_is_available(
        DataProductReleaseAuthorization::new(
            DataProductSourceKind::DifferentialPrivacyRelease,
            false,
            true,
            true,
        ),
    ));
    assert!(!pds_data_product_release_is_available(
        DataProductReleaseAuthorization::new(
            DataProductSourceKind::ThresholdRelease,
            true,
            true,
            false,
        ),
    ));
    assert!(pds_data_product_release_is_available(
        DataProductReleaseAuthorization::new(
            DataProductSourceKind::ThresholdRelease,
            true,
            true,
            true,
        ),
    ));
    assert!(pds_data_product_release_is_available(
        DataProductReleaseAuthorization::new(
            DataProductSourceKind::ShamirThresholdReconstruction,
            true,
            true,
            false,
        ),
    ));
}

#[test]
fn pds_data_product_access_requires_aggregate_read_and_owner_scope() {
    let aggregate_read = AccessRequest::new(
        PrincipalRole::Analyst,
        ResourceClass::AggregateInsight,
        Action::Read,
    );
    assert!(pds_data_product_access_is_scoped(
        aggregate_read,
        DataProductAccessAuthorization::new(true, true, true),
    ));
    assert!(!pds_data_product_access_is_scoped(
        aggregate_read,
        DataProductAccessAuthorization::new(true, false, true),
    ));
    assert!(!pds_data_product_access_is_scoped(
        aggregate_read,
        DataProductAccessAuthorization::new(true, true, false),
    ));
    assert!(!pds_data_product_access_is_scoped(
        AccessRequest::new(
            PrincipalRole::Analyst,
            ResourceClass::RawLearningRecord,
            Action::Read,
        ),
        DataProductAccessAuthorization::new(true, true, true),
    ));
    assert!(pds_data_product_access_is_scoped(
        AccessRequest::new(
            PrincipalRole::DataSubject,
            ResourceClass::PersonalDataStore,
            Action::Read,
        ),
        DataProductAccessAuthorization::new(false, false, false),
    ));
}

#[test]
fn pds_public_sector_governance_profile_requires_separated_operational_metadata() {
    assert!(pds_public_sector_governance_profile_is_admissible(
        PublicSectorGovernanceProfile::from_evidence_bits(
            PublicSectorGovernanceProfile::STRICT_EVIDENCE_BITS,
        ),
    ));
    assert!(!pds_public_sector_governance_profile_is_admissible(
        PublicSectorGovernanceProfile::from_evidence_bits(
            PublicSectorGovernanceProfile::STRICT_EVIDENCE_BITS
                & !PublicSectorGovernanceProfile::REQUIRED_ROLE_SUBJECTS_DISTINCT,
        ),
    ));
    assert!(!pds_public_sector_governance_profile_is_admissible(
        PublicSectorGovernanceProfile::from_evidence_bits(
            PublicSectorGovernanceProfile::STRICT_EVIDENCE_BITS
                & !PublicSectorGovernanceProfile::IDENTITY_AUTHORITY_ABSENT_OR_DISTINCT,
        ),
    ));
    assert!(!pds_public_sector_governance_profile_is_admissible(
        PublicSectorGovernanceProfile::from_evidence_bits(
            PublicSectorGovernanceProfile::STRICT_EVIDENCE_BITS
                & !PublicSectorGovernanceProfile::EMERGENCY_ACCESS_POLICY_DECLARED,
        ),
    ));
    assert!(!pds_public_sector_governance_profile_is_admissible(
        PublicSectorGovernanceProfile::from_evidence_bits(
            PublicSectorGovernanceProfile::STRICT_EVIDENCE_BITS
                & !PublicSectorGovernanceProfile::RUNBOOKS_DECLARED,
        ),
    ));
}

#[test]
fn oauth_scopes_are_exact_resource_server_operations() {
    assert!(oauth_scope_allows_operation(
        OAuthScope::PdsRead,
        ResourceServerOperation::PdsRead,
    ));
    assert!(!oauth_scope_allows_operation(
        OAuthScope::PdsWrite,
        ResourceServerOperation::PdsRead,
    ));
    assert!(!oauth_scope_allows_operation(
        OAuthScope::ConfidentialRead,
        ResourceServerOperation::ConfidentialRelease,
    ));
    assert!(!oauth_scope_allows_operation(
        OAuthScope::XapiRead,
        ResourceServerOperation::XapiWrite,
    ));
    assert!(oauth_scope_allows_operation(
        OAuthScope::AnalyticsRead,
        ResourceServerOperation::AnalyticsRead,
    ));
    assert!(!oauth_scope_allows_operation(
        OAuthScope::PdsRead,
        ResourceServerOperation::AnalyticsRead,
    ));
}

#[test]
fn lifecycle_case_transitions_are_strict_and_terminal() {
    assert!(pds_lifecycle_case_transition_is_allowed(
        None,
        LifecycleCaseStatus::Open,
    ));
    assert!(!pds_lifecycle_case_transition_is_allowed(
        None,
        LifecycleCaseStatus::Resolved,
    ));
    assert!(pds_lifecycle_case_transition_is_allowed(
        Some(LifecycleCaseStatus::Open),
        LifecycleCaseStatus::UnderReview,
    ));
    assert!(!pds_lifecycle_case_transition_is_allowed(
        Some(LifecycleCaseStatus::Open),
        LifecycleCaseStatus::Resolved,
    ));
    assert!(pds_lifecycle_case_transition_is_allowed(
        Some(LifecycleCaseStatus::UnderReview),
        LifecycleCaseStatus::Resolved,
    ));
    assert!(!pds_lifecycle_case_transition_is_allowed(
        Some(LifecycleCaseStatus::Resolved),
        LifecycleCaseStatus::Open,
    ));
}

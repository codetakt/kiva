use super::{
    AccessRequest, Action, ConsentDelegation, DataProductAccessAuthorization,
    DataProductReleaseAuthorization, DataProductSourceKind, Decision, DelegationScope,
    DelegationState, DenyReason, LifecycleCaseStatus, OAuthScope, PrincipalRole,
    PublicSectorGovernanceProfile, ReleaseArtifactAuthorization, ReleaseArtifactKind,
    ReleaseArtifactRef, ResourceClass, ResourceServerOperation, decide,
    decide_personal_data_request, delegation_scope_matches, oauth_scope_allows_operation,
    pds_data_product_access_is_scoped, pds_data_product_release_is_available,
    pds_lifecycle_case_transition_is_allowed, pds_public_sector_governance_profile_is_admissible,
    release_artifact_is_available_for_pds_data_product,
};

fn any_role() -> PrincipalRole {
    match kani::any::<u8>() % 6 {
        0 => PrincipalRole::DataSubject,
        1 => PrincipalRole::DataCustodian,
        2 => PrincipalRole::PolicyAuthority,
        3 => PrincipalRole::Analyst,
        4 => PrincipalRole::Auditor,
        _ => PrincipalRole::ComputationParticipant,
    }
}

fn any_resource() -> ResourceClass {
    match kani::any::<u8>() % 5 {
        0 => ResourceClass::RawLearningRecord,
        1 => ResourceClass::PersonalDataStore,
        2 => ResourceClass::AggregateInsight,
        3 => ResourceClass::AuditEvidence,
        _ => ResourceClass::ConfidentialComputation,
    }
}

fn any_action() -> Action {
    match kani::any::<u8>() % 7 {
        0 => Action::Write,
        1 => Action::Read,
        2 => Action::Delegate,
        3 => Action::Revoke,
        4 => Action::Compute,
        5 => Action::Release,
        _ => Action::Audit,
    }
}

fn any_delegation_scope() -> DelegationScope {
    match kani::any::<u8>() % 3 {
        0 => DelegationScope::RawRecords,
        1 => DelegationScope::PersonalData,
        _ => DelegationScope::AggregateInsights,
    }
}

fn any_data_product_source_kind() -> DataProductSourceKind {
    match kani::any::<u8>() % 3 {
        0 => DataProductSourceKind::ThresholdRelease,
        1 => DataProductSourceKind::DifferentialPrivacyRelease,
        _ => DataProductSourceKind::ShamirThresholdReconstruction,
    }
}

fn any_release_artifact_kind() -> ReleaseArtifactKind {
    match kani::any::<u8>() % 3 {
        0 => ReleaseArtifactKind::ThresholdRelease,
        1 => ReleaseArtifactKind::DifferentialPrivacyRelease,
        _ => ReleaseArtifactKind::ShamirThresholdReconstruction,
    }
}

fn any_oauth_scope() -> OAuthScope {
    match kani::any::<u8>() % 12 {
        0 => OAuthScope::XapiRead,
        1 => OAuthScope::XapiWrite,
        2 => OAuthScope::PdsRead,
        3 => OAuthScope::PdsWrite,
        4 => OAuthScope::PdsDelegate,
        5 => OAuthScope::PdsRevoke,
        6 => OAuthScope::PdsAudit,
        7 => OAuthScope::ConfidentialRead,
        8 => OAuthScope::ConfidentialWrite,
        9 => OAuthScope::ConfidentialCompute,
        10 => OAuthScope::ConfidentialRelease,
        _ => OAuthScope::AnalyticsRead,
    }
}

fn any_resource_server_operation() -> ResourceServerOperation {
    match kani::any::<u8>() % 12 {
        0 => ResourceServerOperation::XapiRead,
        1 => ResourceServerOperation::XapiWrite,
        2 => ResourceServerOperation::PdsRead,
        3 => ResourceServerOperation::PdsWrite,
        4 => ResourceServerOperation::PdsDelegate,
        5 => ResourceServerOperation::PdsRevoke,
        6 => ResourceServerOperation::PdsAudit,
        7 => ResourceServerOperation::ConfidentialRead,
        8 => ResourceServerOperation::ConfidentialWrite,
        9 => ResourceServerOperation::ConfidentialCompute,
        10 => ResourceServerOperation::ConfidentialRelease,
        _ => ResourceServerOperation::AnalyticsRead,
    }
}

fn any_lifecycle_case_status() -> LifecycleCaseStatus {
    match kani::any::<u8>() % 5 {
        0 => LifecycleCaseStatus::Open,
        1 => LifecycleCaseStatus::UnderReview,
        2 => LifecycleCaseStatus::Resolved,
        3 => LifecycleCaseStatus::Rejected,
        _ => LifecycleCaseStatus::Tombstoned,
    }
}

fn any_request() -> AccessRequest {
    AccessRequest::new(any_role(), any_resource(), any_action())
}

fn any_delegated_read_request() -> AccessRequest {
    match kani::any::<u8>() % 2 {
        0 => AccessRequest::new(
            PrincipalRole::DataCustodian,
            ResourceClass::RawLearningRecord,
            Action::Read,
        ),
        _ => AccessRequest::new(
            PrincipalRole::Analyst,
            ResourceClass::AggregateInsight,
            Action::Read,
        ),
    }
}

#[kani::proof]
fn allow_set_is_exactly_the_baseline_policy() {
    let role = any_role();
    let resource = any_resource();
    let action = any_action();

    let expected_allow = matches!(
        (role, resource, action),
        (
            PrincipalRole::DataSubject,
            ResourceClass::PersonalDataStore,
            Action::Read | Action::Delegate | Action::Revoke,
        ) | (
            PrincipalRole::DataCustodian,
            ResourceClass::RawLearningRecord,
            Action::Write | Action::Read,
        ) | (
            PrincipalRole::Auditor,
            ResourceClass::AuditEvidence,
            Action::Audit,
        ) | (
            PrincipalRole::Analyst,
            ResourceClass::AggregateInsight,
            Action::Read,
        ) | (
            PrincipalRole::ComputationParticipant,
            ResourceClass::ConfidentialComputation,
            Action::Compute,
        ) | (
            PrincipalRole::PolicyAuthority,
            ResourceClass::ConfidentialComputation,
            Action::Release,
        )
    );

    let allowed = matches!(
        decide(AccessRequest::new(role, resource, action)),
        Decision::Allow
    );

    assert!(allowed == expected_allow);
}

#[kani::proof]
fn raw_learning_record_reads_are_custodian_only() {
    let role = any_role();
    let decision = decide(AccessRequest::new(
        role,
        ResourceClass::RawLearningRecord,
        Action::Read,
    ));

    if role == PrincipalRole::DataCustodian {
        assert!(matches!(decision, Decision::Allow));
    } else {
        assert!(matches!(
            decision,
            Decision::Deny(DenyReason::RawRecordBoundary)
        ));
    }
}

#[kani::proof]
fn confidential_result_release_requires_policy_authority() {
    let role = any_role();
    let decision = decide(AccessRequest::new(
        role,
        ResourceClass::ConfidentialComputation,
        Action::Release,
    ));

    if role == PrincipalRole::PolicyAuthority {
        assert!(matches!(decision, Decision::Allow));
    } else {
        assert!(matches!(
            decision,
            Decision::Deny(DenyReason::ReleaseRequiresAuthority)
        ));
    }
}

#[kani::proof]
fn pds_delegated_reads_require_active_consent() {
    let request = any_delegated_read_request();

    assert_eq!(
        decide_personal_data_request(request, None),
        Decision::Deny(DenyReason::ConsentRequired)
    );

    let revoked = ConsentDelegation::new(DelegationState::Revoked, any_delegation_scope());
    assert_eq!(
        decide_personal_data_request(request, Some(revoked)),
        Decision::Deny(DenyReason::ConsentRevoked)
    );

    let expired = ConsentDelegation::new(DelegationState::Expired, any_delegation_scope());
    assert_eq!(
        decide_personal_data_request(request, Some(expired)),
        Decision::Deny(DenyReason::ConsentExpired)
    );
}

#[kani::proof]
fn pds_delegated_reads_require_matching_scope() {
    let request = any_delegated_read_request();
    let scope = any_delegation_scope();
    let delegation = ConsentDelegation::active(scope);
    let decision = decide_personal_data_request(request, Some(delegation));

    assert!(matches!(decision, Decision::Allow) == delegation_scope_matches(scope, request));
}

#[kani::proof]
fn pds_delegation_never_grants_analyst_raw_record_reads() {
    let scope = any_delegation_scope();
    let request = AccessRequest::new(
        PrincipalRole::Analyst,
        ResourceClass::RawLearningRecord,
        Action::Read,
    );
    let decision = decide_personal_data_request(
        request,
        Some(ConsentDelegation::new(DelegationState::Active, scope)),
    );

    assert!(matches!(
        decision,
        Decision::Deny(DenyReason::RawRecordBoundary)
    ));
}

#[kani::proof]
fn pds_data_product_release_gate_matches_source_kind_rules() {
    let source_kind = any_data_product_source_kind();
    let release_exists = kani::any::<bool>();
    let principal_matches_release_owner = kani::any::<bool>();
    let threshold_release_released = kani::any::<bool>();

    let decision = pds_data_product_release_is_available(DataProductReleaseAuthorization::new(
        source_kind,
        release_exists,
        principal_matches_release_owner,
        threshold_release_released,
    ));
    let expected = match source_kind {
        DataProductSourceKind::ThresholdRelease => {
            release_exists && principal_matches_release_owner && threshold_release_released
        }
        DataProductSourceKind::DifferentialPrivacyRelease => {
            release_exists && principal_matches_release_owner
        }
        DataProductSourceKind::ShamirThresholdReconstruction => {
            release_exists && principal_matches_release_owner
        }
    };

    assert!(decision == expected);
}

#[kani::proof]
fn release_artifact_gate_matches_artifact_rules() {
    let artifact_kind = any_release_artifact_kind();
    let artifact_exists = kani::any::<bool>();
    let principal_matches_artifact_owner = kani::any::<bool>();
    let artifact_released = kani::any::<bool>();

    let decision =
        release_artifact_is_available_for_pds_data_product(ReleaseArtifactAuthorization::new(
            ReleaseArtifactRef::new(artifact_kind),
            artifact_exists,
            principal_matches_artifact_owner,
            artifact_released,
        ));
    let expected = match artifact_kind {
        ReleaseArtifactKind::ThresholdRelease => {
            artifact_exists && principal_matches_artifact_owner && artifact_released
        }
        ReleaseArtifactKind::DifferentialPrivacyRelease => {
            artifact_exists && principal_matches_artifact_owner
        }
        ReleaseArtifactKind::ShamirThresholdReconstruction => {
            artifact_exists && principal_matches_artifact_owner
        }
    };

    assert!(decision == expected);
}

#[kani::proof]
fn pds_data_product_access_gate_matches_scope_rules() {
    let request = any_request();
    let data_product_requested = kani::any::<bool>();
    let data_product_exists = kani::any::<bool>();
    let product_owner_matches_delegation_subject = kani::any::<bool>();

    let decision = pds_data_product_access_is_scoped(
        request,
        DataProductAccessAuthorization::new(
            data_product_requested,
            data_product_exists,
            product_owner_matches_delegation_subject,
        ),
    );
    let expected = !data_product_requested
        || (data_product_exists
            && product_owner_matches_delegation_subject
            && matches!(
                (request.resource(), request.action()),
                (ResourceClass::AggregateInsight, Action::Read)
            ));

    assert!(decision == expected);
}

#[kani::proof]
fn pds_public_sector_governance_profile_gate_matches_strict_metadata_rules() {
    let evidence_bits = kani::any::<u8>();
    let profile = PublicSectorGovernanceProfile::from_evidence_bits(evidence_bits);
    let decision = pds_public_sector_governance_profile_is_admissible(profile);
    let expected = evidence_bits & PublicSectorGovernanceProfile::STRICT_EVIDENCE_BITS
        == PublicSectorGovernanceProfile::STRICT_EVIDENCE_BITS;

    assert!(decision == expected);
}

#[kani::proof]
fn oauth_scope_gate_matches_exact_resource_server_operations() {
    let scope = any_oauth_scope();
    let operation = any_resource_server_operation();
    let decision = oauth_scope_allows_operation(scope, operation);
    let expected = matches!(
        (scope, operation),
        (OAuthScope::XapiRead, ResourceServerOperation::XapiRead)
            | (OAuthScope::XapiWrite, ResourceServerOperation::XapiWrite)
            | (OAuthScope::PdsRead, ResourceServerOperation::PdsRead)
            | (OAuthScope::PdsWrite, ResourceServerOperation::PdsWrite)
            | (
                OAuthScope::PdsDelegate,
                ResourceServerOperation::PdsDelegate
            )
            | (OAuthScope::PdsRevoke, ResourceServerOperation::PdsRevoke)
            | (OAuthScope::PdsAudit, ResourceServerOperation::PdsAudit)
            | (
                OAuthScope::ConfidentialRead,
                ResourceServerOperation::ConfidentialRead,
            )
            | (
                OAuthScope::ConfidentialWrite,
                ResourceServerOperation::ConfidentialWrite,
            )
            | (
                OAuthScope::ConfidentialCompute,
                ResourceServerOperation::ConfidentialCompute,
            )
            | (
                OAuthScope::ConfidentialRelease,
                ResourceServerOperation::ConfidentialRelease,
            )
            | (
                OAuthScope::AnalyticsRead,
                ResourceServerOperation::AnalyticsRead
            )
    );

    assert!(decision == expected);
}

#[kani::proof]
fn lifecycle_case_transition_gate_matches_strict_state_machine() {
    let has_current = kani::any::<bool>();
    let current = if has_current {
        Some(any_lifecycle_case_status())
    } else {
        None
    };
    let next = any_lifecycle_case_status();

    let decision = pds_lifecycle_case_transition_is_allowed(current, next);
    let expected = match current {
        None => matches!(next, LifecycleCaseStatus::Open),
        Some(LifecycleCaseStatus::Open) => {
            matches!(
                next,
                LifecycleCaseStatus::Open | LifecycleCaseStatus::UnderReview
            )
        }
        Some(LifecycleCaseStatus::UnderReview) => matches!(
            next,
            LifecycleCaseStatus::UnderReview
                | LifecycleCaseStatus::Resolved
                | LifecycleCaseStatus::Rejected
                | LifecycleCaseStatus::Tombstoned
        ),
        Some(LifecycleCaseStatus::Resolved) => matches!(next, LifecycleCaseStatus::Resolved),
        Some(LifecycleCaseStatus::Rejected) => matches!(next, LifecycleCaseStatus::Rejected),
        Some(LifecycleCaseStatus::Tombstoned) => {
            matches!(next, LifecycleCaseStatus::Tombstoned)
        }
    };

    assert!(decision == expected);
}

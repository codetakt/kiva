use super::{
    AccessCheckIntent, ConsentGrantCommandInput, ConsentRevocationCommandInput, ConsentScope,
    ConsentState, DataClass, DataProductRegistrationCommandInput, DataProductSourceKind,
    GovernanceProfileCommandInput, GovernanceProfileKind, LegalBasis, LifecycleCaseCommandInput,
    LifecycleCaseStatus, LifecycleCaseType, LifecycleEventCommandInput, LifecycleEventType,
    LifecycleResourceKind, PdsDomainEvent, RecordLifecycleEventCommand,
    RegisterConsentGrantCommand, RegisterDataProductCommand, RegisterGovernanceProfileCommand,
    RegisterRetentionPolicyCommand, ReleaseArtifactKind, RetentionPolicyCommandInput,
    RevokeConsentGrantCommand, TombstonePublicationRule, UpsertLifecycleCaseCommand,
    normalize_governance_profile_request, normalize_lifecycle_case_request,
    normalize_retention_policy_request, release_artifact_ref_from_data_product_request,
    validate_consent_grant_request, validate_data_product_request_shape,
    validate_lifecycle_event_request,
};
use time::OffsetDateTime;
use uuid::Uuid;

fn now() -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH
}

#[test]
fn consent_scope_round_trips_wire_labels() {
    for (label, scope) in [
        ("raw-records", ConsentScope::RawRecords),
        ("personal-data", ConsentScope::PersonalData),
        ("aggregate-insights", ConsentScope::AggregateInsights),
    ] {
        assert_eq!(ConsentScope::from_label(label), Some(scope));
        assert_eq!(scope.label(), label);
    }
}

#[test]
fn consent_state_round_trips_wire_labels() {
    for (label, state) in [
        ("active", ConsentState::Active),
        ("revoked", ConsentState::Revoked),
    ] {
        assert_eq!(ConsentState::from_label(label), Some(state));
        assert_eq!(state.label(), label);
    }
}

#[test]
fn data_product_source_kind_round_trips_wire_labels() {
    for (label, kind) in [
        ("threshold-release", DataProductSourceKind::ThresholdRelease),
        (
            "differential-privacy-release",
            DataProductSourceKind::DifferentialPrivacyRelease,
        ),
        (
            "shamir-threshold-reconstruction",
            DataProductSourceKind::ShamirThresholdReconstruction,
        ),
    ] {
        assert_eq!(DataProductSourceKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
    }

    assert_eq!(DataProductSourceKind::from_label("raw-record"), None);
}

#[test]
fn data_product_source_kind_maps_to_release_artifact_kind() {
    assert_eq!(
        DataProductSourceKind::DifferentialPrivacyRelease.release_artifact_kind(),
        ReleaseArtifactKind::DifferentialPrivacyRelease
    );
    assert_eq!(
        DataProductSourceKind::from_release_artifact_kind(ReleaseArtifactKind::ThresholdRelease),
        DataProductSourceKind::ThresholdRelease
    );
}

#[test]
fn release_artifact_kind_round_trips_wire_labels() {
    for (label, kind) in [
        ("threshold-release", ReleaseArtifactKind::ThresholdRelease),
        (
            "differential-privacy-release",
            ReleaseArtifactKind::DifferentialPrivacyRelease,
        ),
        (
            "shamir-threshold-reconstruction",
            ReleaseArtifactKind::ShamirThresholdReconstruction,
        ),
    ] {
        assert_eq!(ReleaseArtifactKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
    }
}

#[test]
fn release_artifact_ref_normalizes_data_product_request_artifacts() {
    let request = DataProductRegistrationCommandInput {
        data_product_id: Uuid::nil(),
        source_kind: DataProductSourceKind::DifferentialPrivacyRelease,
        release_id: "01".repeat(32),
        title: "DP release".to_owned(),
        purpose: "reporting".to_owned(),
        legal_basis: LegalBasis::PublicTask,
        data_classes: vec![DataClass::DataProduct],
    };
    let artifact =
        release_artifact_ref_from_data_product_request(&request).expect("release artifact");
    assert_eq!(
        artifact.kind(),
        ReleaseArtifactKind::DifferentialPrivacyRelease
    );
    assert_eq!(
        artifact.source_kind(),
        DataProductSourceKind::DifferentialPrivacyRelease
    );
    assert_eq!(artifact.artifact_id_hex(), request.release_id);

    let mut empty = request.clone();
    empty.release_id = "00".repeat(32);
    assert!(release_artifact_ref_from_data_product_request(&empty).is_err());

    let mut uppercase = request;
    uppercase.release_id = "A1".repeat(32);
    assert!(release_artifact_ref_from_data_product_request(&uppercase).is_err());
}

#[test]
fn data_product_registration_command_projects_validated_event() {
    let request = DataProductRegistrationCommandInput {
        data_product_id: Uuid::nil(),
        source_kind: DataProductSourceKind::ThresholdRelease,
        release_id: "10".repeat(32),
        title: "Threshold release".to_owned(),
        purpose: "district reporting".to_owned(),
        legal_basis: LegalBasis::PublicTask,
        data_classes: vec![DataClass::DataProduct, DataClass::Lifecycle],
    };
    let command = RegisterDataProductCommand::new("learner-a", request.clone(), now());
    let event_result = command.data_product_registration_requested();
    assert!(event_result.is_ok(), "{event_result:?}");
    let Ok(event) = event_result else {
        return;
    };

    assert_eq!(event.owner_subject(), "learner-a");
    assert_eq!(event.data_product_id(), request.data_product_id);
    assert_eq!(event.title(), request.title);
    assert_eq!(event.purpose(), request.purpose);
    assert_eq!(event.legal_basis(), request.legal_basis);
    assert_eq!(event.data_classes(), request.data_classes.as_slice());
    assert_eq!(
        event.release_artifact().kind(),
        ReleaseArtifactKind::ThresholdRelease
    );
    assert_eq!(
        event.release_artifact().artifact_id_hex(),
        request.release_id
    );
}

#[test]
fn data_product_registration_command_rejects_invalid_artifact() {
    let request = DataProductRegistrationCommandInput {
        data_product_id: Uuid::nil(),
        source_kind: DataProductSourceKind::DifferentialPrivacyRelease,
        release_id: "00".repeat(32),
        title: "DP release".to_owned(),
        purpose: "reporting".to_owned(),
        legal_basis: LegalBasis::PublicTask,
        data_classes: vec![DataClass::DataProduct],
    };
    let command = RegisterDataProductCommand::new("learner-a", request, now());

    assert!(command.data_product_registration_requested().is_err());
}

#[test]
fn pds_command_model_projects_consent_and_lifecycle_events() {
    let now = OffsetDateTime::now_utc();
    let consent_request = ConsentGrantCommandInput {
        grant_id: Uuid::nil(),
        grantee_subject: "researcher-a".to_owned(),
        scope: ConsentScope::PersonalData,
        purpose: "district reporting".to_owned(),
        legal_basis: LegalBasis::Consent,
        expires_at: Some(now + time::Duration::days(30)),
        renewed_from_grant_id: None,
    };
    let consent = RegisterConsentGrantCommand::new("learner-a", consent_request, now)
        .consent_grant_registration_requested()
        .expect("valid consent command");
    assert_eq!(consent.data_subject(), "learner-a");
    assert_eq!(
        PdsDomainEvent::from(consent.clone()).controlling_subject(),
        "learner-a"
    );
    assert_eq!(consent.grantee_subject(), "researcher-a");
    assert_eq!(consent.scope(), ConsentScope::PersonalData);

    let revocation = RevokeConsentGrantCommand::new(
        "learner-a",
        ConsentRevocationCommandInput {
            grant_id: consent.grant_id(),
        },
        now,
    )
    .consent_grant_revocation_requested();
    assert_eq!(revocation.data_subject(), "learner-a");
    assert_eq!(
        PdsDomainEvent::from(revocation.clone()).controlling_subject(),
        "learner-a"
    );
    assert_eq!(revocation.grant_id(), consent.grant_id());

    let lifecycle = RecordLifecycleEventCommand::new(
        "learner-a",
        LifecycleEventCommandInput {
            event_id: Uuid::nil(),
            event_type: LifecycleEventType::Exported,
            resource_kind: LifecycleResourceKind::MetadataExport,
            resource_id: " export-1 ".to_owned(),
            reason: " subject request ".to_owned(),
        },
        now,
    )
    .lifecycle_event_recording_requested()
    .expect("valid lifecycle event command");
    assert_eq!(
        PdsDomainEvent::from(lifecycle.clone()).controlling_subject(),
        "learner-a"
    );
    assert_eq!(lifecycle.resource_id(), "export-1");
    assert_eq!(lifecycle.reason(), "subject request");
}

#[test]
fn pds_command_model_projects_policy_case_and_governance_events() {
    let now = OffsetDateTime::now_utc();
    let retention = RegisterRetentionPolicyCommand::new(
        "school-a",
        RetentionPolicyCommandInput {
            policy_id: Uuid::nil(),
            resource_kind: LifecycleResourceKind::XapiStatement,
            data_class: DataClass::Statement,
            retention_period_days: 365,
            review_interval_days: 90,
            tombstone_after_days: Some(365),
            tombstone_publication: TombstonePublicationRule::MetadataOnly,
            legal_basis: LegalBasis::PublicTask,
            description: " annual review ".to_owned(),
        },
        now,
    )
    .retention_policy_registration_requested()
    .expect("valid retention command");
    assert_eq!(retention.owner_subject(), "school-a");
    assert_eq!(
        PdsDomainEvent::from(retention.clone()).controlling_subject(),
        "school-a"
    );
    assert_eq!(retention.description(), "annual review");

    let lifecycle_case = UpsertLifecycleCaseCommand::new(
        "learner-a",
        LifecycleCaseCommandInput {
            case_id: Uuid::nil(),
            case_type: LifecycleCaseType::Correction,
            resource_kind: LifecycleResourceKind::XapiStatement,
            resource_id: " statement-1 ".to_owned(),
            status: LifecycleCaseStatus::Resolved,
            retention_policy_id: Some(retention.policy_id()),
            summary: " corrected score ".to_owned(),
            resolution: Some(" accepted ".to_owned()),
            tombstone_publication: None,
        },
        now,
    )
    .lifecycle_case_upsert_requested()
    .expect("valid lifecycle case command");
    assert_eq!(
        PdsDomainEvent::from(lifecycle_case.clone()).controlling_subject(),
        "learner-a"
    );
    assert_eq!(lifecycle_case.resource_id(), "statement-1");
    assert_eq!(lifecycle_case.summary(), "corrected score");
    assert_eq!(lifecycle_case.resolution(), Some("accepted"));

    let governance = RegisterGovernanceProfileCommand::new(
        "school-a",
        GovernanceProfileCommandInput {
            profile_id: Uuid::nil(),
            profile_kind: GovernanceProfileKind::SchoolDistrict,
            tenant_partition: " tenant-a ".to_owned(),
            subject_partition: " school-a ".to_owned(),
            data_custodian_subject: "custodian".to_owned(),
            platform_operator_subject: "operator".to_owned(),
            policy_authority_subject: "authority".to_owned(),
            analyst_subject: "analyst".to_owned(),
            auditor_subject: "auditor".to_owned(),
            identity_authority_subject: Some("identity".to_owned()),
            emergency_access_policy: "break-glass".to_owned(),
            runbook_refs: vec![" runbook-a ".to_owned()],
        },
        now,
    )
    .governance_profile_registration_requested()
    .expect("valid governance command");
    assert_eq!(governance.profile_owner_subject(), "school-a");
    assert_eq!(
        PdsDomainEvent::from(governance.clone()).controlling_subject(),
        "school-a"
    );
    assert_eq!(governance.tenant_partition(), "tenant-a");
    assert!(governance.distinct_role_subjects());
}

#[test]
fn legal_basis_round_trips_wire_labels() {
    for (label, basis) in [
        ("consent", LegalBasis::Consent),
        ("public-task", LegalBasis::PublicTask),
        ("legitimate-interest", LegalBasis::LegitimateInterest),
        ("contract", LegalBasis::Contract),
        ("legal-obligation", LegalBasis::LegalObligation),
    ] {
        assert_eq!(LegalBasis::from_label(label), Some(basis));
        assert_eq!(basis.label(), label);
    }
}

#[test]
fn data_class_round_trips_wire_labels() {
    for (label, data_class) in [
        ("statement-metadata", DataClass::Statement),
        ("consent-metadata", DataClass::Consent),
        ("data-product-metadata", DataClass::DataProduct),
        ("access-audit-metadata", DataClass::AccessAudit),
        ("lifecycle-metadata", DataClass::Lifecycle),
    ] {
        assert_eq!(DataClass::from_label(label), Some(data_class));
        assert_eq!(data_class.label(), label);
    }
}

#[test]
fn lifecycle_event_type_round_trips_wire_labels() {
    for (label, event_type) in [
        ("exported", LifecycleEventType::Exported),
        ("retention-reviewed", LifecycleEventType::RetentionReviewed),
        (
            "correction-requested",
            LifecycleEventType::CorrectionRequested,
        ),
        ("dispute-opened", LifecycleEventType::DisputeOpened),
        ("tombstone-recorded", LifecycleEventType::TombstoneRecorded),
    ] {
        assert_eq!(LifecycleEventType::from_label(label), Some(event_type));
        assert_eq!(event_type.label(), label);
    }
}

#[test]
fn lifecycle_resource_kind_round_trips_wire_labels() {
    for (label, resource_kind) in [
        ("xapi-statement", LifecycleResourceKind::XapiStatement),
        ("consent-grant", LifecycleResourceKind::ConsentGrant),
        ("data-product", LifecycleResourceKind::DataProduct),
        ("metadata-export", LifecycleResourceKind::MetadataExport),
    ] {
        assert_eq!(
            LifecycleResourceKind::from_label(label),
            Some(resource_kind)
        );
        assert_eq!(resource_kind.label(), label);
    }
}

#[test]
fn tombstone_publication_rule_round_trips_wire_labels() {
    for (label, rule) in [
        ("metadata-only", TombstonePublicationRule::MetadataOnly),
        ("subject-visible", TombstonePublicationRule::SubjectVisible),
        ("public-redacted", TombstonePublicationRule::PublicRedacted),
    ] {
        assert_eq!(TombstonePublicationRule::from_label(label), Some(rule));
        assert_eq!(rule.label(), label);
    }
}

#[test]
fn lifecycle_case_type_round_trips_wire_labels() {
    for (label, case_type) in [
        ("correction", LifecycleCaseType::Correction),
        ("dispute", LifecycleCaseType::Dispute),
        ("tombstone", LifecycleCaseType::Tombstone),
    ] {
        assert_eq!(LifecycleCaseType::from_label(label), Some(case_type));
        assert_eq!(case_type.label(), label);
    }
}

#[test]
fn lifecycle_case_status_round_trips_wire_labels_and_terminal_marker() {
    for (label, status, requires_resolution) in [
        ("open", LifecycleCaseStatus::Open, false),
        ("under-review", LifecycleCaseStatus::UnderReview, false),
        ("resolved", LifecycleCaseStatus::Resolved, true),
        ("rejected", LifecycleCaseStatus::Rejected, true),
        ("tombstoned", LifecycleCaseStatus::Tombstoned, true),
    ] {
        assert_eq!(LifecycleCaseStatus::from_label(label), Some(status));
        assert_eq!(status.label(), label);
        assert_eq!(status.requires_resolution(), requires_resolution);
    }
}

#[test]
fn governance_profile_kind_round_trips_wire_labels() {
    for (label, profile_kind) in [
        ("single-operator", GovernanceProfileKind::SingleOperator),
        ("school-district", GovernanceProfileKind::SchoolDistrict),
        ("multi-agency", GovernanceProfileKind::MultiAgency),
    ] {
        assert_eq!(GovernanceProfileKind::from_label(label), Some(profile_kind));
        assert_eq!(profile_kind.label(), label);
    }
}

#[test]
fn pds_request_validation_rejects_empty_required_fields() {
    let now = OffsetDateTime::now_utc();
    let consent = ConsentGrantCommandInput {
        grant_id: Uuid::nil(),
        grantee_subject: " ".to_owned(),
        scope: ConsentScope::PersonalData,
        purpose: "research".to_owned(),
        legal_basis: LegalBasis::Consent,
        expires_at: None,
        renewed_from_grant_id: None,
    };
    assert!(validate_consent_grant_request(&consent, now).is_err());

    let data_product = DataProductRegistrationCommandInput {
        data_product_id: Uuid::nil(),
        source_kind: DataProductSourceKind::DifferentialPrivacyRelease,
        release_id: "01".repeat(32),
        title: "DP release".to_owned(),
        purpose: "reporting".to_owned(),
        legal_basis: LegalBasis::PublicTask,
        data_classes: vec![DataClass::DataProduct, DataClass::DataProduct],
    };
    assert!(validate_data_product_request_shape(&data_product).is_err());

    let lifecycle_event = LifecycleEventCommandInput {
        event_id: Uuid::nil(),
        event_type: LifecycleEventType::Exported,
        resource_kind: LifecycleResourceKind::MetadataExport,
        resource_id: "export-1".to_owned(),
        reason: " ".to_owned(),
    };
    assert!(validate_lifecycle_event_request(&lifecycle_event).is_err());
}

#[test]
fn pds_normalization_trims_and_checks_lifecycle_inputs() {
    let retention = RetentionPolicyCommandInput {
        policy_id: Uuid::nil(),
        resource_kind: LifecycleResourceKind::XapiStatement,
        data_class: DataClass::Statement,
        retention_period_days: 30,
        review_interval_days: 10,
        tombstone_after_days: Some(30),
        tombstone_publication: TombstonePublicationRule::MetadataOnly,
        legal_basis: LegalBasis::PublicTask,
        description: "  records policy  ".to_owned(),
    };
    assert_eq!(
        normalize_retention_policy_request(&retention)
            .expect("valid retention policy")
            .description,
        "records policy"
    );

    let lifecycle_case = LifecycleCaseCommandInput {
        case_id: Uuid::nil(),
        case_type: LifecycleCaseType::Tombstone,
        resource_kind: LifecycleResourceKind::XapiStatement,
        resource_id: "  statement-1  ".to_owned(),
        status: LifecycleCaseStatus::Tombstoned,
        retention_policy_id: None,
        summary: "  erase marker  ".to_owned(),
        resolution: Some("  published metadata tombstone  ".to_owned()),
        tombstone_publication: Some(TombstonePublicationRule::SubjectVisible),
    };
    let normalized =
        normalize_lifecycle_case_request(&lifecycle_case).expect("valid lifecycle case");
    assert_eq!(normalized.resource_id, "statement-1");
    assert_eq!(normalized.summary, "erase marker");
    assert_eq!(
        normalized.resolution.as_deref(),
        Some("published metadata tombstone")
    );
}

#[test]
fn governance_profile_normalization_tracks_separation_facts() {
    let profile = GovernanceProfileCommandInput {
        profile_id: Uuid::nil(),
        profile_kind: GovernanceProfileKind::SchoolDistrict,
        tenant_partition: " district-a ".to_owned(),
        subject_partition: " school-1 ".to_owned(),
        data_custodian_subject: "custodian".to_owned(),
        platform_operator_subject: "operator".to_owned(),
        policy_authority_subject: "policy".to_owned(),
        analyst_subject: "analyst".to_owned(),
        auditor_subject: "auditor".to_owned(),
        identity_authority_subject: Some("identity".to_owned()),
        emergency_access_policy: "break-glass".to_owned(),
        runbook_refs: vec![" runbook://pds ".to_owned()],
    };
    let normalized =
        normalize_governance_profile_request(&profile).expect("valid governance profile");
    assert_eq!(normalized.tenant_partition, "district-a");
    assert_eq!(normalized.runbook_refs, ["runbook://pds"]);
    assert!(normalized.distinct_role_subjects);
    assert!(normalized.identity_authority_absent_or_distinct);
}

#[test]
fn access_check_request_remains_plain_wire_dto() {
    let request = AccessCheckIntent {
        role: "data-subject".to_owned(),
        resource: "personal-data".to_owned(),
        action: "read".to_owned(),
        grant_id: None,
        data_product_id: None,
    };
    assert_eq!(request.role, "data-subject");
}

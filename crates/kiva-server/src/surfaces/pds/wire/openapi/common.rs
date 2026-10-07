//! Shared `OpenAPI` enum conversion helpers for PDS request DTOs.

use kiva_pds::{
    ConsentScope, DataClass, DataProductSourceKind, GovernanceProfileKind, LegalBasis,
    LifecycleCaseStatus, LifecycleCaseType, LifecycleEventType, LifecycleResourceKind,
    TombstonePublicationRule,
};
use kiva_pds_openapi_contract as pds_openapi;

pub(super) const fn consent_scope_from_openapi(value: pds_openapi::ConsentScope) -> ConsentScope {
    match value {
        pds_openapi::ConsentScope::RawRecords => ConsentScope::RawRecords,
        pds_openapi::ConsentScope::PersonalData => ConsentScope::PersonalData,
        pds_openapi::ConsentScope::AggregateInsights => ConsentScope::AggregateInsights,
    }
}

pub(super) const fn data_product_source_kind_from_openapi(
    value: pds_openapi::DataProductSourceKind,
) -> DataProductSourceKind {
    match value {
        pds_openapi::DataProductSourceKind::ThresholdRelease => {
            DataProductSourceKind::ThresholdRelease
        }
        pds_openapi::DataProductSourceKind::DifferentialPrivacyRelease => {
            DataProductSourceKind::DifferentialPrivacyRelease
        }
        pds_openapi::DataProductSourceKind::ShamirThresholdReconstruction => {
            DataProductSourceKind::ShamirThresholdReconstruction
        }
    }
}

pub(super) const fn legal_basis_from_openapi(value: pds_openapi::LegalBasis) -> LegalBasis {
    match value {
        pds_openapi::LegalBasis::Consent => LegalBasis::Consent,
        pds_openapi::LegalBasis::PublicTask => LegalBasis::PublicTask,
        pds_openapi::LegalBasis::LegitimateInterest => LegalBasis::LegitimateInterest,
        pds_openapi::LegalBasis::Contract => LegalBasis::Contract,
        pds_openapi::LegalBasis::LegalObligation => LegalBasis::LegalObligation,
    }
}

pub(super) const fn data_class_from_openapi(value: pds_openapi::DataClass) -> DataClass {
    match value {
        pds_openapi::DataClass::Statement => DataClass::Statement,
        pds_openapi::DataClass::Consent => DataClass::Consent,
        pds_openapi::DataClass::DataProduct => DataClass::DataProduct,
        pds_openapi::DataClass::AccessAudit => DataClass::AccessAudit,
        pds_openapi::DataClass::Lifecycle => DataClass::Lifecycle,
    }
}

pub(super) const fn lifecycle_event_type_from_openapi(
    value: pds_openapi::LifecycleEventType,
) -> LifecycleEventType {
    match value {
        pds_openapi::LifecycleEventType::Exported => LifecycleEventType::Exported,
        pds_openapi::LifecycleEventType::RetentionReviewed => LifecycleEventType::RetentionReviewed,
        pds_openapi::LifecycleEventType::CorrectionRequested => {
            LifecycleEventType::CorrectionRequested
        }
        pds_openapi::LifecycleEventType::DisputeOpened => LifecycleEventType::DisputeOpened,
        pds_openapi::LifecycleEventType::TombstoneRecorded => LifecycleEventType::TombstoneRecorded,
    }
}

pub(super) const fn lifecycle_resource_kind_from_openapi(
    value: pds_openapi::LifecycleResourceKind,
) -> LifecycleResourceKind {
    match value {
        pds_openapi::LifecycleResourceKind::XapiStatement => LifecycleResourceKind::XapiStatement,
        pds_openapi::LifecycleResourceKind::ConsentGrant => LifecycleResourceKind::ConsentGrant,
        pds_openapi::LifecycleResourceKind::DataProduct => LifecycleResourceKind::DataProduct,
        pds_openapi::LifecycleResourceKind::MetadataExport => LifecycleResourceKind::MetadataExport,
    }
}

pub(super) const fn tombstone_publication_rule_from_openapi(
    value: pds_openapi::TombstonePublicationRule,
) -> TombstonePublicationRule {
    match value {
        pds_openapi::TombstonePublicationRule::MetadataOnly => {
            TombstonePublicationRule::MetadataOnly
        }
        pds_openapi::TombstonePublicationRule::SubjectVisible => {
            TombstonePublicationRule::SubjectVisible
        }
        pds_openapi::TombstonePublicationRule::PublicRedacted => {
            TombstonePublicationRule::PublicRedacted
        }
    }
}

pub(super) const fn lifecycle_case_type_from_openapi(
    value: pds_openapi::LifecycleCaseType,
) -> LifecycleCaseType {
    match value {
        pds_openapi::LifecycleCaseType::Correction => LifecycleCaseType::Correction,
        pds_openapi::LifecycleCaseType::Dispute => LifecycleCaseType::Dispute,
        pds_openapi::LifecycleCaseType::Tombstone => LifecycleCaseType::Tombstone,
    }
}

pub(super) const fn lifecycle_case_status_from_openapi(
    value: pds_openapi::LifecycleCaseStatus,
) -> LifecycleCaseStatus {
    match value {
        pds_openapi::LifecycleCaseStatus::Open => LifecycleCaseStatus::Open,
        pds_openapi::LifecycleCaseStatus::UnderReview => LifecycleCaseStatus::UnderReview,
        pds_openapi::LifecycleCaseStatus::Resolved => LifecycleCaseStatus::Resolved,
        pds_openapi::LifecycleCaseStatus::Rejected => LifecycleCaseStatus::Rejected,
        pds_openapi::LifecycleCaseStatus::Tombstoned => LifecycleCaseStatus::Tombstoned,
    }
}

pub(super) const fn governance_profile_kind_from_openapi(
    value: pds_openapi::GovernanceProfileKind,
) -> GovernanceProfileKind {
    match value {
        pds_openapi::GovernanceProfileKind::SingleOperator => GovernanceProfileKind::SingleOperator,
        pds_openapi::GovernanceProfileKind::SchoolDistrict => GovernanceProfileKind::SchoolDistrict,
        pds_openapi::GovernanceProfileKind::MultiAgency => GovernanceProfileKind::MultiAgency,
    }
}

pub(super) const fn principal_role_label_from_openapi(
    value: pds_openapi::PrincipalRole,
) -> &'static str {
    match value {
        pds_openapi::PrincipalRole::DataSubject => "data-subject",
        pds_openapi::PrincipalRole::DataCustodian => "data-custodian",
        pds_openapi::PrincipalRole::PolicyAuthority => "policy-authority",
        pds_openapi::PrincipalRole::Analyst => "analyst",
        pds_openapi::PrincipalRole::Auditor => "auditor",
        pds_openapi::PrincipalRole::ComputationParticipant => "computation-participant",
    }
}

pub(super) const fn resource_class_label_from_openapi(
    value: pds_openapi::ResourceClass,
) -> &'static str {
    match value {
        pds_openapi::ResourceClass::RawLearningRecord => "raw-learning-record",
        pds_openapi::ResourceClass::PersonalDataStore => "personal-data-store",
        pds_openapi::ResourceClass::AggregateInsight => "aggregate-insight",
        pds_openapi::ResourceClass::AuditEvidence => "audit-evidence",
        pds_openapi::ResourceClass::ConfidentialComputation => "confidential-computation",
    }
}

pub(super) const fn policy_action_label_from_openapi(
    value: pds_openapi::PolicyAction,
) -> &'static str {
    match value {
        pds_openapi::PolicyAction::Write => "write",
        pds_openapi::PolicyAction::Read => "read",
        pds_openapi::PolicyAction::Delegate => "delegate",
        pds_openapi::PolicyAction::Revoke => "revoke",
        pds_openapi::PolicyAction::Compute => "compute",
        pds_openapi::PolicyAction::Release => "release",
        pds_openapi::PolicyAction::Audit => "audit",
    }
}

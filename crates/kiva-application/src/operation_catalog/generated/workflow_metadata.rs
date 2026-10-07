//! Generated application workflow metadata.
//!
//! Generated from `generated/architecture/surface-catalog.yaml` by
//! `scripts/ci/generate_application_operation_catalog.py`.

#![allow(
    clippy::match_same_arms,
    clippy::redundant_pub_crate,
    clippy::too_many_lines
)]

use crate::{
    analytics::AnalyticsWorkflowKind,
    confidential::{ConfidentialWorkflowFamily, ConfidentialWorkflowKind},
    pds::PdsWorkflowKind,
    runtime::{ApplicationWorkflowKind, ApplicationWorkflowSpec},
};

pub(crate) const fn analytics_workflow_spec(
    kind: AnalyticsWorkflowKind,
) -> ApplicationWorkflowSpec {
    match kind {
        AnalyticsWorkflowKind::ListProducts => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("analytics.list_products"),
            )
        }
        AnalyticsWorkflowKind::ReadProduct => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("analytics.read_product"),
            )
        }
        AnalyticsWorkflowKind::ReadAggregate => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("analytics.read_aggregate"),
            )
        }
    }
}

pub(crate) const fn pds_workflow_spec(kind: PdsWorkflowKind) -> ApplicationWorkflowSpec {
    match kind {
        PdsWorkflowKind::ListAccessAuditEvents => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.list_access_audit_events"),
            )
        }
        PdsWorkflowKind::EvaluateAccessCheck => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.evaluate_access_check"),
            )
        }
        PdsWorkflowKind::ListConsentGrants => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.list_consent_grants"),
            )
        }
        PdsWorkflowKind::RegisterConsentGrant => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.register_consent_grant"),
            )
        }
        PdsWorkflowKind::RevokeConsentGrant => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.revoke_consent_grant"),
            )
        }
        PdsWorkflowKind::ExportPortability => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.export_portability"),
            )
        }
        PdsWorkflowKind::ReadDataProduct => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.read_data_product"),
            )
        }
        PdsWorkflowKind::ListDataProducts => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.list_data_products"),
            )
        }
        PdsWorkflowKind::RegisterDataProduct => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.register_data_product"),
            )
        }
        PdsWorkflowKind::ListGovernanceProfiles => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.list_governance_profiles"),
            )
        }
        PdsWorkflowKind::RegisterGovernanceProfile => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.register_governance_profile"),
            )
        }
        PdsWorkflowKind::ListLifecycleCases => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.list_lifecycle_cases"),
            )
        }
        PdsWorkflowKind::RegisterLifecycleCase => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.register_lifecycle_case"),
            )
        }
        PdsWorkflowKind::ListLifecycleEvents => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.list_lifecycle_events"),
            )
        }
        PdsWorkflowKind::RegisterLifecycleEvent => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.register_lifecycle_event"),
            )
        }
        PdsWorkflowKind::ExportMetadata => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.export_metadata"),
            )
        }
        PdsWorkflowKind::ListProviderBindings => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.list_provider_bindings"),
            )
        }
        PdsWorkflowKind::RegisterProviderBinding => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.register_provider_binding"),
            )
        }
        PdsWorkflowKind::ReadRecordInventory => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.read_record_inventory"),
            )
        }
        PdsWorkflowKind::ListRetentionPolicies => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.list_retention_policies"),
            )
        }
        PdsWorkflowKind::RegisterRetentionPolicy => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("pds.register_retention_policy"),
            )
        }
    }
}

pub(crate) const fn confidential_workflow_spec(
    kind: ConfidentialWorkflowKind,
) -> ApplicationWorkflowSpec {
    match kind {
        ConfidentialWorkflowKind::ReleaseSignedAdditiveAggregate => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.release_signed_additive_aggregate"),
            )
        }
        ConfidentialWorkflowKind::ListReplayRecords => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.list_replay_records"),
            )
        }
        ConfidentialWorkflowKind::RevokeShareSigningKey => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.revoke_share_signing_key"),
            )
        }
        ConfidentialWorkflowKind::RotateShareSigningKey => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.rotate_share_signing_key"),
            )
        }
        ConfidentialWorkflowKind::ListShareSigningKeys => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.list_share_signing_keys"),
            )
        }
        ConfidentialWorkflowKind::RegisterShareSigningKey => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.register_share_signing_key"),
            )
        }
        ConfidentialWorkflowKind::ListSubmittedShares => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.list_submitted_shares"),
            )
        }
        ConfidentialWorkflowKind::SubmitAdditiveShare => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.submit_additive_share"),
            )
        }
        ConfidentialWorkflowKind::SpendDifferentialPrivacyBudget => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.spend_differential_privacy_budget"),
            )
        }
        ConfidentialWorkflowKind::ListDifferentialPrivacyBudgets => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.list_differential_privacy_budgets"),
            )
        }
        ConfidentialWorkflowKind::RegisterDifferentialPrivacyBudget => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.register_differential_privacy_budget"),
            )
        }
        ConfidentialWorkflowKind::ListDifferentialPrivacyCompositionSummaries => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.list_differential_privacy_composition_summaries"),
            )
        }
        ConfidentialWorkflowKind::ListDifferentialPrivacyReleases => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.list_differential_privacy_releases"),
            )
        }
        ConfidentialWorkflowKind::ReleaseDifferentialPrivacyAggregate => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.release_differential_privacy_aggregate"),
            )
        }
        ConfidentialWorkflowKind::ReleaseSampledDifferentialPrivacyAggregate => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.release_sampled_differential_privacy_aggregate"),
            )
        }
        ConfidentialWorkflowKind::ListJobAuditEvents => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.list_job_audit_events"),
            )
        }
        ConfidentialWorkflowKind::TransitionJob => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.transition_job"),
            )
        }
        ConfidentialWorkflowKind::ListJobs => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.list_jobs"),
            )
        }
        ConfidentialWorkflowKind::RegisterJob => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.register_job"),
            )
        }
        ConfidentialWorkflowKind::ListParticipantAuditEvents => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.list_participant_audit_events"),
            )
        }
        ConfidentialWorkflowKind::TransitionParticipant => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.transition_participant"),
            )
        }
        ConfidentialWorkflowKind::ListParticipants => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.list_participants"),
            )
        }
        ConfidentialWorkflowKind::RegisterParticipant => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.register_participant"),
            )
        }
        ConfidentialWorkflowKind::ListShamirThresholdReconstructions => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.list_shamir_threshold_reconstructions"),
            )
        }
        ConfidentialWorkflowKind::ReconstructShamirThresholdSecret => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.reconstruct_shamir_threshold_secret"),
            )
        }
        ConfidentialWorkflowKind::ApproveThresholdRelease => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.approve_threshold_release"),
            )
        }
        ConfidentialWorkflowKind::ListThresholdReleaseRequests => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.list_threshold_release_requests"),
            )
        }
        ConfidentialWorkflowKind::RegisterThresholdReleaseRequest => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.register_threshold_release_request"),
            )
        }
        ConfidentialWorkflowKind::ReleaseThresholdAggregate => {
            ApplicationWorkflowSpec::new(
                ApplicationWorkflowKind::new("confidential.release_threshold_aggregate"),
            )
        }
    }
}

pub(crate) const fn confidential_workflow_rust_kind(
    kind: ConfidentialWorkflowKind,
) -> &'static str {
    match kind {
        ConfidentialWorkflowKind::ReleaseSignedAdditiveAggregate => {
            "ConfidentialWorkflowKind::ReleaseSignedAdditiveAggregate"
        }
        ConfidentialWorkflowKind::ListReplayRecords => {
            "ConfidentialWorkflowKind::ListReplayRecords"
        }
        ConfidentialWorkflowKind::RevokeShareSigningKey => {
            "ConfidentialWorkflowKind::RevokeShareSigningKey"
        }
        ConfidentialWorkflowKind::RotateShareSigningKey => {
            "ConfidentialWorkflowKind::RotateShareSigningKey"
        }
        ConfidentialWorkflowKind::ListShareSigningKeys => {
            "ConfidentialWorkflowKind::ListShareSigningKeys"
        }
        ConfidentialWorkflowKind::RegisterShareSigningKey => {
            "ConfidentialWorkflowKind::RegisterShareSigningKey"
        }
        ConfidentialWorkflowKind::ListSubmittedShares => {
            "ConfidentialWorkflowKind::ListSubmittedShares"
        }
        ConfidentialWorkflowKind::SubmitAdditiveShare => {
            "ConfidentialWorkflowKind::SubmitAdditiveShare"
        }
        ConfidentialWorkflowKind::SpendDifferentialPrivacyBudget => {
            "ConfidentialWorkflowKind::SpendDifferentialPrivacyBudget"
        }
        ConfidentialWorkflowKind::ListDifferentialPrivacyBudgets => {
            "ConfidentialWorkflowKind::ListDifferentialPrivacyBudgets"
        }
        ConfidentialWorkflowKind::RegisterDifferentialPrivacyBudget => {
            "ConfidentialWorkflowKind::RegisterDifferentialPrivacyBudget"
        }
        ConfidentialWorkflowKind::ListDifferentialPrivacyCompositionSummaries => {
            "ConfidentialWorkflowKind::ListDifferentialPrivacyCompositionSummaries"
        }
        ConfidentialWorkflowKind::ListDifferentialPrivacyReleases => {
            "ConfidentialWorkflowKind::ListDifferentialPrivacyReleases"
        }
        ConfidentialWorkflowKind::ReleaseDifferentialPrivacyAggregate => {
            "ConfidentialWorkflowKind::ReleaseDifferentialPrivacyAggregate"
        }
        ConfidentialWorkflowKind::ReleaseSampledDifferentialPrivacyAggregate => {
            "ConfidentialWorkflowKind::ReleaseSampledDifferentialPrivacyAggregate"
        }
        ConfidentialWorkflowKind::ListJobAuditEvents => {
            "ConfidentialWorkflowKind::ListJobAuditEvents"
        }
        ConfidentialWorkflowKind::TransitionJob => {
            "ConfidentialWorkflowKind::TransitionJob"
        }
        ConfidentialWorkflowKind::ListJobs => {
            "ConfidentialWorkflowKind::ListJobs"
        }
        ConfidentialWorkflowKind::RegisterJob => {
            "ConfidentialWorkflowKind::RegisterJob"
        }
        ConfidentialWorkflowKind::ListParticipantAuditEvents => {
            "ConfidentialWorkflowKind::ListParticipantAuditEvents"
        }
        ConfidentialWorkflowKind::TransitionParticipant => {
            "ConfidentialWorkflowKind::TransitionParticipant"
        }
        ConfidentialWorkflowKind::ListParticipants => {
            "ConfidentialWorkflowKind::ListParticipants"
        }
        ConfidentialWorkflowKind::RegisterParticipant => {
            "ConfidentialWorkflowKind::RegisterParticipant"
        }
        ConfidentialWorkflowKind::ListShamirThresholdReconstructions => {
            "ConfidentialWorkflowKind::ListShamirThresholdReconstructions"
        }
        ConfidentialWorkflowKind::ReconstructShamirThresholdSecret => {
            "ConfidentialWorkflowKind::ReconstructShamirThresholdSecret"
        }
        ConfidentialWorkflowKind::ApproveThresholdRelease => {
            "ConfidentialWorkflowKind::ApproveThresholdRelease"
        }
        ConfidentialWorkflowKind::ListThresholdReleaseRequests => {
            "ConfidentialWorkflowKind::ListThresholdReleaseRequests"
        }
        ConfidentialWorkflowKind::RegisterThresholdReleaseRequest => {
            "ConfidentialWorkflowKind::RegisterThresholdReleaseRequest"
        }
        ConfidentialWorkflowKind::ReleaseThresholdAggregate => {
            "ConfidentialWorkflowKind::ReleaseThresholdAggregate"
        }
    }
}

pub(crate) const fn confidential_workflow_family(
    kind: ConfidentialWorkflowKind,
) -> ConfidentialWorkflowFamily {
    match kind {
        ConfidentialWorkflowKind::ReleaseSignedAdditiveAggregate => {
            ConfidentialWorkflowFamily::Additive
        }
        ConfidentialWorkflowKind::ListReplayRecords => {
            ConfidentialWorkflowFamily::Additive
        }
        ConfidentialWorkflowKind::RevokeShareSigningKey => {
            ConfidentialWorkflowFamily::Additive
        }
        ConfidentialWorkflowKind::RotateShareSigningKey => {
            ConfidentialWorkflowFamily::Additive
        }
        ConfidentialWorkflowKind::ListShareSigningKeys => {
            ConfidentialWorkflowFamily::Additive
        }
        ConfidentialWorkflowKind::RegisterShareSigningKey => {
            ConfidentialWorkflowFamily::Additive
        }
        ConfidentialWorkflowKind::ListSubmittedShares => {
            ConfidentialWorkflowFamily::Additive
        }
        ConfidentialWorkflowKind::SubmitAdditiveShare => {
            ConfidentialWorkflowFamily::Additive
        }
        ConfidentialWorkflowKind::SpendDifferentialPrivacyBudget => {
            ConfidentialWorkflowFamily::DifferentialPrivacy
        }
        ConfidentialWorkflowKind::ListDifferentialPrivacyBudgets => {
            ConfidentialWorkflowFamily::DifferentialPrivacy
        }
        ConfidentialWorkflowKind::RegisterDifferentialPrivacyBudget => {
            ConfidentialWorkflowFamily::DifferentialPrivacy
        }
        ConfidentialWorkflowKind::ListDifferentialPrivacyCompositionSummaries => {
            ConfidentialWorkflowFamily::DifferentialPrivacy
        }
        ConfidentialWorkflowKind::ListDifferentialPrivacyReleases => {
            ConfidentialWorkflowFamily::DifferentialPrivacy
        }
        ConfidentialWorkflowKind::ReleaseDifferentialPrivacyAggregate => {
            ConfidentialWorkflowFamily::DifferentialPrivacy
        }
        ConfidentialWorkflowKind::ReleaseSampledDifferentialPrivacyAggregate => {
            ConfidentialWorkflowFamily::DifferentialPrivacy
        }
        ConfidentialWorkflowKind::ListJobAuditEvents => {
            ConfidentialWorkflowFamily::Job
        }
        ConfidentialWorkflowKind::TransitionJob => {
            ConfidentialWorkflowFamily::Job
        }
        ConfidentialWorkflowKind::ListJobs => {
            ConfidentialWorkflowFamily::Job
        }
        ConfidentialWorkflowKind::RegisterJob => {
            ConfidentialWorkflowFamily::Job
        }
        ConfidentialWorkflowKind::ListParticipantAuditEvents => {
            ConfidentialWorkflowFamily::Participant
        }
        ConfidentialWorkflowKind::TransitionParticipant => {
            ConfidentialWorkflowFamily::Participant
        }
        ConfidentialWorkflowKind::ListParticipants => {
            ConfidentialWorkflowFamily::Participant
        }
        ConfidentialWorkflowKind::RegisterParticipant => {
            ConfidentialWorkflowFamily::Participant
        }
        ConfidentialWorkflowKind::ListShamirThresholdReconstructions => {
            ConfidentialWorkflowFamily::Shamir
        }
        ConfidentialWorkflowKind::ReconstructShamirThresholdSecret => {
            ConfidentialWorkflowFamily::Shamir
        }
        ConfidentialWorkflowKind::ApproveThresholdRelease => {
            ConfidentialWorkflowFamily::Threshold
        }
        ConfidentialWorkflowKind::ListThresholdReleaseRequests => {
            ConfidentialWorkflowFamily::Threshold
        }
        ConfidentialWorkflowKind::RegisterThresholdReleaseRequest => {
            ConfidentialWorkflowFamily::Threshold
        }
        ConfidentialWorkflowKind::ReleaseThresholdAggregate => {
            ConfidentialWorkflowFamily::Threshold
        }
    }
}

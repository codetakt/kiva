//! PDS lifecycle-ledger server-local state.

use time::OffsetDateTime;

use super::{
    access::PdsAccessAuditEventState,
    consent::PdsConsentGrantState,
    data_products::PdsDataProductState,
    governance::PdsGovernanceProfileState,
    lifecycle::{PdsLifecycleCaseState, PdsLifecycleEventState, PdsRetentionPolicyState},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PdsLedgerEventKind {
    ConsentGrantRegistered,
    ConsentGrantRevoked,
    DataProductRegistered,
    AccessDecisionRecorded,
    LifecycleEventRegistered,
    RetentionPolicyRegistered,
    LifecycleCaseOpened,
    LifecycleCaseTransitioned,
    GovernanceProfileRegistered,
    PortabilityExportCompleted,
    PortabilityExportRejected,
}

impl PdsLedgerEventKind {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::ConsentGrantRegistered => "consent-grant-registered",
            Self::ConsentGrantRevoked => "consent-grant-revoked",
            Self::DataProductRegistered => "data-product-registered",
            Self::AccessDecisionRecorded => "access-decision-recorded",
            Self::LifecycleEventRegistered => "lifecycle-event-registered",
            Self::RetentionPolicyRegistered => "retention-policy-registered",
            Self::LifecycleCaseOpened => "lifecycle-case-opened",
            Self::LifecycleCaseTransitioned => "lifecycle-case-transitioned",
            Self::GovernanceProfileRegistered => "governance-profile-registered",
            Self::PortabilityExportCompleted => "portability-export-completed",
            Self::PortabilityExportRejected => "portability-export-rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdsLedgerEventState {
    pub(crate) event_kind: PdsLedgerEventKind,
    pub(crate) owner_subject: String,
    pub(crate) resource_kind: &'static str,
    pub(crate) resource_id: String,
    pub(crate) occurred_at: OffsetDateTime,
}

impl PdsLedgerEventState {
    fn new(
        event_kind: PdsLedgerEventKind,
        owner_subject: impl Into<String>,
        resource_kind: &'static str,
        resource_id: impl Into<String>,
        occurred_at: OffsetDateTime,
    ) -> Self {
        Self {
            event_kind,
            owner_subject: owner_subject.into(),
            resource_kind,
            resource_id: resource_id.into(),
            occurred_at,
        }
    }

    pub(crate) fn consent_grant_registered(grant: &PdsConsentGrantState) -> Self {
        Self::new(
            PdsLedgerEventKind::ConsentGrantRegistered,
            grant.data_subject.clone(),
            "consent-grant",
            grant.grant_id.to_string(),
            grant.created_at,
        )
    }

    pub(crate) fn consent_grant_revoked(
        grant: &PdsConsentGrantState,
        revoked_at: OffsetDateTime,
    ) -> Self {
        Self::new(
            PdsLedgerEventKind::ConsentGrantRevoked,
            grant.data_subject.clone(),
            "consent-grant",
            grant.grant_id.to_string(),
            revoked_at,
        )
    }

    pub(crate) fn data_product_registered(product: &PdsDataProductState) -> Self {
        Self::new(
            PdsLedgerEventKind::DataProductRegistered,
            product.owner_subject.clone(),
            "data-product",
            product.data_product_id.to_string(),
            product.created_at,
        )
    }

    pub(crate) fn access_decision_recorded(event: &PdsAccessAuditEventState) -> Self {
        Self::new(
            PdsLedgerEventKind::AccessDecisionRecorded,
            event.requester_subject.clone(),
            "access-audit-event",
            event.event_id.to_string(),
            event.created_at,
        )
    }

    pub(crate) fn lifecycle_event_registered(event: &PdsLifecycleEventState) -> Self {
        Self::new(
            PdsLedgerEventKind::LifecycleEventRegistered,
            event.data_subject.clone(),
            "lifecycle-event",
            event.event_id.to_string(),
            event.created_at,
        )
    }

    pub(crate) fn retention_policy_registered(policy: &PdsRetentionPolicyState) -> Self {
        Self::new(
            PdsLedgerEventKind::RetentionPolicyRegistered,
            policy.owner_subject.clone(),
            "retention-policy",
            policy.policy_id.to_string(),
            policy.created_at,
        )
    }

    pub(crate) fn lifecycle_case_opened(case: &PdsLifecycleCaseState) -> Self {
        Self::new(
            PdsLedgerEventKind::LifecycleCaseOpened,
            case.data_subject.clone(),
            "lifecycle-case",
            case.case_id.to_string(),
            case.created_at,
        )
    }

    pub(crate) fn lifecycle_case_transitioned(case: &PdsLifecycleCaseState) -> Self {
        Self::new(
            PdsLedgerEventKind::LifecycleCaseTransitioned,
            case.data_subject.clone(),
            "lifecycle-case",
            case.case_id.to_string(),
            case.updated_at,
        )
    }

    pub(crate) fn governance_profile_registered(profile: &PdsGovernanceProfileState) -> Self {
        Self::new(
            PdsLedgerEventKind::GovernanceProfileRegistered,
            profile.profile_owner_subject.clone(),
            "governance-profile",
            profile.profile_id.to_string(),
            profile.created_at,
        )
    }

    pub(crate) fn portability_export_audited(
        event: &kiva_pds::PortabilityExportAuditRecorded,
    ) -> Self {
        let event_kind = match event.outcome() {
            kiva_pds::PortabilityExportAuditOutcome::Completed => {
                PdsLedgerEventKind::PortabilityExportCompleted
            }
            kiva_pds::PortabilityExportAuditOutcome::Rejected => {
                PdsLedgerEventKind::PortabilityExportRejected
            }
        };
        Self::new(
            event_kind,
            event.data_subject(),
            "data-portability-export",
            event.event_id().to_string(),
            event.occurred_at(),
        )
    }
}

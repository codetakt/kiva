//! PDS state projections from admitted server-independent domain events.

use kiva_pds::{
    ConsentGrantRegistrationRequested, DataProductRegistrationRequested,
    GovernanceProfileRegistrationRequested, LifecycleCaseUpsertRequested,
    LifecycleEventRecordingRequested, RetentionPolicyRegistrationRequested,
};

use super::state::{
    PdsConsentGrantState, PdsDataProductState, PdsGovernanceProfileState, PdsLedgerEventState,
    PdsLifecycleCaseState, PdsLifecycleEventState, PdsRetentionPolicyState,
};

pub(crate) trait PdsProjection {
    fn ledger_event(&self) -> PdsLedgerEventState;
}

pub(crate) trait PdsProjectionFromEvent<Event>: PdsProjection {
    fn from_admitted_event(event: &Event) -> Self;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdsConsentGrantProjection {
    state: PdsConsentGrantState,
}

impl PdsConsentGrantProjection {
    pub(crate) fn from_admitted_event(event: &ConsentGrantRegistrationRequested) -> Self {
        <Self as PdsProjectionFromEvent<ConsentGrantRegistrationRequested>>::from_admitted_event(
            event,
        )
    }

    pub(crate) const fn state(&self) -> &PdsConsentGrantState {
        &self.state
    }

    pub(crate) fn ledger_event(&self) -> PdsLedgerEventState {
        <Self as PdsProjection>::ledger_event(self)
    }
}

impl PdsProjection for PdsConsentGrantProjection {
    fn ledger_event(&self) -> PdsLedgerEventState {
        PdsLedgerEventState::consent_grant_registered(&self.state)
    }
}

impl PdsProjectionFromEvent<ConsentGrantRegistrationRequested> for PdsConsentGrantProjection {
    fn from_admitted_event(event: &ConsentGrantRegistrationRequested) -> Self {
        Self {
            state: PdsConsentGrantState::from_registration_event(event),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdsDataProductProjection {
    state: PdsDataProductState,
}

impl PdsDataProductProjection {
    pub(crate) fn from_admitted_event(event: &DataProductRegistrationRequested) -> Self {
        <Self as PdsProjectionFromEvent<DataProductRegistrationRequested>>::from_admitted_event(
            event,
        )
    }

    pub(crate) const fn state(&self) -> &PdsDataProductState {
        &self.state
    }

    pub(crate) fn ledger_event(&self) -> PdsLedgerEventState {
        <Self as PdsProjection>::ledger_event(self)
    }
}

impl PdsProjection for PdsDataProductProjection {
    fn ledger_event(&self) -> PdsLedgerEventState {
        PdsLedgerEventState::data_product_registered(&self.state)
    }
}

impl PdsProjectionFromEvent<DataProductRegistrationRequested> for PdsDataProductProjection {
    fn from_admitted_event(event: &DataProductRegistrationRequested) -> Self {
        Self {
            state: PdsDataProductState::from_registration_event(event),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdsLifecycleEventProjection {
    state: PdsLifecycleEventState,
}

impl PdsLifecycleEventProjection {
    pub(crate) fn from_admitted_event(event: &LifecycleEventRecordingRequested) -> Self {
        <Self as PdsProjectionFromEvent<LifecycleEventRecordingRequested>>::from_admitted_event(
            event,
        )
    }

    pub(crate) const fn state(&self) -> &PdsLifecycleEventState {
        &self.state
    }

    pub(crate) fn ledger_event(&self) -> PdsLedgerEventState {
        <Self as PdsProjection>::ledger_event(self)
    }
}

impl PdsProjection for PdsLifecycleEventProjection {
    fn ledger_event(&self) -> PdsLedgerEventState {
        PdsLedgerEventState::lifecycle_event_registered(&self.state)
    }
}

impl PdsProjectionFromEvent<LifecycleEventRecordingRequested> for PdsLifecycleEventProjection {
    fn from_admitted_event(event: &LifecycleEventRecordingRequested) -> Self {
        Self {
            state: PdsLifecycleEventState::from_recording_event(event),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdsRetentionPolicyProjection {
    state: PdsRetentionPolicyState,
}

impl PdsRetentionPolicyProjection {
    pub(crate) fn from_admitted_event(event: &RetentionPolicyRegistrationRequested) -> Self {
        <Self as PdsProjectionFromEvent<RetentionPolicyRegistrationRequested>>::from_admitted_event(
            event,
        )
    }

    pub(crate) const fn state(&self) -> &PdsRetentionPolicyState {
        &self.state
    }

    pub(crate) fn ledger_event(&self) -> PdsLedgerEventState {
        <Self as PdsProjection>::ledger_event(self)
    }
}

impl PdsProjection for PdsRetentionPolicyProjection {
    fn ledger_event(&self) -> PdsLedgerEventState {
        PdsLedgerEventState::retention_policy_registered(&self.state)
    }
}

impl PdsProjectionFromEvent<RetentionPolicyRegistrationRequested> for PdsRetentionPolicyProjection {
    fn from_admitted_event(event: &RetentionPolicyRegistrationRequested) -> Self {
        Self {
            state: PdsRetentionPolicyState::from_registration_event(event),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdsLifecycleCaseProjection {
    state: PdsLifecycleCaseState,
}

impl PdsLifecycleCaseProjection {
    pub(crate) fn from_admitted_event(event: &LifecycleCaseUpsertRequested) -> Self {
        Self {
            state: PdsLifecycleCaseState::from_upsert_event(event),
        }
    }

    pub(crate) const fn state(&self) -> &PdsLifecycleCaseState {
        &self.state
    }

    pub(crate) fn opened_ledger_event(&self) -> PdsLedgerEventState {
        PdsLedgerEventState::lifecycle_case_opened(&self.state)
    }

    pub(crate) fn transitioned_ledger_event(&self) -> PdsLedgerEventState {
        PdsLedgerEventState::lifecycle_case_transitioned(&self.state)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdsGovernanceProfileProjection {
    state: PdsGovernanceProfileState,
}

impl PdsGovernanceProfileProjection {
    pub(crate) fn from_admitted_event(event: &GovernanceProfileRegistrationRequested) -> Self {
        <Self as PdsProjectionFromEvent<GovernanceProfileRegistrationRequested>>::from_admitted_event(
            event,
        )
    }

    pub(crate) const fn state(&self) -> &PdsGovernanceProfileState {
        &self.state
    }

    pub(crate) fn ledger_event(&self) -> PdsLedgerEventState {
        <Self as PdsProjection>::ledger_event(self)
    }
}

impl PdsProjection for PdsGovernanceProfileProjection {
    fn ledger_event(&self) -> PdsLedgerEventState {
        PdsLedgerEventState::governance_profile_registered(&self.state)
    }
}

impl PdsProjectionFromEvent<GovernanceProfileRegistrationRequested>
    for PdsGovernanceProfileProjection
{
    fn from_admitted_event(event: &GovernanceProfileRegistrationRequested) -> Self {
        Self {
            state: PdsGovernanceProfileState::from_registration_event(event),
        }
    }
}

#[cfg(test)]
mod tests {
    use kiva_pds::{
        ConsentGrantCommandInput, ConsentScope, DataClass, DataProductRegistrationCommandInput,
        DataProductSourceKind, GovernanceProfileCommandInput, GovernanceProfileKind, LegalBasis,
        LifecycleCaseCommandInput, LifecycleCaseStatus, LifecycleCaseType,
        LifecycleEventCommandInput, LifecycleEventType, LifecycleResourceKind,
        RecordLifecycleEventCommand, RegisterConsentGrantCommand, RegisterDataProductCommand,
        RegisterGovernanceProfileCommand, RegisterRetentionPolicyCommand,
        RetentionPolicyCommandInput, TombstonePublicationRule, UpsertLifecycleCaseCommand,
    };
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::super::state::PdsLedgerEventKind;
    use super::{
        PdsConsentGrantProjection, PdsDataProductProjection, PdsGovernanceProfileProjection,
        PdsLifecycleCaseProjection, PdsLifecycleEventProjection, PdsProjection,
        PdsRetentionPolicyProjection,
    };

    fn assert_single_event_projection<P>(
        projection: &P,
        expected_kind: PdsLedgerEventKind,
        expected_owner: &str,
    ) where
        P: PdsProjection,
    {
        let ledger = projection.ledger_event();
        assert_eq!(ledger.event_kind, expected_kind);
        assert_eq!(ledger.owner_subject, expected_owner);
    }

    fn now() -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH
    }

    #[test]
    fn core_registration_projections_preserve_admitted_event_identity() {
        let consent_event = RegisterConsentGrantCommand::new(
            "learner-a",
            ConsentGrantCommandInput {
                grant_id: Uuid::from_u128(1),
                grantee_subject: "analyst-a".to_owned(),
                scope: ConsentScope::AggregateInsights,
                purpose: "district reporting".to_owned(),
                legal_basis: LegalBasis::Consent,
                expires_at: None,
                renewed_from_grant_id: None,
            },
            now(),
        )
        .consent_grant_registration_requested()
        .expect("valid consent grant event");
        let consent_projection = PdsConsentGrantProjection::from_admitted_event(&consent_event);
        let consent_ledger = consent_projection.ledger_event();
        assert_eq!(consent_projection.state().data_subject, "learner-a");
        assert_eq!(
            consent_projection.state().grant_id,
            consent_event.grant_id()
        );
        assert_eq!(
            consent_ledger.event_kind,
            PdsLedgerEventKind::ConsentGrantRegistered
        );
        assert_eq!(consent_ledger.owner_subject, "learner-a");
        assert_single_event_projection(
            &consent_projection,
            PdsLedgerEventKind::ConsentGrantRegistered,
            "learner-a",
        );

        let data_product_event = RegisterDataProductCommand::new(
            "learner-a",
            DataProductRegistrationCommandInput {
                data_product_id: Uuid::from_u128(2),
                source_kind: DataProductSourceKind::ThresholdRelease,
                release_id: "10".repeat(32),
                title: "Threshold release".to_owned(),
                purpose: "reporting".to_owned(),
                legal_basis: LegalBasis::PublicTask,
                data_classes: vec![DataClass::DataProduct],
            },
            now(),
        )
        .data_product_registration_requested()
        .expect("valid data product event");
        let data_product_projection =
            PdsDataProductProjection::from_admitted_event(&data_product_event);
        let data_product_ledger = data_product_projection.ledger_event();
        assert_eq!(data_product_projection.state().owner_subject, "learner-a");
        assert_eq!(
            data_product_projection.state().data_product_id,
            data_product_event.data_product_id()
        );
        assert_eq!(
            data_product_projection.state().release_artifact,
            data_product_event.release_artifact()
        );
        assert_eq!(
            data_product_ledger.event_kind,
            PdsLedgerEventKind::DataProductRegistered
        );
        assert_eq!(data_product_ledger.resource_kind, "data-product");

        let lifecycle_event = RecordLifecycleEventCommand::new(
            "learner-a",
            LifecycleEventCommandInput {
                event_id: Uuid::from_u128(3),
                event_type: LifecycleEventType::Exported,
                resource_kind: LifecycleResourceKind::MetadataExport,
                resource_id: "export-1".to_owned(),
                reason: "subject request".to_owned(),
            },
            now(),
        )
        .lifecycle_event_recording_requested()
        .expect("valid lifecycle event");
        let lifecycle_projection =
            PdsLifecycleEventProjection::from_admitted_event(&lifecycle_event);
        let lifecycle_ledger = lifecycle_projection.ledger_event();
        assert_eq!(lifecycle_projection.state().data_subject, "learner-a");
        assert_eq!(
            lifecycle_projection.state().event_id,
            lifecycle_event.event_id()
        );
        assert_eq!(
            lifecycle_ledger.event_kind,
            PdsLedgerEventKind::LifecycleEventRegistered
        );
        assert_eq!(lifecycle_ledger.resource_id, Uuid::from_u128(3).to_string());
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn policy_and_governance_projections_bind_state_to_ledger() {
        let retention_event = RegisterRetentionPolicyCommand::new(
            "school-a",
            RetentionPolicyCommandInput {
                policy_id: Uuid::from_u128(4),
                resource_kind: LifecycleResourceKind::XapiStatement,
                data_class: DataClass::Statement,
                retention_period_days: 365,
                review_interval_days: 90,
                tombstone_after_days: Some(365),
                tombstone_publication: TombstonePublicationRule::MetadataOnly,
                legal_basis: LegalBasis::PublicTask,
                description: "annual review".to_owned(),
            },
            now(),
        )
        .retention_policy_registration_requested()
        .expect("valid retention policy event");
        let retention_projection =
            PdsRetentionPolicyProjection::from_admitted_event(&retention_event);
        let retention_ledger = retention_projection.ledger_event();
        assert_eq!(retention_projection.state().owner_subject, "school-a");
        assert_eq!(
            retention_projection.state().policy_id,
            retention_event.policy_id()
        );
        assert_eq!(
            retention_ledger.event_kind,
            PdsLedgerEventKind::RetentionPolicyRegistered
        );
        assert_eq!(retention_ledger.owner_subject, "school-a");
        assert_single_event_projection(
            &retention_projection,
            PdsLedgerEventKind::RetentionPolicyRegistered,
            "school-a",
        );

        let case_event = UpsertLifecycleCaseCommand::new(
            "learner-a",
            LifecycleCaseCommandInput {
                case_id: Uuid::from_u128(5),
                case_type: LifecycleCaseType::Correction,
                resource_kind: LifecycleResourceKind::XapiStatement,
                resource_id: "statement-1".to_owned(),
                status: LifecycleCaseStatus::Open,
                retention_policy_id: Some(retention_event.policy_id()),
                summary: "review requested".to_owned(),
                resolution: None,
                tombstone_publication: None,
            },
            now(),
        )
        .lifecycle_case_upsert_requested()
        .expect("valid lifecycle case event");
        let case_projection = PdsLifecycleCaseProjection::from_admitted_event(&case_event);
        let opened_ledger = case_projection.opened_ledger_event();
        let transitioned_ledger = case_projection.transitioned_ledger_event();
        assert_eq!(case_projection.state().data_subject, "learner-a");
        assert_eq!(case_projection.state().case_id, case_event.case_id());
        assert_eq!(
            opened_ledger.event_kind,
            PdsLedgerEventKind::LifecycleCaseOpened
        );
        assert_eq!(
            transitioned_ledger.event_kind,
            PdsLedgerEventKind::LifecycleCaseTransitioned
        );
        assert_eq!(opened_ledger.occurred_at, case_event.updated_at());
        assert_eq!(transitioned_ledger.occurred_at, case_event.updated_at());

        let governance_event = RegisterGovernanceProfileCommand::new(
            "school-a",
            GovernanceProfileCommandInput {
                profile_id: Uuid::from_u128(6),
                profile_kind: GovernanceProfileKind::SchoolDistrict,
                tenant_partition: "district-a".to_owned(),
                subject_partition: "school-a".to_owned(),
                data_custodian_subject: "custodian".to_owned(),
                platform_operator_subject: "operator".to_owned(),
                policy_authority_subject: "authority".to_owned(),
                analyst_subject: "analyst".to_owned(),
                auditor_subject: "auditor".to_owned(),
                identity_authority_subject: Some("identity".to_owned()),
                emergency_access_policy: "break-glass".to_owned(),
                runbook_refs: vec!["runbook-a".to_owned()],
            },
            now(),
        )
        .governance_profile_registration_requested()
        .expect("valid governance event");
        let governance_projection =
            PdsGovernanceProfileProjection::from_admitted_event(&governance_event);
        let governance_ledger = governance_projection.ledger_event();
        assert_eq!(
            governance_projection.state().profile_owner_subject,
            "school-a"
        );
        assert_eq!(
            governance_projection.state().profile_id,
            governance_event.profile_id()
        );
        assert_eq!(
            governance_ledger.event_kind,
            PdsLedgerEventKind::GovernanceProfileRegistered
        );
        assert_eq!(governance_ledger.resource_kind, "governance-profile");
    }
}

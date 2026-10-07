//! Construction of PDS server-local state from admitted PDS domain events.

use kiva_pds::{
    ConsentGrantRegistrationRequested, ConsentState as PdsConsentState,
    DataProductRegistrationRequested, GovernanceProfileRegistrationRequested,
    LifecycleCaseUpsertRequested, LifecycleEventRecordingRequested,
    RetentionPolicyRegistrationRequested,
};

use super::state::{
    PdsConsentGrantState, PdsDataProductState, PdsGovernanceProfileState, PdsLifecycleCaseState,
    PdsLifecycleEventState, PdsRetentionPolicyState,
};

impl PdsConsentGrantState {
    pub(crate) fn from_registration_event(event: &ConsentGrantRegistrationRequested) -> Self {
        Self {
            grant_id: event.grant_id(),
            data_subject: event.data_subject().to_owned(),
            grantee_subject: event.grantee_subject().to_owned(),
            scope: event.scope(),
            state: PdsConsentState::Active,
            purpose: event.purpose().to_owned(),
            legal_basis: event.legal_basis(),
            created_at: event.created_at(),
            expires_at: event.expires_at(),
            revoked_at: None,
            renewed_from_grant_id: event.renewed_from_grant_id(),
        }
    }
}

impl PdsDataProductState {
    pub(crate) fn from_registration_event(event: &DataProductRegistrationRequested) -> Self {
        Self {
            data_product_id: event.data_product_id(),
            owner_subject: event.owner_subject().to_owned(),
            release_artifact: event.release_artifact(),
            title: event.title().to_owned(),
            purpose: event.purpose().to_owned(),
            legal_basis: event.legal_basis(),
            data_classes: event.data_classes().to_vec(),
            created_at: event.created_at(),
        }
    }
}

impl PdsLifecycleEventState {
    pub(crate) fn from_recording_event(event: &LifecycleEventRecordingRequested) -> Self {
        Self {
            event_id: event.event_id(),
            data_subject: event.data_subject().to_owned(),
            event_type: event.event_type(),
            resource_kind: event.resource_kind(),
            resource_id: event.resource_id().to_owned(),
            reason: event.reason().to_owned(),
            created_at: event.created_at(),
        }
    }
}

impl PdsRetentionPolicyState {
    pub(crate) fn from_registration_event(event: &RetentionPolicyRegistrationRequested) -> Self {
        Self {
            policy_id: event.policy_id(),
            owner_subject: event.owner_subject().to_owned(),
            resource_kind: event.resource_kind(),
            data_class: event.data_class(),
            retention_period_days: event.retention_period_days(),
            review_interval_days: event.review_interval_days(),
            tombstone_after_days: event.tombstone_after_days(),
            tombstone_publication: event.tombstone_publication(),
            legal_basis: event.legal_basis(),
            description: event.description().to_owned(),
            created_at: event.created_at(),
        }
    }
}

impl PdsLifecycleCaseState {
    pub(crate) fn from_upsert_event(event: &LifecycleCaseUpsertRequested) -> Self {
        Self {
            case_id: event.case_id(),
            data_subject: event.data_subject().to_owned(),
            case_type: event.case_type(),
            resource_kind: event.resource_kind(),
            resource_id: event.resource_id().to_owned(),
            status: event.status(),
            retention_policy_id: event.retention_policy_id(),
            summary: event.summary().to_owned(),
            resolution: event.resolution().map(str::to_owned),
            tombstone_publication: event.tombstone_publication(),
            created_at: event.updated_at(),
            updated_at: event.updated_at(),
        }
    }
}

impl PdsGovernanceProfileState {
    pub(crate) fn from_registration_event(event: &GovernanceProfileRegistrationRequested) -> Self {
        Self {
            profile_id: event.profile_id(),
            profile_owner_subject: event.profile_owner_subject().to_owned(),
            profile_kind: event.profile_kind(),
            tenant_partition: event.tenant_partition().to_owned(),
            subject_partition: event.subject_partition().to_owned(),
            data_custodian_subject: event.data_custodian_subject().to_owned(),
            platform_operator_subject: event.platform_operator_subject().to_owned(),
            policy_authority_subject: event.policy_authority_subject().to_owned(),
            analyst_subject: event.analyst_subject().to_owned(),
            auditor_subject: event.auditor_subject().to_owned(),
            identity_authority_subject: event.identity_authority_subject().map(str::to_owned),
            emergency_access_policy: event.emergency_access_policy().to_owned(),
            runbook_refs: event.runbook_refs().to_vec(),
            created_at: event.created_at(),
        }
    }
}

//! PDS storage key newtypes.

use uuid::Uuid;

use super::ledger::{PdsLedgerEventKind, PdsLedgerEventState};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PdsLedgerEventKey {
    pub(crate) event_kind: PdsLedgerEventKind,
    pub(crate) resource_id: String,
}

impl PdsLedgerEventKey {
    pub(crate) fn from_event(event: &PdsLedgerEventState) -> Self {
        Self {
            event_kind: event.event_kind,
            resource_id: event.resource_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PdsConsentGrantKey {
    pub(crate) grant_id: Uuid,
}

impl PdsConsentGrantKey {
    pub(crate) const fn from_grant_id(grant_id: Uuid) -> Self {
        Self { grant_id }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PdsDataProductKey {
    pub(crate) data_product_id: Uuid,
}

impl PdsDataProductKey {
    pub(crate) const fn from_data_product_id(data_product_id: Uuid) -> Self {
        Self { data_product_id }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PdsAccessAuditEventKey {
    pub(crate) event_id: Uuid,
}

impl PdsAccessAuditEventKey {
    pub(crate) const fn from_event_id(event_id: Uuid) -> Self {
        Self { event_id }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PdsLifecycleEventKey {
    pub(crate) event_id: Uuid,
}

impl PdsLifecycleEventKey {
    pub(crate) const fn from_event_id(event_id: Uuid) -> Self {
        Self { event_id }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PdsRetentionPolicyKey {
    pub(crate) policy_id: Uuid,
}

impl PdsRetentionPolicyKey {
    pub(crate) const fn from_policy_id(policy_id: Uuid) -> Self {
        Self { policy_id }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PdsLifecycleCaseKey {
    pub(crate) case_id: Uuid,
}

impl PdsLifecycleCaseKey {
    pub(crate) const fn from_case_id(case_id: Uuid) -> Self {
        Self { case_id }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PdsGovernanceProfileKey {
    pub(crate) profile_id: Uuid,
}

impl PdsGovernanceProfileKey {
    pub(crate) const fn from_profile_id(profile_id: Uuid) -> Self {
        Self { profile_id }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PdsProviderBindingKey {
    pub(crate) binding_id: Uuid,
}

impl PdsProviderBindingKey {
    pub(crate) const fn from_binding_id(binding_id: Uuid) -> Self {
        Self { binding_id }
    }
}

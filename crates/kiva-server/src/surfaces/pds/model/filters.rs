//! Query filters for PDS server-local repositories.

use std::collections::BTreeMap;

use uuid::Uuid;

use crate::core::{request_util::optional_uuid_query, response::HttpError};

use super::state::{
    PdsAccessAuditEventState, PdsConsentGrantState, PdsDataProductState, PdsGovernanceProfileState,
    PdsLifecycleCaseState, PdsLifecycleEventState, PdsProviderBindingState,
    PdsRetentionPolicyState,
};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PdsConsentGrantFilter {
    pub(crate) grant_id: Option<Uuid>,
}

impl PdsConsentGrantFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        let grant_id = optional_uuid_query(query, "grantId")?;
        Ok(Self { grant_id })
    }

    pub(crate) fn matches(self, grant: &PdsConsentGrantState) -> bool {
        self.grant_id
            .is_none_or(|grant_id| grant.grant_id == grant_id)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PdsDataProductFilter {
    pub(crate) data_product_id: Option<Uuid>,
}

impl PdsDataProductFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        let data_product_id = optional_uuid_query(query, "dataProductId")?;
        Ok(Self { data_product_id })
    }

    pub(crate) fn matches(self, product: &PdsDataProductState) -> bool {
        self.data_product_id
            .is_none_or(|data_product_id| product.data_product_id == data_product_id)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PdsProviderBindingFilter {
    pub(crate) binding_id: Option<Uuid>,
}

impl PdsProviderBindingFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        let binding_id = optional_uuid_query(query, "bindingId")?;
        Ok(Self { binding_id })
    }

    pub(crate) fn matches(self, binding: &PdsProviderBindingState) -> bool {
        self.binding_id
            .is_none_or(|binding_id| binding.binding_id == binding_id)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PdsAccessAuditEventFilter {
    pub(crate) event_id: Option<Uuid>,
}

impl PdsAccessAuditEventFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        let event_id = optional_uuid_query(query, "eventId")?;
        Ok(Self { event_id })
    }

    pub(crate) fn matches(self, event: &PdsAccessAuditEventState) -> bool {
        self.event_id
            .is_none_or(|event_id| event.event_id == event_id)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PdsLifecycleEventFilter {
    pub(crate) event_id: Option<Uuid>,
}

impl PdsLifecycleEventFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        let event_id = optional_uuid_query(query, "eventId")?;
        Ok(Self { event_id })
    }

    pub(crate) fn matches(self, event: &PdsLifecycleEventState) -> bool {
        self.event_id
            .is_none_or(|event_id| event.event_id == event_id)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PdsRetentionPolicyFilter {
    pub(crate) policy_id: Option<Uuid>,
}

impl PdsRetentionPolicyFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        let policy_id = optional_uuid_query(query, "policyId")?;
        Ok(Self { policy_id })
    }

    pub(crate) fn matches(self, policy: &PdsRetentionPolicyState) -> bool {
        self.policy_id
            .is_none_or(|policy_id| policy.policy_id == policy_id)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PdsLifecycleCaseFilter {
    pub(crate) case_id: Option<Uuid>,
}

impl PdsLifecycleCaseFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        let case_id = optional_uuid_query(query, "caseId")?;
        Ok(Self { case_id })
    }

    pub(crate) fn matches(self, case: &PdsLifecycleCaseState) -> bool {
        self.case_id.is_none_or(|case_id| case.case_id == case_id)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PdsGovernanceProfileFilter {
    pub(crate) profile_id: Option<Uuid>,
}

impl PdsGovernanceProfileFilter {
    pub(crate) fn from_query(query: &BTreeMap<String, String>) -> Result<Self, HttpError> {
        let profile_id = optional_uuid_query(query, "profileId")?;
        Ok(Self { profile_id })
    }

    pub(crate) fn matches(self, profile: &PdsGovernanceProfileState) -> bool {
        self.profile_id
            .is_none_or(|profile_id| profile.profile_id == profile_id)
    }
}

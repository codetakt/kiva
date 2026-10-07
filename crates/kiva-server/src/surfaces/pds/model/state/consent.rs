//! PDS consent-grant server-local state.

use kiva_pds::{ConsentScope as PdsConsentScope, ConsentState as PdsConsentState};
use kiva_policy::policy::{ConsentDelegation, DelegationState};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::surfaces::pds::model::policy_adapter::{
    pds_consent_scope_delegation_scope, pds_consent_state_delegation_state,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdsConsentGrantState {
    pub(crate) grant_id: Uuid,
    pub(crate) data_subject: String,
    pub(crate) grantee_subject: String,
    pub(crate) scope: PdsConsentScope,
    pub(crate) state: PdsConsentState,
    pub(crate) purpose: String,
    pub(crate) legal_basis: kiva_pds::LegalBasis,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) expires_at: Option<OffsetDateTime>,
    pub(crate) revoked_at: Option<OffsetDateTime>,
    pub(crate) renewed_from_grant_id: Option<Uuid>,
}

impl PdsConsentGrantState {
    pub(crate) fn same_active_registration(&self, other: &Self) -> bool {
        self.grant_id == other.grant_id
            && self.data_subject == other.data_subject
            && self.grantee_subject == other.grantee_subject
            && self.scope == other.scope
            && self.purpose == other.purpose
            && self.legal_basis == other.legal_basis
            && self.expires_at == other.expires_at
            && self.renewed_from_grant_id == other.renewed_from_grant_id
            && self.state == PdsConsentState::Active
            && other.state == PdsConsentState::Active
            && self.revoked_at.is_none()
            && other.revoked_at.is_none()
    }

    pub(crate) fn consent_delegation_at(&self, now: OffsetDateTime) -> ConsentDelegation {
        let state = if self.state == PdsConsentState::Active
            && self.expires_at.is_some_and(|expires_at| expires_at <= now)
        {
            DelegationState::Expired
        } else {
            pds_consent_state_delegation_state(self.state)
        };
        ConsentDelegation::new(state, pds_consent_scope_delegation_scope(self.scope))
    }
}

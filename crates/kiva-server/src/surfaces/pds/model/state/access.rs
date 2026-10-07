//! PDS access-audit server-local state.

use kiva_policy::policy::{Action, Decision, PrincipalRole, ResourceClass};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdsAccessAuditEventState {
    pub(crate) event_id: Uuid,
    pub(crate) requester_subject: String,
    pub(crate) role: PrincipalRole,
    pub(crate) resource: ResourceClass,
    pub(crate) action: Action,
    pub(crate) grant_id: Option<Uuid>,
    pub(crate) data_product_id: Option<Uuid>,
    pub(crate) decision: Decision,
    pub(crate) created_at: OffsetDateTime,
}

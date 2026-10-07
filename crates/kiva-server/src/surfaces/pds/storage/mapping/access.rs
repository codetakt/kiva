#![allow(clippy::wildcard_imports)]

use super::*;

pub(in crate::surfaces::pds::storage) fn pds_access_audit_event_from_row(
    (
        event_id,
        requester_subject,
        role,
        resource,
        action,
        grant_id,
        data_product_id,
        decision,
        deny_reason,
        created_at,
    ): PdsAccessAuditEventRow,
) -> Result<PdsAccessAuditEventState, PdsRepositoryError> {
    if requester_subject.is_empty() {
        return Err(PdsRepositoryError::InvalidStoredData);
    }
    let decision = pds_decision_from_labels(&decision, deny_reason.as_deref())?;
    Ok(PdsAccessAuditEventState {
        event_id,
        requester_subject,
        role: pds_principal_role_from_label(&role).ok_or(PdsRepositoryError::InvalidStoredData)?,
        resource: pds_resource_class_from_label(&resource)
            .ok_or(PdsRepositoryError::InvalidStoredData)?,
        action: pds_action_from_label(&action).ok_or(PdsRepositoryError::InvalidStoredData)?,
        grant_id,
        data_product_id,
        decision,
        created_at,
    })
}

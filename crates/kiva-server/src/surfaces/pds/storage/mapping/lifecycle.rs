#![allow(clippy::wildcard_imports)]

use super::*;

pub(in crate::surfaces::pds::storage) fn pds_lifecycle_event_from_row(
    (
        event_id,
        data_subject,
        event_type,
        resource_kind,
        resource_id,
        reason,
        created_at,
    ): PdsLifecycleEventRow,
) -> Result<PdsLifecycleEventState, PdsRepositoryError> {
    if data_subject.is_empty() || resource_id.is_empty() || reason.is_empty() {
        return Err(PdsRepositoryError::InvalidStoredData);
    }
    Ok(PdsLifecycleEventState {
        event_id,
        data_subject,
        event_type: PdsLifecycleEventType::from_label(&event_type)
            .ok_or(PdsRepositoryError::InvalidStoredData)?,
        resource_kind: PdsLifecycleResourceKind::from_label(&resource_kind)
            .ok_or(PdsRepositoryError::InvalidStoredData)?,
        resource_id,
        reason,
        created_at,
    })
}

pub(in crate::surfaces::pds::storage) fn pds_retention_policy_from_row(
    (
        policy_id,
        owner_subject,
        resource_kind,
        data_class,
        retention_period_days,
        review_interval_days,
        tombstone_after_days,
        tombstone_publication,
        legal_basis,
        description,
        created_at,
    ): PdsRetentionPolicyRow,
) -> Result<PdsRetentionPolicyState, PdsRepositoryError> {
    if owner_subject.is_empty() || description.is_empty() {
        return Err(PdsRepositoryError::InvalidStoredData);
    }
    let retention_period_days = pds_i32_days_to_u32(retention_period_days)?;
    let review_interval_days = pds_i32_days_to_u32(review_interval_days)?;
    let tombstone_after_days = tombstone_after_days.map(pds_i32_days_to_u32).transpose()?;
    if retention_period_days == 0
        || review_interval_days == 0
        || tombstone_after_days.is_some_and(|days| days < retention_period_days)
    {
        return Err(PdsRepositoryError::InvalidStoredData);
    }
    Ok(PdsRetentionPolicyState {
        policy_id,
        owner_subject,
        resource_kind: PdsLifecycleResourceKind::from_label(&resource_kind)
            .ok_or(PdsRepositoryError::InvalidStoredData)?,
        data_class: PdsDataClass::from_label(&data_class)
            .ok_or(PdsRepositoryError::InvalidStoredData)?,
        retention_period_days,
        review_interval_days,
        tombstone_after_days,
        tombstone_publication: PdsTombstonePublicationRule::from_label(&tombstone_publication)
            .ok_or(PdsRepositoryError::InvalidStoredData)?,
        legal_basis: PdsLegalBasis::from_label(&legal_basis)
            .ok_or(PdsRepositoryError::InvalidStoredData)?,
        description,
        created_at,
    })
}

pub(in crate::surfaces::pds::storage) fn pds_lifecycle_case_from_row(
    (
        case_id,
        data_subject,
        case_type,
        resource_kind,
        resource_id,
        status,
        retention_policy_id,
        summary,
        resolution,
        tombstone_publication,
        created_at,
        updated_at,
    ): PdsLifecycleCaseRow,
) -> Result<PdsLifecycleCaseState, PdsRepositoryError> {
    let case_type = PdsLifecycleCaseType::from_label(&case_type)
        .ok_or(PdsRepositoryError::InvalidStoredData)?;
    let status =
        PdsLifecycleCaseStatus::from_label(&status).ok_or(PdsRepositoryError::InvalidStoredData)?;
    let tombstone_publication = tombstone_publication
        .as_deref()
        .map(|value| {
            PdsTombstonePublicationRule::from_label(value)
                .ok_or(PdsRepositoryError::InvalidStoredData)
        })
        .transpose()?;
    if data_subject.is_empty()
        || resource_id.is_empty()
        || summary.is_empty()
        || resolution.as_ref().is_some_and(String::is_empty)
        || (status.requires_resolution() && resolution.is_none())
        || (status == PdsLifecycleCaseStatus::Tombstoned
            && case_type != PdsLifecycleCaseType::Tombstone)
        || (case_type == PdsLifecycleCaseType::Tombstone && tombstone_publication.is_none())
        || updated_at < created_at
    {
        return Err(PdsRepositoryError::InvalidStoredData);
    }
    Ok(PdsLifecycleCaseState {
        case_id,
        data_subject,
        case_type,
        resource_kind: PdsLifecycleResourceKind::from_label(&resource_kind)
            .ok_or(PdsRepositoryError::InvalidStoredData)?,
        resource_id,
        status,
        retention_policy_id,
        summary,
        resolution,
        tombstone_publication,
        created_at,
        updated_at,
    })
}

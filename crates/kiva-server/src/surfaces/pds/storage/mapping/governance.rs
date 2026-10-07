#![allow(clippy::wildcard_imports)]

use super::*;

pub(in crate::surfaces::pds::storage) fn pds_governance_profile_from_row(
    (
        profile_id,
        profile_owner_subject,
        profile_kind,
        tenant_partition,
        subject_partition,
        data_custodian_subject,
        platform_operator_subject,
        policy_authority_subject,
        analyst_subject,
        auditor_subject,
        identity_authority_subject,
        emergency_access_policy,
        runbook_refs,
        created_at,
    ): PdsGovernanceProfileRow,
) -> Result<PdsGovernanceProfileState, PdsRepositoryError> {
    let profile_kind = PdsGovernanceProfileKind::from_label(&profile_kind)
        .ok_or(PdsRepositoryError::InvalidStoredData)?;
    if profile_owner_subject.is_empty()
        || tenant_partition.is_empty()
        || subject_partition.is_empty()
        || data_custodian_subject.is_empty()
        || platform_operator_subject.is_empty()
        || policy_authority_subject.is_empty()
        || analyst_subject.is_empty()
        || auditor_subject.is_empty()
        || identity_authority_subject
            .as_ref()
            .is_some_and(String::is_empty)
        || emergency_access_policy.is_empty()
        || runbook_refs.is_empty()
        || runbook_refs.iter().any(String::is_empty)
    {
        return Err(PdsRepositoryError::InvalidStoredData);
    }
    let role_subjects = [
        data_custodian_subject.as_str(),
        platform_operator_subject.as_str(),
        policy_authority_subject.as_str(),
        analyst_subject.as_str(),
        auditor_subject.as_str(),
    ];
    let distinct_role_subjects =
        role_subjects.iter().copied().collect::<BTreeSet<_>>().len() == role_subjects.len();
    let identity_authority_is_absent_or_distinct = identity_authority_subject
        .as_ref()
        .is_none_or(|subject| !role_subjects.contains(&subject.as_str()));
    if !pds_governance_profile_metadata_is_admissible(PdsGovernanceProfileMetadataFacts {
        tenant_partition: &tenant_partition,
        subject_partition: &subject_partition,
        required_role_subjects_declared: PdsEvidenceFlag::Present,
        distinct_role_subjects: PdsEvidenceFlag::from_bool(distinct_role_subjects),
        identity_authority_absent_or_distinct: PdsEvidenceFlag::from_bool(
            identity_authority_is_absent_or_distinct,
        ),
        emergency_access_policy: &emergency_access_policy,
        runbook_refs_present: PdsEvidenceFlag::from_bool(!runbook_refs.is_empty()),
    }) {
        return Err(PdsRepositoryError::InvalidStoredData);
    }
    Ok(PdsGovernanceProfileState {
        profile_id,
        profile_owner_subject,
        profile_kind,
        tenant_partition,
        subject_partition,
        data_custodian_subject,
        platform_operator_subject,
        policy_authority_subject,
        analyst_subject,
        auditor_subject,
        identity_authority_subject,
        emergency_access_policy,
        runbook_refs,
        created_at,
    })
}

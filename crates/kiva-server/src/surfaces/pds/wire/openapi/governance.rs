//! OpenAPI-to-PDS conversion for governance request DTOs.

use kiva_pds::GovernanceProfileCommandInput as PdsGovernanceProfileCommandInput;
use kiva_pds_openapi_contract as pds_openapi;

use super::common::governance_profile_kind_from_openapi;

pub(crate) fn governance_profile_command_input_from_openapi(
    request: pds_openapi::GovernanceProfileRequest,
) -> PdsGovernanceProfileCommandInput {
    PdsGovernanceProfileCommandInput {
        profile_id: request.profile_id,
        profile_kind: governance_profile_kind_from_openapi(request.profile_kind),
        tenant_partition: request.tenant_partition,
        subject_partition: request.subject_partition,
        data_custodian_subject: request.data_custodian_subject,
        platform_operator_subject: request.platform_operator_subject,
        policy_authority_subject: request.policy_authority_subject,
        analyst_subject: request.analyst_subject,
        auditor_subject: request.auditor_subject,
        identity_authority_subject: request.identity_authority_subject,
        emergency_access_policy: request.emergency_access_policy,
        runbook_refs: request.runbook_refs,
    }
}

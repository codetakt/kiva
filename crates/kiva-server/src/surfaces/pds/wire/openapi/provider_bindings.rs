//! OpenAPI-to-PDS conversion for provider-binding request DTOs.

use kiva_pds::ProviderBindingRegistrationCommandInput as PdsProviderBindingRegistrationCommandInput;
use kiva_pds_openapi_contract as pds_openapi;

pub(crate) fn provider_binding_registration_command_input_from_openapi(
    request: pds_openapi::ProviderBindingRequest,
) -> PdsProviderBindingRegistrationCommandInput {
    PdsProviderBindingRegistrationCommandInput {
        binding_id: request.binding_id,
        mechanism: request.mechanism,
        provider_kind: request.provider_kind,
        capability: request.capability,
        profile: request.profile,
        provider_id_digest: request.provider_id_digest,
        artifact_digest: request.artifact_digest,
        parameter_set_digest: request.parameter_set_digest,
        review_evidence_digest: request.review_evidence_digest,
        production_claim_requested: request.production_claim_requested,
    }
}

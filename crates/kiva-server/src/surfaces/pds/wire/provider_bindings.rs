//! Secret/confidential-compute provider-binding PDS wire projection.

use kiva_pds::{
    ProviderBinding as ProviderBindingDto, ProviderBindingList as ProviderBindingListDto,
};
use serde_json::Value;

use crate::{core::response::rfc3339, surfaces::pds::model::PdsProviderBindingState};

use super::common::dto_value;

pub(crate) fn provider_binding_list_response(
    bindings: &[PdsProviderBindingState],
) -> Result<Value, serde_json::Error> {
    dto_value(&ProviderBindingListDto {
        provider_bindings: bindings.iter().map(provider_binding_dto).collect(),
    })
}

fn provider_binding_dto(binding: &PdsProviderBindingState) -> ProviderBindingDto {
    ProviderBindingDto {
        binding_id: binding.binding_id.to_string(),
        owner_subject: binding.owner_subject.clone(),
        mechanism: binding.mechanism.clone(),
        provider_kind: binding.provider_kind.clone(),
        capability: binding.capability.clone(),
        profile: binding.profile.clone(),
        provider_id_digest: binding.provider_id_digest.clone(),
        artifact_digest: binding.artifact_digest.clone(),
        parameter_set_digest: binding.parameter_set_digest.clone(),
        review_evidence_digest: binding.review_evidence_digest.clone(),
        production_claim_requested: binding.production_claim_requested,
        binding_digest: binding.binding_digest.clone(),
        evidence_gate: binding.evidence_gate.clone(),
        release_gate: binding.release_gate.clone(),
        runtime_class: binding.runtime_class.clone(),
        production_ready: binding.production_ready,
        created_at: rfc3339(binding.created_at),
    }
}

#![allow(clippy::wildcard_imports)]

use super::*;

pub(in crate::surfaces::pds::storage) fn pds_provider_binding_from_row(
    row: PdsProviderBindingRow,
) -> Result<PdsProviderBindingState, PdsRepositoryError> {
    if row.owner_subject.is_empty()
        || row.mechanism.is_empty()
        || row.provider_kind.is_empty()
        || row.capability.is_empty()
        || row.profile.is_empty()
        || row.evidence_gate.is_empty()
        || row.release_gate.is_empty()
        || row.runtime_class.is_empty()
        || !pds_hex32_is_valid(&row.provider_id_digest)
        || !pds_hex32_is_valid(&row.artifact_digest)
        || !pds_hex32_is_valid(&row.binding_digest)
        || row
            .parameter_set_digest
            .as_ref()
            .is_some_and(|digest| !pds_hex32_is_valid(digest))
        || row
            .review_evidence_digest
            .as_ref()
            .is_some_and(|digest| !pds_hex32_is_valid(digest))
    {
        return Err(PdsRepositoryError::InvalidStoredData);
    }
    Ok(PdsProviderBindingState {
        binding_id: row.binding_id,
        owner_subject: row.owner_subject,
        mechanism: row.mechanism,
        provider_kind: row.provider_kind,
        capability: row.capability,
        profile: row.profile,
        provider_id_digest: row.provider_id_digest,
        artifact_digest: row.artifact_digest,
        parameter_set_digest: row.parameter_set_digest,
        review_evidence_digest: row.review_evidence_digest,
        production_claim_requested: row.production_claim_requested,
        binding_digest: row.binding_digest,
        evidence_gate: row.evidence_gate,
        release_gate: row.release_gate,
        runtime_class: row.runtime_class,
        production_ready: row.production_ready,
        created_at: row.created_at,
    })
}

fn pds_hex32_is_valid(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

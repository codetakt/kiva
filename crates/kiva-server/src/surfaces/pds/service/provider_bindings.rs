//! Provider-binding admission helpers for the PDS service boundary.

use kiva_confidential::{
    mechanism::{
        provider::{SecretComputeProviderBinding, admit_secret_compute_provider_runtime_boundary},
        workflow::ConfidentialMechanismKind,
    },
    primitive::SecureAggregateCommitment,
};
use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::ProviderBindingRegistrationCommandInput as PdsProviderBindingRegistrationCommandInput;
use kiva_secret_provider_contract::{
    SecretComputeProviderCapability, SecretComputeProviderKind, SecretComputeProviderProfile,
};
use time::OffsetDateTime;

use crate::surfaces::pds::{model::PdsProviderBindingState, outcome::PdsRepositoryError};

pub(crate) fn admit_provider_binding_registration(
    principal: &AuthenticatedPrincipal,
    request: PdsProviderBindingRegistrationCommandInput,
    created_at: OffsetDateTime,
) -> Result<PdsProviderBindingState, PdsRepositoryError> {
    let mechanism = ConfidentialMechanismKind::from_label(&request.mechanism)
        .ok_or_else(|| unsupported_provider_binding_label("mechanism"))?;
    let kind = SecretComputeProviderKind::from_label(&request.provider_kind)
        .ok_or_else(|| unsupported_provider_binding_label("provider_kind"))?;
    let capability = SecretComputeProviderCapability::from_label(&request.capability)
        .ok_or_else(|| unsupported_provider_binding_label("capability"))?;
    let profile = SecretComputeProviderProfile::from_label(&request.profile)
        .ok_or_else(|| unsupported_provider_binding_label("profile"))?;
    let provider_id_digest =
        secure_commitment_from_hex(&request.provider_id_digest, "provider_id_digest")?;
    let artifact_digest = secure_commitment_from_hex(&request.artifact_digest, "artifact_digest")?;
    let parameter_set_digest = request
        .parameter_set_digest
        .as_deref()
        .map(|digest| secure_commitment_from_hex(digest, "parameter_set_digest"))
        .transpose()?;
    let review_evidence_digest = request
        .review_evidence_digest
        .as_deref()
        .map(|digest| secure_commitment_from_hex(digest, "review_evidence_digest"))
        .transpose()?;
    let binding = SecretComputeProviderBinding::new(
        kind,
        capability,
        profile,
        provider_id_digest,
        artifact_digest,
        parameter_set_digest,
        review_evidence_digest,
        review_evidence_digest.is_some(),
        request.production_claim_requested,
    )
    .map_err(provider_admission_error)?;
    let runtime_boundary = admit_secret_compute_provider_runtime_boundary(binding, mechanism)
        .map_err(provider_admission_error)?;
    let admission = runtime_boundary.admission();
    Ok(PdsProviderBindingState {
        binding_id: request.binding_id,
        owner_subject: principal.subject.clone(),
        mechanism: request.mechanism,
        provider_kind: admission.binding().kind().label().to_owned(),
        capability: admission.binding().capability().label().to_owned(),
        profile: admission.binding().profile().label().to_owned(),
        provider_id_digest: commitment_hex(admission.binding().provider_id_digest()),
        artifact_digest: commitment_hex(admission.binding().artifact_digest()),
        parameter_set_digest: admission
            .binding()
            .parameter_set_digest()
            .map(commitment_hex),
        review_evidence_digest: admission
            .binding()
            .review_evidence_digest()
            .map(commitment_hex),
        production_claim_requested: admission.binding().production_claim_requested(),
        binding_digest: commitment_hex(admission.binding_digest()),
        evidence_gate: admission.evidence_gate().label().to_owned(),
        release_gate: admission.release_gate().label().to_owned(),
        runtime_class: runtime_boundary.runtime_class().label().to_owned(),
        production_ready: runtime_boundary.production_ready(),
        created_at,
    })
}

fn provider_admission_error(error: impl std::fmt::Display) -> PdsRepositoryError {
    PdsRepositoryError::InvalidRequest(format!(
        "PDS provider binding failed confidential provider admission: {error}"
    ))
}

fn unsupported_provider_binding_label(field: &str) -> PdsRepositoryError {
    PdsRepositoryError::InvalidRequest(format!("PDS provider binding {field} is unsupported"))
}

fn secure_commitment_from_hex(
    value: &str,
    field: &str,
) -> Result<SecureAggregateCommitment, PdsRepositoryError> {
    let bytes = hex32(value).ok_or_else(|| {
        PdsRepositoryError::InvalidRequest(format!("PDS provider binding {field} must be Hex32"))
    })?;
    SecureAggregateCommitment::new(bytes).map_err(provider_admission_error)
}

fn hex32(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut bytes = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(chunk[0])?;
        let low = hex_nibble(chunk[1])?;
        bytes[index] = (high << 4) | low;
    }
    Some(bytes)
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn commitment_hex(commitment: SecureAggregateCommitment) -> String {
    const LOWER_HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(64);
    for byte in commitment.bytes() {
        value.push(char::from(LOWER_HEX[(byte >> 4) as usize]));
        value.push(char::from(LOWER_HEX[(byte & 0x0f) as usize]));
    }
    value
}

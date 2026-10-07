//! Shared `OpenAPI` request conversion helpers.

use axum::http::StatusCode;
use kiva_confidential::{
    mechanism::{
        additive::SecureAggregateError, job::ConfidentialJobStatus as DomainConfidentialJobStatus,
        shamir::ShamirShareOpeningNonce,
        workflow::ConfidentialParticipantStatus as DomainConfidentialParticipantStatus,
    },
    primitive::{
        SecureAggregateCommitment, SecureAggregateFieldElement, SecureAggregateSignature,
        SecureAggregateSigningKeyId, SecureAggregateSigningPublicKey, SecureAggregateTranscriptId,
        SecureAggregateTransportProof,
    },
};
use kiva_confidential_contract as contract;

use crate::core::response::HttpError;

pub(super) const fn confidential_job_status(
    status: contract::ConfidentialJobStatus,
) -> DomainConfidentialJobStatus {
    match status {
        contract::ConfidentialJobStatus::Draft => DomainConfidentialJobStatus::Draft,
        contract::ConfidentialJobStatus::Admitted => DomainConfidentialJobStatus::Admitted,
        contract::ConfidentialJobStatus::Running => DomainConfidentialJobStatus::Running,
        contract::ConfidentialJobStatus::AwaitingRelease => {
            DomainConfidentialJobStatus::AwaitingRelease
        }
        contract::ConfidentialJobStatus::Released => DomainConfidentialJobStatus::Released,
        contract::ConfidentialJobStatus::Failed => DomainConfidentialJobStatus::Failed,
        contract::ConfidentialJobStatus::Cancelled => DomainConfidentialJobStatus::Cancelled,
        contract::ConfidentialJobStatus::Expired => DomainConfidentialJobStatus::Expired,
        contract::ConfidentialJobStatus::Retired => DomainConfidentialJobStatus::Retired,
    }
}

pub(super) const fn confidential_participant_status(
    status: contract::ConfidentialParticipantStatus,
) -> DomainConfidentialParticipantStatus {
    match status {
        contract::ConfidentialParticipantStatus::Invited => {
            DomainConfidentialParticipantStatus::Invited
        }
        contract::ConfidentialParticipantStatus::Active => {
            DomainConfidentialParticipantStatus::Active
        }
        contract::ConfidentialParticipantStatus::Suspended => {
            DomainConfidentialParticipantStatus::Suspended
        }
        contract::ConfidentialParticipantStatus::Frozen => {
            DomainConfidentialParticipantStatus::Frozen
        }
        contract::ConfidentialParticipantStatus::Removed => {
            DomainConfidentialParticipantStatus::Removed
        }
    }
}

pub(super) fn transcript_id(
    value: contract::ByteArray32,
    name: &'static str,
) -> Result<SecureAggregateTranscriptId, HttpError> {
    SecureAggregateTranscriptId::new(fixed_byte_array(value, name)?)
        .map_err(|error| invalid_secure_value(name, error))
}

pub(super) fn commitment(
    value: contract::ByteArray32,
    name: &'static str,
) -> Result<SecureAggregateCommitment, HttpError> {
    SecureAggregateCommitment::new(fixed_byte_array(value, name)?)
        .map_err(|error| invalid_secure_value(name, error))
}

pub(super) fn signing_key_id(
    value: contract::ByteArray32,
    name: &'static str,
) -> Result<SecureAggregateSigningKeyId, HttpError> {
    SecureAggregateSigningKeyId::new(fixed_byte_array(value, name)?)
        .map_err(|error| invalid_secure_value(name, error))
}

pub(super) fn signing_public_key(
    value: contract::ByteArray32,
    name: &'static str,
) -> Result<SecureAggregateSigningPublicKey, HttpError> {
    SecureAggregateSigningPublicKey::new(fixed_byte_array(value, name)?)
        .map_err(|error| invalid_secure_value(name, error))
}

pub(super) fn transport_proof(
    value: contract::ByteArray32,
    name: &'static str,
) -> Result<SecureAggregateTransportProof, HttpError> {
    SecureAggregateTransportProof::new(fixed_byte_array(value, name)?)
        .map_err(|error| invalid_secure_value(name, error))
}

pub(super) fn shamir_opening_nonce(
    value: contract::ByteArray32,
    name: &'static str,
) -> Result<ShamirShareOpeningNonce, HttpError> {
    ShamirShareOpeningNonce::new(fixed_byte_array(value, name)?)
        .map_err(|error| invalid_secure_value(name, error))
}

pub(super) fn signature(
    value: contract::ByteArray64,
    name: &'static str,
) -> Result<SecureAggregateSignature, HttpError> {
    SecureAggregateSignature::new(fixed_byte_array(value, name)?)
        .map_err(|error| invalid_secure_value(name, error))
}

pub(super) fn field_element(
    value: u64,
    name: &'static str,
) -> Result<SecureAggregateFieldElement, HttpError> {
    SecureAggregateFieldElement::new(value).map_err(|error| invalid_secure_value(name, error))
}

fn fixed_byte_array<const N: usize>(
    value: Vec<u8>,
    name: &'static str,
) -> Result<[u8; N], HttpError> {
    let actual = value.len();
    value.try_into().map_err(|_| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            format!("confidential {name} must contain exactly {N} bytes; received {actual}"),
        )
    })
}

fn invalid_secure_value(name: &'static str, error: SecureAggregateError) -> HttpError {
    HttpError::new(
        StatusCode::BAD_REQUEST,
        format!("confidential {name} is invalid: {error}"),
    )
}

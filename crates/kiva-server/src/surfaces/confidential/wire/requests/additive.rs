//! Additive `OpenAPI` request DTO conversion.

use kiva_confidential::{
    mechanism::additive::{
        AdditiveShareCommitmentSlot, AdditiveShareReplayWindow, AuthenticatedAdditiveShareEnvelope,
        SecureAggregateShare, SignedAuthenticatedAdditiveShareEnvelope,
    },
    primitive::SecureAggregateEnvelopeSequence,
};
use kiva_confidential_contract as contract;

use crate::{core::response::HttpError, surfaces::confidential::model as domain};

use super::common::{
    commitment, field_element, signature, signing_key_id, signing_public_key, transcript_id,
    transport_proof,
};

pub(crate) fn additive_aggregate_request_from_openapi(
    request: contract::AdditiveAggregateRequest,
) -> Result<domain::ConfidentialAdditiveAggregateRequest, HttpError> {
    Ok(domain::ConfidentialAdditiveAggregateRequest {
        participant_count: request.participant_count,
        transcript_id: transcript_id(request.transcript_id, "transcript_id")?,
        slots: request
            .slots
            .into_iter()
            .map(additive_share_commitment_slot)
            .collect::<Result<Vec<_>, _>>()?,
        replay_window: additive_share_replay_window(&request.replay_window)?,
        signed_envelopes: request
            .signed_envelopes
            .into_iter()
            .map(signed_authenticated_additive_share_envelope)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub(crate) fn share_submission_request_from_openapi(
    request: contract::ShareSubmission,
) -> Result<domain::ConfidentialShareSubmissionRequest, HttpError> {
    Ok(domain::ConfidentialShareSubmissionRequest {
        signed_envelope: signed_authenticated_additive_share_envelope(request.signed_envelope)?,
    })
}

pub(crate) fn share_signing_key_request_from_openapi(
    request: contract::ShareSigningKeyRegistration,
) -> Result<domain::ConfidentialShareSigningKeyRequest, HttpError> {
    Ok(domain::ConfidentialShareSigningKeyRequest {
        signing_key_id: signing_key_id(request.signing_key_id, "signing_key_id")?,
        participant_index: request.participant_index,
        public_key: signing_public_key(request.public_key, "public_key")?,
    })
}

pub(crate) fn share_signing_key_rotation_request_from_openapi(
    request: contract::ShareSigningKeyRotation,
) -> Result<domain::ConfidentialShareSigningKeyRotationRequest, HttpError> {
    Ok(domain::ConfidentialShareSigningKeyRotationRequest {
        old_signing_key_id: signing_key_id(request.old_signing_key_id, "old_signing_key_id")?,
        new_signing_key_id: signing_key_id(request.new_signing_key_id, "new_signing_key_id")?,
        participant_index: request.participant_index,
        public_key: signing_public_key(request.public_key, "public_key")?,
    })
}

pub(crate) fn share_signing_key_revocation_request_from_openapi(
    request: contract::ShareSigningKeyRevocation,
) -> Result<domain::ConfidentialShareSigningKeyRevocationRequest, HttpError> {
    Ok(domain::ConfidentialShareSigningKeyRevocationRequest {
        signing_key_id: signing_key_id(request.signing_key_id, "signing_key_id")?,
    })
}

fn additive_share_replay_window(
    value: &contract::AdditiveShareReplayWindow,
) -> Result<AdditiveShareReplayWindow, HttpError> {
    AdditiveShareReplayWindow::new(
        SecureAggregateEnvelopeSequence::new(value.first_sequence),
        SecureAggregateEnvelopeSequence::new(value.last_sequence),
    )
    .map_err(|error| {
        crate::core::response::HttpError::new(
            axum::http::StatusCode::BAD_REQUEST,
            format!("confidential replay_window is invalid: {error}"),
        )
    })
}

fn additive_share_commitment_slot(
    value: contract::AdditiveShareCommitmentSlot,
) -> Result<AdditiveShareCommitmentSlot, HttpError> {
    Ok(AdditiveShareCommitmentSlot::new(
        transcript_id(value.transcript_id, "slot.transcript_id")?,
        value.participant_index,
        commitment(value.commitment, "slot.commitment")?,
        signing_key_id(value.signing_key_id, "slot.signing_key_id")?,
    ))
}

fn signed_authenticated_additive_share_envelope(
    value: contract::SignedAuthenticatedAdditiveShareEnvelope,
) -> Result<SignedAuthenticatedAdditiveShareEnvelope, HttpError> {
    Ok(SignedAuthenticatedAdditiveShareEnvelope::new(
        authenticated_additive_share_envelope(value.envelope)?,
        signature(value.signature, "signed_envelope.signature")?,
    ))
}

fn authenticated_additive_share_envelope(
    value: contract::AuthenticatedAdditiveShareEnvelope,
) -> Result<AuthenticatedAdditiveShareEnvelope, HttpError> {
    Ok(AuthenticatedAdditiveShareEnvelope::new(
        transcript_id(value.transcript_id, "envelope.transcript_id")?,
        secure_aggregate_share(&value.share)?,
        commitment(value.commitment, "envelope.commitment")?,
        signing_key_id(value.signing_key_id, "envelope.signing_key_id")?,
        SecureAggregateEnvelopeSequence::new(value.sequence),
        transport_proof(value.proof, "envelope.proof")?,
    ))
}

fn secure_aggregate_share(
    value: &contract::SecureAggregateShare,
) -> Result<SecureAggregateShare, HttpError> {
    Ok(SecureAggregateShare::new(
        value.participant_index,
        field_element(value.value, "share.value")?,
    ))
}

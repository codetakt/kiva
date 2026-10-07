//! Shamir `OpenAPI` request DTO conversion.

use kiva_confidential::mechanism::shamir::{
    ShamirShare as DomainShamirShare, VerifiableShamirShare as DomainVerifiableShamirShare,
};
use kiva_confidential_contract as contract;

use crate::{core::response::HttpError, surfaces::confidential::model as domain};

use super::common::{commitment, field_element, shamir_opening_nonce, transcript_id};

pub(crate) fn shamir_threshold_reconstruction_request_from_openapi(
    request: contract::ShamirThresholdReconstruction,
) -> Result<domain::ConfidentialShamirThresholdReconstructionRequest, HttpError> {
    Ok(domain::ConfidentialShamirThresholdReconstructionRequest {
        reconstruction_id: transcript_id(request.reconstruction_id, "reconstruction_id")?,
        transcript_id: transcript_id(request.transcript_id, "transcript_id")?,
        participant_count: request.participant_count,
        threshold: request.threshold,
        share_openings: request
            .share_openings
            .into_iter()
            .map(verifiable_shamir_share)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn verifiable_shamir_share(
    value: contract::VerifiableShamirShare,
) -> Result<DomainVerifiableShamirShare, HttpError> {
    Ok(DomainVerifiableShamirShare::new(
        DomainShamirShare::new(
            value.share.participant_index,
            field_element(value.share.value, "share.value")?,
        ),
        commitment(value.commitment, "share.commitment")?,
        shamir_opening_nonce(value.opening_nonce, "share.opening_nonce")?,
    ))
}

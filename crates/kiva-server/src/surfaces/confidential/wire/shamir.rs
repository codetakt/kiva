//! Shamir threshold reconstruction response projection.

use serde_json::Value;

use kiva_confidential_contract::{
    ShamirThresholdReconstructionMetadata, ShamirThresholdReconstructionMetadataList,
    ShamirThresholdReconstructionResponse,
};

use crate::{
    core::response::{hex_lower, rfc3339},
    surfaces::confidential::model::ConfidentialShamirThresholdReconstructionState,
};

use super::dto_value;

pub(crate) fn shamir_threshold_reconstruction_response(
    reconstruction: &ConfidentialShamirThresholdReconstructionState,
) -> Result<Value, serde_json::Error> {
    dto_value(&ShamirThresholdReconstructionResponse {
        reconstruction: shamir_threshold_reconstruction_metadata(reconstruction),
        reconstructed_value: reconstruction.reconstructed_value.value(),
    })
}

pub(crate) fn shamir_threshold_reconstruction_list_response(
    reconstructions: &[ConfidentialShamirThresholdReconstructionState],
) -> Result<Value, serde_json::Error> {
    dto_value(&ShamirThresholdReconstructionMetadataList {
        reconstructions: reconstructions
            .iter()
            .map(shamir_threshold_reconstruction_metadata)
            .collect(),
    })
}

fn shamir_threshold_reconstruction_metadata(
    reconstruction: &ConfidentialShamirThresholdReconstructionState,
) -> ShamirThresholdReconstructionMetadata {
    ShamirThresholdReconstructionMetadata {
        reconstruction_id: hex_lower(&reconstruction.reconstruction_id.bytes()),
        transcript_id: hex_lower(&reconstruction.transcript_id.bytes()),
        owner_subject: reconstruction.owner_subject.clone(),
        participant_count: reconstruction.participant_count.get(),
        threshold: reconstruction.threshold.get(),
        share_count: reconstruction.share_count,
        verification_profile: reconstruction.verification_profile.clone(),
        verified_share_count: reconstruction.verified_share_count,
        complaint_count: reconstruction.complaint_count,
        mechanism: "shamirThreshold".to_owned(),
        created_at: rfc3339(reconstruction.created_at),
    }
}

//! Threshold release response projection.

use serde_json::Value;

use kiva_confidential_contract::{
    ThresholdReleaseMetadata, ThresholdReleaseMetadataList, ThresholdReleaseResponse,
};

use crate::{
    core::response::{hex_lower, rfc3339},
    surfaces::confidential::model::ConfidentialThresholdReleaseState,
};

use super::dto_value;

pub(crate) fn threshold_release_response(
    release: &ConfidentialThresholdReleaseState,
) -> Result<Value, serde_json::Error> {
    dto_value(&ThresholdReleaseResponse {
        release: threshold_release_metadata(release),
        aggregate: release.aggregate.value(),
    })
}

pub(crate) fn threshold_release_list_response(
    releases: &[ConfidentialThresholdReleaseState],
) -> Result<Value, serde_json::Error> {
    dto_value(&ThresholdReleaseMetadataList {
        releases: releases.iter().map(threshold_release_metadata).collect(),
    })
}

fn threshold_release_metadata(
    release: &ConfidentialThresholdReleaseState,
) -> ThresholdReleaseMetadata {
    ThresholdReleaseMetadata {
        release_id: hex_lower(&release.release_id.bytes()),
        transcript_id: hex_lower(&release.transcript_id.bytes()),
        requester_subject: release.requester_subject.clone(),
        authority_subjects: vec![
            release.left_authority_subject.clone(),
            release.right_authority_subject.clone(),
        ],
        approved_authority_subjects: release
            .approvals
            .iter()
            .map(|approval| approval.authority_subject.clone())
            .collect(),
        status: if release.released_at.is_some() {
            "released".to_owned()
        } else {
            "pending".to_owned()
        },
        created_at: rfc3339(release.created_at),
        released_at: release.released_at.map(rfc3339),
    }
}

//! Threshold-release `OpenAPI` request DTO conversion.

use kiva_confidential_contract as contract;

use crate::{core::response::HttpError, surfaces::confidential::model as domain};

use super::common::{field_element, transcript_id};

pub(crate) fn threshold_release_request_from_openapi(
    request: contract::ThresholdReleaseRequest,
) -> Result<domain::ConfidentialThresholdReleaseRequest, HttpError> {
    Ok(domain::ConfidentialThresholdReleaseRequest {
        release_id: transcript_id(request.release_id, "release_id")?,
        transcript_id: transcript_id(request.transcript_id, "transcript_id")?,
        aggregate: field_element(request.aggregate, "aggregate")?,
        left_authority_subject: request.left_authority_subject,
        right_authority_subject: request.right_authority_subject,
    })
}

pub(crate) fn threshold_release_approval_request_from_openapi(
    request: contract::ThresholdReleaseApproval,
) -> Result<domain::ConfidentialThresholdReleaseApprovalRequest, HttpError> {
    Ok(domain::ConfidentialThresholdReleaseApprovalRequest {
        release_id: transcript_id(request.release_id, "release_id")?,
    })
}

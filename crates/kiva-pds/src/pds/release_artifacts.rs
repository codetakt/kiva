//! PDS adapters for release artifact references.

use kiva_release_artifacts::{ReleaseArtifactRef, ReleaseArtifactValidationError};

use super::{DataProductRegistrationCommandInput, ValidationError};

/// Parses the release artifact reference carried by a data-product request.
pub fn release_artifact_ref_from_data_product_request(
    request: &DataProductRegistrationCommandInput,
) -> Result<ReleaseArtifactRef, ValidationError> {
    ReleaseArtifactRef::from_data_product_source_hex(request.source_kind, &request.release_id)
        .map_err(|error| release_artifact_validation_error(&error))
}

/// Creates an artifact reference from a legacy source kind and validates the identifier.
pub fn release_artifact_ref_from_data_product_source_bytes(
    source_kind: kiva_release_artifacts::DataProductSourceKind,
    artifact_id: [u8; 32],
) -> Result<ReleaseArtifactRef, ValidationError> {
    ReleaseArtifactRef::from_data_product_source_bytes(source_kind, artifact_id)
        .map_err(|error| release_artifact_validation_error(&error))
}

fn release_artifact_validation_error(error: &ReleaseArtifactValidationError) -> ValidationError {
    ValidationError::new(error.message())
}

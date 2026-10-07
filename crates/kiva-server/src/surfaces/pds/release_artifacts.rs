//! Release-artifact boundary used by PDS data-product registration.

use kiva_pds::{
    ReleaseArtifactKind as PdsReleaseArtifactKind, ReleaseArtifactRef as PdsReleaseArtifactRef,
};
use kiva_policy::policy::{
    ReleaseArtifactAuthorization, ReleaseArtifactKind as PolicyReleaseArtifactKind,
    ReleaseArtifactRef as PolicyReleaseArtifactRef,
    release_artifact_is_available_for_pds_data_product as policy_release_artifact_is_available_for_pds_data_product,
};

pub(crate) const fn release_artifact_available_from_metadata(
    artifact: PdsReleaseArtifactRef,
    artifact_exists: bool,
    principal_matches_artifact_owner: bool,
    artifact_released: bool,
) -> bool {
    policy_release_artifact_is_available_for_pds_data_product(ReleaseArtifactAuthorization::new(
        PolicyReleaseArtifactRef::new(policy_release_artifact_kind(artifact.kind())),
        artifact_exists,
        principal_matches_artifact_owner,
        artifact_released,
    ))
}

const fn policy_release_artifact_kind(
    source_kind: PdsReleaseArtifactKind,
) -> PolicyReleaseArtifactKind {
    match source_kind {
        PdsReleaseArtifactKind::ThresholdRelease => PolicyReleaseArtifactKind::ThresholdRelease,
        PdsReleaseArtifactKind::DifferentialPrivacyRelease => {
            PolicyReleaseArtifactKind::DifferentialPrivacyRelease
        }
        PdsReleaseArtifactKind::ShamirThresholdReconstruction => {
            PolicyReleaseArtifactKind::ShamirThresholdReconstruction
        }
    }
}

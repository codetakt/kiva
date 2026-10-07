#![allow(clippy::wildcard_imports)]

use super::*;

impl MemoryConfidentialRepository {
    pub(crate) async fn release_artifact_is_available_for_pds_data_product(
        &self,
        principal: &AuthenticatedPrincipal,
        artifact: PdsReleaseArtifactRef,
    ) -> bool {
        let Some(artifact_id) = pds_release_artifact_transcript_id(artifact) else {
            return false;
        };
        match artifact.kind() {
            PdsReleaseArtifactKind::ThresholdRelease => {
                let (release_exists, principal_matches_release_owner, threshold_release_released) =
                    self.threshold
                        .releases
                        .read()
                        .await
                        .get(&ConfidentialThresholdReleaseKey::from_release_id(
                            artifact_id,
                        ))
                        .map_or((false, false, false), |release| {
                            (
                                true,
                                release.requester_subject == principal.subject,
                                release.released_at.is_some(),
                            )
                        });
                release_artifact_available_from_metadata(
                    artifact,
                    release_exists,
                    principal_matches_release_owner,
                    threshold_release_released,
                )
            }
            PdsReleaseArtifactKind::DifferentialPrivacyRelease => {
                let (release_exists, principal_matches_release_owner) = self
                    .differential_privacy
                    .releases
                    .read()
                    .await
                    .get(&ConfidentialDifferentialPrivacyReleaseKey::from_release_id(
                        artifact_id,
                    ))
                    .map_or((false, false), |release| {
                        (true, release.owner_subject == principal.subject)
                    });
                release_artifact_available_from_metadata(
                    artifact,
                    release_exists,
                    principal_matches_release_owner,
                    false,
                )
            }
            PdsReleaseArtifactKind::ShamirThresholdReconstruction => {
                let (release_exists, principal_matches_release_owner) = self
                    .shamir
                    .reconstructions
                    .read()
                    .await
                    .get(
                        &ConfidentialShamirThresholdReconstructionKey::from_reconstruction_id(
                            artifact_id,
                        ),
                    )
                    .map_or((false, false), |reconstruction| {
                        (true, reconstruction.owner_subject == principal.subject)
                    });
                release_artifact_available_from_metadata(
                    artifact,
                    release_exists,
                    principal_matches_release_owner,
                    false,
                )
            }
        }
    }
}

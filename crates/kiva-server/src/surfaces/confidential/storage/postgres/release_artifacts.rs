#![allow(clippy::wildcard_imports)]

use super::*;

impl PostgresConfidentialRepository {
    pub(crate) async fn release_artifact_is_available_for_pds_data_product(
        &self,
        principal: &AuthenticatedPrincipal,
        artifact: PdsReleaseArtifactRef,
    ) -> Result<bool, ConfidentialAggregateStoreError> {
        let Some(artifact_id) = pds_release_artifact_transcript_id(artifact) else {
            return Ok(false);
        };
        match artifact.kind() {
            PdsReleaseArtifactKind::ThresholdRelease => {
                let row = sqlx::query_as::<_, (bool, bool)>(
                    "SELECT requester_subject = $2, released_at IS NOT NULL \
                 FROM kiva.confidential_threshold_release_requests \
                 WHERE tenant_id = $1 AND release_id = $3",
                )
                .bind(&self.tenant_id)
                .bind(&principal.subject)
                .bind(artifact_id.bytes().to_vec())
                .fetch_optional(&self.pool)
                .await
                .map_err(confidential_store)?;
                let release_exists = row.is_some();
                let (principal_matches_release_owner, threshold_release_released) =
                    row.unwrap_or((false, false));
                Ok(release_artifact_available_from_metadata(
                    artifact,
                    release_exists,
                    principal_matches_release_owner,
                    threshold_release_released,
                ))
            }
            PdsReleaseArtifactKind::DifferentialPrivacyRelease => {
                let principal_matches_release_owner = sqlx::query_scalar::<_, bool>(
                    "SELECT owner_subject = $2 \
                     FROM kiva.confidential_differential_privacy_releases \
                     WHERE tenant_id = $1 AND release_id = $3",
                )
                .bind(&self.tenant_id)
                .bind(&principal.subject)
                .bind(artifact_id.bytes().to_vec())
                .fetch_optional(&self.pool)
                .await
                .map_err(confidential_store)?;
                let release_exists = principal_matches_release_owner.is_some();
                Ok(release_artifact_available_from_metadata(
                    artifact,
                    release_exists,
                    principal_matches_release_owner.unwrap_or(false),
                    false,
                ))
            }
            PdsReleaseArtifactKind::ShamirThresholdReconstruction => {
                let principal_matches_release_owner = sqlx::query_scalar::<_, bool>(
                    "SELECT owner_subject = $2 \
                     FROM kiva.confidential_shamir_threshold_reconstructions \
                     WHERE tenant_id = $1 AND reconstruction_id = $3",
                )
                .bind(&self.tenant_id)
                .bind(&principal.subject)
                .bind(artifact_id.bytes().to_vec())
                .fetch_optional(&self.pool)
                .await
                .map_err(confidential_store)?;
                let release_exists = principal_matches_release_owner.is_some();
                Ok(release_artifact_available_from_metadata(
                    artifact,
                    release_exists,
                    principal_matches_release_owner.unwrap_or(false),
                    false,
                ))
            }
        }
    }
}

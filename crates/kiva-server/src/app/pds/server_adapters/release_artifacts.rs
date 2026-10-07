//! Confidential release-artifact adapter for PDS application data products.

use kiva_application::pds::{ApplicationFuture, ReleaseArtifactAvailability};
use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::ReleaseArtifactRef as PdsReleaseArtifactRef;

use crate::surfaces::confidential::repository::ConfidentialRepository;

use super::{super::ReleaseArtifactAvailabilityError, records::release_error};

#[derive(Debug, Clone)]
pub(crate) struct ReleaseArtifactAvailabilityPort {
    repository: ConfidentialRepository,
}

impl ReleaseArtifactAvailabilityPort {
    pub(crate) const fn from_server_adapter(repository: ConfidentialRepository) -> Self {
        Self { repository }
    }

    async fn is_available_for_pds_data_product_impl(
        &self,
        principal: &AuthenticatedPrincipal,
        artifact: PdsReleaseArtifactRef,
    ) -> Result<bool, ReleaseArtifactAvailabilityError> {
        self.repository
            .release_artifact_is_available_for_pds_data_product(principal, artifact)
            .await
            .map_err(release_error)
    }
}

impl ReleaseArtifactAvailability for ReleaseArtifactAvailabilityPort {
    fn is_available_for_pds_data_product<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        artifact: PdsReleaseArtifactRef,
    ) -> ApplicationFuture<'a, Result<bool, ReleaseArtifactAvailabilityError>> {
        Box::pin(async move {
            self.is_available_for_pds_data_product_impl(principal, artifact)
                .await
        })
    }
}

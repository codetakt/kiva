#![allow(clippy::wildcard_imports)]

use super::*;

impl MemoryConfidentialRepository {
    pub(crate) async fn reconstruct_shamir_threshold_secret(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShamirThresholdReconstructionRequest,
    ) -> Result<ConfidentialShamirThresholdReconstructionState, ConfidentialShamirReconstructionError>
    {
        let reconstruction =
            ConfidentialShamirThresholdReconstructionState::from_request(principal, &request)?;
        let key = ConfidentialShamirThresholdReconstructionKey::from_reconstruction_id(
            reconstruction.reconstruction_id,
        );
        let mut reconstructions = self.shamir.reconstructions.write().await;
        let outcome = match reconstructions.get(&key) {
            Some(existing) if existing.same_reconstruction(&reconstruction) => existing.clone(),
            Some(_existing) => return Err(ConfidentialShamirReconstructionError::Conflict),
            None => {
                reconstructions.insert(key, reconstruction.clone());
                reconstruction
            }
        };
        drop(reconstructions);
        Ok(outcome)
    }

    pub(crate) async fn list_shamir_threshold_reconstructions(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialShamirThresholdReconstructionFilter,
    ) -> Result<
        Vec<ConfidentialShamirThresholdReconstructionState>,
        ConfidentialShamirReconstructionError,
    > {
        let mut reconstructions = {
            let reconstructions = self.shamir.reconstructions.read().await;
            reconstructions
                .values()
                .filter(|reconstruction| {
                    reconstruction.owner_subject == principal.subject
                        && filter.matches(reconstruction)
                })
                .cloned()
                .collect::<Vec<_>>()
        };
        reconstructions.sort_by_key(|reconstruction| {
            (
                reconstruction.created_at,
                reconstruction.reconstruction_id.bytes(),
            )
        });
        Ok(reconstructions)
    }
}

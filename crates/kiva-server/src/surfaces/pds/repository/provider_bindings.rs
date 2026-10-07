//! PDS repository facade methods for provider-binding registry metadata.

use kiva_identity::AuthenticatedPrincipal;

use crate::{
    core::outcome::StoreOutcome,
    surfaces::pds::{
        model::{PdsProviderBindingFilter, PdsProviderBindingState},
        outcome::PdsRepositoryError,
        repository::PdsRepository,
    },
};

impl PdsRepository {
    pub(crate) async fn list_provider_bindings(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsProviderBindingFilter,
    ) -> Result<Vec<PdsProviderBindingState>, PdsRepositoryError> {
        dispatch_repository!(self, |repository| repository
            .list_provider_bindings(principal, filter)
            .await)
    }

    pub(crate) async fn register_provider_binding(
        &self,
        binding: PdsProviderBindingState,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        dispatch_repository!(self, |repository| repository
            .register_provider_binding(binding)
            .await)
    }
}

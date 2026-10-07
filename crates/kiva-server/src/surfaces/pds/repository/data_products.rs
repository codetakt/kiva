//! PDS repository facade methods for approved data products.

use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::{
    DataProductReadIntent as PdsDataProductReadIntent,
    DataProductRegistrationRequested as PdsDataProductRegistrationRequested,
};

use crate::{
    core::outcome::StoreOutcome,
    surfaces::pds::{
        model::{
            PdsDataProductFilter, PdsDataProductProjection, PdsDataProductReadOutcome,
            PdsDataProductState,
        },
        outcome::PdsRepositoryError,
        repository::PdsRepository,
    },
};

impl PdsRepository {
    pub(crate) async fn list_data_products(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsDataProductFilter,
    ) -> Result<Vec<PdsDataProductState>, PdsRepositoryError> {
        dispatch_repository!(self, |repository| repository
            .list_data_products(principal, filter)
            .await)
    }

    pub(crate) async fn register_data_product(
        &self,
        event: PdsDataProductRegistrationRequested,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let projection = PdsDataProductProjection::from_admitted_event(&event);
        dispatch_repository!(self, |repository| repository
            .register_data_product(projection)
            .await)
    }

    pub(crate) async fn read_data_product(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsDataProductReadIntent,
    ) -> Result<PdsDataProductReadOutcome, PdsRepositoryError> {
        dispatch_repository!(self, |repository| repository
            .read_data_product(principal, request)
            .await)
    }
}

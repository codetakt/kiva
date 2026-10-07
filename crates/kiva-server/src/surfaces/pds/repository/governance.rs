//! PDS repository facade methods for governance profiles.

use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::GovernanceProfileRegistrationRequested as PdsGovernanceProfileRegistrationRequested;

use crate::{
    core::outcome::StoreOutcome,
    surfaces::pds::{
        model::{
            PdsGovernanceProfileFilter, PdsGovernanceProfileProjection, PdsGovernanceProfileState,
        },
        outcome::PdsRepositoryError,
        repository::PdsRepository,
    },
};

impl PdsRepository {
    pub(crate) async fn register_governance_profile(
        &self,
        event: PdsGovernanceProfileRegistrationRequested,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let projection = PdsGovernanceProfileProjection::from_admitted_event(&event);
        dispatch_repository!(self, |repository| repository
            .register_governance_profile(projection)
            .await)
    }

    pub(crate) async fn list_governance_profiles(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsGovernanceProfileFilter,
    ) -> Result<Vec<PdsGovernanceProfileState>, PdsRepositoryError> {
        dispatch_repository!(self, |repository| repository
            .list_governance_profiles(principal, filter)
            .await)
    }
}

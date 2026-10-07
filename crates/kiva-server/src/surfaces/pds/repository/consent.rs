//! PDS repository facade methods for consent grants and revocations.

use kiva_identity::AuthenticatedPrincipal;
use kiva_pds::{
    ConsentGrantRegistrationRequested as PdsConsentGrantRegistrationRequested,
    ConsentGrantRevocationRequested as PdsConsentGrantRevocationRequested,
};

use crate::{
    core::outcome::StoreOutcome,
    surfaces::pds::{
        model::{PdsConsentGrantFilter, PdsConsentGrantProjection, PdsConsentGrantState},
        outcome::{PdsRepositoryError, PdsRevocationOutcome},
        repository::PdsRepository,
    },
};

impl PdsRepository {
    pub(crate) async fn list_consent_grants(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsConsentGrantFilter,
    ) -> Result<Vec<PdsConsentGrantState>, PdsRepositoryError> {
        dispatch_repository!(self, |repository| repository
            .list_consent_grants(principal, filter)
            .await)
    }

    pub(crate) async fn register_consent_grant(
        &self,
        event: PdsConsentGrantRegistrationRequested,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let projection = PdsConsentGrantProjection::from_admitted_event(&event);
        dispatch_repository!(self, |repository| repository
            .register_consent_grant(projection)
            .await)
    }

    pub(crate) async fn revoke_consent_grant(
        &self,
        event: PdsConsentGrantRevocationRequested,
    ) -> Result<PdsRevocationOutcome, PdsRepositoryError> {
        dispatch_repository!(self, |repository| repository
            .revoke_consent_grant(event)
            .await)
    }
}

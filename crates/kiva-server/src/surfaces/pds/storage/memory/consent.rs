#![allow(clippy::wildcard_imports)]

use super::*;

impl MemoryPdsRepository {
    #[cfg(test)]
    pub(crate) async fn insert_consent_grant_for_test(&self, grant: PdsConsentGrantState) {
        self.consent
            .grants
            .write()
            .await
            .insert(PdsConsentGrantKey::from_grant_id(grant.grant_id), grant);
    }

    pub(crate) async fn list_consent_grants(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsConsentGrantFilter,
    ) -> Result<Vec<PdsConsentGrantState>, PdsRepositoryError> {
        let mut grants = {
            let grants = self.consent.grants.read().await;
            grants
                .values()
                .filter(|grant| {
                    grant.data_subject == principal.subject
                        || grant.grantee_subject == principal.subject
                })
                .filter(|grant| filter.matches(grant))
                .cloned()
                .collect::<Vec<_>>()
        };
        grants.sort_by_key(|grant| (grant.created_at, grant.grant_id));
        Ok(grants)
    }

    pub(crate) async fn register_consent_grant(
        &self,
        projection: PdsConsentGrantProjection,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let grant = projection.state().clone();
        let key = PdsConsentGrantKey::from_grant_id(grant.grant_id);
        let mut grants = self.consent.grants.write().await;
        let outcome = match grants.get(&key) {
            Some(existing) if existing.same_active_registration(&grant) => StoreOutcome::Unchanged,
            Some(_existing) => StoreOutcome::Conflict,
            None => {
                grants.insert(key, grant.clone());
                StoreOutcome::Stored
            }
        };
        drop(grants);
        if matches!(outcome, StoreOutcome::Stored) {
            self.record_ledger_event(projection.ledger_event()).await;
        }
        Ok(outcome)
    }

    pub(crate) async fn revoke_consent_grant(
        &self,
        event: PdsConsentGrantRevocationRequested,
    ) -> Result<PdsRevocationOutcome, PdsRepositoryError> {
        let mut grants = self.consent.grants.write().await;
        let key = PdsConsentGrantKey::from_grant_id(event.grant_id());
        let mut ledger_event = None;
        let outcome = match grants.get_mut(&key) {
            None => PdsRevocationOutcome::NotFound,
            Some(grant) if grant.data_subject != event.data_subject() => {
                PdsRevocationOutcome::Forbidden
            }
            Some(grant) if grant.state == PdsConsentState::Revoked => {
                PdsRevocationOutcome::AlreadyRevoked
            }
            Some(grant) => {
                let revoked_at = event.revoked_at();
                grant.state = PdsConsentState::Revoked;
                grant.revoked_at = Some(revoked_at);
                ledger_event = Some(PdsLedgerEventState::consent_grant_revoked(
                    grant, revoked_at,
                ));
                PdsRevocationOutcome::Revoked
            }
        };
        drop(grants);
        if let Some(event) = ledger_event {
            self.record_ledger_event(event).await;
        }
        Ok(outcome)
    }
}

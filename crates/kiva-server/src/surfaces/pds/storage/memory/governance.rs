#![allow(clippy::wildcard_imports)]

use super::*;

impl MemoryPdsRepository {
    pub(crate) async fn register_governance_profile(
        &self,
        projection: PdsGovernanceProfileProjection,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let profile = projection.state().clone();
        let key = PdsGovernanceProfileKey::from_profile_id(profile.profile_id);
        let mut profiles = self.governance.profiles.write().await;
        let outcome = match profiles.get(&key) {
            Some(existing) if existing.same_registration(&profile) => StoreOutcome::Unchanged,
            Some(_existing) => StoreOutcome::Conflict,
            None => {
                profiles.insert(key, profile.clone());
                StoreOutcome::Stored
            }
        };
        drop(profiles);
        if matches!(outcome, StoreOutcome::Stored) {
            self.record_ledger_event(projection.ledger_event()).await;
        }
        Ok(outcome)
    }

    pub(crate) async fn list_governance_profiles(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsGovernanceProfileFilter,
    ) -> Result<Vec<PdsGovernanceProfileState>, PdsRepositoryError> {
        let mut profiles = {
            let profiles = self.governance.profiles.read().await;
            profiles
                .values()
                .filter(|profile| profile.profile_owner_subject == principal.subject)
                .filter(|profile| filter.matches(profile))
                .cloned()
                .collect::<Vec<_>>()
        };
        profiles.sort_by_key(|profile| (profile.created_at, profile.profile_id));
        Ok(profiles)
    }
}

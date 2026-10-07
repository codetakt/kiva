#![allow(clippy::wildcard_imports)]

use super::*;

impl MemoryPdsRepository {
    pub(crate) async fn list_provider_bindings(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsProviderBindingFilter,
    ) -> Result<Vec<PdsProviderBindingState>, PdsRepositoryError> {
        let mut bindings = {
            let bindings = self.provider_bindings.bindings.read().await;
            bindings
                .values()
                .filter(|binding| binding.owner_subject == principal.subject)
                .filter(|binding| filter.matches(binding))
                .cloned()
                .collect::<Vec<_>>()
        };
        bindings.sort_by_key(|binding| (binding.created_at, binding.binding_id));
        Ok(bindings)
    }

    pub(crate) async fn register_provider_binding(
        &self,
        binding: PdsProviderBindingState,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let key = PdsProviderBindingKey::from_binding_id(binding.binding_id);
        let outcome = {
            let mut bindings = self.provider_bindings.bindings.write().await;
            match bindings.get(&key) {
                Some(existing) if existing.same_registration(&binding) => StoreOutcome::Unchanged,
                Some(_existing) => StoreOutcome::Conflict,
                None => {
                    bindings.insert(key, binding);
                    StoreOutcome::Stored
                }
            }
        };
        Ok(outcome)
    }
}

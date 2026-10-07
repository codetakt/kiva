#![allow(clippy::wildcard_imports)]

use super::*;

impl MemoryPdsRepository {
    pub(crate) async fn list_data_products(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsDataProductFilter,
    ) -> Result<Vec<PdsDataProductState>, PdsRepositoryError> {
        let mut products = {
            let products = self.data_products.products.read().await;
            products
                .values()
                .filter(|product| product.owner_subject == principal.subject)
                .filter(|product| filter.matches(product))
                .cloned()
                .collect::<Vec<_>>()
        };
        products.sort_by_key(|product| (product.created_at, product.data_product_id));
        Ok(products)
    }

    pub(crate) async fn register_data_product(
        &self,
        projection: PdsDataProductProjection,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let product = projection.state().clone();
        let key = PdsDataProductKey::from_data_product_id(product.data_product_id);
        let mut products = self.data_products.products.write().await;
        let outcome = match products.get(&key) {
            Some(existing) if existing.same_registration(&product) => StoreOutcome::Unchanged,
            Some(_existing) => StoreOutcome::Conflict,
            None => {
                products.insert(key, product.clone());
                StoreOutcome::Stored
            }
        };
        drop(products);
        if matches!(outcome, StoreOutcome::Stored) {
            self.record_ledger_event(projection.ledger_event()).await;
        }
        Ok(outcome)
    }

    pub(crate) async fn read_data_product(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsDataProductReadIntent,
    ) -> Result<PdsDataProductReadOutcome, PdsRepositoryError> {
        let data_product = {
            let products = self.data_products.products.read().await;
            products
                .get(&PdsDataProductKey::from_data_product_id(
                    request.data_product_id,
                ))
                .cloned()
        };
        let grant = {
            let grants = self.consent.grants.read().await;
            grants
                .get(&PdsConsentGrantKey::from_grant_id(request.grant_id))
                .cloned()
        };
        let event = pds_access_audit_event_from_decision(
            principal,
            pds_data_product_read_access_check(request),
            grant.as_ref(),
            data_product.as_ref(),
        );
        let key = PdsAccessAuditEventKey::from_event_id(event.event_id);
        let mut events = self.access.audit_events.write().await;
        events.insert(key, event.clone());
        drop(events);
        self.record_ledger_event(PdsLedgerEventState::access_decision_recorded(&event))
            .await;
        Ok(PdsDataProductReadOutcome::from_event_and_product(
            event,
            data_product,
        ))
    }
}

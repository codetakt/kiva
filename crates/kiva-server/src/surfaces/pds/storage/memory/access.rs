#![allow(clippy::wildcard_imports)]

use super::*;

impl MemoryPdsRepository {
    pub(crate) async fn record_analytics_read(
        &self,
        event: PdsAccessAuditEventState,
    ) -> Result<(), PdsRepositoryError> {
        let key = PdsAccessAuditEventKey::from_event_id(event.event_id);
        let mut events = self.access.audit_events.write().await;
        events.insert(key, event.clone());
        drop(events);
        self.record_ledger_event(PdsLedgerEventState::access_decision_recorded(&event))
            .await;
        Ok(())
    }

    pub(crate) async fn evaluate_access_check(
        &self,
        principal: &AuthenticatedPrincipal,
        request: PdsAccessCheckIntent,
    ) -> Result<PdsAccessAuditEventState, PdsRepositoryError> {
        let request = PdsParsedAccessCheckRequest::from_request(&request)?;
        let data_product = if let Some(data_product_id) = request.data_product_id {
            let products = self.data_products.products.read().await;
            let product = products
                .get(&PdsDataProductKey::from_data_product_id(data_product_id))
                .cloned();
            drop(products);
            product
        } else {
            None
        };
        let grant = if let Some(grant_id) = request.grant_id {
            let grants = self.consent.grants.read().await;
            let grant = grants
                .get(&PdsConsentGrantKey::from_grant_id(grant_id))
                .cloned();
            drop(grants);
            grant
        } else {
            None
        };
        let event = pds_access_audit_event_from_decision(
            principal,
            request,
            grant.as_ref(),
            data_product.as_ref(),
        );
        let key = PdsAccessAuditEventKey::from_event_id(event.event_id);
        let mut events = self.access.audit_events.write().await;
        events.insert(key, event.clone());
        drop(events);
        self.record_ledger_event(PdsLedgerEventState::access_decision_recorded(&event))
            .await;
        Ok(event)
    }

    pub(crate) async fn list_access_audit_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsAccessAuditEventFilter,
    ) -> Result<Vec<PdsAccessAuditEventState>, PdsRepositoryError> {
        let mut events = {
            let events = self.access.audit_events.read().await;
            events
                .values()
                .filter(|event| event.requester_subject == principal.subject)
                .filter(|event| filter.matches(event))
                .cloned()
                .collect::<Vec<_>>()
        };
        events.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| left.event_id.cmp(&right.event_id))
        });
        Ok(events)
    }
}

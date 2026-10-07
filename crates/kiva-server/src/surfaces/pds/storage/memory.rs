//! PDS memory repository module owner.

#![allow(clippy::wildcard_imports)]

use super::*;

mod access;
mod consent;
mod data_products;
mod governance;
mod lifecycle;
mod provider_bindings;

impl MemoryPdsRepository {
    pub(crate) async fn record_portability_export_audit(
        &self,
        event: &kiva_pds::PortabilityExportAuditRecorded,
    ) -> StoreOutcome {
        self.record_ledger_event(PdsLedgerEventState::portability_export_audited(event))
            .await;
        StoreOutcome::Stored
    }

    #[cfg(test)]
    pub(crate) async fn ledger_event_for_test(
        &self,
        event_kind: &str,
        resource_id: &str,
    ) -> Option<PdsLedgerEventState> {
        let events = self.lifecycle.ledger_events.read().await;
        events
            .values()
            .find(|event| {
                event.event_kind.label() == event_kind && event.resource_id == resource_id
            })
            .cloned()
    }

    #[cfg(test)]
    pub(crate) async fn ledger_event_count_for_test(&self) -> usize {
        self.lifecycle.ledger_events.read().await.len()
    }

    #[cfg(test)]
    pub(crate) async fn ledger_events_for_test(
        &self,
        event_kind: &str,
        owner_subject: &str,
    ) -> usize {
        self.lifecycle
            .ledger_events
            .read()
            .await
            .values()
            .filter(|event| {
                event.event_kind.label() == event_kind && event.owner_subject == owner_subject
            })
            .count()
    }

    async fn record_ledger_event(&self, event: PdsLedgerEventState) {
        let key = PdsLedgerEventKey::from_event(&event);
        let mut events = self.lifecycle.ledger_events.write().await;
        events.insert(key, event);
    }
}

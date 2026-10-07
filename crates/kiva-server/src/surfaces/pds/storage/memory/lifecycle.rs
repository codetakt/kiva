#![allow(clippy::wildcard_imports)]

use super::*;

impl MemoryPdsRepository {
    pub(crate) async fn register_lifecycle_event(
        &self,
        projection: PdsLifecycleEventProjection,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let event = projection.state().clone();
        let key = PdsLifecycleEventKey::from_event_id(event.event_id);
        let mut events = self.lifecycle.events.write().await;
        let outcome = match events.get(&key) {
            Some(existing) if existing.same_registration(&event) => StoreOutcome::Unchanged,
            Some(_existing) => StoreOutcome::Conflict,
            None => {
                events.insert(key, event.clone());
                StoreOutcome::Stored
            }
        };
        drop(events);
        if matches!(outcome, StoreOutcome::Stored) {
            self.record_ledger_event(projection.ledger_event()).await;
        }
        Ok(outcome)
    }

    pub(crate) async fn list_lifecycle_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsLifecycleEventFilter,
    ) -> Result<Vec<PdsLifecycleEventState>, PdsRepositoryError> {
        let mut events = {
            let events = self.lifecycle.events.read().await;
            events
                .values()
                .filter(|event| event.data_subject == principal.subject)
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

    pub(crate) async fn register_retention_policy(
        &self,
        projection: PdsRetentionPolicyProjection,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let policy = projection.state().clone();
        let key = PdsRetentionPolicyKey::from_policy_id(policy.policy_id);
        let mut policies = self.lifecycle.retention_policies.write().await;
        let outcome = match policies.get(&key) {
            Some(existing) if existing.same_registration(&policy) => StoreOutcome::Unchanged,
            Some(_existing) => StoreOutcome::Conflict,
            None => {
                policies.insert(key, policy.clone());
                StoreOutcome::Stored
            }
        };
        drop(policies);
        if matches!(outcome, StoreOutcome::Stored) {
            self.record_ledger_event(projection.ledger_event()).await;
        }
        Ok(outcome)
    }

    pub(crate) async fn list_retention_policies(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsRetentionPolicyFilter,
    ) -> Result<Vec<PdsRetentionPolicyState>, PdsRepositoryError> {
        let mut policies = {
            let policies = self.lifecycle.retention_policies.read().await;
            policies
                .values()
                .filter(|policy| policy.owner_subject == principal.subject)
                .filter(|policy| filter.matches(policy))
                .cloned()
                .collect::<Vec<_>>()
        };
        policies.sort_by_key(|policy| (policy.created_at, policy.policy_id));
        Ok(policies)
    }

    pub(crate) async fn register_lifecycle_case(
        &self,
        projection: PdsLifecycleCaseProjection,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let requested = projection.state().clone();
        if let Some(policy_id) = requested.retention_policy_id {
            let policy_owner_subject = self
                .lifecycle
                .retention_policies
                .read()
                .await
                .get(&PdsRetentionPolicyKey::from_policy_id(policy_id))
                .map(|policy| policy.owner_subject.clone());
            let Some(policy_owner_subject) = policy_owner_subject else {
                return Err(PdsRepositoryError::InvalidRequest(
                    "PDS lifecycle case retention_policy_id must reference an existing retention policy".to_owned(),
                ));
            };
            if policy_owner_subject != requested.data_subject {
                return Err(PdsRepositoryError::InvalidRequest(
                    "PDS lifecycle case retention policy belongs to a different subject".to_owned(),
                ));
            }
        }
        let key = PdsLifecycleCaseKey::from_case_id(requested.case_id);
        let mut cases = self.lifecycle.cases.write().await;
        let mut ledger_event = None;
        let outcome = if let Some(existing) = cases.get_mut(&key) {
            match existing.transition_from(&requested)? {
                StoreOutcome::Stored => {
                    existing.status = requested.status;
                    existing.resolution = requested.resolution;
                    existing.updated_at = requested.updated_at;
                    ledger_event = Some(projection.transitioned_ledger_event());
                    StoreOutcome::Stored
                }
                outcome => outcome,
            }
        } else {
            if !pds_lifecycle_case_initial_status_is_allowed(requested.status) {
                return Err(PdsRepositoryError::InvalidRequest(
                    "PDS lifecycle case must start in open status".to_owned(),
                ));
            }
            ledger_event = Some(projection.opened_ledger_event());
            cases.insert(key, requested);
            StoreOutcome::Stored
        };
        drop(cases);
        if let Some(event) = ledger_event {
            self.record_ledger_event(event).await;
        }
        Ok(outcome)
    }

    pub(crate) async fn list_lifecycle_cases(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsLifecycleCaseFilter,
    ) -> Result<Vec<PdsLifecycleCaseState>, PdsRepositoryError> {
        let mut cases = {
            let cases = self.lifecycle.cases.read().await;
            cases
                .values()
                .filter(|case| case.data_subject == principal.subject)
                .filter(|case| filter.matches(case))
                .cloned()
                .collect::<Vec<_>>()
        };
        cases.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.case_id.cmp(&right.case_id))
        });
        Ok(cases)
    }
}

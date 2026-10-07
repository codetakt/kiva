#![allow(clippy::wildcard_imports)]

use super::*;

impl MemoryConfidentialRepository {
    pub(crate) async fn register_threshold_release_request(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialThresholdReleaseRequest,
    ) -> Result<StoreOutcome, ConfidentialThresholdReleaseError> {
        let release = ConfidentialThresholdReleaseState::from_request(principal, request)?;
        let aggregate_results = self.additive.aggregate_results.read().await;
        ensure_matching_aggregate_result(
            aggregate_results.get(&ConfidentialAggregateResultKey::from_transcript_id(
                release.transcript_id,
            )),
            &release,
        )?;
        drop(aggregate_results);
        let key = ConfidentialThresholdReleaseKey::from_release_id(release.release_id);
        let mut releases = self.threshold.releases.write().await;
        let outcome = match releases.get(&key) {
            Some(existing) if existing.same_request(&release) => StoreOutcome::Unchanged,
            Some(_existing) => StoreOutcome::Conflict,
            None => {
                releases.insert(key, release);
                StoreOutcome::Stored
            }
        };
        drop(releases);
        Ok(outcome)
    }

    pub(crate) async fn approve_threshold_release(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialThresholdReleaseApprovalRequest,
    ) -> Result<StoreOutcome, ConfidentialThresholdReleaseError> {
        let key = ConfidentialThresholdReleaseKey::from_release_id(request.release_id);
        let mut releases = self.threshold.releases.write().await;
        let Some(release) = releases.get_mut(&key) else {
            return Err(ConfidentialThresholdReleaseError::NotFound);
        };
        release.ensure_authority(principal)?;
        let outcome = if release
            .approvals
            .iter()
            .any(|approval| approval.authority_subject == principal.subject)
        {
            StoreOutcome::Unchanged
        } else {
            release
                .approvals
                .push(ConfidentialThresholdReleaseApproval {
                    authority_subject: principal.subject.clone(),
                    approved_at: OffsetDateTime::now_utc(),
                });
            StoreOutcome::Stored
        };
        release.sort_approvals();
        drop(releases);
        Ok(outcome)
    }

    pub(crate) async fn release_threshold_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialThresholdReleaseApprovalRequest,
    ) -> Result<ConfidentialThresholdReleaseState, ConfidentialThresholdReleaseError> {
        let key = ConfidentialThresholdReleaseKey::from_release_id(request.release_id);
        let mut releases = self.threshold.releases.write().await;
        let Some(release) = releases.get_mut(&key) else {
            return Err(ConfidentialThresholdReleaseError::NotFound);
        };
        release.ensure_authority(principal)?;
        release.ensure_threshold_approved()?;
        if release.released_at.is_none() {
            release.released_at = Some(OffsetDateTime::now_utc());
        }
        let release = release.clone();
        drop(releases);
        Ok(release)
    }

    pub(crate) async fn list_threshold_release_requests(
        &self,
        filter: ConfidentialThresholdReleaseFilter,
    ) -> Result<Vec<ConfidentialThresholdReleaseState>, ConfidentialThresholdReleaseError> {
        let mut releases = {
            let releases = self.threshold.releases.read().await;
            releases
                .values()
                .filter(|release| filter.matches(release))
                .cloned()
                .collect::<Vec<_>>()
        };
        releases.sort_by_key(|release| (release.created_at, release.release_id.bytes()));
        Ok(releases)
    }
}

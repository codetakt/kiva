#![allow(clippy::wildcard_imports)]

use super::*;

impl MemoryConfidentialRepository {
    pub(crate) async fn register_confidential_job(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialJobRequest,
    ) -> Result<ConfidentialJobRegistrationOutcome, ConfidentialJobError> {
        let job = ConfidentialJobState::from_request(principal, request)?;
        self.ensure_active_confidential_job_participants(principal, &job)
            .await?;
        let key = ConfidentialJobKey::from_job_id(job.job_id);
        let mut jobs = self.jobs.states.write().await;
        let outcome = match jobs.get(&key) {
            Some(existing) if existing.same_registration(&job) => {
                ConfidentialJobRegistrationOutcome::Unchanged(existing.clone())
            }
            Some(_existing) => ConfidentialJobRegistrationOutcome::Conflict,
            None => {
                jobs.insert(key, job.clone());
                ConfidentialJobRegistrationOutcome::Stored(job.clone())
            }
        };
        drop(jobs);
        if matches!(outcome, ConfidentialJobRegistrationOutcome::Stored(_)) {
            self.record_confidential_job_audit_event(
                principal,
                job.job_id,
                None,
                job.status,
                "job registered",
                &job.evidence_refs,
            )
            .await;
        }
        Ok(outcome)
    }

    pub(crate) async fn transition_confidential_job(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialJobTransitionRequest,
    ) -> Result<ConfidentialJobTransitionOutcome, ConfidentialJobError> {
        let key = ConfidentialJobKey::from_job_id(request.job_id);
        let now = OffsetDateTime::now_utc();
        let active_for_quota = self.active_jobs_for_quota_key(request.job_id).await;
        let running_jobs = self.running_jobs().await;
        let mut jobs = self.jobs.states.write().await;
        let Some(job) = jobs.get_mut(&key) else {
            return Err(ConfidentialJobError::NotFound);
        };
        job.ensure_owner(principal)?;
        if job.status == request.target_status {
            return Ok(ConfidentialJobTransitionOutcome::Unchanged(job.clone()));
        }
        validate_confidential_job_transition_request(
            job,
            &request,
            now,
            active_for_quota,
            running_jobs,
        )?;
        let previous_status = job.status;
        job.status = transition_confidential_job_status(job.status, request.target_status)
            .map_err(ConfidentialJobError::Transition)?;
        if job.status == ConfidentialJobStatus::Running {
            job.retry_count = job.retry_count.checked_add(1).ok_or_else(|| {
                ConfidentialJobError::InvalidRequest(
                    "confidential job retry counter overflowed".to_owned(),
                )
            })?;
        }
        append_unique_strings(&mut job.evidence_refs, &request.evidence_refs);
        job.updated_at = now;
        let job = job.clone();
        drop(jobs);
        self.record_confidential_job_audit_event(
            principal,
            job.job_id,
            Some(previous_status),
            job.status,
            request.reason.as_deref().unwrap_or("job state transition"),
            &request.evidence_refs,
        )
        .await;
        Ok(ConfidentialJobTransitionOutcome::Updated(job))
    }

    pub(crate) async fn list_confidential_jobs(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialJobFilter,
    ) -> Result<Vec<ConfidentialJobState>, ConfidentialJobError> {
        let mut jobs = {
            let jobs = self.jobs.states.read().await;
            jobs.values()
                .filter(|job| job.owner_subject == principal.subject)
                .filter(|job| filter.matches(job))
                .cloned()
                .collect::<Vec<_>>()
        };
        jobs.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.job_id.bytes().cmp(&right.job_id.bytes()))
        });
        Ok(jobs)
    }

    pub(crate) async fn list_confidential_job_audit_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialJobAuditEventFilter,
    ) -> Result<Vec<ConfidentialJobAuditEventState>, ConfidentialJobError> {
        let owned_job_ids = {
            let jobs = self.jobs.states.read().await;
            jobs.values()
                .filter(|job| job.owner_subject == principal.subject)
                .map(|job| job.job_id.bytes())
                .collect::<BTreeSet<_>>()
        };
        let mut events = {
            let events = self.jobs.audit_events.read().await;
            events
                .values()
                .filter(|event| owned_job_ids.contains(&event.job_id.bytes()))
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

    pub(crate) async fn ensure_active_confidential_job_participants(
        &self,
        principal: &AuthenticatedPrincipal,
        job: &ConfidentialJobState,
    ) -> Result<(), ConfidentialJobError> {
        for participant_subject in &job.participant_subjects {
            let active = {
                let participants = self.participants.states.read().await;
                participants.values().any(|participant| {
                    participant.owner_subject == principal.subject
                        && participant.participant_subject == *participant_subject
                        && confidential_participant_accepts_jobs(participant.status)
                })
            };
            if !active {
                return Err(ConfidentialJobError::InvalidRequest(format!(
                    "confidential job participant {participant_subject} must be active"
                )));
            }
        }
        Ok(())
    }
    pub(crate) async fn active_jobs_for_quota_key(
        &self,
        job_id: SecureAggregateTranscriptId,
    ) -> usize {
        let jobs = self.jobs.states.read().await;
        let Some(job) = jobs.get(&ConfidentialJobKey::from_job_id(job_id)) else {
            return 0;
        };
        let quota_key = job.admission_quota_key.clone();
        jobs.values()
            .filter(|candidate| candidate.admission_quota_key == quota_key)
            .filter(|candidate| {
                confidential_job_status_counts_against_admission_quota(candidate.status)
            })
            .count()
    }

    pub(crate) async fn running_jobs(&self) -> usize {
        let jobs = self.jobs.states.read().await;
        jobs.values()
            .filter(|job| job.status == ConfidentialJobStatus::Running)
            .count()
    }

    pub(crate) async fn record_confidential_job_audit_event(
        &self,
        principal: &AuthenticatedPrincipal,
        job_id: SecureAggregateTranscriptId,
        previous_status: Option<ConfidentialJobStatus>,
        new_status: ConfidentialJobStatus,
        reason: &str,
        evidence_refs: &[String],
    ) {
        let event = ConfidentialJobAuditEventState::new(
            job_id,
            previous_status,
            new_status,
            principal.subject.clone(),
            reason.to_owned(),
            evidence_refs.to_vec(),
        );
        let key = ConfidentialJobAuditEventKey::from_event_id(event.event_id);
        let mut events = self.jobs.audit_events.write().await;
        events.insert(key, event);
    }
}

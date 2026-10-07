#![allow(clippy::wildcard_imports)]

use super::*;

impl PostgresConfidentialRepository {
    pub(crate) async fn register_confidential_job(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialJobRequest,
    ) -> Result<ConfidentialJobRegistrationOutcome, ConfidentialJobError> {
        let job = ConfidentialJobState::from_request(principal, request)?;
        ensure_active_confidential_job_participants(&self.pool, &self.tenant_id, principal, &job)
            .await?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialJobError::Store)?;
        let row = sqlx::query_as::<_, ConfidentialJobRow>(
            "INSERT INTO kiva.confidential_jobs \
             (tenant_id, job_id, owner_subject, mechanism, mechanism_profile, workload_id, \
              participant_subjects, policy_binding_id, status, retry_count, max_retry_count, \
              deadline_at, admission_quota_key, evidence_refs) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) \
             ON CONFLICT (tenant_id, job_id) DO NOTHING \
             RETURNING job_id, owner_subject, mechanism, mechanism_profile, workload_id, \
              participant_subjects, policy_binding_id, status, retry_count, max_retry_count, \
              deadline_at, admission_quota_key, evidence_refs, created_at, updated_at",
        )
        .bind(&self.tenant_id)
        .bind(job.job_id.bytes().to_vec())
        .bind(&job.owner_subject)
        .bind(&job.mechanism)
        .bind(&job.mechanism_profile)
        .bind(&job.workload_id)
        .bind(&job.participant_subjects)
        .bind(&job.policy_binding_id)
        .bind(job.status.label())
        .bind(job.retry_count)
        .bind(job.max_retry_count)
        .bind(job.deadline_at)
        .bind(&job.admission_quota_key)
        .bind(&job.evidence_refs)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialJobError::Store)?;

        if let Some(row) = row {
            let stored_job = confidential_job_from_row(row)?;
            insert_confidential_job_audit_event_in_transaction(
                &mut transaction,
                &self.tenant_id,
                principal,
                stored_job.job_id,
                None,
                stored_job.status,
                "job registered",
                &stored_job.evidence_refs,
            )
            .await?;
            transaction
                .commit()
                .await
                .map_err(confidential_store)
                .map_err(ConfidentialJobError::Store)?;
            return Ok(ConfidentialJobRegistrationOutcome::Stored(stored_job));
        }

        transaction
            .commit()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialJobError::Store)?;
        let existing = select_confidential_job(&self.pool, &self.tenant_id, job.job_id)
            .await?
            .ok_or(ConfidentialJobError::NotFound)?;
        if existing.same_registration(&job) {
            Ok(ConfidentialJobRegistrationOutcome::Unchanged(existing))
        } else {
            Ok(ConfidentialJobRegistrationOutcome::Conflict)
        }
    }

    pub(crate) async fn transition_confidential_job(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialJobTransitionRequest,
    ) -> Result<ConfidentialJobTransitionOutcome, ConfidentialJobError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialJobError::Store)?;
        let mut job = select_confidential_job_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.job_id,
        )
        .await?
        .ok_or(ConfidentialJobError::NotFound)?;
        job.ensure_owner(principal)?;
        if job.status == request.target_status {
            return Ok(ConfidentialJobTransitionOutcome::Unchanged(job));
        }

        let now = OffsetDateTime::now_utc();
        let active_for_quota = count_confidential_active_jobs_for_quota_in_transaction(
            &mut transaction,
            &self.tenant_id,
            &job.admission_quota_key,
        )
        .await?;
        let running_jobs =
            count_confidential_running_jobs_in_transaction(&mut transaction, &self.tenant_id)
                .await?;
        validate_confidential_job_transition_request(
            &job,
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
        let updated =
            update_confidential_job_in_transaction(&mut transaction, &self.tenant_id, &job).await?;
        insert_confidential_job_audit_event_in_transaction(
            &mut transaction,
            &self.tenant_id,
            principal,
            updated.job_id,
            Some(previous_status),
            updated.status,
            request.reason.as_deref().unwrap_or("job state transition"),
            &request.evidence_refs,
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialJobError::Store)?;
        Ok(ConfidentialJobTransitionOutcome::Updated(updated))
    }

    pub(crate) async fn list_confidential_jobs(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialJobFilter,
    ) -> Result<Vec<ConfidentialJobState>, ConfidentialJobError> {
        let rows = sqlx::query_as::<_, ConfidentialJobRow>(
            "SELECT job_id, owner_subject, mechanism, mechanism_profile, workload_id, \
             participant_subjects, policy_binding_id, status, retry_count, max_retry_count, \
             deadline_at, admission_quota_key, evidence_refs, created_at, updated_at \
             FROM kiva.confidential_jobs \
             WHERE tenant_id = $1 AND owner_subject = $2 \
              AND ($3::bytea IS NULL OR job_id = $3) \
              AND ($4::text IS NULL OR status = $4) \
              AND ($5::text IS NULL OR mechanism = $5) \
             ORDER BY updated_at DESC, job_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(&principal.subject)
        .bind(filter.job_id.map(|job_id| job_id.bytes().to_vec()))
        .bind(filter.status.map(ConfidentialJobStatus::label))
        .bind(filter.mechanism)
        .fetch_all(&self.pool)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialJobError::Store)?;

        rows.into_iter().map(confidential_job_from_row).collect()
    }

    pub(crate) async fn list_confidential_job_audit_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialJobAuditEventFilter,
    ) -> Result<Vec<ConfidentialJobAuditEventState>, ConfidentialJobError> {
        let rows = sqlx::query_as::<_, ConfidentialJobAuditEventRow>(
            "SELECT event_id, event.job_id, previous_status, new_status, actor_subject, \
              reason, event.evidence_refs, event.created_at \
             FROM kiva.confidential_job_audit_events AS event \
             INNER JOIN kiva.confidential_jobs AS job \
              ON job.tenant_id = event.tenant_id AND job.job_id = event.job_id \
             WHERE event.tenant_id = $1 AND job.owner_subject = $2 \
              AND ($3::bytea IS NULL OR event.job_id = $3) \
             ORDER BY event.created_at DESC, event.event_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(&principal.subject)
        .bind(filter.job_id.map(|job_id| job_id.bytes().to_vec()))
        .fetch_all(&self.pool)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialJobError::Store)?;

        rows.into_iter()
            .map(confidential_job_audit_event_from_row)
            .collect()
    }
}

#![allow(clippy::wildcard_imports)]

use super::*;

pub(crate) async fn select_confidential_job(
    pool: &PgPool,
    tenant_id: &str,
    job_id: SecureAggregateTranscriptId,
) -> Result<Option<ConfidentialJobState>, ConfidentialJobError> {
    let row = sqlx::query_as::<_, ConfidentialJobRow>(
        "SELECT job_id, owner_subject, mechanism, mechanism_profile, workload_id, \
         participant_subjects, policy_binding_id, status, retry_count, max_retry_count, \
         deadline_at, admission_quota_key, evidence_refs, created_at, updated_at \
         FROM kiva.confidential_jobs \
         WHERE tenant_id = $1 AND job_id = $2",
    )
    .bind(tenant_id)
    .bind(job_id.bytes().to_vec())
    .fetch_optional(pool)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialJobError::Store)?;

    row.map(confidential_job_from_row).transpose()
}

pub(crate) async fn select_confidential_job_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    job_id: SecureAggregateTranscriptId,
) -> Result<Option<ConfidentialJobState>, ConfidentialJobError> {
    let row = sqlx::query_as::<_, ConfidentialJobRow>(
        "SELECT job_id, owner_subject, mechanism, mechanism_profile, workload_id, \
         participant_subjects, policy_binding_id, status, retry_count, max_retry_count, \
         deadline_at, admission_quota_key, evidence_refs, created_at, updated_at \
         FROM kiva.confidential_jobs \
         WHERE tenant_id = $1 AND job_id = $2 \
         FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(job_id.bytes().to_vec())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialJobError::Store)?;

    row.map(confidential_job_from_row).transpose()
}

pub(crate) async fn update_confidential_job_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    job: &ConfidentialJobState,
) -> Result<ConfidentialJobState, ConfidentialJobError> {
    let row = sqlx::query_as::<_, ConfidentialJobRow>(
        "UPDATE kiva.confidential_jobs \
         SET status = $3, retry_count = $4, evidence_refs = $5, updated_at = $6 \
         WHERE tenant_id = $1 AND job_id = $2 \
         RETURNING job_id, owner_subject, mechanism, mechanism_profile, workload_id, \
          participant_subjects, policy_binding_id, status, retry_count, max_retry_count, \
          deadline_at, admission_quota_key, evidence_refs, created_at, updated_at",
    )
    .bind(tenant_id)
    .bind(job.job_id.bytes().to_vec())
    .bind(job.status.label())
    .bind(job.retry_count)
    .bind(&job.evidence_refs)
    .bind(job.updated_at)
    .fetch_one(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialJobError::Store)?;

    confidential_job_from_row(row)
}

pub(crate) async fn count_confidential_active_jobs_for_quota_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    quota_key: &str,
) -> Result<usize, ConfidentialJobError> {
    let count: (i64,) = sqlx::query_as(
        "SELECT count(*)::bigint \
         FROM kiva.confidential_jobs \
         WHERE tenant_id = $1 AND admission_quota_key = $2 \
          AND status IN ('admitted', 'running', 'awaitingRelease')",
    )
    .bind(tenant_id)
    .bind(quota_key)
    .fetch_one(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialJobError::Store)?;
    usize::try_from(count.0).map_err(|_error| {
        ConfidentialJobError::Store(ConfidentialAggregateStoreError::InvalidStoredConfidentialJob)
    })
}

pub(crate) async fn count_confidential_running_jobs_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
) -> Result<usize, ConfidentialJobError> {
    let count: (i64,) = sqlx::query_as(
        "SELECT count(*)::bigint \
         FROM kiva.confidential_jobs \
         WHERE tenant_id = $1 AND status = 'running'",
    )
    .bind(tenant_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialJobError::Store)?;
    usize::try_from(count.0).map_err(|_error| {
        ConfidentialJobError::Store(ConfidentialAggregateStoreError::InvalidStoredConfidentialJob)
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn insert_confidential_job_audit_event_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    principal: &AuthenticatedPrincipal,
    job_id: SecureAggregateTranscriptId,
    previous_status: Option<ConfidentialJobStatus>,
    new_status: ConfidentialJobStatus,
    reason: &str,
    evidence_refs: &[String],
) -> Result<(), ConfidentialJobError> {
    sqlx::query(
        "INSERT INTO kiva.confidential_job_audit_events \
         (tenant_id, event_id, job_id, previous_status, new_status, actor_subject, reason, evidence_refs) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(tenant_id)
    .bind(Uuid::new_v4())
    .bind(job_id.bytes().to_vec())
    .bind(previous_status.map(ConfidentialJobStatus::label))
    .bind(new_status.label())
    .bind(&principal.subject)
    .bind(reason)
    .bind(evidence_refs)
    .execute(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialJobError::Store)?;
    Ok(())
}

pub(crate) async fn ensure_active_confidential_job_participants(
    pool: &PgPool,
    tenant_id: &str,
    principal: &AuthenticatedPrincipal,
    job: &ConfidentialJobState,
) -> Result<(), ConfidentialJobError> {
    for participant_subject in &job.participant_subjects {
        let rows = sqlx::query_as::<_, ConfidentialParticipantRow>(
            "SELECT owner_subject, participant_subject, participant_index, status, \
              mechanism_roles, authorization_scope, key_ceremony_ref, recovery_contact_ref, \
              evidence_refs, created_at, updated_at \
             FROM kiva.confidential_participants \
             WHERE tenant_id = $1 AND owner_subject = $2 AND participant_subject = $3 \
             ORDER BY participant_index ASC",
        )
        .bind(tenant_id)
        .bind(&principal.subject)
        .bind(participant_subject)
        .fetch_all(pool)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialJobError::Store)?;

        let mut active = false;
        for row in rows {
            let participant =
                confidential_participant_from_row(row).map_err(|error| match error {
                    ConfidentialParticipantError::Store(error) => {
                        ConfidentialJobError::Store(error)
                    }
                    ConfidentialParticipantError::InvalidRequest(_)
                    | ConfidentialParticipantError::ApplicationAdmission(_)
                    | ConfidentialParticipantError::Transition(_)
                    | ConfidentialParticipantError::NotFound
                    | ConfidentialParticipantError::Forbidden
                    | ConfidentialParticipantError::Conflict
                    | ConfidentialParticipantError::Inactive => ConfidentialJobError::Store(
                        ConfidentialAggregateStoreError::InvalidStoredConfidentialParticipant,
                    ),
                })?;
            if confidential_participant_accepts_jobs(participant.status) {
                active = true;
                break;
            }
        }
        if !active {
            return Err(ConfidentialJobError::InvalidRequest(format!(
                "confidential job participant {participant_subject} must be active"
            )));
        }
    }
    Ok(())
}

pub(crate) async fn ensure_active_confidential_participant(
    pool: &PgPool,
    tenant_id: &str,
    owner_subject: &str,
    participant_subject: &str,
    participant_index: NonZeroUsize,
) -> Result<(), ConfidentialParticipantError> {
    let participant = select_confidential_participant(
        pool,
        tenant_id,
        owner_subject,
        participant_subject,
        participant_index,
    )
    .await?
    .ok_or(ConfidentialParticipantError::NotFound)?;
    if confidential_participant_accepts_new_keys(participant.status) {
        Ok(())
    } else {
        Err(ConfidentialParticipantError::Inactive)
    }
}

pub(crate) async fn ensure_active_confidential_participant_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    owner_subject: &str,
    participant_subject: &str,
    participant_index: NonZeroUsize,
) -> Result<(), ConfidentialParticipantError> {
    let participant = select_confidential_participant_in_transaction(
        transaction,
        tenant_id,
        owner_subject,
        participant_subject,
        participant_index,
    )
    .await?
    .ok_or(ConfidentialParticipantError::NotFound)?;
    if confidential_participant_accepts_new_keys(participant.status) {
        Ok(())
    } else {
        Err(ConfidentialParticipantError::Inactive)
    }
}

pub(crate) async fn select_confidential_participant(
    pool: &PgPool,
    tenant_id: &str,
    owner_subject: &str,
    participant_subject: &str,
    participant_index: NonZeroUsize,
) -> Result<Option<ConfidentialParticipantState>, ConfidentialParticipantError> {
    let row = sqlx::query_as::<_, ConfidentialParticipantRow>(
        "SELECT owner_subject, participant_subject, participant_index, status, \
          mechanism_roles, authorization_scope, key_ceremony_ref, recovery_contact_ref, \
          evidence_refs, created_at, updated_at \
         FROM kiva.confidential_participants \
         WHERE tenant_id = $1 AND owner_subject = $2 AND participant_subject = $3 \
          AND participant_index = $4",
    )
    .bind(tenant_id)
    .bind(owner_subject)
    .bind(participant_subject)
    .bind(nonzero_usize_i64(participant_index).map_err(ConfidentialParticipantError::Store)?)
    .fetch_optional(pool)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialParticipantError::Store)?;

    row.map(confidential_participant_from_row).transpose()
}

pub(crate) async fn select_confidential_participant_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    owner_subject: &str,
    participant_subject: &str,
    participant_index: NonZeroUsize,
) -> Result<Option<ConfidentialParticipantState>, ConfidentialParticipantError> {
    let row = sqlx::query_as::<_, ConfidentialParticipantRow>(
        "SELECT owner_subject, participant_subject, participant_index, status, \
          mechanism_roles, authorization_scope, key_ceremony_ref, recovery_contact_ref, \
          evidence_refs, created_at, updated_at \
         FROM kiva.confidential_participants \
         WHERE tenant_id = $1 AND owner_subject = $2 AND participant_subject = $3 \
          AND participant_index = $4 \
         FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(owner_subject)
    .bind(participant_subject)
    .bind(nonzero_usize_i64(participant_index).map_err(ConfidentialParticipantError::Store)?)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialParticipantError::Store)?;

    row.map(confidential_participant_from_row).transpose()
}

pub(crate) async fn update_confidential_participant_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    participant: &ConfidentialParticipantState,
) -> Result<ConfidentialParticipantState, ConfidentialParticipantError> {
    let row = sqlx::query_as::<_, ConfidentialParticipantRow>(
        "UPDATE kiva.confidential_participants \
         SET status = $5, evidence_refs = $6, updated_at = $7 \
         WHERE tenant_id = $1 AND owner_subject = $2 AND participant_subject = $3 \
          AND participant_index = $4 \
         RETURNING owner_subject, participant_subject, participant_index, status, \
          mechanism_roles, authorization_scope, key_ceremony_ref, recovery_contact_ref, \
          evidence_refs, created_at, updated_at",
    )
    .bind(tenant_id)
    .bind(&participant.owner_subject)
    .bind(&participant.participant_subject)
    .bind(
        nonzero_usize_i64(participant.participant_index)
            .map_err(ConfidentialParticipantError::Store)?,
    )
    .bind(participant.status.label())
    .bind(&participant.evidence_refs)
    .bind(participant.updated_at)
    .fetch_one(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialParticipantError::Store)?;

    confidential_participant_from_row(row)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn insert_confidential_participant_audit_event_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    principal: &AuthenticatedPrincipal,
    participant: &ConfidentialParticipantState,
    previous_status: Option<ConfidentialParticipantStatus>,
    new_status: ConfidentialParticipantStatus,
    reason: &str,
    evidence_refs: &[String],
) -> Result<(), ConfidentialParticipantError> {
    sqlx::query(
        "INSERT INTO kiva.confidential_participant_audit_events \
         (tenant_id, event_id, owner_subject, participant_subject, participant_index, \
          previous_status, new_status, actor_subject, reason, evidence_refs) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(tenant_id)
    .bind(Uuid::new_v4())
    .bind(&participant.owner_subject)
    .bind(&participant.participant_subject)
    .bind(
        nonzero_usize_i64(participant.participant_index)
            .map_err(ConfidentialParticipantError::Store)?,
    )
    .bind(previous_status.map(ConfidentialParticipantStatus::label))
    .bind(new_status.label())
    .bind(&principal.subject)
    .bind(reason)
    .bind(evidence_refs)
    .execute(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialParticipantError::Store)?;
    Ok(())
}

pub(super) fn confidential_participant_from_row(
    row: ConfidentialParticipantRow,
) -> Result<ConfidentialParticipantState, ConfidentialParticipantError> {
    validate_non_empty_confidential_participant_text("owner_subject", &row.owner_subject)?;
    validate_non_empty_confidential_participant_text(
        "participant_subject",
        &row.participant_subject,
    )?;
    validate_non_empty_confidential_participant_text(
        "authorization_scope",
        &row.authorization_scope,
    )?;
    validate_non_empty_confidential_participant_text("key_ceremony_ref", &row.key_ceremony_ref)?;
    validate_non_empty_confidential_participant_text(
        "recovery_contact_ref",
        &row.recovery_contact_ref,
    )?;
    validate_confidential_participant_roles(&row.mechanism_roles)?;
    validate_confidential_participant_evidence_refs(&row.evidence_refs)?;
    let participant_index = usize::try_from(row.participant_index)
        .ok()
        .and_then(NonZeroUsize::new)
        .ok_or(ConfidentialParticipantError::Store(
            ConfidentialAggregateStoreError::InvalidStoredConfidentialParticipant,
        ))?;
    let status = ConfidentialParticipantStatus::from_label(&row.status).ok_or(
        ConfidentialParticipantError::Store(
            ConfidentialAggregateStoreError::InvalidStoredConfidentialParticipant,
        ),
    )?;
    if row.updated_at < row.created_at {
        return Err(ConfidentialParticipantError::Store(
            ConfidentialAggregateStoreError::InvalidStoredConfidentialParticipant,
        ));
    }

    Ok(ConfidentialParticipantState {
        owner_subject: row.owner_subject.trim().to_owned(),
        participant_subject: row.participant_subject.trim().to_owned(),
        participant_index,
        status,
        mechanism_roles: canonical_unique_strings(row.mechanism_roles),
        authorization_scope: row.authorization_scope.trim().to_owned(),
        key_ceremony_ref: row.key_ceremony_ref.trim().to_owned(),
        recovery_contact_ref: row.recovery_contact_ref.trim().to_owned(),
        evidence_refs: canonical_unique_strings(row.evidence_refs),
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub(super) fn confidential_participant_audit_event_from_row(
    row: ConfidentialParticipantAuditEventRow,
) -> Result<ConfidentialParticipantAuditEventState, ConfidentialParticipantError> {
    validate_non_empty_confidential_participant_text(
        "participant_subject",
        &row.participant_subject,
    )?;
    validate_non_empty_confidential_participant_text("actor_subject", &row.actor_subject)?;
    validate_non_empty_confidential_participant_text("reason", &row.reason)?;
    validate_confidential_participant_evidence_refs(&row.evidence_refs)?;
    let participant_index = usize::try_from(row.participant_index)
        .ok()
        .and_then(NonZeroUsize::new)
        .ok_or(ConfidentialParticipantError::Store(
            ConfidentialAggregateStoreError::InvalidStoredConfidentialParticipant,
        ))?;
    let previous_status = row
        .previous_status
        .as_deref()
        .map(|status| {
            ConfidentialParticipantStatus::from_label(status).ok_or(
                ConfidentialParticipantError::Store(
                    ConfidentialAggregateStoreError::InvalidStoredConfidentialParticipant,
                ),
            )
        })
        .transpose()?;
    let new_status = ConfidentialParticipantStatus::from_label(&row.new_status).ok_or(
        ConfidentialParticipantError::Store(
            ConfidentialAggregateStoreError::InvalidStoredConfidentialParticipant,
        ),
    )?;

    Ok(ConfidentialParticipantAuditEventState {
        event_id: row.event_id,
        participant_subject: row.participant_subject.trim().to_owned(),
        participant_index,
        previous_status,
        new_status,
        actor_subject: row.actor_subject.trim().to_owned(),
        reason: row.reason.trim().to_owned(),
        evidence_refs: canonical_unique_strings(row.evidence_refs),
        created_at: row.created_at,
    })
}

#![allow(clippy::wildcard_imports)]

use super::*;

pub(crate) async fn select_confidential(
    pool: &PgPool,
    tenant_id: &str,
    transcript_id: SecureAggregateTranscriptId,
) -> Result<Vec<AcceptedAdditiveShareRecord>, ConfidentialAggregateStoreError> {
    let rows = sqlx::query_as::<_, ConfidentialReplayRecordRow>(
        "SELECT transcript_id, participant_index, sequence::text AS sequence, signing_key_id \
         FROM kiva.confidential_additive_share_replay_records \
         WHERE tenant_id = $1 AND transcript_id = $2 \
         ORDER BY participant_index ASC",
    )
    .bind(tenant_id)
    .bind(transcript_id.bytes().to_vec())
    .fetch_all(pool)
    .await
    .map_err(confidential_store)?;

    rows.into_iter()
        .map(confidential_replay_record_from_row)
        .collect()
}

pub(crate) async fn select_differential_privacy_budget(
    pool: &PgPool,
    tenant_id: &str,
    budget_id: SecureAggregateTranscriptId,
) -> Result<
    Option<ConfidentialDifferentialPrivacyBudgetState>,
    ConfidentialDifferentialPrivacyBudgetError,
> {
    let row = sqlx::query_as::<_, ConfidentialDifferentialPrivacyBudgetRow>(
        "SELECT budget_id, owner_subject, total_epsilon_microunits::text AS total_epsilon_microunits, \
         spent_epsilon_microunits::text AS spent_epsilon_microunits, created_at, updated_at \
         FROM kiva.confidential_differential_privacy_budgets \
         WHERE tenant_id = $1 AND budget_id = $2",
    )
    .bind(tenant_id)
    .bind(budget_id.bytes().to_vec())
    .fetch_optional(pool)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialDifferentialPrivacyBudgetError::Store)?;

    row.map(differential_privacy_budget_from_row)
        .transpose()
        .map_err(ConfidentialDifferentialPrivacyBudgetError::Store)
}

pub(crate) async fn select_differential_privacy_budget_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    budget_id: SecureAggregateTranscriptId,
) -> Result<
    Option<ConfidentialDifferentialPrivacyBudgetState>,
    ConfidentialDifferentialPrivacyBudgetError,
> {
    let row = sqlx::query_as::<_, ConfidentialDifferentialPrivacyBudgetRow>(
        "SELECT budget_id, owner_subject, total_epsilon_microunits::text AS total_epsilon_microunits, \
         spent_epsilon_microunits::text AS spent_epsilon_microunits, created_at, updated_at \
         FROM kiva.confidential_differential_privacy_budgets \
         WHERE tenant_id = $1 AND budget_id = $2 \
         FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(budget_id.bytes().to_vec())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialDifferentialPrivacyBudgetError::Store)?;

    row.map(differential_privacy_budget_from_row)
        .transpose()
        .map_err(ConfidentialDifferentialPrivacyBudgetError::Store)
}

pub(crate) async fn select_differential_privacy_release_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    release_id: SecureAggregateTranscriptId,
) -> Result<
    Option<ConfidentialDifferentialPrivacyReleaseState>,
    ConfidentialDifferentialPrivacyReleaseError,
> {
    let row = sqlx::query_as::<_, ConfidentialDifferentialPrivacyReleaseRow>(
        "SELECT release_id, budget_id, transcript_id, owner_subject, aggregate_value, \
         noise_value, noised_value, mechanism, accounting_scope, epsilon_microunits::text AS epsilon_microunits, \
         l1_sensitivity::text AS l1_sensitivity, data_subject_count, minimum_population, created_at \
         FROM kiva.confidential_differential_privacy_releases \
         WHERE tenant_id = $1 AND release_id = $2 \
         FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(release_id.bytes().to_vec())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialDifferentialPrivacyReleaseError::Store)?;

    row.map(differential_privacy_release_from_row)
        .transpose()
        .map_err(ConfidentialDifferentialPrivacyReleaseError::Store)
}

pub(crate) async fn select_confidential_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    transcript_id: SecureAggregateTranscriptId,
) -> Result<Vec<AcceptedAdditiveShareRecord>, ConfidentialAggregateStoreError> {
    let rows = sqlx::query_as::<_, ConfidentialReplayRecordRow>(
        "SELECT transcript_id, participant_index, sequence::text AS sequence, signing_key_id \
         FROM kiva.confidential_additive_share_replay_records \
         WHERE tenant_id = $1 AND transcript_id = $2 \
         ORDER BY participant_index ASC",
    )
    .bind(tenant_id)
    .bind(transcript_id.bytes().to_vec())
    .fetch_all(&mut **transaction)
    .await
    .map_err(confidential_store)?;

    rows.into_iter()
        .map(confidential_replay_record_from_row)
        .collect()
}

pub(crate) async fn insert_confidential_replay_record_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    record: AcceptedAdditiveShareRecord,
) -> Result<InsertOutcome, ConfidentialAggregateStoreError> {
    let result = sqlx::query(
        "INSERT INTO kiva.confidential_additive_share_replay_records \
         (tenant_id, transcript_id, participant_index, sequence, signing_key_id) \
         VALUES ($1, $2, $3, ($4)::numeric, $5) \
         ON CONFLICT (tenant_id, transcript_id, participant_index) DO NOTHING",
    )
    .bind(tenant_id)
    .bind(record.transcript_id().bytes().to_vec())
    .bind(participant_index_i64(record)?)
    .bind(record.sequence().value().get().to_string())
    .bind(record.signing_key_id().bytes().to_vec())
    .execute(&mut **transaction)
    .await
    .map_err(confidential_store)?;

    Ok(InsertOutcome::from_rows_affected(result.rows_affected()))
}

pub(crate) async fn select_confidential_additive_aggregate_result(
    pool: &PgPool,
    tenant_id: &str,
    transcript_id: SecureAggregateTranscriptId,
) -> Result<Option<ConfidentialAdditiveAggregateResult>, ConfidentialAggregateStoreError> {
    let row = sqlx::query_as::<_, ConfidentialAggregateResultRow>(
        "SELECT transcript_id, aggregate_value, participant_count, computed_at \
         FROM kiva.confidential_additive_aggregate_results \
         WHERE tenant_id = $1 AND transcript_id = $2",
    )
    .bind(tenant_id)
    .bind(transcript_id.bytes().to_vec())
    .fetch_optional(pool)
    .await
    .map_err(confidential_store)?;

    row.map(confidential_additive_aggregate_result_from_row)
        .transpose()
}

pub(crate) async fn select_confidential_additive_aggregate_result_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    transcript_id: SecureAggregateTranscriptId,
) -> Result<Option<ConfidentialAdditiveAggregateResult>, ConfidentialAggregateStoreError> {
    let row = sqlx::query_as::<_, ConfidentialAggregateResultRow>(
        "SELECT transcript_id, aggregate_value, participant_count, computed_at \
         FROM kiva.confidential_additive_aggregate_results \
         WHERE tenant_id = $1 AND transcript_id = $2",
    )
    .bind(tenant_id)
    .bind(transcript_id.bytes().to_vec())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(confidential_store)?;

    row.map(confidential_additive_aggregate_result_from_row)
        .transpose()
}

pub(crate) async fn select_shamir_threshold_reconstruction(
    pool: &PgPool,
    tenant_id: &str,
    reconstruction_id: SecureAggregateTranscriptId,
) -> Result<
    Option<ConfidentialShamirThresholdReconstructionState>,
    ConfidentialShamirReconstructionError,
> {
    let row = sqlx::query_as::<_, ConfidentialShamirThresholdReconstructionRow>(
        "SELECT reconstruction_id, transcript_id, owner_subject, participant_count, threshold, \
         share_count, reconstructed_value, verification_profile, verified_share_count, \
         complaint_count, created_at \
         FROM kiva.confidential_shamir_threshold_reconstructions \
         WHERE tenant_id = $1 AND reconstruction_id = $2",
    )
    .bind(tenant_id)
    .bind(reconstruction_id.bytes().to_vec())
    .fetch_optional(pool)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialShamirReconstructionError::Store)?;

    row.map(shamir_threshold_reconstruction_from_row)
        .transpose()
        .map_err(ConfidentialShamirReconstructionError::Store)
}

pub(crate) async fn insert_confidential_additive_aggregate_result_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    result: &ConfidentialAdditiveAggregateResult,
) -> Result<InsertOutcome, ConfidentialAggregateError> {
    let insert = sqlx::query(
        "INSERT INTO kiva.confidential_additive_aggregate_results \
         (tenant_id, transcript_id, aggregate_value, participant_count) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (tenant_id, transcript_id) DO NOTHING",
    )
    .bind(tenant_id)
    .bind(result.transcript_id.bytes().to_vec())
    .bind(field_element_i64(result.aggregate).map_err(ConfidentialAggregateError::Secure)?)
    .bind(nonzero_usize_i64(result.participant_count).map_err(ConfidentialAggregateError::Store)?)
    .execute(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialAggregateError::Store)?;

    Ok(InsertOutcome::from_rows_affected(insert.rows_affected()))
}

pub(super) fn confidential_additive_aggregate_result_from_row(
    row: ConfidentialAggregateResultRow,
) -> Result<ConfidentialAdditiveAggregateResult, ConfidentialAggregateStoreError> {
    let transcript_id = SecureAggregateTranscriptId::new(fixed_32_bytes(row.transcript_id)?)
        .map_err(ConfidentialAggregateStoreError::InvalidStoredReplayRecord)?;
    let aggregate = u64::try_from(row.aggregate_value)
        .ok()
        .and_then(|value| SecureAggregateFieldElement::new(value).ok())
        .ok_or(ConfidentialAggregateStoreError::InvalidStoredReplayRecord(
            SecureAggregateError::ValueOutsideField,
        ))?;
    let participant_count = usize::try_from(row.participant_count)
        .ok()
        .and_then(std::num::NonZeroUsize::new)
        .ok_or(ConfidentialAggregateStoreError::ParticipantIndexTooLarge)?;

    Ok(ConfidentialAdditiveAggregateResult {
        transcript_id,
        aggregate,
        participant_count,
        computed_at: row.computed_at,
    })
}

pub(super) fn shamir_threshold_reconstruction_from_row(
    row: ConfidentialShamirThresholdReconstructionRow,
) -> Result<ConfidentialShamirThresholdReconstructionState, ConfidentialAggregateStoreError> {
    let reconstruction_id =
        SecureAggregateTranscriptId::new(fixed_32_bytes(row.reconstruction_id)?)
            .map_err(|_error| ConfidentialAggregateStoreError::InvalidStoredShamirReconstruction)?;
    let transcript_id = SecureAggregateTranscriptId::new(fixed_32_bytes(row.transcript_id)?)
        .map_err(|_error| ConfidentialAggregateStoreError::InvalidStoredShamirReconstruction)?;
    let participant_count = usize::try_from(row.participant_count)
        .ok()
        .and_then(std::num::NonZeroUsize::new)
        .ok_or(ConfidentialAggregateStoreError::InvalidStoredShamirReconstruction)?;
    let threshold = usize::try_from(row.threshold)
        .ok()
        .and_then(std::num::NonZeroUsize::new)
        .ok_or(ConfidentialAggregateStoreError::InvalidStoredShamirReconstruction)?;
    let share_count = usize::try_from(row.share_count)
        .map_err(|_error| ConfidentialAggregateStoreError::InvalidStoredShamirReconstruction)?;
    let verified_share_count = usize::try_from(row.verified_share_count)
        .map_err(|_error| ConfidentialAggregateStoreError::InvalidStoredShamirReconstruction)?;
    let complaint_count = usize::try_from(row.complaint_count)
        .map_err(|_error| ConfidentialAggregateStoreError::InvalidStoredShamirReconstruction)?;
    let reconstructed_value = u64::try_from(row.reconstructed_value)
        .ok()
        .and_then(|value| SecureAggregateFieldElement::new(value).ok())
        .ok_or(ConfidentialAggregateStoreError::InvalidStoredShamirReconstruction)?;
    let release_policy = ReleasePolicy::new(participant_count, threshold)
        .map_err(|_error| ConfidentialAggregateStoreError::InvalidStoredShamirReconstruction)?;
    ShamirThresholdPlan::new(release_policy)
        .map_err(|_error| ConfidentialAggregateStoreError::InvalidStoredShamirReconstruction)?;
    if row.verification_profile != CONFIDENTIAL_SHAMIR_VERIFICATION_PROFILE
        || verified_share_count < threshold.get()
        || verified_share_count > share_count
        || complaint_count > share_count
    {
        return Err(ConfidentialAggregateStoreError::InvalidStoredShamirReconstruction);
    }

    Ok(ConfidentialShamirThresholdReconstructionState {
        reconstruction_id,
        transcript_id,
        owner_subject: row.owner_subject,
        participant_count,
        threshold,
        share_count,
        verification_profile: row.verification_profile,
        verified_share_count,
        complaint_count,
        reconstructed_value,
        created_at: row.created_at,
    })
}

pub(super) fn differential_privacy_budget_from_row(
    row: ConfidentialDifferentialPrivacyBudgetRow,
) -> Result<ConfidentialDifferentialPrivacyBudgetState, ConfidentialAggregateStoreError> {
    let budget_id =
        SecureAggregateTranscriptId::new(fixed_32_bytes(row.budget_id)?).map_err(|_error| {
            ConfidentialAggregateStoreError::InvalidStoredDifferentialPrivacyBudget(
                DifferentialPrivacyBudgetError::SpentBudgetExceedsTotal,
            )
        })?;
    let total_epsilon_microunits = row
        .total_epsilon_microunits
        .parse::<u64>()
        .ok()
        .and_then(NonZeroU64::new)
        .ok_or(
            ConfidentialAggregateStoreError::InvalidStoredDifferentialPrivacyBudget(
                DifferentialPrivacyBudgetError::SpentBudgetExceedsTotal,
            ),
        )?;
    let spent_epsilon_microunits =
        row.spent_epsilon_microunits
            .parse::<u64>()
            .map_err(|_error| {
                ConfidentialAggregateStoreError::InvalidStoredDifferentialPrivacyBudget(
                    DifferentialPrivacyBudgetError::SpentBudgetExceedsTotal,
                )
            })?;
    let ledger =
        DifferentialPrivacyBudgetLedger::new(total_epsilon_microunits, spent_epsilon_microunits)
            .map_err(ConfidentialAggregateStoreError::InvalidStoredDifferentialPrivacyBudget)?;

    Ok(ConfidentialDifferentialPrivacyBudgetState {
        budget_id,
        owner_subject: row.owner_subject,
        ledger,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub(super) fn differential_privacy_release_from_row(
    row: ConfidentialDifferentialPrivacyReleaseRow,
) -> Result<ConfidentialDifferentialPrivacyReleaseState, ConfidentialAggregateStoreError> {
    let release_id = SecureAggregateTranscriptId::new(fixed_32_bytes(row.release_id)?)
        .map_err(|_error| invalid_differential_privacy_release_store_error())?;
    let budget_id = SecureAggregateTranscriptId::new(fixed_32_bytes(row.budget_id)?)
        .map_err(|_error| invalid_differential_privacy_release_store_error())?;
    let transcript_id = SecureAggregateTranscriptId::new(fixed_32_bytes(row.transcript_id)?)
        .map_err(|_error| invalid_differential_privacy_release_store_error())?;
    if row.owner_subject.is_empty() {
        return Err(invalid_differential_privacy_release_store_error());
    }
    let mechanism = match row.mechanism.as_str() {
        "discrete-laplace" => DifferentialPrivacyNoiseMechanism::DiscreteLaplace,
        _other => return Err(invalid_differential_privacy_release_store_error()),
    };
    let accounting_scope = match row.accounting_scope.as_str() {
        "subject-budget" => DifferentialPrivacyAccountingScope::SubjectBudget,
        _other => return Err(invalid_differential_privacy_release_store_error()),
    };
    let epsilon_microunits = row
        .epsilon_microunits
        .parse::<u64>()
        .ok()
        .and_then(NonZeroU64::new)
        .ok_or_else(invalid_differential_privacy_release_store_error)?;
    let l1_sensitivity = row
        .l1_sensitivity
        .parse::<u64>()
        .ok()
        .and_then(NonZeroU64::new)
        .ok_or_else(invalid_differential_privacy_release_store_error)?;
    let aggregate_is_field_element = u64::try_from(row.aggregate_value)
        .ok()
        .and_then(|value| SecureAggregateFieldElement::new(value).ok())
        .is_some();
    if !aggregate_is_field_element {
        return Err(invalid_differential_privacy_release_store_error());
    }

    let parameters = DifferentialPrivacyNoiseParameters::new(
        mechanism,
        accounting_scope,
        epsilon_microunits,
        l1_sensitivity,
    );
    let noised_aggregate =
        apply_differential_privacy_integer_noise(parameters, row.aggregate_value, row.noise_value)
            .map_err(|_error| invalid_differential_privacy_release_store_error())?;
    if noised_aggregate.noised_value() != row.noised_value {
        return Err(invalid_differential_privacy_release_store_error());
    }

    let data_subject_count = usize::try_from(row.data_subject_count)
        .ok()
        .and_then(NonZeroUsize::new)
        .ok_or_else(invalid_differential_privacy_release_store_error)?;
    let minimum_population = usize::try_from(row.minimum_population)
        .ok()
        .and_then(NonZeroUsize::new)
        .ok_or_else(invalid_differential_privacy_release_store_error)?;
    let policy = DifferentialPrivacyReleasePolicy::new(minimum_population, epsilon_microunits)
        .map_err(|_error| invalid_differential_privacy_release_store_error())?;
    let context = DifferentialPrivacyReleaseContext::new(
        true,
        data_subject_count.get(),
        epsilon_microunits.get(),
        noised_aggregate.noise_applied(),
    );
    validate_differential_privacy_release(policy, context)
        .map_err(|_error| invalid_differential_privacy_release_store_error())?;

    Ok(ConfidentialDifferentialPrivacyReleaseState {
        release_id,
        budget_id,
        transcript_id,
        owner_subject: row.owner_subject,
        noised_aggregate,
        data_subject_count,
        minimum_population,
        created_at: row.created_at,
    })
}

pub(super) fn confidential_job_from_row(
    row: ConfidentialJobRow,
) -> Result<ConfidentialJobState, ConfidentialJobError> {
    let job_id = SecureAggregateTranscriptId::new(
        fixed_32_bytes(row.job_id).map_err(ConfidentialJobError::Store)?,
    )
    .map_err(|_error| {
        ConfidentialJobError::Store(ConfidentialAggregateStoreError::InvalidStoredConfidentialJob)
    })?;
    let status = ConfidentialJobStatus::from_label(&row.status).ok_or(
        ConfidentialJobError::Store(ConfidentialAggregateStoreError::InvalidStoredConfidentialJob),
    )?;
    validate_confidential_job_mechanism(&row.mechanism)?;
    validate_non_empty_confidential_job_text("owner_subject", &row.owner_subject)?;
    validate_non_empty_confidential_job_text("mechanism_profile", &row.mechanism_profile)?;
    validate_non_empty_confidential_job_text("workload_id", &row.workload_id)?;
    validate_non_empty_confidential_job_text("policy_binding_id", &row.policy_binding_id)?;
    validate_non_empty_confidential_job_text("admission_quota_key", &row.admission_quota_key)?;
    validate_confidential_job_subjects(&row.participant_subjects)?;
    validate_confidential_job_evidence_refs(&row.evidence_refs)?;
    if row.retry_count < 0 || row.max_retry_count < 1 || row.retry_count > row.max_retry_count {
        return Err(ConfidentialJobError::Store(
            ConfidentialAggregateStoreError::InvalidStoredConfidentialJob,
        ));
    }
    if row.updated_at < row.created_at {
        return Err(ConfidentialJobError::Store(
            ConfidentialAggregateStoreError::InvalidStoredConfidentialJob,
        ));
    }
    Ok(ConfidentialJobState {
        job_id,
        owner_subject: row.owner_subject,
        mechanism: row.mechanism,
        mechanism_profile: row.mechanism_profile,
        workload_id: row.workload_id,
        participant_subjects: canonical_confidential_job_subjects(row.participant_subjects),
        policy_binding_id: row.policy_binding_id,
        status,
        retry_count: row.retry_count,
        max_retry_count: row.max_retry_count,
        deadline_at: row.deadline_at,
        admission_quota_key: row.admission_quota_key,
        evidence_refs: canonical_unique_strings(row.evidence_refs),
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub(super) fn confidential_job_audit_event_from_row(
    row: ConfidentialJobAuditEventRow,
) -> Result<ConfidentialJobAuditEventState, ConfidentialJobError> {
    let job_id = SecureAggregateTranscriptId::new(
        fixed_32_bytes(row.job_id).map_err(ConfidentialJobError::Store)?,
    )
    .map_err(|_error| {
        ConfidentialJobError::Store(ConfidentialAggregateStoreError::InvalidStoredConfidentialJob)
    })?;
    let previous_status = row
        .previous_status
        .as_deref()
        .map(|status| {
            ConfidentialJobStatus::from_label(status).ok_or(ConfidentialJobError::Store(
                ConfidentialAggregateStoreError::InvalidStoredConfidentialJob,
            ))
        })
        .transpose()?;
    let new_status = ConfidentialJobStatus::from_label(&row.new_status).ok_or(
        ConfidentialJobError::Store(ConfidentialAggregateStoreError::InvalidStoredConfidentialJob),
    )?;
    validate_non_empty_confidential_job_text("actor_subject", &row.actor_subject)?;
    validate_non_empty_confidential_job_text("reason", &row.reason)?;
    validate_confidential_job_evidence_ref_values(&row.evidence_refs)?;
    Ok(ConfidentialJobAuditEventState {
        event_id: row.event_id,
        job_id,
        previous_status,
        new_status,
        actor_subject: row.actor_subject,
        reason: row.reason,
        evidence_refs: canonical_unique_strings(row.evidence_refs),
        created_at: row.created_at,
    })
}

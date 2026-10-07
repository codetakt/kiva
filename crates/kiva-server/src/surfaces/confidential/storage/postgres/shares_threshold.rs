#![allow(clippy::wildcard_imports)]

use super::*;

pub(crate) const fn invalid_differential_privacy_release_store_error()
-> ConfidentialAggregateStoreError {
    ConfidentialAggregateStoreError::InvalidStoredDifferentialPrivacyRelease
}

pub(crate) async fn select_confidential_signing_keys_for_slots_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    slots: &[AdditiveShareCommitmentSlot],
) -> Result<Vec<ConfidentialShareSigningKey>, ConfidentialAggregateStoreError> {
    let mut signing_keys = Vec::with_capacity(slots.len());
    for slot in slots {
        if let Some(signing_key) = select_confidential_signing_key_in_transaction(
            transaction,
            tenant_id,
            slot.signing_key_id(),
        )
        .await?
        {
            signing_keys.push(signing_key);
        }
    }
    Ok(signing_keys)
}

pub(crate) async fn select_confidential_signing_key_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    signing_key_id: SecureAggregateSigningKeyId,
) -> Result<Option<ConfidentialShareSigningKey>, ConfidentialAggregateStoreError> {
    let row = sqlx::query_as::<_, ConfidentialSigningKeyRow>(
        "SELECT signing_key_id, participant_index, public_key, oauth_subject, revoked_at \
         FROM kiva.confidential_additive_share_signing_keys \
         WHERE tenant_id = $1 AND signing_key_id = $2 AND revoked_at IS NULL",
    )
    .bind(tenant_id)
    .bind(signing_key_id.bytes().to_vec())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(confidential_store)?;

    row.map(active_confidential_signing_key_from_row)
        .transpose()
}

pub(crate) async fn select_confidential_signing_key_in_transaction_including_revoked(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    signing_key_id: SecureAggregateSigningKeyId,
) -> Result<Option<ConfidentialShareSigningKey>, ConfidentialAggregateStoreError> {
    let row = sqlx::query_as::<_, ConfidentialSigningKeyRow>(
        "SELECT signing_key_id, participant_index, public_key, oauth_subject, revoked_at \
         FROM kiva.confidential_additive_share_signing_keys \
         WHERE tenant_id = $1 AND signing_key_id = $2",
    )
    .bind(tenant_id)
    .bind(signing_key_id.bytes().to_vec())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(confidential_store)?;

    row.map(confidential_signing_key_from_row).transpose()
}

pub(crate) async fn insert_confidential_signing_key_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    signing_key: &ConfidentialShareSigningKey,
) -> Result<InsertOutcome, ConfidentialAggregateStoreError> {
    let result = sqlx::query(
        "INSERT INTO kiva.confidential_additive_share_signing_keys \
         (tenant_id, signing_key_id, participant_index, public_key, oauth_subject) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (tenant_id, signing_key_id) DO NOTHING",
    )
    .bind(tenant_id)
    .bind(signing_key.signing_key_id.bytes().to_vec())
    .bind(signing_key_participant_index_i64(signing_key)?)
    .bind(signing_key.public_key.bytes().to_vec())
    .bind(&signing_key.oauth_subject)
    .execute(&mut **transaction)
    .await
    .map_err(confidential_store)?;

    Ok(InsertOutcome::from_rows_affected(result.rows_affected()))
}

pub(crate) async fn revoke_confidential_signing_key_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    signing_key_id: SecureAggregateSigningKeyId,
) -> Result<InsertOutcome, ConfidentialAggregateStoreError> {
    let result = sqlx::query(
        "UPDATE kiva.confidential_additive_share_signing_keys \
         SET revoked_at = now() \
         WHERE tenant_id = $1 AND signing_key_id = $2 AND revoked_at IS NULL",
    )
    .bind(tenant_id)
    .bind(signing_key_id.bytes().to_vec())
    .execute(&mut **transaction)
    .await
    .map_err(confidential_store)?;

    Ok(InsertOutcome::from_rows_affected(result.rows_affected()))
}

pub(crate) async fn select_confidential_signing_key_by_id(
    pool: &PgPool,
    tenant_id: &str,
    signing_key_id: SecureAggregateSigningKeyId,
) -> Result<Option<ConfidentialShareSigningKey>, ConfidentialAggregateStoreError> {
    let signing_key =
        select_confidential_signing_key_by_id_including_revoked(pool, tenant_id, signing_key_id)
            .await?;
    Ok(signing_key.and_then(|signing_key| {
        if signing_key.revoked_at.is_some() {
            None
        } else {
            Some(signing_key)
        }
    }))
}

pub(crate) async fn select_confidential_signing_key_by_id_including_revoked(
    pool: &PgPool,
    tenant_id: &str,
    signing_key_id: SecureAggregateSigningKeyId,
) -> Result<Option<ConfidentialShareSigningKey>, ConfidentialAggregateStoreError> {
    let row = sqlx::query_as::<_, ConfidentialSigningKeyRow>(
        "SELECT signing_key_id, participant_index, public_key, oauth_subject, revoked_at \
         FROM kiva.confidential_additive_share_signing_keys \
         WHERE tenant_id = $1 AND signing_key_id = $2",
    )
    .bind(tenant_id)
    .bind(signing_key_id.bytes().to_vec())
    .fetch_optional(pool)
    .await
    .map_err(confidential_store)?;

    row.map(confidential_signing_key_from_row).transpose()
}

pub(crate) async fn select_confidential_submitted_shares(
    pool: &PgPool,
    tenant_id: &str,
    oauth_subject: &str,
    transcript_id: SecureAggregateTranscriptId,
) -> Result<Vec<ConfidentialSubmittedShare>, ConfidentialAggregateStoreError> {
    let rows = sqlx::query_as::<_, ConfidentialSubmittedShareRow>(
        "SELECT signed_envelope, oauth_subject \
         FROM kiva.confidential_additive_share_submissions \
         WHERE tenant_id = $1 AND oauth_subject = $2 AND transcript_id = $3 \
         ORDER BY participant_index ASC",
    )
    .bind(tenant_id)
    .bind(oauth_subject)
    .bind(transcript_id.bytes().to_vec())
    .fetch_all(pool)
    .await
    .map_err(confidential_store)?;

    Ok(rows
        .into_iter()
        .map(|row| ConfidentialSubmittedShare {
            oauth_subject: row.oauth_subject,
            signed_envelope: row.signed_envelope.0,
        })
        .collect())
}

pub(crate) async fn select_confidential_submitted_shares_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    transcript_id: SecureAggregateTranscriptId,
) -> Result<Vec<SignedAuthenticatedAdditiveShareEnvelope>, ConfidentialAggregateStoreError> {
    let rows = sqlx::query_as::<_, ConfidentialSubmittedShareRow>(
        "SELECT signed_envelope, oauth_subject \
         FROM kiva.confidential_additive_share_submissions \
         WHERE tenant_id = $1 AND transcript_id = $2 \
         ORDER BY participant_index ASC",
    )
    .bind(tenant_id)
    .bind(transcript_id.bytes().to_vec())
    .fetch_all(&mut **transaction)
    .await
    .map_err(confidential_store)?;

    Ok(rows.into_iter().map(|row| row.signed_envelope.0).collect())
}

pub(crate) async fn insert_confidential_submitted_share_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    principal: &AuthenticatedPrincipal,
    signed_envelope: SignedAuthenticatedAdditiveShareEnvelope,
) -> Result<StoreOutcome, ConfidentialShareSubmissionError> {
    let envelope = signed_envelope.envelope();
    let result = sqlx::query(
        "INSERT INTO kiva.confidential_additive_share_submissions \
         (tenant_id, transcript_id, participant_index, signing_key_id, oauth_subject, signed_envelope) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (tenant_id, transcript_id, participant_index) DO NOTHING",
    )
    .bind(tenant_id)
    .bind(envelope.transcript_id().bytes().to_vec())
    .bind(nonzero_usize_i64(envelope.share().participant_index()).map_err(
        ConfidentialShareSubmissionError::Store,
    )?)
    .bind(envelope.signing_key_id().bytes().to_vec())
    .bind(&principal.subject)
    .bind(SqlJson(signed_envelope))
    .execute(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialShareSubmissionError::Store)?;

    if InsertOutcome::from_rows_affected(result.rows_affected()) == InsertOutcome::Inserted {
        return Ok(StoreOutcome::Stored);
    }

    let existing = select_confidential_submitted_share_in_transaction(
        transaction,
        tenant_id,
        envelope.transcript_id(),
        envelope.share().participant_index(),
    )
    .await
    .map_err(ConfidentialShareSubmissionError::Store)?;

    Ok(existing.map_or(StoreOutcome::Conflict, |existing| {
        if existing.oauth_subject == principal.subject
            && existing.signed_envelope == signed_envelope
        {
            StoreOutcome::Unchanged
        } else {
            StoreOutcome::Conflict
        }
    }))
}

pub(crate) async fn select_confidential_submitted_share_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    transcript_id: SecureAggregateTranscriptId,
    participant_index: std::num::NonZeroUsize,
) -> Result<Option<ConfidentialSubmittedShare>, ConfidentialAggregateStoreError> {
    let row = sqlx::query_as::<_, ConfidentialSubmittedShareRow>(
        "SELECT signed_envelope, oauth_subject \
         FROM kiva.confidential_additive_share_submissions \
         WHERE tenant_id = $1 AND transcript_id = $2 AND participant_index = $3",
    )
    .bind(tenant_id)
    .bind(transcript_id.bytes().to_vec())
    .bind(nonzero_usize_i64(participant_index)?)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(confidential_store)?;

    Ok(row.map(|row| ConfidentialSubmittedShare {
        oauth_subject: row.oauth_subject,
        signed_envelope: row.signed_envelope.0,
    }))
}

pub(crate) async fn select_threshold_release_request(
    pool: &PgPool,
    tenant_id: &str,
    release_id: SecureAggregateTranscriptId,
) -> Result<Option<ConfidentialThresholdReleaseState>, ConfidentialThresholdReleaseError> {
    let row = sqlx::query_as::<_, ConfidentialThresholdReleaseRequestRow>(
        "SELECT release_id, transcript_id, aggregate_value, requester_subject, \
         left_authority_subject, right_authority_subject, created_at, released_at \
         FROM kiva.confidential_threshold_release_requests \
         WHERE tenant_id = $1 AND release_id = $2",
    )
    .bind(tenant_id)
    .bind(release_id.bytes().to_vec())
    .fetch_optional(pool)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialThresholdReleaseError::Store)?;

    let Some(row) = row else {
        return Ok(None);
    };
    let approvals = select_threshold_release_approvals(pool, tenant_id, release_id).await?;
    Ok(Some(threshold_release_state_from_row(row, approvals)?))
}

pub(crate) async fn select_threshold_release_request_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    release_id: SecureAggregateTranscriptId,
) -> Result<Option<ConfidentialThresholdReleaseState>, ConfidentialThresholdReleaseError> {
    let row = sqlx::query_as::<_, ConfidentialThresholdReleaseRequestRow>(
        "SELECT release_id, transcript_id, aggregate_value, requester_subject, \
         left_authority_subject, right_authority_subject, created_at, released_at \
         FROM kiva.confidential_threshold_release_requests \
         WHERE tenant_id = $1 AND release_id = $2 \
         FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(release_id.bytes().to_vec())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialThresholdReleaseError::Store)?;

    let Some(row) = row else {
        return Ok(None);
    };
    let approvals =
        select_threshold_release_approvals_in_transaction(transaction, tenant_id, release_id)
            .await?;
    Ok(Some(threshold_release_state_from_row(row, approvals)?))
}

pub(crate) async fn select_threshold_release_approvals(
    pool: &PgPool,
    tenant_id: &str,
    release_id: SecureAggregateTranscriptId,
) -> Result<Vec<ConfidentialThresholdReleaseApproval>, ConfidentialThresholdReleaseError> {
    let rows = sqlx::query_as::<_, ConfidentialThresholdReleaseApprovalRow>(
        "SELECT authority_subject, approved_at \
         FROM kiva.confidential_threshold_release_approvals \
         WHERE tenant_id = $1 AND release_id = $2 \
         ORDER BY authority_subject ASC",
    )
    .bind(tenant_id)
    .bind(release_id.bytes().to_vec())
    .fetch_all(pool)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialThresholdReleaseError::Store)?;
    Ok(threshold_release_approvals_from_rows(rows))
}

pub(crate) async fn select_threshold_release_approvals_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: &str,
    release_id: SecureAggregateTranscriptId,
) -> Result<Vec<ConfidentialThresholdReleaseApproval>, ConfidentialThresholdReleaseError> {
    let rows = sqlx::query_as::<_, ConfidentialThresholdReleaseApprovalRow>(
        "SELECT authority_subject, approved_at \
         FROM kiva.confidential_threshold_release_approvals \
         WHERE tenant_id = $1 AND release_id = $2 \
         ORDER BY authority_subject ASC",
    )
    .bind(tenant_id)
    .bind(release_id.bytes().to_vec())
    .fetch_all(&mut **transaction)
    .await
    .map_err(confidential_store)
    .map_err(ConfidentialThresholdReleaseError::Store)?;
    Ok(threshold_release_approvals_from_rows(rows))
}

pub(super) async fn threshold_release_states_from_rows(
    pool: &PgPool,
    tenant_id: &str,
    rows: Vec<ConfidentialThresholdReleaseRequestRow>,
) -> Result<Vec<ConfidentialThresholdReleaseState>, ConfidentialThresholdReleaseError> {
    let mut releases = Vec::with_capacity(rows.len());
    for row in rows {
        let release_id = SecureAggregateTranscriptId::new(fixed_32_bytes(row.release_id.clone())?)
            .map_err(ConfidentialThresholdReleaseError::Secure)?;
        let approvals = select_threshold_release_approvals(pool, tenant_id, release_id).await?;
        releases.push(threshold_release_state_from_row(row, approvals)?);
    }
    Ok(releases)
}

pub(super) fn threshold_release_state_from_row(
    row: ConfidentialThresholdReleaseRequestRow,
    mut approvals: Vec<ConfidentialThresholdReleaseApproval>,
) -> Result<ConfidentialThresholdReleaseState, ConfidentialThresholdReleaseError> {
    approvals.sort_by(|left, right| left.authority_subject.cmp(&right.authority_subject));
    let release_id = SecureAggregateTranscriptId::new(fixed_32_bytes(row.release_id)?)
        .map_err(ConfidentialThresholdReleaseError::Secure)?;
    let transcript_id = SecureAggregateTranscriptId::new(fixed_32_bytes(row.transcript_id)?)
        .map_err(ConfidentialThresholdReleaseError::Secure)?;
    let aggregate = u64::try_from(row.aggregate_value)
        .ok()
        .and_then(|value| SecureAggregateFieldElement::new(value).ok())
        .ok_or(ConfidentialThresholdReleaseError::Secure(
            SecureAggregateError::ValueOutsideField,
        ))?;
    let release = ConfidentialThresholdReleaseState {
        release_id,
        transcript_id,
        aggregate,
        requester_subject: row.requester_subject,
        left_authority_subject: row.left_authority_subject,
        right_authority_subject: row.right_authority_subject,
        created_at: row.created_at,
        released_at: row.released_at,
        approvals,
    };
    release.validate_authority_set()?;
    Ok(release)
}

pub(super) fn threshold_release_approvals_from_rows(
    rows: Vec<ConfidentialThresholdReleaseApprovalRow>,
) -> Vec<ConfidentialThresholdReleaseApproval> {
    rows.into_iter()
        .map(|row| ConfidentialThresholdReleaseApproval {
            authority_subject: row.authority_subject,
            approved_at: row.approved_at,
        })
        .collect()
}

#![allow(clippy::wildcard_imports)]

use super::*;

impl PostgresConfidentialRepository {
    pub(crate) async fn register_threshold_release_request(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialThresholdReleaseRequest,
    ) -> Result<StoreOutcome, ConfidentialThresholdReleaseError> {
        let release = ConfidentialThresholdReleaseState::from_request(principal, request)?;
        let aggregate_result = select_confidential_additive_aggregate_result(
            &self.pool,
            &self.tenant_id,
            release.transcript_id,
        )
        .await
        .map_err(ConfidentialThresholdReleaseError::Store)?;
        ensure_matching_aggregate_result(aggregate_result.as_ref(), &release)?;
        let result = sqlx::query(
            "INSERT INTO kiva.confidential_threshold_release_requests \
             (tenant_id, release_id, transcript_id, aggregate_value, requester_subject, \
              left_authority_subject, right_authority_subject) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (tenant_id, release_id) DO NOTHING",
        )
        .bind(&self.tenant_id)
        .bind(release.release_id.bytes().to_vec())
        .bind(release.transcript_id.bytes().to_vec())
        .bind(
            field_element_i64(release.aggregate)
                .map_err(ConfidentialThresholdReleaseError::Secure)?,
        )
        .bind(&release.requester_subject)
        .bind(&release.left_authority_subject)
        .bind(&release.right_authority_subject)
        .execute(&self.pool)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialThresholdReleaseError::Store)?;

        if InsertOutcome::from_rows_affected(result.rows_affected()) == InsertOutcome::Inserted {
            return Ok(StoreOutcome::Stored);
        }

        let existing =
            select_threshold_release_request(&self.pool, &self.tenant_id, release.release_id)
                .await?
                .ok_or(ConfidentialThresholdReleaseError::NotFound)?;
        if existing.same_request(&release) {
            Ok(StoreOutcome::Unchanged)
        } else {
            Ok(StoreOutcome::Conflict)
        }
    }

    pub(crate) async fn approve_threshold_release(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialThresholdReleaseApprovalRequest,
    ) -> Result<StoreOutcome, ConfidentialThresholdReleaseError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialThresholdReleaseError::Store)?;
        let release = select_threshold_release_request_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.release_id,
        )
        .await?
        .ok_or(ConfidentialThresholdReleaseError::NotFound)?;
        release.ensure_authority(principal)?;
        let result = sqlx::query(
            "INSERT INTO kiva.confidential_threshold_release_approvals \
             (tenant_id, release_id, authority_subject) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (tenant_id, release_id, authority_subject) DO NOTHING",
        )
        .bind(&self.tenant_id)
        .bind(request.release_id.bytes().to_vec())
        .bind(&principal.subject)
        .execute(&mut *transaction)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialThresholdReleaseError::Store)?;
        transaction
            .commit()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialThresholdReleaseError::Store)?;
        Ok(
            match InsertOutcome::from_rows_affected(result.rows_affected()) {
                InsertOutcome::Inserted => StoreOutcome::Stored,
                InsertOutcome::Existing => StoreOutcome::Unchanged,
            },
        )
    }

    pub(crate) async fn release_threshold_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialThresholdReleaseApprovalRequest,
    ) -> Result<ConfidentialThresholdReleaseState, ConfidentialThresholdReleaseError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialThresholdReleaseError::Store)?;
        let mut release = select_threshold_release_request_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.release_id,
        )
        .await?
        .ok_or(ConfidentialThresholdReleaseError::NotFound)?;
        release.ensure_authority(principal)?;
        release.ensure_threshold_approved()?;
        if release.released_at.is_none() {
            let released_at = OffsetDateTime::now_utc();
            sqlx::query(
                "UPDATE kiva.confidential_threshold_release_requests \
                 SET released_at = $3 \
                 WHERE tenant_id = $1 AND release_id = $2 AND released_at IS NULL",
            )
            .bind(&self.tenant_id)
            .bind(request.release_id.bytes().to_vec())
            .bind(released_at)
            .execute(&mut *transaction)
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialThresholdReleaseError::Store)?;
            release.released_at = Some(released_at);
        }
        transaction
            .commit()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialThresholdReleaseError::Store)?;
        Ok(release)
    }

    pub(crate) async fn list_threshold_release_requests(
        &self,
        filter: ConfidentialThresholdReleaseFilter,
    ) -> Result<Vec<ConfidentialThresholdReleaseState>, ConfidentialThresholdReleaseError> {
        let rows = match (filter.release_id, filter.transcript_id) {
            (Some(release_id), Some(transcript_id)) => {
                sqlx::query_as::<_, ConfidentialThresholdReleaseRequestRow>(
                    "SELECT release_id, transcript_id, aggregate_value, requester_subject, \
                 left_authority_subject, right_authority_subject, created_at, released_at \
                 FROM kiva.confidential_threshold_release_requests \
                 WHERE tenant_id = $1 AND release_id = $2 AND transcript_id = $3 \
                 ORDER BY created_at ASC, release_id ASC",
                )
                .bind(&self.tenant_id)
                .bind(release_id.bytes().to_vec())
                .bind(transcript_id.bytes().to_vec())
                .fetch_all(&self.pool)
                .await
            }
            (Some(release_id), None) => {
                sqlx::query_as::<_, ConfidentialThresholdReleaseRequestRow>(
                    "SELECT release_id, transcript_id, aggregate_value, requester_subject, \
                 left_authority_subject, right_authority_subject, created_at, released_at \
                 FROM kiva.confidential_threshold_release_requests \
                 WHERE tenant_id = $1 AND release_id = $2 \
                 ORDER BY created_at ASC, release_id ASC",
                )
                .bind(&self.tenant_id)
                .bind(release_id.bytes().to_vec())
                .fetch_all(&self.pool)
                .await
            }
            (None, Some(transcript_id)) => {
                sqlx::query_as::<_, ConfidentialThresholdReleaseRequestRow>(
                    "SELECT release_id, transcript_id, aggregate_value, requester_subject, \
                 left_authority_subject, right_authority_subject, created_at, released_at \
                 FROM kiva.confidential_threshold_release_requests \
                 WHERE tenant_id = $1 AND transcript_id = $2 \
                 ORDER BY created_at ASC, release_id ASC",
                )
                .bind(&self.tenant_id)
                .bind(transcript_id.bytes().to_vec())
                .fetch_all(&self.pool)
                .await
            }
            (None, None) => {
                sqlx::query_as::<_, ConfidentialThresholdReleaseRequestRow>(
                    "SELECT release_id, transcript_id, aggregate_value, requester_subject, \
                 left_authority_subject, right_authority_subject, created_at, released_at \
                 FROM kiva.confidential_threshold_release_requests \
                 WHERE tenant_id = $1 \
                 ORDER BY created_at ASC, release_id ASC",
                )
                .bind(&self.tenant_id)
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(confidential_store)
        .map_err(ConfidentialThresholdReleaseError::Store)?;

        threshold_release_states_from_rows(&self.pool, &self.tenant_id, rows).await
    }
}

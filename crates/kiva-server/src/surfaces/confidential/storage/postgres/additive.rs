#![allow(clippy::wildcard_imports)]

use super::*;

impl PostgresConfidentialRepository {
    pub(crate) async fn list_share_signing_keys(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<Vec<ConfidentialShareSigningKey>, ConfidentialAggregateError> {
        let rows = sqlx::query_as::<_, ConfidentialSigningKeyRow>(
            "SELECT signing_key_id, participant_index, public_key, oauth_subject, revoked_at \
             FROM kiva.confidential_additive_share_signing_keys \
             WHERE tenant_id = $1 AND oauth_subject = $2 \
             ORDER BY participant_index ASC, signing_key_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(&principal.subject)
        .fetch_all(&self.pool)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialAggregateError::Store)?;

        rows.into_iter()
            .map(confidential_signing_key_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ConfidentialAggregateError::Store)
    }

    pub(crate) async fn list_replay_records(
        &self,
        transcript_id: SecureAggregateTranscriptId,
    ) -> Result<Vec<AcceptedAdditiveShareRecord>, ConfidentialAggregateError> {
        select_confidential(&self.pool, &self.tenant_id, transcript_id)
            .await
            .map_err(ConfidentialAggregateError::Store)
    }

    pub(crate) async fn list_submitted_shares(
        &self,
        principal: &AuthenticatedPrincipal,
        transcript_id: SecureAggregateTranscriptId,
    ) -> Result<Vec<ConfidentialSubmittedShare>, ConfidentialAggregateError> {
        select_confidential_submitted_shares(
            &self.pool,
            &self.tenant_id,
            &principal.subject,
            transcript_id,
        )
        .await
        .map_err(ConfidentialAggregateError::Store)
    }

    pub(crate) async fn release_signed_additive_aggregate(
        &self,
        request: &ConfidentialAdditiveAggregateRequest,
    ) -> Result<ReplayWindowedAdditiveAggregate, ConfidentialAggregateError> {
        commands::release_signed_additive_aggregate_command(&self.pool, &self.tenant_id, request)
            .await
    }

    pub(crate) async fn submit_additive_share(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSubmissionRequest,
    ) -> Result<StoreOutcome, ConfidentialShareSubmissionError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialShareSubmissionError::Store)?;
        let signing_key = select_confidential_signing_key_in_transaction(
            &mut transaction,
            &self.tenant_id,
            request.signed_envelope.envelope().signing_key_id(),
        )
        .await
        .map_err(ConfidentialShareSubmissionError::Store)?
        .ok_or(ConfidentialShareSubmissionError::Secure(
            SecureAggregateError::VerificationKeyMissing,
        ))?;
        validate_share_submission(principal, &signing_key, request.signed_envelope)?;
        ensure_active_confidential_participant_in_transaction(
            &mut transaction,
            &self.tenant_id,
            &principal.subject,
            &principal.subject,
            signing_key.participant_index,
        )
        .await
        .map_err(|_error| ConfidentialShareSubmissionError::InactiveParticipant)?;
        let outcome = insert_confidential_submitted_share_in_transaction(
            &mut transaction,
            &self.tenant_id,
            principal,
            request.signed_envelope,
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialShareSubmissionError::Store)?;
        Ok(outcome)
    }

    pub(crate) async fn register_share_signing_key(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSigningKeyRequest,
    ) -> Result<StoreOutcome, ConfidentialAggregateError> {
        let registration = ConfidentialShareSigningKey::from_request(principal, request);
        ensure_active_confidential_participant(
            &self.pool,
            &self.tenant_id,
            &principal.subject,
            &principal.subject,
            registration.participant_index,
        )
        .await
        .map_err(|_error| ConfidentialAggregateError::InactiveParticipant)?;
        let participant_index = signing_key_participant_index_i64(&registration)
            .map_err(ConfidentialAggregateError::Store)?;
        let result = sqlx::query(
            "INSERT INTO kiva.confidential_additive_share_signing_keys \
             (tenant_id, signing_key_id, participant_index, public_key, oauth_subject) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (tenant_id, signing_key_id) DO NOTHING",
        )
        .bind(&self.tenant_id)
        .bind(registration.signing_key_id.bytes().to_vec())
        .bind(participant_index)
        .bind(registration.public_key.bytes().to_vec())
        .bind(&registration.oauth_subject)
        .execute(&self.pool)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialAggregateError::Store)?;

        if InsertOutcome::from_rows_affected(result.rows_affected()) == InsertOutcome::Inserted {
            return Ok(StoreOutcome::Stored);
        }

        let existing = select_confidential_signing_key_by_id(
            &self.pool,
            &self.tenant_id,
            registration.signing_key_id,
        )
        .await
        .map_err(ConfidentialAggregateError::Store)?;

        Ok(existing.map_or(StoreOutcome::Conflict, |existing| {
            if existing == registration {
                StoreOutcome::Unchanged
            } else {
                StoreOutcome::Conflict
            }
        }))
    }

    pub(crate) async fn revoke_share_signing_key(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSigningKeyRevocationRequest,
    ) -> Result<ConfidentialShareSigningKeyRevocationOutcome, ConfidentialAggregateError> {
        let existing = select_confidential_signing_key_by_id_including_revoked(
            &self.pool,
            &self.tenant_id,
            request.signing_key_id,
        )
        .await
        .map_err(ConfidentialAggregateError::Store)?;

        let Some(existing) = existing else {
            return Ok(ConfidentialShareSigningKeyRevocationOutcome::NotFound);
        };
        if existing.oauth_subject != principal.subject {
            return Ok(ConfidentialShareSigningKeyRevocationOutcome::Forbidden);
        }
        if existing.revoked_at.is_some() {
            return Ok(ConfidentialShareSigningKeyRevocationOutcome::AlreadyRevoked);
        }

        let result = sqlx::query(
            "UPDATE kiva.confidential_additive_share_signing_keys \
             SET revoked_at = now() \
             WHERE tenant_id = $1 AND signing_key_id = $2 AND revoked_at IS NULL",
        )
        .bind(&self.tenant_id)
        .bind(request.signing_key_id.bytes().to_vec())
        .execute(&self.pool)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialAggregateError::Store)?;

        if result.rows_affected() == 0 {
            Ok(ConfidentialShareSigningKeyRevocationOutcome::AlreadyRevoked)
        } else {
            Ok(ConfidentialShareSigningKeyRevocationOutcome::Revoked)
        }
    }

    pub(crate) async fn rotate_share_signing_key(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSigningKeyRotationRequest,
    ) -> Result<ConfidentialShareSigningKeyRotationOutcome, ConfidentialAggregateError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialAggregateError::Store)?;
        let existing = select_confidential_signing_key_in_transaction_including_revoked(
            &mut transaction,
            &self.tenant_id,
            request.old_signing_key_id,
        )
        .await
        .map_err(ConfidentialAggregateError::Store)?;

        let Some(existing) = existing else {
            return Ok(ConfidentialShareSigningKeyRotationOutcome::NotFound);
        };
        let outcome = if existing.oauth_subject != principal.subject {
            ConfidentialShareSigningKeyRotationOutcome::Forbidden
        } else if existing.revoked_at.is_some() {
            ConfidentialShareSigningKeyRotationOutcome::OldKeyRevoked
        } else {
            ensure_active_confidential_participant_in_transaction(
                &mut transaction,
                &self.tenant_id,
                &principal.subject,
                &principal.subject,
                request.participant_index,
            )
            .await
            .map_err(|_error| ConfidentialAggregateError::InactiveParticipant)?;
            let new_key_exists = select_confidential_signing_key_in_transaction_including_revoked(
                &mut transaction,
                &self.tenant_id,
                request.new_signing_key_id,
            )
            .await
            .map_err(ConfidentialAggregateError::Store)?
            .is_some();
            if existing.participant_index != request.participant_index || new_key_exists {
                return Ok(ConfidentialShareSigningKeyRotationOutcome::NewKeyConflict);
            }
            let insert_outcome = insert_confidential_signing_key_in_transaction(
                &mut transaction,
                &self.tenant_id,
                &ConfidentialShareSigningKey::from_rotation(principal, request),
            )
            .await
            .map_err(ConfidentialAggregateError::Store)?;
            if insert_outcome == InsertOutcome::Existing {
                return Ok(ConfidentialShareSigningKeyRotationOutcome::NewKeyConflict);
            }
            let revoke_outcome = revoke_confidential_signing_key_in_transaction(
                &mut transaction,
                &self.tenant_id,
                request.old_signing_key_id,
            )
            .await
            .map_err(ConfidentialAggregateError::Store)?;
            if revoke_outcome == InsertOutcome::Existing {
                return Ok(ConfidentialShareSigningKeyRotationOutcome::OldKeyRevoked);
            }
            ConfidentialShareSigningKeyRotationOutcome::Rotated
        };

        if outcome == ConfidentialShareSigningKeyRotationOutcome::Rotated {
            transaction
                .commit()
                .await
                .map_err(confidential_store)
                .map_err(ConfidentialAggregateError::Store)?;
        }
        Ok(outcome)
    }
}

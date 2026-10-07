#![allow(clippy::wildcard_imports)]

use super::*;

impl PostgresConfidentialRepository {
    pub(crate) async fn register_confidential_participant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialParticipantRequest,
    ) -> Result<ConfidentialParticipantRegistrationOutcome, ConfidentialParticipantError> {
        let participant = ConfidentialParticipantState::from_request(principal, request)?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialParticipantError::Store)?;
        let row = sqlx::query_as::<_, ConfidentialParticipantRow>(
            "INSERT INTO kiva.confidential_participants \
             (tenant_id, owner_subject, participant_subject, participant_index, status, \
              mechanism_roles, authorization_scope, key_ceremony_ref, recovery_contact_ref, \
              evidence_refs) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             ON CONFLICT (tenant_id, owner_subject, participant_subject, participant_index) \
             DO NOTHING \
             RETURNING owner_subject, participant_subject, participant_index, status, \
              mechanism_roles, authorization_scope, key_ceremony_ref, recovery_contact_ref, \
              evidence_refs, created_at, updated_at",
        )
        .bind(&self.tenant_id)
        .bind(&participant.owner_subject)
        .bind(&participant.participant_subject)
        .bind(
            nonzero_usize_i64(participant.participant_index)
                .map_err(ConfidentialParticipantError::Store)?,
        )
        .bind(participant.status.label())
        .bind(&participant.mechanism_roles)
        .bind(&participant.authorization_scope)
        .bind(&participant.key_ceremony_ref)
        .bind(&participant.recovery_contact_ref)
        .bind(&participant.evidence_refs)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialParticipantError::Store)?;

        if let Some(row) = row {
            let stored = confidential_participant_from_row(row)?;
            insert_confidential_participant_audit_event_in_transaction(
                &mut transaction,
                &self.tenant_id,
                principal,
                &stored,
                None,
                stored.status,
                "participant invited",
                &stored.evidence_refs,
            )
            .await?;
            transaction
                .commit()
                .await
                .map_err(confidential_store)
                .map_err(ConfidentialParticipantError::Store)?;
            return Ok(ConfidentialParticipantRegistrationOutcome::Stored(stored));
        }

        transaction
            .commit()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialParticipantError::Store)?;
        let existing = select_confidential_participant(
            &self.pool,
            &self.tenant_id,
            &participant.owner_subject,
            &participant.participant_subject,
            participant.participant_index,
        )
        .await?
        .ok_or(ConfidentialParticipantError::NotFound)?;
        if existing.same_registration(&participant) {
            Ok(ConfidentialParticipantRegistrationOutcome::Unchanged(
                existing,
            ))
        } else {
            Ok(ConfidentialParticipantRegistrationOutcome::Conflict)
        }
    }

    pub(crate) async fn transition_confidential_participant(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialParticipantTransitionRequest,
    ) -> Result<ConfidentialParticipantTransitionOutcome, ConfidentialParticipantError> {
        validate_confidential_participant_transition_request(&request)?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialParticipantError::Store)?;
        let mut participant = select_confidential_participant_in_transaction(
            &mut transaction,
            &self.tenant_id,
            &principal.subject,
            request.participant_subject.trim(),
            request.participant_index,
        )
        .await?
        .ok_or(ConfidentialParticipantError::NotFound)?;
        participant.ensure_owner(principal)?;
        if participant.status == request.target_status {
            return Ok(ConfidentialParticipantTransitionOutcome::Unchanged(
                participant,
            ));
        }
        let previous_status = participant.status;
        participant.status =
            transition_confidential_participant_status(participant.status, request.target_status)
                .map_err(ConfidentialParticipantError::Transition)?;
        participant.updated_at = OffsetDateTime::now_utc();
        append_unique_strings(&mut participant.evidence_refs, &request.evidence_refs);
        let updated = update_confidential_participant_in_transaction(
            &mut transaction,
            &self.tenant_id,
            &participant,
        )
        .await?;
        insert_confidential_participant_audit_event_in_transaction(
            &mut transaction,
            &self.tenant_id,
            principal,
            &updated,
            Some(previous_status),
            updated.status,
            request
                .reason
                .as_deref()
                .unwrap_or("participant lifecycle transition"),
            &request.evidence_refs,
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(confidential_store)
            .map_err(ConfidentialParticipantError::Store)?;
        Ok(ConfidentialParticipantTransitionOutcome::Updated(updated))
    }

    pub(crate) async fn list_confidential_participants(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialParticipantFilter,
    ) -> Result<Vec<ConfidentialParticipantState>, ConfidentialParticipantError> {
        let rows = sqlx::query_as::<_, ConfidentialParticipantRow>(
            "SELECT owner_subject, participant_subject, participant_index, status, \
              mechanism_roles, authorization_scope, key_ceremony_ref, recovery_contact_ref, \
              evidence_refs, created_at, updated_at \
             FROM kiva.confidential_participants \
             WHERE tenant_id = $1 AND owner_subject = $2 \
              AND ($3::text IS NULL OR participant_subject = $3) \
              AND ($4::bigint IS NULL OR participant_index = $4) \
              AND ($5::text IS NULL OR status = $5) \
             ORDER BY participant_subject ASC, participant_index ASC",
        )
        .bind(&self.tenant_id)
        .bind(&principal.subject)
        .bind(filter.participant_subject)
        .bind(
            filter
                .participant_index
                .map(nonzero_usize_i64)
                .transpose()
                .map_err(ConfidentialParticipantError::Store)?,
        )
        .bind(filter.status.map(ConfidentialParticipantStatus::label))
        .fetch_all(&self.pool)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialParticipantError::Store)?;

        rows.into_iter()
            .map(confidential_participant_from_row)
            .collect()
    }

    pub(crate) async fn list_confidential_participant_audit_events(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialParticipantAuditEventFilter,
    ) -> Result<Vec<ConfidentialParticipantAuditEventState>, ConfidentialParticipantError> {
        let rows = sqlx::query_as::<_, ConfidentialParticipantAuditEventRow>(
            "SELECT event.event_id, event.participant_subject, event.participant_index, \
              event.previous_status, event.new_status, event.actor_subject, event.reason, \
              event.evidence_refs, event.created_at \
             FROM kiva.confidential_participant_audit_events AS event \
             INNER JOIN kiva.confidential_participants AS participant \
              ON participant.tenant_id = event.tenant_id \
              AND participant.owner_subject = event.owner_subject \
              AND participant.participant_subject = event.participant_subject \
              AND participant.participant_index = event.participant_index \
             WHERE event.tenant_id = $1 AND participant.owner_subject = $2 \
              AND ($3::text IS NULL OR event.participant_subject = $3) \
              AND ($4::bigint IS NULL OR event.participant_index = $4) \
             ORDER BY event.created_at DESC, event.event_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(&principal.subject)
        .bind(filter.participant_subject)
        .bind(
            filter
                .participant_index
                .map(nonzero_usize_i64)
                .transpose()
                .map_err(ConfidentialParticipantError::Store)?,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialParticipantError::Store)?;

        rows.into_iter()
            .map(confidential_participant_audit_event_from_row)
            .collect()
    }
}

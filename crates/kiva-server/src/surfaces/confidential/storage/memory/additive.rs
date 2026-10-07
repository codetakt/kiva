#![allow(clippy::wildcard_imports)]

use super::*;

impl MemoryConfidentialRepository {
    pub(crate) async fn list_share_signing_keys(
        &self,
        principal: &AuthenticatedPrincipal,
    ) -> Result<Vec<ConfidentialShareSigningKey>, ConfidentialAggregateError> {
        let mut signing_keys = {
            let signing_key_guard = self.additive.signing_keys.read().await;
            signing_key_guard
                .values()
                .filter(|signing_key| signing_key.oauth_subject == principal.subject)
                .cloned()
                .collect::<Vec<_>>()
        };
        signing_keys.sort_by_key(|signing_key| {
            (
                signing_key.participant_index.get(),
                signing_key.signing_key_id.bytes(),
            )
        });
        Ok(signing_keys)
    }

    pub(crate) async fn list_replay_records(
        &self,
        transcript_id: SecureAggregateTranscriptId,
    ) -> Result<Vec<AcceptedAdditiveShareRecord>, ConfidentialAggregateError> {
        let mut records = {
            let record_guard = self.additive.replay_records.read().await;
            record_guard
                .values()
                .filter(|record| record.transcript_id() == transcript_id)
                .copied()
                .collect::<Vec<_>>()
        };
        records.sort_by_key(|record| record.participant_index().get());
        Ok(records)
    }

    pub(crate) async fn list_submitted_shares(
        &self,
        principal: &AuthenticatedPrincipal,
        transcript_id: SecureAggregateTranscriptId,
    ) -> Result<Vec<ConfidentialSubmittedShare>, ConfidentialAggregateError> {
        let mut submissions = {
            let submitted_share_guard = self.additive.submitted_shares.read().await;
            submitted_share_guard
                .values()
                .filter(|submission| {
                    submission.oauth_subject == principal.subject
                        && submission.signed_envelope.envelope().transcript_id() == transcript_id
                })
                .cloned()
                .collect::<Vec<_>>()
        };
        submissions.sort_by_key(|submission| {
            submission
                .signed_envelope
                .envelope()
                .share()
                .participant_index()
                .get()
        });
        Ok(submissions)
    }
    pub(crate) async fn register_share_signing_key(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSigningKeyRequest,
    ) -> Result<StoreOutcome, ConfidentialAggregateError> {
        self.ensure_active_confidential_participant(
            &principal.subject,
            &principal.subject,
            request.participant_index,
        )
        .await
        .map_err(|_error| ConfidentialAggregateError::InactiveParticipant)?;
        let registration = ConfidentialShareSigningKey::from_request(principal, request);
        let mut signing_keys = self.additive.signing_keys.write().await;
        let key =
            ConfidentialSigningKeyStorageKey::from_signing_key_id(registration.signing_key_id);

        let outcome = match signing_keys.get(&key) {
            Some(existing) if *existing == registration => StoreOutcome::Unchanged,
            Some(_existing) => StoreOutcome::Conflict,
            None => {
                signing_keys.insert(key, registration);
                StoreOutcome::Stored
            }
        };
        drop(signing_keys);
        Ok(outcome)
    }

    pub(crate) async fn revoke_share_signing_key(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSigningKeyRevocationRequest,
    ) -> Result<ConfidentialShareSigningKeyRevocationOutcome, ConfidentialAggregateError> {
        let mut signing_keys = self.additive.signing_keys.write().await;
        let key = ConfidentialSigningKeyStorageKey::from_signing_key_id(request.signing_key_id);
        let outcome = match signing_keys.get_mut(&key) {
            None => ConfidentialShareSigningKeyRevocationOutcome::NotFound,
            Some(signing_key) if signing_key.oauth_subject != principal.subject => {
                ConfidentialShareSigningKeyRevocationOutcome::Forbidden
            }
            Some(signing_key) if signing_key.revoked_at.is_some() => {
                ConfidentialShareSigningKeyRevocationOutcome::AlreadyRevoked
            }
            Some(signing_key) => {
                signing_key.revoked_at = Some(OffsetDateTime::now_utc());
                ConfidentialShareSigningKeyRevocationOutcome::Revoked
            }
        };
        drop(signing_keys);
        Ok(outcome)
    }

    pub(crate) async fn rotate_share_signing_key(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSigningKeyRotationRequest,
    ) -> Result<ConfidentialShareSigningKeyRotationOutcome, ConfidentialAggregateError> {
        self.ensure_active_confidential_participant(
            &principal.subject,
            &principal.subject,
            request.participant_index,
        )
        .await
        .map_err(|_error| ConfidentialAggregateError::InactiveParticipant)?;
        let old_key =
            ConfidentialSigningKeyStorageKey::from_signing_key_id(request.old_signing_key_id);
        let new_key =
            ConfidentialSigningKeyStorageKey::from_signing_key_id(request.new_signing_key_id);
        let mut signing_keys = self.additive.signing_keys.write().await;
        let outcome = match signing_keys.get(&old_key).cloned() {
            None => ConfidentialShareSigningKeyRotationOutcome::NotFound,
            Some(existing) if existing.oauth_subject != principal.subject => {
                ConfidentialShareSigningKeyRotationOutcome::Forbidden
            }
            Some(existing) if existing.revoked_at.is_some() => {
                ConfidentialShareSigningKeyRotationOutcome::OldKeyRevoked
            }
            Some(existing) if existing.participant_index != request.participant_index => {
                ConfidentialShareSigningKeyRotationOutcome::NewKeyConflict
            }
            Some(_existing) if signing_keys.contains_key(&new_key) => {
                ConfidentialShareSigningKeyRotationOutcome::NewKeyConflict
            }
            Some(_existing) => {
                if let Some(old_registration) = signing_keys.get_mut(&old_key) {
                    old_registration.revoked_at = Some(OffsetDateTime::now_utc());
                }
                signing_keys.insert(
                    new_key,
                    ConfidentialShareSigningKey::from_rotation(principal, request),
                );
                ConfidentialShareSigningKeyRotationOutcome::Rotated
            }
        };
        drop(signing_keys);
        Ok(outcome)
    }

    pub(crate) async fn submit_additive_share(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShareSubmissionRequest,
    ) -> Result<StoreOutcome, ConfidentialShareSubmissionError> {
        let signing_keys = self.additive.signing_keys.read().await;
        let signing_key = active_signing_key_for_envelope(&signing_keys, request.signed_envelope)
            .map_err(ConfidentialShareSubmissionError::Secure)?;
        validate_share_submission(principal, signing_key, request.signed_envelope)?;
        self.ensure_active_confidential_participant(
            &principal.subject,
            &principal.subject,
            signing_key.participant_index,
        )
        .await
        .map_err(|_error| ConfidentialShareSubmissionError::InactiveParticipant)?;
        drop(signing_keys);

        let submitted_share = ConfidentialSubmittedShare::from_submission(principal, request);
        let key = ConfidentialSubmittedShareKey::from_envelope(request.signed_envelope);
        let mut submitted_shares = self.additive.submitted_shares.write().await;
        let outcome = match submitted_shares.get(&key) {
            Some(existing) if *existing == submitted_share => StoreOutcome::Unchanged,
            Some(_existing) => StoreOutcome::Conflict,
            None => {
                submitted_shares.insert(key, submitted_share);
                StoreOutcome::Stored
            }
        };
        drop(submitted_shares);
        Ok(outcome)
    }

    pub(crate) async fn release_signed_additive_aggregate(
        &self,
        request: &ConfidentialAdditiveAggregateRequest,
    ) -> Result<ReplayWindowedAdditiveAggregate, ConfidentialAggregateError> {
        let signing_keys = self.additive.signing_keys.read().await;
        let verification_keys = verification_keys_for_slots(&signing_keys, &request.slots)
            .map_err(ConfidentialAggregateError::Secure)?;
        for slot in &request.slots {
            let key = ConfidentialSigningKeyStorageKey::from_signing_key_id(slot.signing_key_id());
            let Some(signing_key) = signing_keys.get(&key) else {
                return Err(ConfidentialAggregateError::Secure(
                    SecureAggregateError::VerificationKeyMissing,
                ));
            };
            self.ensure_active_confidential_participant(
                &signing_key.oauth_subject,
                &signing_key.oauth_subject,
                signing_key.participant_index,
            )
            .await
            .map_err(|_error| ConfidentialAggregateError::InactiveParticipant)?;
        }
        drop(signing_keys);

        let signed_envelopes = if request.signed_envelopes.is_empty() {
            let submitted_shares = self.additive.submitted_shares.read().await;
            let signed_envelopes = submitted_shares
                .values()
                .filter(|submission| {
                    submission.signed_envelope.envelope().transcript_id() == request.transcript_id
                })
                .map(|submission| submission.signed_envelope)
                .collect::<Vec<_>>();
            drop(submitted_shares);
            signed_envelopes
        } else {
            request.signed_envelopes.clone()
        };

        let mut records = self.additive.replay_records.write().await;
        let accepted_records = records
            .values()
            .filter(|record| record.transcript_id() == request.transcript_id)
            .copied()
            .collect::<Vec<_>>();
        let aggregate = combine_confidential_additive_aggregate(
            request,
            &verification_keys,
            &accepted_records,
            &signed_envelopes,
        )?;
        for record in aggregate.accepted_records() {
            let key = ConfidentialReplayRecordKey::from_record(*record);
            if records.insert(key, *record).is_some() {
                return Err(ConfidentialAggregateError::Secure(
                    SecureAggregateError::EnvelopeReplayDetected,
                ));
            }
        }
        drop(records);
        let result = ConfidentialAdditiveAggregateResult::from_aggregate(
            request.participant_count,
            request.transcript_id,
            &aggregate,
        );
        let mut aggregate_results = self.additive.aggregate_results.write().await;
        aggregate_results.insert(
            ConfidentialAggregateResultKey::from_transcript_id(request.transcript_id),
            result,
        );
        drop(aggregate_results);
        Ok(aggregate)
    }
}

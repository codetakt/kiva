#![allow(clippy::wildcard_imports)]

use super::*;

pub(crate) async fn release_signed_additive_aggregate_command(
    pool: &PgPool,
    tenant_id: &str,
    request: &ConfidentialAdditiveAggregateRequest,
) -> Result<ReplayWindowedAdditiveAggregate, ConfidentialAggregateError> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialAggregateError::Store)?;
    let verification_keys = select_confidential_signing_keys_for_slots_in_transaction(
        &mut transaction,
        tenant_id,
        &request.slots,
    )
    .await
    .map_err(ConfidentialAggregateError::Store)
    .and_then(|keys| {
        verification_keys_from_signing_key_rows(&request.slots, keys)
            .map_err(ConfidentialAggregateError::Secure)
    })?;
    let signing_keys = select_confidential_signing_keys_for_slots_in_transaction(
        &mut transaction,
        tenant_id,
        &request.slots,
    )
    .await
    .map_err(ConfidentialAggregateError::Store)?;
    for signing_key in signing_keys {
        ensure_active_confidential_participant_in_transaction(
            &mut transaction,
            tenant_id,
            &signing_key.oauth_subject,
            &signing_key.oauth_subject,
            signing_key.participant_index,
        )
        .await
        .map_err(|_error| ConfidentialAggregateError::InactiveParticipant)?;
    }
    let accepted_records =
        select_confidential_in_transaction(&mut transaction, tenant_id, request.transcript_id)
            .await
            .map_err(ConfidentialAggregateError::Store)?;
    let signed_envelopes = if request.signed_envelopes.is_empty() {
        select_confidential_submitted_shares_in_transaction(
            &mut transaction,
            tenant_id,
            request.transcript_id,
        )
        .await
        .map_err(ConfidentialAggregateError::Store)?
    } else {
        request.signed_envelopes.clone()
    };
    let aggregate = combine_confidential_additive_aggregate(
        request,
        &verification_keys,
        &accepted_records,
        &signed_envelopes,
    )?;
    for record in aggregate.accepted_records() {
        let outcome =
            insert_confidential_replay_record_in_transaction(&mut transaction, tenant_id, *record)
                .await
                .map_err(ConfidentialAggregateError::Store)?;
        if outcome == InsertOutcome::Existing {
            return Err(ConfidentialAggregateError::Secure(
                SecureAggregateError::EnvelopeReplayDetected,
            ));
        }
    }
    insert_confidential_additive_aggregate_result_in_transaction(
        &mut transaction,
        tenant_id,
        &ConfidentialAdditiveAggregateResult::from_aggregate(
            request.participant_count,
            request.transcript_id,
            &aggregate,
        ),
    )
    .await?;
    transaction
        .commit()
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialAggregateError::Store)?;
    Ok(aggregate)
}

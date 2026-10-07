#![allow(clippy::wildcard_imports)]

use super::*;

impl PostgresConfidentialRepository {
    pub(crate) async fn reconstruct_shamir_threshold_secret(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialShamirThresholdReconstructionRequest,
    ) -> Result<ConfidentialShamirThresholdReconstructionState, ConfidentialShamirReconstructionError>
    {
        let reconstruction =
            ConfidentialShamirThresholdReconstructionState::from_request(principal, &request)?;
        let row = sqlx::query_as::<_, ConfidentialShamirThresholdReconstructionRow>(
            "INSERT INTO kiva.confidential_shamir_threshold_reconstructions \
             (tenant_id, reconstruction_id, transcript_id, owner_subject, participant_count, \
              threshold, share_count, reconstructed_value, verification_profile, \
              verified_share_count, complaint_count) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
             ON CONFLICT (tenant_id, reconstruction_id) DO NOTHING \
             RETURNING reconstruction_id, transcript_id, owner_subject, participant_count, \
              threshold, share_count, reconstructed_value, verification_profile, \
              verified_share_count, complaint_count, created_at",
        )
        .bind(&self.tenant_id)
        .bind(reconstruction.reconstruction_id.bytes().to_vec())
        .bind(reconstruction.transcript_id.bytes().to_vec())
        .bind(&reconstruction.owner_subject)
        .bind(
            nonzero_usize_i64(reconstruction.participant_count)
                .map_err(ConfidentialShamirReconstructionError::Store)?,
        )
        .bind(
            nonzero_usize_i64(reconstruction.threshold)
                .map_err(ConfidentialShamirReconstructionError::Store)?,
        )
        .bind(
            usize_i64(reconstruction.share_count)
                .map_err(ConfidentialShamirReconstructionError::Store)?,
        )
        .bind(
            field_element_i64(reconstruction.reconstructed_value)
                .map_err(ConfidentialShamirReconstructionError::Secure)?,
        )
        .bind(&reconstruction.verification_profile)
        .bind(
            usize_i64(reconstruction.verified_share_count)
                .map_err(ConfidentialShamirReconstructionError::Store)?,
        )
        .bind(
            usize_i64(reconstruction.complaint_count)
                .map_err(ConfidentialShamirReconstructionError::Store)?,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialShamirReconstructionError::Store)?;

        if let Some(row) = row {
            return shamir_threshold_reconstruction_from_row(row)
                .map_err(ConfidentialShamirReconstructionError::Store);
        }

        let existing = select_shamir_threshold_reconstruction(
            &self.pool,
            &self.tenant_id,
            reconstruction.reconstruction_id,
        )
        .await?
        .ok_or(ConfidentialShamirReconstructionError::NotFound)?;
        if existing.same_reconstruction(&reconstruction) {
            Ok(existing)
        } else {
            Err(ConfidentialShamirReconstructionError::Conflict)
        }
    }

    pub(crate) async fn list_shamir_threshold_reconstructions(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialShamirThresholdReconstructionFilter,
    ) -> Result<
        Vec<ConfidentialShamirThresholdReconstructionState>,
        ConfidentialShamirReconstructionError,
    > {
        let rows = sqlx::query_as::<_, ConfidentialShamirThresholdReconstructionRow>(
            "SELECT reconstruction_id, transcript_id, owner_subject, participant_count, \
             threshold, share_count, reconstructed_value, verification_profile, \
             verified_share_count, complaint_count, created_at \
             FROM kiva.confidential_shamir_threshold_reconstructions \
             WHERE tenant_id = $1 AND owner_subject = $2 \
              AND ($3::bytea IS NULL OR reconstruction_id = $3) \
              AND ($4::bytea IS NULL OR transcript_id = $4) \
             ORDER BY created_at ASC, reconstruction_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(&principal.subject)
        .bind(
            filter
                .reconstruction_id
                .map(|reconstruction_id| reconstruction_id.bytes().to_vec()),
        )
        .bind(
            filter
                .transcript_id
                .map(|transcript_id| transcript_id.bytes().to_vec()),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(confidential_store)
        .map_err(ConfidentialShamirReconstructionError::Store)?;

        rows.into_iter()
            .map(shamir_threshold_reconstruction_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ConfidentialShamirReconstructionError::Store)
    }
}

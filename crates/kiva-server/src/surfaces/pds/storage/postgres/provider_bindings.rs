#![allow(clippy::wildcard_imports)]

use super::super::{mapping::*, postgres_queries::*, *};

impl PostgresPdsRepository {
    pub(crate) async fn list_provider_bindings(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: PdsProviderBindingFilter,
    ) -> Result<Vec<PdsProviderBindingState>, PdsRepositoryError> {
        let rows = sqlx::query_as::<_, PdsProviderBindingRow>(
            "SELECT binding_id, owner_subject, mechanism, provider_kind, capability, profile, \
              provider_id_digest, artifact_digest, parameter_set_digest, review_evidence_digest, \
              production_claim_requested, binding_digest, evidence_gate, release_gate, \
              runtime_class, production_ready, created_at \
             FROM kiva.pds_provider_bindings \
             WHERE tenant_id = $1 AND owner_subject = $2 \
              AND ($3::uuid IS NULL OR binding_id = $3) \
             ORDER BY created_at ASC, binding_id ASC",
        )
        .bind(&self.tenant_id)
        .bind(&principal.subject)
        .bind(filter.binding_id)
        .fetch_all(&self.pool)
        .await
        .map_err(PdsRepositoryError::store)?;

        rows.into_iter()
            .map(pds_provider_binding_from_row)
            .collect()
    }

    pub(crate) async fn register_provider_binding(
        &self,
        binding: PdsProviderBindingState,
    ) -> Result<StoreOutcome, PdsRepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(PdsRepositoryError::store)?;
        let result = sqlx::query(
            "INSERT INTO kiva.pds_provider_bindings \
             (tenant_id, binding_id, owner_subject, mechanism, provider_kind, capability, \
              profile, provider_id_digest, artifact_digest, parameter_set_digest, \
              review_evidence_digest, production_claim_requested, binding_digest, evidence_gate, \
              release_gate, runtime_class, production_ready) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17) \
             ON CONFLICT (tenant_id, binding_id) DO NOTHING",
        )
        .bind(&self.tenant_id)
        .bind(binding.binding_id)
        .bind(&binding.owner_subject)
        .bind(&binding.mechanism)
        .bind(&binding.provider_kind)
        .bind(&binding.capability)
        .bind(&binding.profile)
        .bind(&binding.provider_id_digest)
        .bind(&binding.artifact_digest)
        .bind(&binding.parameter_set_digest)
        .bind(&binding.review_evidence_digest)
        .bind(binding.production_claim_requested)
        .bind(&binding.binding_digest)
        .bind(&binding.evidence_gate)
        .bind(&binding.release_gate)
        .bind(&binding.runtime_class)
        .bind(binding.production_ready)
        .execute(&mut *transaction)
        .await
        .map_err(PdsRepositoryError::store)?;

        let outcome = if InsertOutcome::from_rows_affected(result.rows_affected())
            == InsertOutcome::Inserted
        {
            StoreOutcome::Stored
        } else {
            let existing = select_pds_provider_binding_in_transaction(
                &mut transaction,
                &self.tenant_id,
                binding.binding_id,
            )
            .await?
            .ok_or_else(|| {
                PdsRepositoryError::InvalidRequest("PDS provider binding was not found".to_owned())
            })?;
            if existing.same_registration(&binding) {
                StoreOutcome::Unchanged
            } else {
                StoreOutcome::Conflict
            }
        };

        transaction
            .commit()
            .await
            .map_err(PdsRepositoryError::store)?;
        Ok(outcome)
    }
}

#![allow(clippy::wildcard_imports)]

use super::*;

pub(in crate::surfaces::pds::storage) fn pds_consent_grant_from_row(
    (
        grant_id,
        data_subject,
        grantee_subject,
        scope,
        state,
        purpose,
        legal_basis,
        created_at,
        expires_at,
        revoked_at,
        renewed_from_grant_id,
    ): PdsConsentGrantRow,
) -> Result<PdsConsentGrantState, PdsRepositoryError> {
    let scope = PdsConsentScope::from_label(&scope).ok_or(PdsRepositoryError::InvalidStoredData)?;
    let state = PdsConsentState::from_label(&state).ok_or(PdsRepositoryError::InvalidStoredData)?;
    let legal_basis =
        PdsLegalBasis::from_label(&legal_basis).ok_or(PdsRepositoryError::InvalidStoredData)?;
    if data_subject.is_empty()
        || grantee_subject.is_empty()
        || purpose.is_empty()
        || matches!(state, PdsConsentState::Active) != revoked_at.is_none()
        || expires_at.is_some_and(|expires_at| expires_at <= created_at)
    {
        return Err(PdsRepositoryError::InvalidStoredData);
    }
    Ok(PdsConsentGrantState {
        grant_id,
        data_subject,
        grantee_subject,
        scope,
        state,
        purpose,
        legal_basis,
        created_at,
        expires_at,
        revoked_at,
        renewed_from_grant_id,
    })
}

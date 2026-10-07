//! OpenAPI-to-PDS conversion for consent request DTOs.

use kiva_pds::{
    ConsentGrantCommandInput as PdsConsentGrantCommandInput,
    ConsentRevocationCommandInput as PdsConsentRevocationCommandInput,
};
use kiva_pds_openapi_contract as pds_openapi;

use super::common::{consent_scope_from_openapi, legal_basis_from_openapi};

pub(crate) fn consent_grant_command_input_from_openapi(
    request: pds_openapi::ConsentGrantRequest,
) -> PdsConsentGrantCommandInput {
    PdsConsentGrantCommandInput {
        grant_id: request.grant_id,
        grantee_subject: request.grantee_subject,
        scope: consent_scope_from_openapi(request.scope),
        purpose: request.purpose,
        legal_basis: legal_basis_from_openapi(request.legal_basis),
        expires_at: request.expires_at,
        renewed_from_grant_id: request.renewed_from_grant_id,
    }
}

pub(crate) const fn consent_revocation_command_input_from_openapi(
    request: pds_openapi::ConsentRevocationRequest,
) -> PdsConsentRevocationCommandInput {
    PdsConsentRevocationCommandInput {
        grant_id: request.grant_id,
    }
}

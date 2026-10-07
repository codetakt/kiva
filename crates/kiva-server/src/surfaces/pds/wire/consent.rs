//! Consent-grant PDS wire projection.

use kiva_pds::{
    ConsentGrant as ConsentGrantDto, ConsentGrantList, ConsentScope, ConsentState, LegalBasis,
};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    app::pds as pds_app, core::response::rfc3339, surfaces::pds::model::PdsConsentGrantState,
};

use super::common::dto_value;

pub(crate) fn consent_grant_list_response(
    grants: &[PdsConsentGrantState],
) -> Result<Value, serde_json::Error> {
    dto_value(&ConsentGrantList {
        grants: grants.iter().map(consent_grant_dto).collect(),
    })
}

pub(super) trait ConsentGrantProjection {
    fn grant_id(&self) -> Uuid;
    fn data_subject(&self) -> &str;
    fn grantee_subject(&self) -> &str;
    fn scope(&self) -> ConsentScope;
    fn state(&self) -> ConsentState;
    fn purpose(&self) -> &str;
    fn legal_basis(&self) -> LegalBasis;
    fn created_at(&self) -> OffsetDateTime;
    fn expires_at(&self) -> Option<OffsetDateTime>;
    fn revoked_at(&self) -> Option<OffsetDateTime>;
    fn renewed_from_grant_id(&self) -> Option<Uuid>;
}

impl ConsentGrantProjection for PdsConsentGrantState {
    fn grant_id(&self) -> Uuid {
        self.grant_id
    }

    fn data_subject(&self) -> &str {
        &self.data_subject
    }

    fn grantee_subject(&self) -> &str {
        &self.grantee_subject
    }

    fn scope(&self) -> ConsentScope {
        self.scope
    }

    fn state(&self) -> ConsentState {
        self.state
    }

    fn purpose(&self) -> &str {
        &self.purpose
    }

    fn legal_basis(&self) -> LegalBasis {
        self.legal_basis
    }

    fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    fn expires_at(&self) -> Option<OffsetDateTime> {
        self.expires_at
    }

    fn revoked_at(&self) -> Option<OffsetDateTime> {
        self.revoked_at
    }

    fn renewed_from_grant_id(&self) -> Option<Uuid> {
        self.renewed_from_grant_id
    }
}

impl ConsentGrantProjection for pds_app::ConsentGrantRecord {
    fn grant_id(&self) -> Uuid {
        self.grant_id
    }

    fn data_subject(&self) -> &str {
        &self.data_subject
    }

    fn grantee_subject(&self) -> &str {
        &self.grantee_subject
    }

    fn scope(&self) -> ConsentScope {
        self.scope
    }

    fn state(&self) -> ConsentState {
        self.state
    }

    fn purpose(&self) -> &str {
        &self.purpose
    }

    fn legal_basis(&self) -> LegalBasis {
        self.legal_basis
    }

    fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    fn expires_at(&self) -> Option<OffsetDateTime> {
        self.expires_at
    }

    fn revoked_at(&self) -> Option<OffsetDateTime> {
        self.revoked_at
    }

    fn renewed_from_grant_id(&self) -> Option<Uuid> {
        self.renewed_from_grant_id
    }
}

pub(super) fn consent_grant_dto<T: ConsentGrantProjection>(grant: &T) -> ConsentGrantDto {
    ConsentGrantDto {
        grant_id: grant.grant_id().to_string(),
        data_subject: grant.data_subject().to_owned(),
        grantee_subject: grant.grantee_subject().to_owned(),
        scope: grant.scope(),
        state: grant.state(),
        purpose: grant.purpose().to_owned(),
        legal_basis: grant.legal_basis(),
        created_at: rfc3339(grant.created_at()),
        expires_at: grant.expires_at().map(rfc3339),
        revoked_at: grant.revoked_at().map(rfc3339),
        renewed_from_grant_id: grant
            .renewed_from_grant_id()
            .map(|grant_id| grant_id.to_string()),
    }
}

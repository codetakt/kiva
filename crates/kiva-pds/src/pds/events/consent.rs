#![allow(clippy::wildcard_imports)]

use super::super::*;

/// Domain event produced when a consent grant registration command is admitted.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConsentGrantRegistrationRequested {
    data_subject: String,
    grant_id: Uuid,
    grantee_subject: String,
    scope: ConsentScope,
    purpose: String,
    legal_basis: LegalBasis,
    expires_at: Option<OffsetDateTime>,
    renewed_from_grant_id: Option<Uuid>,
    created_at: OffsetDateTime,
}

impl ConsentGrantRegistrationRequested {
    /// Projects a consent-grant registration command into a validated event.
    pub fn from_command(command: &RegisterConsentGrantCommand) -> Result<Self, ValidationError> {
        validate_consent_grant_request(command.request(), command.created_at())?;
        let request = command.request();
        Ok(Self {
            data_subject: command.data_subject().to_owned(),
            grant_id: request.grant_id,
            grantee_subject: request.grantee_subject.clone(),
            scope: request.scope,
            purpose: request.purpose.clone(),
            legal_basis: request.legal_basis,
            expires_at: request.expires_at,
            renewed_from_grant_id: request.renewed_from_grant_id,
            created_at: command.created_at(),
        })
    }

    /// Returns the data subject granting access.
    #[must_use]
    pub fn data_subject(&self) -> &str {
        &self.data_subject
    }

    /// Returns the stable consent grant identifier.
    #[must_use]
    pub const fn grant_id(&self) -> Uuid {
        self.grant_id
    }

    /// Returns the subject receiving delegated access.
    #[must_use]
    pub fn grantee_subject(&self) -> &str {
        &self.grantee_subject
    }

    /// Returns the delegated consent scope.
    #[must_use]
    pub const fn scope(&self) -> ConsentScope {
        self.scope
    }

    /// Returns the declared access purpose.
    #[must_use]
    pub fn purpose(&self) -> &str {
        &self.purpose
    }

    /// Returns the legal basis attached to the grant.
    #[must_use]
    pub const fn legal_basis(&self) -> LegalBasis {
        self.legal_basis
    }

    /// Returns the optional expiration timestamp.
    #[must_use]
    pub const fn expires_at(&self) -> Option<OffsetDateTime> {
        self.expires_at
    }

    /// Returns the optional predecessor grant identifier.
    #[must_use]
    pub const fn renewed_from_grant_id(&self) -> Option<Uuid> {
        self.renewed_from_grant_id
    }

    /// Returns the command timestamp.
    #[must_use]
    pub const fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

/// Domain event produced when a consent grant revocation command is admitted.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConsentGrantRevocationRequested {
    data_subject: String,
    grant_id: Uuid,
    revoked_at: OffsetDateTime,
}

impl ConsentGrantRevocationRequested {
    /// Projects a consent-grant revocation command into an event.
    #[must_use]
    pub fn from_command(command: &RevokeConsentGrantCommand) -> Self {
        Self {
            data_subject: command.data_subject().to_owned(),
            grant_id: command.request().grant_id,
            revoked_at: command.revoked_at(),
        }
    }

    /// Returns the data subject that owns the grant.
    #[must_use]
    pub fn data_subject(&self) -> &str {
        &self.data_subject
    }

    /// Returns the revoked grant identifier.
    #[must_use]
    pub const fn grant_id(&self) -> Uuid {
        self.grant_id
    }

    /// Returns the revocation timestamp.
    #[must_use]
    pub const fn revoked_at(&self) -> OffsetDateTime {
        self.revoked_at
    }
}

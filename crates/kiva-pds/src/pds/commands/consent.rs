#![allow(clippy::wildcard_imports)]

use super::super::*;

/// Command requesting registration of a PDS consent grant.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegisterConsentGrantCommand {
    data_subject: String,
    request: ConsentGrantCommandInput,
    created_at: OffsetDateTime,
}

impl RegisterConsentGrantCommand {
    /// Creates a consent-grant registration command.
    #[must_use]
    pub fn new(
        data_subject: impl Into<String>,
        request: ConsentGrantCommandInput,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            data_subject: data_subject.into(),
            request,
            created_at,
        }
    }

    /// Returns the data subject granting access.
    #[must_use]
    pub fn data_subject(&self) -> &str {
        &self.data_subject
    }

    /// Returns the original consent-grant request.
    #[must_use]
    pub const fn request(&self) -> &ConsentGrantCommandInput {
        &self.request
    }

    /// Returns the command timestamp used for consent expiry validation.
    #[must_use]
    pub const fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    /// Projects this command into the domain event it intends to append.
    pub fn consent_grant_registration_requested(
        &self,
    ) -> Result<ConsentGrantRegistrationRequested, ValidationError> {
        ConsentGrantRegistrationRequested::from_command(self)
    }
}

/// Command requesting revocation of a PDS consent grant.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RevokeConsentGrantCommand {
    data_subject: String,
    request: ConsentRevocationCommandInput,
    revoked_at: OffsetDateTime,
}

impl RevokeConsentGrantCommand {
    /// Creates a consent-grant revocation command.
    #[must_use]
    pub fn new(
        data_subject: impl Into<String>,
        request: ConsentRevocationCommandInput,
        revoked_at: OffsetDateTime,
    ) -> Self {
        Self {
            data_subject: data_subject.into(),
            request,
            revoked_at,
        }
    }

    /// Returns the data subject that owns the grant.
    #[must_use]
    pub fn data_subject(&self) -> &str {
        &self.data_subject
    }

    /// Returns the original consent-revocation request.
    #[must_use]
    pub const fn request(&self) -> ConsentRevocationCommandInput {
        self.request
    }

    /// Returns the revocation timestamp.
    #[must_use]
    pub const fn revoked_at(&self) -> OffsetDateTime {
        self.revoked_at
    }

    /// Projects this command into the domain event it intends to append.
    #[must_use]
    pub fn consent_grant_revocation_requested(&self) -> ConsentGrantRevocationRequested {
        ConsentGrantRevocationRequested::from_command(self)
    }
}

#![allow(clippy::wildcard_imports)]

use super::super::*;

/// Command requesting registration of a PDS governance profile.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegisterGovernanceProfileCommand {
    profile_owner_subject: String,
    request: GovernanceProfileCommandInput,
    created_at: OffsetDateTime,
}

impl RegisterGovernanceProfileCommand {
    /// Creates a governance-profile registration command.
    #[must_use]
    pub fn new(
        profile_owner_subject: impl Into<String>,
        request: GovernanceProfileCommandInput,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            profile_owner_subject: profile_owner_subject.into(),
            request,
            created_at,
        }
    }

    /// Returns the subject that owns the governance profile.
    #[must_use]
    pub fn profile_owner_subject(&self) -> &str {
        &self.profile_owner_subject
    }

    /// Returns the original governance-profile request.
    #[must_use]
    pub const fn request(&self) -> &GovernanceProfileCommandInput {
        &self.request
    }

    /// Returns the registration timestamp.
    #[must_use]
    pub const fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    /// Projects this command into the domain event it intends to append.
    pub fn governance_profile_registration_requested(
        &self,
    ) -> Result<GovernanceProfileRegistrationRequested, ValidationError> {
        GovernanceProfileRegistrationRequested::from_command(self)
    }
}

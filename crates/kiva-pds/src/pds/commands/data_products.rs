#![allow(clippy::wildcard_imports)]

use super::super::*;

/// Command requesting registration of an approved PDS data product.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegisterDataProductCommand {
    owner_subject: String,
    request: DataProductRegistrationCommandInput,
    created_at: OffsetDateTime,
}

impl RegisterDataProductCommand {
    /// Creates a data-product registration command.
    #[must_use]
    pub fn new(
        owner_subject: impl Into<String>,
        request: DataProductRegistrationCommandInput,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            owner_subject: owner_subject.into(),
            request,
            created_at,
        }
    }

    /// Returns the data subject that will own the data product.
    #[must_use]
    pub fn owner_subject(&self) -> &str {
        &self.owner_subject
    }

    /// Returns the original data-product request.
    #[must_use]
    pub const fn request(&self) -> &DataProductRegistrationCommandInput {
        &self.request
    }

    /// Returns the command timestamp.
    #[must_use]
    pub const fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    /// Projects this command into the domain event it intends to append.
    ///
    /// Projection validates the request shape and normalized release-artifact
    /// reference before the server application layer asks a persistence adapter
    /// to store the product.
    pub fn data_product_registration_requested(
        &self,
    ) -> Result<DataProductRegistrationRequested, ValidationError> {
        DataProductRegistrationRequested::from_command(self)
    }
}

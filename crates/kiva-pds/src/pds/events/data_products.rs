#![allow(clippy::wildcard_imports)]

use super::super::*;

/// Domain event produced when a data-product registration command is admitted.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataProductRegistrationRequested {
    owner_subject: String,
    data_product_id: Uuid,
    release_artifact: ReleaseArtifactRef,
    title: String,
    purpose: String,
    legal_basis: LegalBasis,
    data_classes: Vec<DataClass>,
    created_at: OffsetDateTime,
}

impl DataProductRegistrationRequested {
    /// Projects a data-product registration command into a validated event.
    pub fn from_command(command: &RegisterDataProductCommand) -> Result<Self, ValidationError> {
        let request = command.request();
        validate_data_product_request_shape(request)?;
        let release_artifact = release_artifact_ref_from_data_product_request(request)?;
        Ok(Self {
            owner_subject: command.owner_subject().to_owned(),
            data_product_id: request.data_product_id,
            release_artifact,
            title: request.title.clone(),
            purpose: request.purpose.clone(),
            legal_basis: request.legal_basis,
            data_classes: request.data_classes.clone(),
            created_at: command.created_at(),
        })
    }

    /// Returns the data subject that owns the data product.
    #[must_use]
    pub fn owner_subject(&self) -> &str {
        &self.owner_subject
    }

    /// Returns the registered data-product identifier.
    #[must_use]
    pub const fn data_product_id(&self) -> Uuid {
        self.data_product_id
    }

    /// Returns the normalized release artifact backing the data product.
    #[must_use]
    pub const fn release_artifact(&self) -> ReleaseArtifactRef {
        self.release_artifact
    }

    /// Returns the human-readable data-product title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the declared data-product purpose.
    #[must_use]
    pub fn purpose(&self) -> &str {
        &self.purpose
    }

    /// Returns the legal basis attached to the data product.
    #[must_use]
    pub const fn legal_basis(&self) -> LegalBasis {
        self.legal_basis
    }

    /// Returns the metadata classes included in the data product.
    #[must_use]
    pub fn data_classes(&self) -> &[DataClass] {
        &self.data_classes
    }

    /// Returns the registration timestamp.
    #[must_use]
    pub const fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

#![allow(clippy::wildcard_imports)]

use super::*;

pub use kiva_release_artifacts::{DataProductSourceKind, ReleaseArtifactKind};

/// Metadata reference to a governed confidential release artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseArtifactRef {
    kind: ReleaseArtifactKind,
}

impl ReleaseArtifactRef {
    /// Creates a governed release-artifact reference.
    #[must_use]
    pub const fn new(kind: ReleaseArtifactKind) -> Self {
        Self { kind }
    }

    /// Creates a release-artifact reference from the legacy PDS source kind.
    #[must_use]
    pub const fn from_source_kind(source_kind: DataProductSourceKind) -> Self {
        Self::new(source_kind.release_artifact_kind())
    }

    /// Returns the release-artifact kind.
    #[must_use]
    pub const fn kind(self) -> ReleaseArtifactKind {
        self.kind
    }

    /// Returns the legacy PDS source kind.
    #[must_use]
    pub const fn source_kind(self) -> DataProductSourceKind {
        self.kind.source_kind()
    }
}

/// Release artifact metadata available when registering a PDS data product.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseArtifactAuthorization {
    artifact: ReleaseArtifactRef,
    artifact_exists: bool,
    principal_matches_artifact_owner: bool,
    artifact_released: bool,
}

impl ReleaseArtifactAuthorization {
    /// Creates release artifact metadata for PDS data-product admission.
    #[must_use]
    pub const fn new(
        artifact: ReleaseArtifactRef,
        artifact_exists: bool,
        principal_matches_artifact_owner: bool,
        artifact_released: bool,
    ) -> Self {
        Self {
            artifact,
            artifact_exists,
            principal_matches_artifact_owner,
            artifact_released,
        }
    }

    /// Returns the release artifact being referenced.
    #[must_use]
    pub const fn artifact(self) -> ReleaseArtifactRef {
        self.artifact
    }

    /// Returns the release artifact kind being referenced.
    #[must_use]
    pub const fn artifact_kind(self) -> ReleaseArtifactKind {
        self.artifact.kind()
    }

    /// Returns whether the referenced artifact metadata exists.
    #[must_use]
    pub const fn artifact_exists(self) -> bool {
        self.artifact_exists
    }

    /// Returns whether the authenticated principal owns the referenced artifact.
    #[must_use]
    pub const fn principal_matches_artifact_owner(self) -> bool {
        self.principal_matches_artifact_owner
    }

    /// Returns whether a release artifact has completed its governed release.
    #[must_use]
    pub const fn artifact_released(self) -> bool {
        self.artifact_released
    }
}

/// Release metadata available when registering a PDS data product.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataProductReleaseAuthorization {
    artifact: ReleaseArtifactAuthorization,
}

impl DataProductReleaseAuthorization {
    /// Creates release metadata for PDS data-product admission.
    #[must_use]
    pub const fn new(
        source_kind: DataProductSourceKind,
        release_exists: bool,
        principal_matches_release_owner: bool,
        threshold_release_released: bool,
    ) -> Self {
        Self {
            artifact: ReleaseArtifactAuthorization::new(
                ReleaseArtifactRef::from_source_kind(source_kind),
                release_exists,
                principal_matches_release_owner,
                threshold_release_released,
            ),
        }
    }

    /// Creates release metadata from a release-artifact authorization.
    #[must_use]
    pub const fn from_release_artifact(artifact: ReleaseArtifactAuthorization) -> Self {
        Self { artifact }
    }

    /// Returns the underlying release-artifact authorization.
    #[must_use]
    pub const fn release_artifact(self) -> ReleaseArtifactAuthorization {
        self.artifact
    }

    /// Returns the source kind being referenced.
    #[must_use]
    pub const fn source_kind(self) -> DataProductSourceKind {
        self.artifact.artifact().source_kind()
    }

    /// Returns whether the referenced release metadata exists.
    #[must_use]
    pub const fn release_exists(self) -> bool {
        self.artifact.artifact_exists()
    }

    /// Returns whether the authenticated principal owns the referenced release.
    #[must_use]
    pub const fn principal_matches_release_owner(self) -> bool {
        self.artifact.principal_matches_artifact_owner()
    }

    /// Returns whether a threshold-release artifact has completed release.
    #[must_use]
    pub const fn threshold_release_released(self) -> bool {
        self.artifact.artifact_released()
    }
}

/// Data-product metadata available when evaluating delegated PDS access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataProductAccessAuthorization {
    data_product_requested: bool,
    data_product_exists: bool,
    product_owner_matches_delegation_subject: bool,
}

impl DataProductAccessAuthorization {
    /// Creates data-product metadata for PDS access evaluation.
    #[must_use]
    pub const fn new(
        data_product_requested: bool,
        data_product_exists: bool,
        product_owner_matches_delegation_subject: bool,
    ) -> Self {
        Self {
            data_product_requested,
            data_product_exists,
            product_owner_matches_delegation_subject,
        }
    }

    /// Returns whether the access request references a data product.
    #[must_use]
    pub const fn data_product_requested(self) -> bool {
        self.data_product_requested
    }

    /// Returns whether the referenced data product exists.
    #[must_use]
    pub const fn data_product_exists(self) -> bool {
        self.data_product_exists
    }

    /// Returns whether the product owner is the delegated data subject.
    #[must_use]
    pub const fn product_owner_matches_delegation_subject(self) -> bool {
        self.product_owner_matches_delegation_subject
    }
}

//! Release artifact reference boundary types for Kiva.

use serde::{Deserialize, Serialize, Serializer, ser::SerializeStruct};

/// A Kiva confidential release artifact kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ReleaseArtifactKind {
    /// A threshold-authority approved aggregate release.
    #[serde(rename = "threshold-release")]
    ThresholdRelease,
    /// A differential-privacy protected aggregate release.
    #[serde(rename = "differential-privacy-release")]
    DifferentialPrivacyRelease,
    /// A Shamir threshold reconstruction release.
    #[serde(rename = "shamir-threshold-reconstruction")]
    ShamirThresholdReconstruction,
}

impl ReleaseArtifactKind {
    /// Returns the wire label for this release artifact kind.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ThresholdRelease => "threshold-release",
            Self::DifferentialPrivacyRelease => "differential-privacy-release",
            Self::ShamirThresholdReconstruction => "shamir-threshold-reconstruction",
        }
    }

    /// Parses a release artifact kind from its wire label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "threshold-release" => Some(Self::ThresholdRelease),
            "differential-privacy-release" => Some(Self::DifferentialPrivacyRelease),
            "shamir-threshold-reconstruction" => Some(Self::ShamirThresholdReconstruction),
            _other => None,
        }
    }

    /// Returns the compatibility data-product source kind for this artifact kind.
    #[must_use]
    pub const fn source_kind(self) -> DataProductSourceKind {
        DataProductSourceKind::from_release_artifact_kind(self)
    }
}

/// Source kind for an approved Kiva PDS data product.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DataProductSourceKind {
    /// A threshold-authority approved aggregate release.
    #[serde(rename = "threshold-release")]
    ThresholdRelease,
    /// A differential-privacy protected aggregate release.
    #[serde(rename = "differential-privacy-release")]
    DifferentialPrivacyRelease,
    /// A Shamir threshold reconstruction release.
    #[serde(rename = "shamir-threshold-reconstruction")]
    ShamirThresholdReconstruction,
}

impl DataProductSourceKind {
    /// Returns the wire label for this data-product source kind.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ThresholdRelease => "threshold-release",
            Self::DifferentialPrivacyRelease => "differential-privacy-release",
            Self::ShamirThresholdReconstruction => "shamir-threshold-reconstruction",
        }
    }

    /// Parses a data-product source kind from its wire label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "threshold-release" => Some(Self::ThresholdRelease),
            "differential-privacy-release" => Some(Self::DifferentialPrivacyRelease),
            "shamir-threshold-reconstruction" => Some(Self::ShamirThresholdReconstruction),
            _other => None,
        }
    }

    /// Returns the corresponding release artifact kind.
    #[must_use]
    pub const fn release_artifact_kind(self) -> ReleaseArtifactKind {
        match self {
            Self::ThresholdRelease => ReleaseArtifactKind::ThresholdRelease,
            Self::DifferentialPrivacyRelease => ReleaseArtifactKind::DifferentialPrivacyRelease,
            Self::ShamirThresholdReconstruction => {
                ReleaseArtifactKind::ShamirThresholdReconstruction
            }
        }
    }

    /// Returns the compatibility data-product source kind for an artifact kind.
    #[must_use]
    pub const fn from_release_artifact_kind(kind: ReleaseArtifactKind) -> Self {
        match kind {
            ReleaseArtifactKind::ThresholdRelease => Self::ThresholdRelease,
            ReleaseArtifactKind::DifferentialPrivacyRelease => Self::DifferentialPrivacyRelease,
            ReleaseArtifactKind::ShamirThresholdReconstruction => {
                Self::ShamirThresholdReconstruction
            }
        }
    }
}

/// A normalized reference to a confidential release artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub struct ReleaseArtifactRef {
    kind: ReleaseArtifactKind,
    #[serde(rename = "artifactId")]
    artifact_id: [u8; 32],
}

impl ReleaseArtifactRef {
    /// Creates a normalized release artifact reference.
    #[must_use]
    pub const fn new(kind: ReleaseArtifactKind, artifact_id: [u8; 32]) -> Self {
        Self { kind, artifact_id }
    }

    /// Creates a normalized artifact reference from the legacy data-product source kind.
    #[must_use]
    pub const fn from_data_product_source_kind(
        source_kind: DataProductSourceKind,
        artifact_id: [u8; 32],
    ) -> Self {
        Self::new(source_kind.release_artifact_kind(), artifact_id)
    }

    /// Parses a normalized artifact reference from a legacy source kind and hex identifier.
    pub fn from_data_product_source_hex(
        source_kind: DataProductSourceKind,
        artifact_id: &str,
    ) -> Result<Self, ReleaseArtifactValidationError> {
        let artifact_id = fixed_hex_32(artifact_id, "release_id")?;
        Self::from_data_product_source_bytes(source_kind, artifact_id)
    }

    /// Creates an artifact reference from a legacy source kind and validates the identifier.
    pub fn from_data_product_source_bytes(
        source_kind: DataProductSourceKind,
        artifact_id: [u8; 32],
    ) -> Result<Self, ReleaseArtifactValidationError> {
        if artifact_id.iter().all(|byte| *byte == 0) {
            return Err(ReleaseArtifactValidationError::new(
                "PDS data product release_id must not be the reserved empty identifier",
            ));
        }
        Ok(Self::from_data_product_source_kind(
            source_kind,
            artifact_id,
        ))
    }

    /// Returns the artifact kind.
    #[must_use]
    pub const fn kind(self) -> ReleaseArtifactKind {
        self.kind
    }

    /// Returns the artifact identifier bytes.
    #[must_use]
    pub const fn artifact_id(self) -> [u8; 32] {
        self.artifact_id
    }

    /// Returns the compatibility data-product source kind for this artifact.
    #[must_use]
    pub const fn source_kind(self) -> DataProductSourceKind {
        DataProductSourceKind::from_release_artifact_kind(self.kind)
    }

    /// Returns the artifact identifier as a lowercase hex string.
    #[must_use]
    pub fn artifact_id_hex(self) -> String {
        hex_lower(&self.artifact_id)
    }
}

impl Serialize for ReleaseArtifactRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ReleaseArtifactRef", 2)?;
        state.serialize_field("kind", &self.kind)?;
        state.serialize_field("artifactId", &hex_lower(&self.artifact_id))?;
        state.end()
    }
}

/// Release artifact reference validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseArtifactValidationError {
    message: String,
}

impl ReleaseArtifactValidationError {
    /// Creates a validation error with a stable human-readable message.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the validation message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn fixed_hex_32(value: &str, field: &str) -> Result<[u8; 32], ReleaseArtifactValidationError> {
    if value.len() != 64 {
        return Err(ReleaseArtifactValidationError::new(format!(
            "{field} must be a 64-character lowercase hex string"
        )));
    }
    let mut output = [0_u8; 32];
    let bytes = value.as_bytes();
    for index in 0..32 {
        output[index] =
            (hex_nibble(bytes[index * 2], field)? << 4) | hex_nibble(bytes[index * 2 + 1], field)?;
    }
    Ok(output)
}

fn hex_nibble(value: u8, field: &str) -> Result<u8, ReleaseArtifactValidationError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _other => Err(ReleaseArtifactValidationError::new(format!(
            "{field} must be a 64-character lowercase hex string"
        ))),
    }
}

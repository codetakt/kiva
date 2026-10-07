//! Small confidential-computation boundary types shared by pure application
//! workflows and server adapters.

use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Byte width for confidential transcript, job, and release identifiers.
pub const CONFIDENTIAL_ARTIFACT_BYTES: usize = 32;

/// Ownership plane for a confidential-computation boundary fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfidentialBoundaryPlane {
    /// Kiva-owned metadata, evidence, admission, policy, or release control.
    KivaControlPlane,
    /// External provider-owned secret, ciphertext, runtime, or cryptographic execution boundary.
    ExternalDataPlane,
}

impl ConfidentialBoundaryPlane {
    /// Returns the stable evidence label for this boundary plane.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::KivaControlPlane => "kiva-control-plane",
            Self::ExternalDataPlane => "external-data-plane",
        }
    }
}

/// Marker implemented by zero-sized boundary plane owners.
pub trait ConfidentialPlaneMarker {
    /// Boundary plane represented by the marker.
    const PLANE: ConfidentialBoundaryPlane;
}

/// Marker for Kiva-owned control-plane evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KivaControlPlane;

impl ConfidentialPlaneMarker for KivaControlPlane {
    const PLANE: ConfidentialBoundaryPlane = ConfidentialBoundaryPlane::KivaControlPlane;
}

/// Marker for externally owned data-plane boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalDataPlane;

impl ConfidentialPlaneMarker for ExternalDataPlane {
    const PLANE: ConfidentialBoundaryPlane = ConfidentialBoundaryPlane::ExternalDataPlane;
}

/// Value tagged with a confidential-computation ownership plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlaneScoped<T, P> {
    value: T,
    #[serde(skip)]
    _plane: PhantomData<P>,
}

impl<T, P> PlaneScoped<T, P> {
    /// Tags a value with a confidential-computation ownership plane.
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self {
            value,
            _plane: PhantomData,
        }
    }

    /// Returns a shared reference to the tagged value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Removes the plane marker and returns the tagged value.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.value
    }

    /// Maps the tagged value while preserving the same ownership plane.
    #[must_use]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> PlaneScoped<U, P> {
        PlaneScoped::new(f(self.value))
    }
}

impl<T, P> PlaneScoped<T, P>
where
    P: ConfidentialPlaneMarker,
{
    /// Returns the ownership plane attached to this value.
    #[must_use]
    pub const fn plane(&self) -> ConfidentialBoundaryPlane {
        P::PLANE
    }

    /// Returns the stable evidence label for this ownership plane.
    #[must_use]
    pub const fn plane_label(&self) -> &'static str {
        P::PLANE.label()
    }
}

/// Kiva-owned confidential-computation control-plane evidence.
pub type ControlPlaneEvidence<T> = PlaneScoped<T, KivaControlPlane>;

/// External provider-owned confidential-computation data-plane boundary.
pub type ExternalDataPlaneBoundary<T> = PlaneScoped<T, ExternalDataPlane>;

/// Boundary validation errors for confidential core identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ConfidentialCoreError {
    /// The all-zero transcript identifier is reserved and invalid.
    #[error("confidential transcript identifier must not be empty")]
    EmptyTranscriptIdentifier,
}

/// Identifier for one confidential computation transcript or job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "[u8; 32]", into = "[u8; 32]")]
pub struct SecureAggregateTranscriptId {
    bytes: [u8; CONFIDENTIAL_ARTIFACT_BYTES],
}

impl SecureAggregateTranscriptId {
    /// Creates a transcript identifier after rejecting the reserved empty value.
    pub fn new(bytes: [u8; CONFIDENTIAL_ARTIFACT_BYTES]) -> Result<Self, ConfidentialCoreError> {
        if bytes.iter().all(|byte| *byte == 0) {
            return Err(ConfidentialCoreError::EmptyTranscriptIdentifier);
        }
        Ok(Self { bytes })
    }

    /// Creates an identifier from bytes that were validated by another
    /// confidential boundary type.
    #[must_use]
    pub const fn from_validated_bytes(bytes: [u8; CONFIDENTIAL_ARTIFACT_BYTES]) -> Self {
        Self { bytes }
    }

    /// Returns the canonical bytes for this transcript identifier.
    #[must_use]
    pub const fn bytes(self) -> [u8; CONFIDENTIAL_ARTIFACT_BYTES] {
        self.bytes
    }
}

impl TryFrom<[u8; CONFIDENTIAL_ARTIFACT_BYTES]> for SecureAggregateTranscriptId {
    type Error = ConfidentialCoreError;

    fn try_from(bytes: [u8; CONFIDENTIAL_ARTIFACT_BYTES]) -> Result<Self, Self::Error> {
        Self::new(bytes)
    }
}

impl From<SecureAggregateTranscriptId> for [u8; CONFIDENTIAL_ARTIFACT_BYTES] {
    fn from(value: SecureAggregateTranscriptId) -> Self {
        value.bytes()
    }
}

/// Durable confidential job lifecycle status used at application boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfidentialJobStatus {
    /// Job metadata is registered but not admitted for execution.
    Draft,
    /// Job passed admission checks and is queued for execution.
    Admitted,
    /// Job is leased by a worker or scheduler.
    Running,
    /// Job computation completed and waits for a governed release.
    AwaitingRelease,
    /// Job output was released through the applicable policy gate.
    Released,
    /// Job failed without a released result.
    Failed,
    /// Job was cancelled by an authorized actor.
    Cancelled,
    /// Job deadline elapsed before completion or release.
    Expired,
    /// Job metadata was retained only for audit and is not executable.
    Retired,
}

impl ConfidentialJobStatus {
    /// Returns the stable API label for this status.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Admitted => "admitted",
            Self::Running => "running",
            Self::AwaitingRelease => "awaitingRelease",
            Self::Released => "released",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
            Self::Retired => "retired",
        }
    }

    /// Parses a stable API status label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(Self::Draft),
            "admitted" => Some(Self::Admitted),
            "running" => Some(Self::Running),
            "awaitingRelease" => Some(Self::AwaitingRelease),
            "released" => Some(Self::Released),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            "expired" => Some(Self::Expired),
            "retired" => Some(Self::Retired),
            _ => None,
        }
    }

    /// Returns whether the status accepts no further execution or release work.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Released | Self::Failed | Self::Cancelled | Self::Expired | Self::Retired
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_scoped_values_preserve_plane_labels() {
        let control = ControlPlaneEvidence::new(7_u8);
        assert_eq!(control.plane(), ConfidentialBoundaryPlane::KivaControlPlane);
        assert_eq!(control.plane_label(), "kiva-control-plane");
        assert_eq!(*control.value(), 7);

        let data = ExternalDataPlaneBoundary::new("runtime");
        assert_eq!(data.plane(), ConfidentialBoundaryPlane::ExternalDataPlane);
        assert_eq!(data.plane_label(), "external-data-plane");
        assert_eq!(data.into_inner(), "runtime");
    }

    #[test]
    fn plane_scoped_map_preserves_plane() {
        let control = ControlPlaneEvidence::new(5_u8).map(u16::from);
        assert_eq!(control.plane(), ConfidentialBoundaryPlane::KivaControlPlane);
        assert_eq!(control.into_inner(), 5_u16);
    }
}

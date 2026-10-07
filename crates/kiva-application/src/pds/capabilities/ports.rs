//! Aggregated PDS application capability ports.

use crate::runtime::{ApplicationRuntimePort, DefaultApplicationRuntime};

use super::{
    projection::{PdsSubjectIdentityMapper, PdsSubjectProjection, PdsSubjectProjectionPort},
    records::{
        PdsRecordReadPort, PdsRecordWritePort, ReleaseArtifactAvailability, XapiStatementReader,
    },
};

/// Port aggregate for pure PDS application workflows.
pub trait PdsApplicationPortSet: Clone + Send + Sync + 'static {
    /// Subject projection capability.
    type SubjectProjection: PdsSubjectProjection;
    /// PDS write capability.
    type PdsWrites: PdsRecordWritePort;
    /// Release-artifact availability capability.
    type ReleaseArtifacts: ReleaseArtifactAvailability;

    /// Returns the subject projection port used by read-side workflows.
    fn subject_projection(&self) -> &Self::SubjectProjection;

    /// Returns the PDS write port.
    fn pds_writes(&self) -> &Self::PdsWrites;

    /// Returns the release-artifact availability port.
    fn release_artifacts(&self) -> &Self::ReleaseArtifacts;
}

/// Concrete PDS application ports assembled by the composition root.
#[derive(Debug, Clone)]
pub struct PdsApplicationPorts<S, PR, PW, R, I>
where
    S: XapiStatementReader,
    PR: PdsRecordReadPort,
    PW: PdsRecordWritePort,
    R: ReleaseArtifactAvailability,
    I: PdsSubjectIdentityMapper,
{
    subject_projection: PdsSubjectProjectionPort<S, PR, I>,
    pds_writes: PW,
    release_artifacts: R,
}

impl<S, PR, PW, R, I> PdsApplicationPorts<S, PR, PW, R, I>
where
    S: XapiStatementReader,
    PR: PdsRecordReadPort,
    PW: PdsRecordWritePort,
    R: ReleaseArtifactAvailability,
    I: PdsSubjectIdentityMapper,
{
    /// Creates a concrete PDS application port aggregate.
    pub const fn new(
        subject_projection: PdsSubjectProjectionPort<S, PR, I>,
        pds_writes: PW,
        release_artifacts: R,
    ) -> Self {
        Self {
            subject_projection,
            pds_writes,
            release_artifacts,
        }
    }
}

impl<S, PR, PW, R, I> PdsApplicationPortSet for PdsApplicationPorts<S, PR, PW, R, I>
where
    S: XapiStatementReader,
    PR: PdsRecordReadPort,
    PW: PdsRecordWritePort,
    R: ReleaseArtifactAvailability,
    I: PdsSubjectIdentityMapper,
{
    type SubjectProjection = PdsSubjectProjectionPort<S, PR, I>;
    type PdsWrites = PW;
    type ReleaseArtifacts = R;

    fn subject_projection(&self) -> &Self::SubjectProjection {
        &self.subject_projection
    }

    fn pds_writes(&self) -> &Self::PdsWrites {
        &self.pds_writes
    }

    fn release_artifacts(&self) -> &Self::ReleaseArtifacts {
        &self.release_artifacts
    }
}

/// Capability aggregate for pure PDS application workflows.
#[derive(Debug, Clone)]
pub struct PdsApplicationCapabilities<P, A = DefaultApplicationRuntime>
where
    P: PdsApplicationPortSet,
    A: ApplicationRuntimePort,
{
    ports: P,
    runtime: A,
}

impl<P> PdsApplicationCapabilities<P, DefaultApplicationRuntime>
where
    P: PdsApplicationPortSet,
{
    /// Creates a capability aggregate from explicit ports and the default runtime.
    pub fn new(ports: P) -> Self {
        Self::with_runtime(ports, DefaultApplicationRuntime::default())
    }
}

impl<P, A> PdsApplicationCapabilities<P, A>
where
    P: PdsApplicationPortSet,
    A: ApplicationRuntimePort,
{
    /// Creates a capability aggregate from explicit ports and runtime.
    pub const fn with_runtime(ports: P, runtime: A) -> Self {
        Self { ports, runtime }
    }

    /// Returns the PDS write port.
    pub fn pds_writes(&self) -> &P::PdsWrites {
        self.ports.pds_writes()
    }

    /// Returns the subject projection port used by read-side workflows.
    pub fn subject_projection(&self) -> &P::SubjectProjection {
        self.ports.subject_projection()
    }

    /// Returns the release-artifact availability port.
    pub fn release_artifacts(&self) -> &P::ReleaseArtifacts {
        self.ports.release_artifacts()
    }

    /// Returns the application runtime port.
    pub const fn runtime(&self) -> &A {
        &self.runtime
    }
}

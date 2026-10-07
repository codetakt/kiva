//! PDS application orchestration across xAPI and confidential-computation capabilities.

use crate::runtime::{ApplicationRuntimePort, DefaultApplicationRuntime};

mod capabilities;
mod commands;
mod model;
mod queries;
pub(crate) mod workflow;

pub use crate::runtime::ApplicationFuture;
pub use capabilities::{
    AccountHomePagePdsSubjectIdentityMapper, PdsApplicationCapabilities, PdsApplicationPortSet,
    PdsApplicationPorts, PdsRecordReadPort, PdsRecordWritePort, PdsSubjectIdentityMapper,
    PdsSubjectProjection, PdsSubjectProjectionPort, ReleaseArtifactAvailability,
    XapiStatementReader,
};
pub use model::{
    ApplicationRevocationOutcome, ApplicationRuntimeError, ApplicationStoreOutcome,
    ApplicationWriteOutcome, ConsentGrantRecord, DataProductRecord, GovernanceProfileRecord,
    LifecycleCaseRecord, LifecycleEventRecord, MetadataExport, PdsApplicationStoreError,
    PdsApplicationWriteError, PdsMetadataRecords, PdsReadModelError, PortabilityExport,
    PortabilityStatementPage, PortabilityStatementRecord, RecordInventory,
    RegisterDataProductError, ReleaseArtifactAvailabilityError, RetentionPolicyRecord,
    StatementReadError, StatementRecord,
};
pub(crate) use workflow::PdsWorkflowKind;

/// PDS application service backed by explicit xAPI, PDS, and release-artifact ports.
#[derive(Clone)]
pub struct PdsApplication<P, A = DefaultApplicationRuntime>
where
    P: PdsApplicationPortSet,
    A: ApplicationRuntimePort,
{
    capabilities: PdsApplicationCapabilities<P, A>,
}

impl<P, A> PdsApplication<P, A>
where
    P: PdsApplicationPortSet,
    A: ApplicationRuntimePort,
{
    /// Creates a PDS application service from an already assembled capability set.
    pub const fn new(capabilities: PdsApplicationCapabilities<P, A>) -> Self {
        Self { capabilities }
    }
}

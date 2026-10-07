//! PDS application result and error module owner.

mod errors;
mod records;
mod snapshot;

pub(super) use errors::PdsApplicationCommandError;
pub use errors::{
    ApplicationRevocationOutcome, ApplicationRuntimeError, ApplicationStoreOutcome,
    ApplicationWriteOutcome, PdsApplicationStoreError, PdsApplicationWriteError, PdsReadModelError,
    RegisterDataProductError, ReleaseArtifactAvailabilityError, StatementReadError,
};
pub use records::{
    ConsentGrantRecord, DataProductRecord, GovernanceProfileRecord, LifecycleCaseRecord,
    LifecycleEventRecord, PortabilityStatementPage, PortabilityStatementRecord,
    RetentionPolicyRecord, StatementRecord,
};
pub use snapshot::{MetadataExport, PdsMetadataRecords, PortabilityExport, RecordInventory};
pub(super) use snapshot::{PdsPortabilitySnapshot, PdsSubjectId, PdsSubjectSnapshot};

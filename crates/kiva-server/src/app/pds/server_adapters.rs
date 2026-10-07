//! Server adapter bindings for PDS application capabilities.

mod pds_records;
mod projection;
mod records;
mod release_artifacts;
mod xapi;

pub(crate) use pds_records::PdsRecordPort;
pub(crate) use projection::{PdsSubjectIdentityMapper, PdsSubjectProjectionPort};
pub(crate) use release_artifacts::ReleaseArtifactAvailabilityPort;
pub(crate) use xapi::XapiStatementReadPort;

use crate::core::config::PDS_SUBJECT_ACCOUNT_HOME_PAGE;
use crate::surfaces::{pds::repository::PdsRepository, xapi::repository::StatementRepository};

pub(crate) fn capabilities_from_server_adapters(
    statements: &StatementRepository,
    pds: &PdsRepository,
    release_artifacts: &ReleaseArtifactAvailabilityPort,
) -> super::PdsApplicationCapabilities {
    let statements = XapiStatementReadPort::from_server_adapter(statements);
    let pds_reads = PdsRecordPort::from_server_adapter(pds);
    let pds_writes = pds_reads.clone();
    let identity = PdsSubjectIdentityMapper::new(PDS_SUBJECT_ACCOUNT_HOME_PAGE);
    let subject_projection = PdsSubjectProjectionPort::new(identity, statements, pds_reads);
    let ports =
        super::PdsApplicationPorts::new(subject_projection, pds_writes, release_artifacts.clone());
    super::PdsApplicationCapabilities::new(ports)
}

//! Subject projection adapter type binding for PDS application reads.

use super::{pds_records::PdsRecordPort, xapi::XapiStatementReadPort};

pub(crate) type PdsSubjectIdentityMapper =
    kiva_application::pds::AccountHomePagePdsSubjectIdentityMapper;
pub(crate) type PdsSubjectProjectionPort = kiva_application::pds::PdsSubjectProjectionPort<
    XapiStatementReadPort,
    PdsRecordPort,
    PdsSubjectIdentityMapper,
>;

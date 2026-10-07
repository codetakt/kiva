//! PDS application adapter outcome mapping.

use crate::{
    app::pds::{ApplicationRevocationOutcome, ApplicationStoreOutcome},
    core::outcome::StoreOutcome,
    surfaces::pds::outcome::PdsRevocationOutcome,
};

pub(super) const fn application_store_outcome(outcome: StoreOutcome) -> ApplicationStoreOutcome {
    match outcome {
        StoreOutcome::Stored => ApplicationStoreOutcome::Stored,
        StoreOutcome::Unchanged => ApplicationStoreOutcome::Unchanged,
        StoreOutcome::Conflict => ApplicationStoreOutcome::Conflict,
    }
}

pub(super) const fn application_revocation_outcome(
    outcome: PdsRevocationOutcome,
) -> ApplicationRevocationOutcome {
    match outcome {
        PdsRevocationOutcome::Revoked => ApplicationRevocationOutcome::Revoked,
        PdsRevocationOutcome::AlreadyRevoked => ApplicationRevocationOutcome::AlreadyRevoked,
        PdsRevocationOutcome::NotFound => ApplicationRevocationOutcome::NotFound,
        PdsRevocationOutcome::Forbidden => ApplicationRevocationOutcome::Forbidden,
    }
}

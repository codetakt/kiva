//! PDS approved-data-product server-local state.

use kiva_pds::{
    DataClass as PdsDataClass, DataProductSourceKind as PdsDataProductSourceKind,
    LegalBasis as PdsLegalBasis, ReleaseArtifactRef as PdsReleaseArtifactRef,
    pds_subject_actor_ifi as pds_account_subject_actor_ifi,
};
use kiva_policy::policy::Decision;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::core::config::PDS_SUBJECT_ACCOUNT_HOME_PAGE;

use super::access::PdsAccessAuditEventState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdsDataProductState {
    pub(crate) data_product_id: Uuid,
    pub(crate) owner_subject: String,
    pub(crate) release_artifact: PdsReleaseArtifactRef,
    pub(crate) title: String,
    pub(crate) purpose: String,
    pub(crate) legal_basis: PdsLegalBasis,
    pub(crate) data_classes: Vec<PdsDataClass>,
    pub(crate) created_at: OffsetDateTime,
}

impl PdsDataProductState {
    pub(crate) fn same_registration(&self, other: &Self) -> bool {
        self.data_product_id == other.data_product_id
            && self.owner_subject == other.owner_subject
            && self.release_artifact == other.release_artifact
            && self.title == other.title
            && self.purpose == other.purpose
            && self.legal_basis == other.legal_basis
            && self.data_classes == other.data_classes
    }

    pub(crate) const fn source_kind(&self) -> PdsDataProductSourceKind {
        self.release_artifact.source_kind()
    }

    pub(crate) const fn release_artifact(&self) -> PdsReleaseArtifactRef {
        self.release_artifact
    }
}

pub(crate) fn pds_subject_actor_ifi(subject: &str) -> String {
    pds_account_subject_actor_ifi(PDS_SUBJECT_ACCOUNT_HOME_PAGE, subject)
}

#[derive(Debug, Clone)]
pub(crate) struct PdsDataProductReadOutcome {
    pub(crate) event: PdsAccessAuditEventState,
    pub(crate) data_product: Option<PdsDataProductState>,
}

impl PdsDataProductReadOutcome {
    pub(crate) fn from_event_and_product(
        event: PdsAccessAuditEventState,
        data_product: Option<PdsDataProductState>,
    ) -> Self {
        let data_product = if matches!(event.decision, Decision::Allow) {
            data_product
        } else {
            None
        };
        Self {
            event,
            data_product,
        }
    }
}

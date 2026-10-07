#![allow(clippy::wildcard_imports)]

use super::*;

#[path = "events/consent.rs"]
mod consent;
#[path = "events/data_products.rs"]
mod data_products;
#[path = "events/governance.rs"]
mod governance;
#[path = "events/lifecycle.rs"]
mod lifecycle;
#[path = "events/portability.rs"]
mod portability;

pub use consent::{ConsentGrantRegistrationRequested, ConsentGrantRevocationRequested};
pub use data_products::DataProductRegistrationRequested;
pub use governance::GovernanceProfileRegistrationRequested;
pub use lifecycle::{
    LifecycleCaseUpsertRequested, LifecycleEventRecordingRequested,
    RetentionPolicyRegistrationRequested,
};
pub use portability::{PortabilityExportAuditOutcome, PortabilityExportAuditRecorded};

/// PDS domain event envelope.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum PdsDomainEvent {
    /// A consent grant registration has been admitted by the PDS command model.
    #[serde(rename = "consent-grant-registration-requested")]
    ConsentGrantRegistrationRequested(ConsentGrantRegistrationRequested),

    /// A consent grant revocation has been admitted by the PDS command model.
    #[serde(rename = "consent-grant-revocation-requested")]
    ConsentGrantRevocationRequested(ConsentGrantRevocationRequested),

    /// A data-product registration has been admitted by the PDS command model.
    #[serde(rename = "data-product-registration-requested")]
    DataProductRegistrationRequested(DataProductRegistrationRequested),

    /// A lifecycle event recording has been admitted by the PDS command model.
    #[serde(rename = "lifecycle-event-recording-requested")]
    LifecycleEventRecordingRequested(LifecycleEventRecordingRequested),

    /// A retention-policy registration has been admitted by the PDS command model.
    #[serde(rename = "retention-policy-registration-requested")]
    RetentionPolicyRegistrationRequested(RetentionPolicyRegistrationRequested),

    /// A lifecycle-case upsert has been admitted by the PDS command model.
    #[serde(rename = "lifecycle-case-upsert-requested")]
    LifecycleCaseUpsertRequested(LifecycleCaseUpsertRequested),

    /// A governance-profile registration has been admitted by the PDS command model.
    #[serde(rename = "governance-profile-registration-requested")]
    GovernanceProfileRegistrationRequested(GovernanceProfileRegistrationRequested),

    /// A learner-initiated portability export audit event has been admitted.
    #[serde(rename = "portability-export-audit-recorded")]
    PortabilityExportAuditRecorded(PortabilityExportAuditRecorded),
}

impl PdsDomainEvent {
    /// Returns the principal subject that controls this domain event.
    #[must_use]
    pub fn controlling_subject(&self) -> &str {
        match self {
            Self::ConsentGrantRegistrationRequested(event) => event.data_subject(),
            Self::ConsentGrantRevocationRequested(event) => event.data_subject(),
            Self::DataProductRegistrationRequested(event) => event.owner_subject(),
            Self::LifecycleEventRecordingRequested(event) => event.data_subject(),
            Self::RetentionPolicyRegistrationRequested(event) => event.owner_subject(),
            Self::LifecycleCaseUpsertRequested(event) => event.data_subject(),
            Self::GovernanceProfileRegistrationRequested(event) => event.profile_owner_subject(),
            Self::PortabilityExportAuditRecorded(event) => event.data_subject(),
        }
    }
}

impl From<ConsentGrantRegistrationRequested> for PdsDomainEvent {
    fn from(event: ConsentGrantRegistrationRequested) -> Self {
        Self::ConsentGrantRegistrationRequested(event)
    }
}

impl From<ConsentGrantRevocationRequested> for PdsDomainEvent {
    fn from(event: ConsentGrantRevocationRequested) -> Self {
        Self::ConsentGrantRevocationRequested(event)
    }
}

impl From<DataProductRegistrationRequested> for PdsDomainEvent {
    fn from(event: DataProductRegistrationRequested) -> Self {
        Self::DataProductRegistrationRequested(event)
    }
}

impl From<LifecycleEventRecordingRequested> for PdsDomainEvent {
    fn from(event: LifecycleEventRecordingRequested) -> Self {
        Self::LifecycleEventRecordingRequested(event)
    }
}

impl From<RetentionPolicyRegistrationRequested> for PdsDomainEvent {
    fn from(event: RetentionPolicyRegistrationRequested) -> Self {
        Self::RetentionPolicyRegistrationRequested(event)
    }
}

impl From<LifecycleCaseUpsertRequested> for PdsDomainEvent {
    fn from(event: LifecycleCaseUpsertRequested) -> Self {
        Self::LifecycleCaseUpsertRequested(event)
    }
}

impl From<GovernanceProfileRegistrationRequested> for PdsDomainEvent {
    fn from(event: GovernanceProfileRegistrationRequested) -> Self {
        Self::GovernanceProfileRegistrationRequested(event)
    }
}

impl From<PortabilityExportAuditRecorded> for PdsDomainEvent {
    fn from(event: PortabilityExportAuditRecorded) -> Self {
        Self::PortabilityExportAuditRecorded(event)
    }
}

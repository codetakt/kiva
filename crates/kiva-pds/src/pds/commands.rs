#![allow(clippy::wildcard_imports)]

use super::*;

#[path = "commands/consent.rs"]
mod consent;
#[path = "commands/data_products.rs"]
mod data_products;
#[path = "commands/governance.rs"]
mod governance;
#[path = "commands/lifecycle.rs"]
mod lifecycle;

pub use consent::{RegisterConsentGrantCommand, RevokeConsentGrantCommand};
pub use data_products::RegisterDataProductCommand;
pub use governance::RegisterGovernanceProfileCommand;
pub use lifecycle::{
    RecordLifecycleEventCommand, RegisterRetentionPolicyCommand, UpsertLifecycleCaseCommand,
};

/// PDS domain command envelope.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum PdsCommand {
    /// Requests registration of a PDS consent grant.
    #[serde(rename = "register-consent-grant")]
    RegisterConsentGrant(RegisterConsentGrantCommand),

    /// Requests revocation of a PDS consent grant.
    #[serde(rename = "revoke-consent-grant")]
    RevokeConsentGrant(RevokeConsentGrantCommand),

    /// Requests registration of an approved PDS data product.
    #[serde(rename = "register-data-product")]
    RegisterDataProduct(RegisterDataProductCommand),

    /// Requests recording of a PDS lifecycle event.
    #[serde(rename = "record-lifecycle-event")]
    RecordLifecycleEvent(RecordLifecycleEventCommand),

    /// Requests registration of a PDS retention policy.
    #[serde(rename = "register-retention-policy")]
    RegisterRetentionPolicy(RegisterRetentionPolicyCommand),

    /// Requests creation or transition of a PDS lifecycle case.
    #[serde(rename = "upsert-lifecycle-case")]
    UpsertLifecycleCase(UpsertLifecycleCaseCommand),

    /// Requests registration of a PDS governance profile.
    #[serde(rename = "register-governance-profile")]
    RegisterGovernanceProfile(RegisterGovernanceProfileCommand),
}

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

pub use kiva_release_artifacts::{DataProductSourceKind, ReleaseArtifactKind, ReleaseArtifactRef};

mod commands;
mod events;
mod identity;
mod release_artifacts;
mod requests;
mod responses;
#[cfg(test)]
mod tests;
mod validation;
mod wire_enums;

pub use commands::{
    PdsCommand, RecordLifecycleEventCommand, RegisterConsentGrantCommand,
    RegisterDataProductCommand, RegisterGovernanceProfileCommand, RegisterRetentionPolicyCommand,
    RevokeConsentGrantCommand, UpsertLifecycleCaseCommand,
};
pub use events::{
    ConsentGrantRegistrationRequested, ConsentGrantRevocationRequested,
    DataProductRegistrationRequested, GovernanceProfileRegistrationRequested,
    LifecycleCaseUpsertRequested, LifecycleEventRecordingRequested, PdsDomainEvent,
    PortabilityExportAuditOutcome, PortabilityExportAuditRecorded,
    RetentionPolicyRegistrationRequested,
};
pub use identity::{PdsSubjectAccountNamespace, pds_subject_actor_ifi, pds_subject_agent};
pub use release_artifacts::{
    release_artifact_ref_from_data_product_request,
    release_artifact_ref_from_data_product_source_bytes,
};
pub use requests::{
    AccessCheckIntent, ConsentGrantCommandInput, ConsentRevocationCommandInput,
    DataProductReadIntent, DataProductRegistrationCommandInput, GovernanceProfileCommandInput,
    LifecycleCaseCommandInput, LifecycleEventCommandInput, ProviderBindingRegistrationCommandInput,
    RetentionPolicyCommandInput,
};
pub use responses::{
    AccessAuditEvent, AccessAuditEventList, AccessCheckResponse, ConsentGrant, ConsentGrantList,
    DataProduct, DataProductList, DataProductReadResponse, GovernanceProfile,
    GovernanceProfileList, LifecycleCase, LifecycleCaseList, LifecycleEvent, LifecycleEventList,
    MetadataExport, PdsSubjectAccount, PdsSubjectAgent, PortabilityExport, PortabilityStatement,
    ProviderBinding, ProviderBindingList, RecordInventory, RetentionPolicy, RetentionPolicyList,
    StatementInventoryRecord,
};
pub use validation::{
    NormalizedGovernanceProfileInput, NormalizedLifecycleCaseInput, NormalizedRetentionPolicyInput,
    ValidationError, normalize_governance_profile_request, normalize_lifecycle_case_request,
    normalize_retention_policy_request, validate_consent_grant_request,
    validate_data_product_request_shape, validate_lifecycle_event_request,
};
pub use wire_enums::{
    ConsentScope, ConsentState, DataClass, GovernanceProfileKind, LegalBasis, LifecycleCaseStatus,
    LifecycleCaseType, LifecycleEventType, LifecycleResourceKind, TombstonePublicationRule,
};

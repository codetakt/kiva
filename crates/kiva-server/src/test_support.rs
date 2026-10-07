//! Compatibility imports for the extracted server unit-test harness.

#![allow(unused_imports)]

pub(crate) use kiva_confidential::mechanism::{
    additive::{AcceptedAdditiveShareRecord, ReplayWindowedAdditiveAggregate},
    differential_privacy::{
        DifferentialPrivacyAccountingScope, DifferentialPrivacyAdjacencyModel,
        DifferentialPrivacyNoiseMechanism,
    },
    job::ConfidentialJobStatus,
    workflow::ConfidentialParticipantStatus,
};
pub(crate) use kiva_pds::{
    AccessCheckIntent as PdsAccessCheckRequest, ConsentGrantCommandInput as PdsConsentGrantRequest,
    ConsentRevocationCommandInput as PdsConsentRevocationRequest, ConsentScope as PdsConsentScope,
    ConsentState as PdsConsentState, DataClass as PdsDataClass,
    DataProductReadIntent as PdsDataProductReadRequest,
    DataProductRegistrationCommandInput as PdsDataProductRequest,
    DataProductSourceKind as PdsDataProductSourceKind,
    GovernanceProfileCommandInput as PdsGovernanceProfileRequest,
    GovernanceProfileKind as PdsGovernanceProfileKind, LegalBasis as PdsLegalBasis,
    LifecycleCaseCommandInput as PdsLifecycleCaseRequest,
    LifecycleCaseStatus as PdsLifecycleCaseStatus, LifecycleCaseType as PdsLifecycleCaseType,
    LifecycleEventCommandInput as PdsLifecycleEventRequest,
    LifecycleEventType as PdsLifecycleEventType, LifecycleResourceKind as PdsLifecycleResourceKind,
    ProviderBindingRegistrationCommandInput as PdsProviderBindingRequest,
    RetentionPolicyCommandInput as PdsRetentionPolicyRequest,
    TombstonePublicationRule as PdsTombstonePublicationRule,
};
pub(crate) use kiva_xapi::StatementMetadata;

pub(crate) mod app {
    pub(crate) use crate::app::confidential;
    pub(crate) use crate::surfaces::pds::service as pds;
    pub(crate) use crate::surfaces::xapi::service as xapi;
}

pub(crate) mod auth {
    pub(crate) use crate::core::auth::*;
}

pub(crate) mod config {
    pub(crate) use crate::core::config::*;
}

pub(crate) mod extract {
    pub(crate) use crate::core::extract::*;
}

pub(crate) mod handlers {
    pub(crate) use crate::surfaces::confidential::handlers as confidential;
    pub(crate) use crate::surfaces::pds::handlers as pds;
    pub(crate) use crate::surfaces::xapi::handlers as xapi;
}

pub(crate) mod http {
    pub(crate) use crate::surfaces::confidential::routes as confidential;
    pub(crate) use crate::surfaces::pds::routes as pds;
    pub(crate) use crate::surfaces::xapi::routes as xapi;
}

pub(crate) mod outcome {
    pub(crate) use crate::core::outcome::*;
    pub(crate) use crate::surfaces::confidential::outcome::*;
    pub(crate) use crate::surfaces::pds::outcome::*;
    pub(crate) use crate::surfaces::xapi::outcome::*;
}

pub(crate) mod paths {
    pub(crate) use crate::core::paths::*;
}

pub(crate) mod release_artifacts {
    pub(crate) use crate::surfaces::pds::release_artifacts::*;
}

pub(crate) mod repository {
    pub(crate) use crate::surfaces::confidential::repository::ConfidentialRepository;
    pub(crate) use crate::surfaces::pds::repository::PdsRepository;
    pub(crate) use crate::surfaces::xapi::repository::{DocumentRepository, StatementRepository};
}

pub(crate) mod request_util {
    pub(crate) use crate::core::request_util::*;
}

pub(crate) mod response {
    pub(crate) use crate::core::response::*;
}

pub(crate) mod runtime {
    pub(crate) use crate::core::runtime::*;
}

pub(crate) mod state {
    pub(crate) use crate::core::state::*;
}

pub(crate) mod storage {
    pub(crate) use crate::surfaces::confidential::storage as confidential;
    pub(crate) use crate::surfaces::pds::storage as pds;
}

pub(crate) mod wire {
    pub(crate) use crate::surfaces::confidential::wire as confidential;
    pub(crate) use crate::surfaces::pds::wire as pds;
    pub(crate) use crate::surfaces::xapi::wire as xapi;
}

pub(crate) use crate::core::auth::{
    AuthConfig, authorize_confidential_request, authorize_pds_request, authorize_xapi_request,
};
pub(crate) use crate::core::config::*;
pub(crate) use crate::core::runtime::RuntimeLimits;
pub(crate) use crate::surfaces::confidential::model::*;
pub(crate) use crate::surfaces::pds::model::*;
pub(crate) use crate::surfaces::pds::model::{
    pds_action_label, pds_decision_label, pds_deny_reason, pds_deny_reason_label,
    pds_principal_role_label, pds_resource_class_label,
};
pub(crate) use crate::surfaces::xapi::model::*;
pub(crate) use crate::surfaces::xapi::request::*;
pub(crate) use crate::surfaces::{
    confidential::repository::ConfidentialRepository,
    pds::repository::PdsRepository,
    xapi::repository::{DocumentRepository, StatementRepository},
};
pub(crate) use handlers::confidential::*;
pub(crate) use handlers::pds::*;
pub(crate) use handlers::xapi::*;
pub(crate) use outcome::*;
pub(crate) use request_util::*;
pub(crate) use response::*;
pub(crate) use storage::confidential::{
    ensure_matching_differential_privacy_aggregate_result, field_element_i64,
};
pub(crate) use storage::pds::pds_data_class_labels;

//! Common imports for PDS HTTP handler resource modules.

pub(super) use std::collections::BTreeMap;

pub(super) use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
pub(super) use kiva_application::operation_catalog::ApplicationOperationId;
pub(super) use kiva_pds_openapi_contract::{
    AccessCheckRequest as PdsOpenApiAccessCheckRequest,
    ConsentGrantRequest as PdsOpenApiConsentGrantRequest,
    ConsentRevocationRequest as PdsOpenApiConsentRevocationRequest,
    DataProductReadRequest as PdsOpenApiDataProductReadRequest,
    DataProductRequest as PdsOpenApiDataProductRequest,
    GovernanceProfileRequest as PdsOpenApiGovernanceProfileRequest,
    LifecycleCaseRequest as PdsOpenApiLifecycleCaseRequest,
    LifecycleEventRequest as PdsOpenApiLifecycleEventRequest,
    ProviderBindingRequest as PdsOpenApiProviderBindingRequest,
    RetentionPolicyRequest as PdsOpenApiRetentionPolicyRequest,
};
pub(super) use kiva_policy::policy::Decision;

pub(super) use crate::{
    core::{
        paths::{
            PDS_ACCESS_AUDIT_EVENTS_PATH, PDS_ACCESS_CHECKS_PATH, PDS_CONSENT_GRANTS_PATH,
            PDS_CONSENT_REVOCATIONS_PATH, PDS_DATA_PORTABILITY_EXPORTS_PATH,
            PDS_DATA_PRODUCT_READS_PATH, PDS_DATA_PRODUCTS_PATH, PDS_GOVERNANCE_PROFILES_PATH,
            PDS_LIFECYCLE_CASES_PATH, PDS_LIFECYCLE_EVENTS_PATH, PDS_METADATA_EXPORTS_PATH,
            PDS_PROVIDER_BINDINGS_PATH, PDS_RECORD_INVENTORY_PATH, PDS_RETENTION_POLICIES_PATH,
        },
        response::{HttpError, empty_response, json_projection_response},
        state::{PdsApplicationHttpContext, PdsHttpContext},
    },
    surfaces::pds::{
        model::{
            PdsAccessAuditEventFilter, PdsConsentGrantFilter, PdsDataProductFilter,
            PdsGovernanceProfileFilter, PdsLifecycleCaseFilter, PdsLifecycleEventFilter,
            PdsProviderBindingFilter, PdsRetentionPolicyFilter,
        },
        outcome::PdsRepositoryError,
        response::{
            pds_read_model_error_response, pds_store_error_response, pds_write_error_response,
        },
        service as pds_service, wire,
    },
};

//! PDS HTTP routes.

use axum::{
    Router,
    routing::{get, post},
};

use crate::core::{
    paths::{
        PDS_ACCESS_AUDIT_EVENTS_PATH, PDS_ACCESS_CHECKS_PATH, PDS_CONSENT_GRANTS_PATH,
        PDS_CONSENT_REVOCATIONS_PATH, PDS_DATA_PORTABILITY_EXPORTS_PATH,
        PDS_DATA_PRODUCT_READS_PATH, PDS_DATA_PRODUCTS_PATH, PDS_GOVERNANCE_PROFILES_PATH,
        PDS_LIFECYCLE_CASES_PATH, PDS_LIFECYCLE_EVENTS_PATH, PDS_METADATA_EXPORTS_PATH,
        PDS_PROVIDER_BINDINGS_PATH, PDS_RECORD_INVENTORY_PATH, PDS_RETENTION_POLICIES_PATH,
    },
    state::{PdsApplicationHttpContext, PdsHttpContext},
};

use super::handlers::{
    get_pds_access_audit_events, get_pds_consent_grants, get_pds_data_portability_exports,
    get_pds_data_products, get_pds_governance_profiles, get_pds_lifecycle_cases,
    get_pds_lifecycle_events, get_pds_metadata_exports, get_pds_provider_bindings,
    get_pds_record_inventory, get_pds_retention_policies, post_pds_access_check,
    post_pds_consent_grant, post_pds_consent_revocation, post_pds_data_product,
    post_pds_data_product_read, post_pds_governance_profile, post_pds_lifecycle_case,
    post_pds_lifecycle_event, post_pds_provider_binding, post_pds_retention_policy,
};

pub(crate) fn router(
    pds_state: PdsHttpContext,
    application_state: PdsApplicationHttpContext,
) -> Router {
    debug_assert_eq!(
        PdsHttpContext::surface_profile_spec().application_surface,
        Some("pds")
    );
    debug_assert_eq!(
        PdsHttpContext::surface_profile_spec().pds_route_context,
        Some("pds")
    );
    debug_assert_eq!(
        PdsApplicationHttpContext::surface_profile_spec().application_surface,
        Some("pds")
    );
    debug_assert_eq!(
        PdsApplicationHttpContext::surface_profile_spec().pds_route_context,
        Some("pds-application")
    );

    let pds_router = Router::new()
        .route(PDS_CONSENT_GRANTS_PATH, get(get_pds_consent_grants))
        .route(PDS_DATA_PRODUCTS_PATH, get(get_pds_data_products))
        .route(
            PDS_PROVIDER_BINDINGS_PATH,
            get(get_pds_provider_bindings).post(post_pds_provider_binding),
        )
        .route(
            PDS_DATA_PRODUCT_READS_PATH,
            post(post_pds_data_product_read),
        )
        .route(PDS_ACCESS_CHECKS_PATH, post(post_pds_access_check))
        .route(
            PDS_ACCESS_AUDIT_EVENTS_PATH,
            get(get_pds_access_audit_events),
        )
        .route(PDS_LIFECYCLE_EVENTS_PATH, get(get_pds_lifecycle_events))
        .route(PDS_RETENTION_POLICIES_PATH, get(get_pds_retention_policies))
        .route(PDS_LIFECYCLE_CASES_PATH, get(get_pds_lifecycle_cases))
        .route(
            PDS_GOVERNANCE_PROFILES_PATH,
            get(get_pds_governance_profiles),
        )
        .with_state(pds_state);

    let application_router = Router::new()
        .route(PDS_CONSENT_GRANTS_PATH, post(post_pds_consent_grant))
        .route(
            PDS_CONSENT_REVOCATIONS_PATH,
            post(post_pds_consent_revocation),
        )
        .route(PDS_DATA_PRODUCTS_PATH, post(post_pds_data_product))
        .route(PDS_LIFECYCLE_EVENTS_PATH, post(post_pds_lifecycle_event))
        .route(PDS_RETENTION_POLICIES_PATH, post(post_pds_retention_policy))
        .route(PDS_LIFECYCLE_CASES_PATH, post(post_pds_lifecycle_case))
        .route(
            PDS_GOVERNANCE_PROFILES_PATH,
            post(post_pds_governance_profile),
        )
        .route(PDS_RECORD_INVENTORY_PATH, get(get_pds_record_inventory))
        .route(PDS_METADATA_EXPORTS_PATH, get(get_pds_metadata_exports))
        .route(
            PDS_DATA_PORTABILITY_EXPORTS_PATH,
            get(get_pds_data_portability_exports),
        )
        .with_state(application_state);

    pds_router.merge(application_router)
}

//! Generated from `generated/architecture/surface-catalog.yaml`.
//! Do not edit by hand.

use super::surface_operation::{SurfaceOperationProfile, SurfaceProfileSpec};

/// Number of generated server surface profiles.
pub(crate) const SURFACE_PROFILE_COUNT: usize = 4;

/// Stable generated profile id for the governed analytics application surface.
pub(crate) const ANALYTICS_PROFILE_ID: &str = "analytics-application";

/// Runtime role for the generated governed analytics application surface.
pub(crate) const ANALYTICS_RUNTIME_ROLE: &str = "application-workflow";

/// Application-surface label for the generated governed analytics application surface.
pub(crate) const ANALYTICS_APPLICATION_SURFACE: Option<&'static str> = Some("analytics");

/// PDS route-context label for the generated governed analytics application surface.
pub(crate) const ANALYTICS_PDS_ROUTE_CONTEXT: Option<&'static str> = None;

/// Surface operation count covered by the generated governed analytics application surface.
pub(crate) const ANALYTICS_OPERATION_COUNT: usize = 3;

/// Generated marker for the governed analytics application surface.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AnalyticsSurfaceProfile;

impl SurfaceOperationProfile for AnalyticsSurfaceProfile {
    const APPLICATION_SURFACE: Option<&'static str> = ANALYTICS_APPLICATION_SURFACE;
    const FAILURE_LABEL: &'static str = "analytics";
    const OPERATION_COUNT: usize = ANALYTICS_OPERATION_COUNT;
    const PDS_ROUTE_CONTEXT: Option<&'static str> = ANALYTICS_PDS_ROUTE_CONTEXT;
    const PROFILE_ID: &'static str = ANALYTICS_PROFILE_ID;
    const RUNTIME_ROLE: &'static str = ANALYTICS_RUNTIME_ROLE;
}

/// Stable generated profile id for the PDS repository/query surface.
pub(crate) const PDS_REPOSITORY_PROFILE_ID: &str = "pds-repository";

/// Runtime role for the generated PDS repository/query surface.
pub(crate) const PDS_REPOSITORY_RUNTIME_ROLE: &str = "repository-query";

/// Application-surface label for the generated PDS repository/query surface.
pub(crate) const PDS_REPOSITORY_APPLICATION_SURFACE: Option<&'static str> = Some("pds");

/// PDS route-context label for the generated PDS repository/query surface.
pub(crate) const PDS_REPOSITORY_PDS_ROUTE_CONTEXT: Option<&'static str> = Some("pds");

/// Surface operation count covered by the generated PDS repository/query surface.
pub(crate) const PDS_REPOSITORY_OPERATION_COUNT: usize = 11;

/// Generated marker for the PDS repository/query surface.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PdsRepositorySurfaceProfile;

impl SurfaceOperationProfile for PdsRepositorySurfaceProfile {
    const APPLICATION_SURFACE: Option<&'static str> = PDS_REPOSITORY_APPLICATION_SURFACE;
    const FAILURE_LABEL: &'static str = "PDS";
    const OPERATION_COUNT: usize = PDS_REPOSITORY_OPERATION_COUNT;
    const PDS_ROUTE_CONTEXT: Option<&'static str> = PDS_REPOSITORY_PDS_ROUTE_CONTEXT;
    const PROFILE_ID: &'static str = PDS_REPOSITORY_PROFILE_ID;
    const RUNTIME_ROLE: &'static str = PDS_REPOSITORY_RUNTIME_ROLE;
}

/// Stable generated profile id for the PDS application workflow surface.
pub(crate) const PDS_APPLICATION_PROFILE_ID: &str = "pds-application";

/// Runtime role for the generated PDS application workflow surface.
pub(crate) const PDS_APPLICATION_RUNTIME_ROLE: &str = "application-workflow";

/// Application-surface label for the generated PDS application workflow surface.
pub(crate) const PDS_APPLICATION_APPLICATION_SURFACE: Option<&'static str> = Some("pds");

/// PDS route-context label for the generated PDS application workflow surface.
pub(crate) const PDS_APPLICATION_PDS_ROUTE_CONTEXT: Option<&'static str> = Some("pds-application");

/// Surface operation count covered by the generated PDS application workflow surface.
pub(crate) const PDS_APPLICATION_OPERATION_COUNT: usize = 10;

/// Generated marker for the PDS application workflow surface.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PdsApplicationSurfaceProfile;

impl SurfaceOperationProfile for PdsApplicationSurfaceProfile {
    const APPLICATION_SURFACE: Option<&'static str> = PDS_APPLICATION_APPLICATION_SURFACE;
    const FAILURE_LABEL: &'static str = "PDS application";
    const OPERATION_COUNT: usize = PDS_APPLICATION_OPERATION_COUNT;
    const PDS_ROUTE_CONTEXT: Option<&'static str> = PDS_APPLICATION_PDS_ROUTE_CONTEXT;
    const PROFILE_ID: &'static str = PDS_APPLICATION_PROFILE_ID;
    const RUNTIME_ROLE: &'static str = PDS_APPLICATION_RUNTIME_ROLE;
}

/// Stable generated profile id for the Confidential-computation application surface.
pub(crate) const CONFIDENTIAL_PROFILE_ID: &str = "confidential-application";

/// Runtime role for the generated Confidential-computation application surface.
pub(crate) const CONFIDENTIAL_RUNTIME_ROLE: &str = "application-workflow";

/// Application-surface label for the generated Confidential-computation application surface.
pub(crate) const CONFIDENTIAL_APPLICATION_SURFACE: Option<&'static str> = Some("confidential");

/// PDS route-context label for the generated Confidential-computation application surface.
pub(crate) const CONFIDENTIAL_PDS_ROUTE_CONTEXT: Option<&'static str> = None;

/// Surface operation count covered by the generated Confidential-computation application surface.
pub(crate) const CONFIDENTIAL_OPERATION_COUNT: usize = 29;

/// Generated marker for the Confidential-computation application surface.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ConfidentialSurfaceProfile;

impl SurfaceOperationProfile for ConfidentialSurfaceProfile {
    const APPLICATION_SURFACE: Option<&'static str> = CONFIDENTIAL_APPLICATION_SURFACE;
    const FAILURE_LABEL: &'static str = "confidential";
    const OPERATION_COUNT: usize = CONFIDENTIAL_OPERATION_COUNT;
    const PDS_ROUTE_CONTEXT: Option<&'static str> = CONFIDENTIAL_PDS_ROUTE_CONTEXT;
    const PROFILE_ID: &'static str = CONFIDENTIAL_PROFILE_ID;
    const RUNTIME_ROLE: &'static str = CONFIDENTIAL_RUNTIME_ROLE;
}

/// Generated static profile metadata aligned with the surface catalog labels.
pub(crate) const SURFACE_PROFILE_SPECS: [SurfaceProfileSpec; SURFACE_PROFILE_COUNT] = [
    SurfaceProfileSpec::for_profile::<AnalyticsSurfaceProfile>(),
    SurfaceProfileSpec::for_profile::<PdsRepositorySurfaceProfile>(),
    SurfaceProfileSpec::for_profile::<PdsApplicationSurfaceProfile>(),
    SurfaceProfileSpec::for_profile::<ConfidentialSurfaceProfile>(),
];

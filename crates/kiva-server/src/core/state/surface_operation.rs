//! Shared surface operation runner over the application workflow gateway.

use std::{future::Future, marker::PhantomData};

use kiva_application::{
    operation_catalog::ApplicationOperationId,
    runtime::{
        ApplicationOperationGateway, ApplicationRuntimeError, ApplicationWorkflowFailure,
        DefaultApplicationRuntime,
    },
};

pub(crate) use super::generated_surface_profiles::{
    AnalyticsSurfaceProfile, ConfidentialSurfaceProfile, PdsApplicationSurfaceProfile,
    PdsRepositorySurfaceProfile,
};
use super::generated_surface_profiles::{SURFACE_PROFILE_COUNT, SURFACE_PROFILE_SPECS};

/// Compile-time marker for a server surface capability profile.
pub(crate) trait SurfaceOperationProfile {
    /// Stable generated profile id.
    const PROFILE_ID: &'static str;

    /// Public application surface label, when this profile enters one.
    const APPLICATION_SURFACE: Option<&'static str>;

    /// Short label used in workflow failure audit text.
    const FAILURE_LABEL: &'static str;

    /// Number of surface operations covered by this generated profile.
    const OPERATION_COUNT: usize;

    /// PDS route context label, when this profile is a PDS route.
    const PDS_ROUTE_CONTEXT: Option<&'static str>;

    /// Runtime role used by the composition root for this profile.
    const RUNTIME_ROLE: &'static str;
}

/// Static profile metadata aligned with the surface catalog labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SurfaceProfileSpec {
    pub(crate) application_surface: Option<&'static str>,
    pub(crate) operation_count: usize,
    pub(crate) pds_route_context: Option<&'static str>,
    pub(crate) profile_id: &'static str,
    pub(crate) runtime_role: &'static str,
}

impl SurfaceProfileSpec {
    /// Create a profile spec from a marker profile.
    #[must_use]
    pub(crate) const fn for_profile<P>() -> Self
    where
        P: SurfaceOperationProfile,
    {
        Self {
            application_surface: P::APPLICATION_SURFACE,
            operation_count: P::OPERATION_COUNT,
            pds_route_context: P::PDS_ROUTE_CONTEXT,
            profile_id: P::PROFILE_ID,
            runtime_role: P::RUNTIME_ROLE,
        }
    }
}

/// Number of generated surface profiles.
#[must_use]
pub(crate) const fn surface_profile_count() -> usize {
    SURFACE_PROFILE_COUNT
}

/// Surface profile registry used to keep HTTP state contexts aligned with the catalog.
#[must_use]
pub(crate) const fn surface_profile_specs() -> &'static [SurfaceProfileSpec] {
    &SURFACE_PROFILE_SPECS
}

/// Validate generated surface runtime profile metadata used by the composition root.
#[must_use]
pub(crate) fn surface_profile_registry_is_valid() -> bool {
    let profiles = surface_profile_specs();
    profiles.len() == surface_profile_count()
        && profiles.iter().all(|profile| {
            !profile.profile_id.is_empty()
                && !profile.runtime_role.is_empty()
                && profile.operation_count > 0
                && (profile.application_surface.is_some() || profile.pds_route_context.is_some())
        })
}

/// Runs one surface operation through the application workflow gateway.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SurfaceOperationRunner<P> {
    _profile: PhantomData<P>,
}

impl<P> SurfaceOperationRunner<P>
where
    P: SurfaceOperationProfile,
{
    /// Run the provided surface service under the catalogued operation lifecycle.
    pub(crate) async fn run<T, E, Fut, S>(
        operation_id: ApplicationOperationId,
        service: S,
        execute: impl FnOnce(S) -> Fut,
    ) -> Result<T, E>
    where
        E: From<ApplicationRuntimeError>,
        Fut: Future<Output = Result<T, E>>,
    {
        let runtime = DefaultApplicationRuntime::default();
        ApplicationOperationGateway::new(&runtime)
            .execute(operation_id, |_context| async move {
                execute(service).await.map_err(|error| {
                    ApplicationWorkflowFailure::new(
                        format!(
                            "{} {} surface operation failed",
                            operation_id.catalog_id(),
                            P::FAILURE_LABEL,
                        ),
                        error,
                    )
                })
            })
            .await
    }
}

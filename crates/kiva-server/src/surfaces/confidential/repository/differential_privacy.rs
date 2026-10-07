#![allow(clippy::wildcard_imports)]

use super::*;

impl ConfidentialRepository {
    pub(crate) async fn register_differential_privacy_budget(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialDifferentialPrivacyBudgetRequest,
    ) -> Result<StoreOutcome, ConfidentialDifferentialPrivacyBudgetError> {
        dispatch_repository!(self, |repository| repository
            .register_differential_privacy_budget(principal, request)
            .await)
    }

    pub(crate) async fn spend_differential_privacy_budget(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialDifferentialPrivacyBudgetSpendRequest,
    ) -> Result<
        ConfidentialDifferentialPrivacyBudgetState,
        ConfidentialDifferentialPrivacyBudgetError,
    > {
        dispatch_repository!(self, |repository| repository
            .spend_differential_privacy_budget(principal, request)
            .await)
    }

    pub(crate) async fn list_differential_privacy_budgets(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialDifferentialPrivacyBudgetFilter,
    ) -> Result<
        Vec<ConfidentialDifferentialPrivacyBudgetState>,
        ConfidentialDifferentialPrivacyBudgetError,
    > {
        dispatch_repository!(self, |repository| repository
            .list_differential_privacy_budgets(principal, filter)
            .await)
    }

    pub(crate) async fn list_differential_privacy_composition_summaries(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialDifferentialPrivacyBudgetFilter,
    ) -> Result<
        Vec<ConfidentialDifferentialPrivacyCompositionSummaryState>,
        ConfidentialDifferentialPrivacyBudgetError,
    > {
        dispatch_repository!(self, |repository| repository
            .list_differential_privacy_composition_summaries(principal, filter)
            .await)
    }

    pub(crate) async fn release_differential_privacy_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialDifferentialPrivacyReleaseRequest,
    ) -> Result<
        ConfidentialDifferentialPrivacyReleaseState,
        ConfidentialDifferentialPrivacyReleaseError,
    > {
        dispatch_repository!(self, |repository| repository
            .release_differential_privacy_aggregate(principal, request)
            .await)
    }

    pub(crate) async fn release_sampled_differential_privacy_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialDifferentialPrivacySampledReleaseRequest,
    ) -> Result<
        ConfidentialDifferentialPrivacyReleaseState,
        ConfidentialDifferentialPrivacyReleaseError,
    > {
        dispatch_repository!(self, |repository| repository
            .release_sampled_differential_privacy_aggregate(principal, request)
            .await)
    }

    pub(crate) async fn list_differential_privacy_releases(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialDifferentialPrivacyReleaseFilter,
    ) -> Result<
        Vec<ConfidentialDifferentialPrivacyReleaseState>,
        ConfidentialDifferentialPrivacyReleaseError,
    > {
        dispatch_repository!(self, |repository| repository
            .list_differential_privacy_releases(principal, filter)
            .await)
    }
}

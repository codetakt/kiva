//! Differential-privacy confidential application methods.

use kiva_application::confidential as confidential_app;
use kiva_identity::AuthenticatedPrincipal;

use crate::{
    app::confidential::{ConfidentialApplicationAdapter, ConfidentialWorkflowPort},
    core::outcome::StoreOutcome,
    surfaces::confidential::{
        model::{
            ConfidentialDifferentialPrivacyBudgetFilter,
            ConfidentialDifferentialPrivacyBudgetRequest,
            ConfidentialDifferentialPrivacyBudgetSpendRequest,
            ConfidentialDifferentialPrivacyBudgetState,
            ConfidentialDifferentialPrivacyCompositionSummaryState,
            ConfidentialDifferentialPrivacyReleaseFilter,
            ConfidentialDifferentialPrivacyReleaseRequest,
            ConfidentialDifferentialPrivacyReleaseState,
            ConfidentialDifferentialPrivacySampledReleaseRequest,
        },
        outcome::{
            ConfidentialDifferentialPrivacyBudgetError, ConfidentialDifferentialPrivacyReleaseError,
        },
    },
};

impl ConfidentialApplicationAdapter {
    pub(crate) async fn register_differential_privacy_budget(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialDifferentialPrivacyBudgetRequest,
    ) -> Result<StoreOutcome, ConfidentialDifferentialPrivacyBudgetError> {
        self.begin_workflow()
            .register_differential_privacy_budget(principal, request)
            .await
    }

    pub(crate) async fn spend_differential_privacy_budget(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialDifferentialPrivacyBudgetSpendRequest,
    ) -> Result<
        ConfidentialDifferentialPrivacyBudgetState,
        ConfidentialDifferentialPrivacyBudgetError,
    > {
        self.begin_workflow()
            .spend_differential_privacy_budget(principal, request)
            .await
    }

    pub(crate) async fn list_differential_privacy_budgets(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialDifferentialPrivacyBudgetFilter,
    ) -> Result<
        Vec<ConfidentialDifferentialPrivacyBudgetState>,
        ConfidentialDifferentialPrivacyBudgetError,
    > {
        self.begin_workflow()
            .list_differential_privacy_budgets(principal, filter)
            .await
    }

    pub(crate) async fn list_differential_privacy_composition_summaries(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialDifferentialPrivacyBudgetFilter,
    ) -> Result<
        Vec<ConfidentialDifferentialPrivacyCompositionSummaryState>,
        ConfidentialDifferentialPrivacyBudgetError,
    > {
        self.begin_workflow()
            .list_differential_privacy_composition_summaries(principal, filter)
            .await
    }

    pub(crate) async fn release_differential_privacy_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialDifferentialPrivacyReleaseRequest,
    ) -> Result<
        ConfidentialDifferentialPrivacyReleaseState,
        ConfidentialDifferentialPrivacyReleaseError,
    > {
        self.begin_workflow()
            .release_differential_privacy_aggregate(principal, request)
            .await
    }

    pub(crate) async fn release_sampled_differential_privacy_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        request: ConfidentialDifferentialPrivacySampledReleaseRequest,
    ) -> Result<
        ConfidentialDifferentialPrivacyReleaseState,
        ConfidentialDifferentialPrivacyReleaseError,
    > {
        self.begin_workflow()
            .release_sampled_differential_privacy_aggregate(principal, request)
            .await
    }

    pub(crate) async fn list_differential_privacy_releases(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: ConfidentialDifferentialPrivacyReleaseFilter,
    ) -> Result<
        Vec<ConfidentialDifferentialPrivacyReleaseState>,
        ConfidentialDifferentialPrivacyReleaseError,
    > {
        self.begin_workflow()
            .list_differential_privacy_releases(principal, filter)
            .await
    }
}

impl confidential_app::ConfidentialDifferentialPrivacyRepository for ConfidentialWorkflowPort {
    type BudgetRequest = ConfidentialDifferentialPrivacyBudgetRequest;
    type BudgetSpendRequest = ConfidentialDifferentialPrivacyBudgetSpendRequest;
    type BudgetFilter = ConfidentialDifferentialPrivacyBudgetFilter;
    type ReleaseFilter = ConfidentialDifferentialPrivacyReleaseFilter;
    type ReleaseRequest = ConfidentialDifferentialPrivacyReleaseRequest;
    type SampledReleaseRequest = ConfidentialDifferentialPrivacySampledReleaseRequest;
    type StoreOutcome = StoreOutcome;
    type BudgetState = ConfidentialDifferentialPrivacyBudgetState;
    type ReleaseState = ConfidentialDifferentialPrivacyReleaseState;
    type CompositionSummaryState = ConfidentialDifferentialPrivacyCompositionSummaryState;
    type BudgetError = ConfidentialDifferentialPrivacyBudgetError;
    type ReleaseError = ConfidentialDifferentialPrivacyReleaseError;

    fn register_differential_privacy_budget<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::BudgetRequest,
    ) -> confidential_app::ApplicationFuture<'a, Result<Self::StoreOutcome, Self::BudgetError>>
    {
        Box::pin(async move {
            self.repository
                .register_differential_privacy_budget(principal, request)
                .await
        })
    }

    fn spend_differential_privacy_budget<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::BudgetSpendRequest,
    ) -> confidential_app::ApplicationFuture<'a, Result<Self::BudgetState, Self::BudgetError>> {
        Box::pin(async move {
            self.repository
                .spend_differential_privacy_budget(principal, request)
                .await
        })
    }

    fn list_differential_privacy_budgets<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: Self::BudgetFilter,
    ) -> confidential_app::ApplicationFuture<'a, Result<Vec<Self::BudgetState>, Self::BudgetError>>
    {
        Box::pin(async move {
            self.repository
                .list_differential_privacy_budgets(principal, filter)
                .await
        })
    }

    fn list_differential_privacy_composition_summaries<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: Self::BudgetFilter,
    ) -> confidential_app::ApplicationFuture<
        'a,
        Result<Vec<Self::CompositionSummaryState>, Self::BudgetError>,
    > {
        Box::pin(async move {
            self.repository
                .list_differential_privacy_composition_summaries(principal, filter)
                .await
        })
    }

    fn release_differential_privacy_aggregate<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ReleaseRequest,
    ) -> confidential_app::ApplicationFuture<'a, Result<Self::ReleaseState, Self::ReleaseError>>
    {
        Box::pin(async move {
            self.repository
                .release_differential_privacy_aggregate(principal, request)
                .await
        })
    }

    fn release_sampled_differential_privacy_aggregate<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::SampledReleaseRequest,
    ) -> confidential_app::ApplicationFuture<'a, Result<Self::ReleaseState, Self::ReleaseError>>
    {
        Box::pin(async move {
            self.repository
                .release_sampled_differential_privacy_aggregate(principal, request)
                .await
        })
    }

    fn list_differential_privacy_releases<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: Self::ReleaseFilter,
    ) -> confidential_app::ApplicationFuture<'a, Result<Vec<Self::ReleaseState>, Self::ReleaseError>>
    {
        Box::pin(async move {
            self.repository
                .list_differential_privacy_releases(principal, filter)
                .await
        })
    }
}

//! Differential-privacy confidential-computation application workflow.

use kiva_identity::AuthenticatedPrincipal;

use super::{
    ApplicationFuture, ApplicationRuntimePort, ConfidentialApplicationError,
    ConfidentialApplicationWorkflow, ConfidentialWorkflowAdmissionRecorder,
    ConfidentialWorkflowKind, ConfidentialWorkflowRepository,
};

/// Port for differential-privacy budget and release workflows.
pub trait ConfidentialDifferentialPrivacyRepository: ConfidentialWorkflowRepository {
    /// Budget registration command.
    type BudgetRequest: Send + 'static;
    /// Budget spend command.
    type BudgetSpendRequest: Send + 'static;
    /// Budget query filter.
    type BudgetFilter: Send + 'static;
    /// Release query filter.
    type ReleaseFilter: Send + 'static;
    /// Release command.
    type ReleaseRequest: Send + 'static;
    /// Sampled release command.
    type SampledReleaseRequest: Send + 'static;
    /// Store outcome.
    type StoreOutcome: Send + 'static;
    /// Budget state view.
    type BudgetState: Send + 'static;
    /// Release state view.
    type ReleaseState: Send + 'static;
    /// Composition summary view.
    type CompositionSummaryState: Send + 'static;
    /// Budget workflow error.
    type BudgetError: From<ConfidentialApplicationError> + Send + 'static;
    /// Release workflow error.
    type ReleaseError: From<ConfidentialApplicationError> + Send + 'static;

    /// Registers a differential-privacy budget.
    fn register_differential_privacy_budget<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::BudgetRequest,
    ) -> ApplicationFuture<'a, Result<Self::StoreOutcome, Self::BudgetError>>;

    /// Spends budget.
    fn spend_differential_privacy_budget<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::BudgetSpendRequest,
    ) -> ApplicationFuture<'a, Result<Self::BudgetState, Self::BudgetError>>;

    /// Lists budgets.
    fn list_differential_privacy_budgets<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: Self::BudgetFilter,
    ) -> ApplicationFuture<'a, Result<Vec<Self::BudgetState>, Self::BudgetError>>;

    /// Lists composition summaries.
    fn list_differential_privacy_composition_summaries<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: Self::BudgetFilter,
    ) -> ApplicationFuture<'a, Result<Vec<Self::CompositionSummaryState>, Self::BudgetError>>;

    /// Releases a noised aggregate.
    fn release_differential_privacy_aggregate<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::ReleaseRequest,
    ) -> ApplicationFuture<'a, Result<Self::ReleaseState, Self::ReleaseError>>;

    /// Releases a sampled noised aggregate.
    fn release_sampled_differential_privacy_aggregate<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        request: Self::SampledReleaseRequest,
    ) -> ApplicationFuture<'a, Result<Self::ReleaseState, Self::ReleaseError>>;

    /// Lists releases.
    fn list_differential_privacy_releases<'a>(
        &'a self,
        principal: &'a AuthenticatedPrincipal,
        filter: Self::ReleaseFilter,
    ) -> ApplicationFuture<'a, Result<Vec<Self::ReleaseState>, Self::ReleaseError>>;
}

impl<R, D, A> ConfidentialApplicationWorkflow<R, D, A>
where
    R: ConfidentialDifferentialPrivacyRepository,
    D: ConfidentialWorkflowAdmissionRecorder,
    A: ApplicationRuntimePort,
{
    /// Registers a differential-privacy budget after application-level workflow admission.
    pub async fn register_differential_privacy_budget(
        &self,
        principal: &AuthenticatedPrincipal,
        request: R::BudgetRequest,
    ) -> Result<R::StoreOutcome, R::BudgetError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::RegisterDifferentialPrivacyBudget,
            |repository, _admission| async move {
                repository
                    .register_differential_privacy_budget(principal, request)
                    .await
            },
        )
        .await
    }

    /// Spends budget after application-level workflow admission.
    pub async fn spend_differential_privacy_budget(
        &self,
        principal: &AuthenticatedPrincipal,
        request: R::BudgetSpendRequest,
    ) -> Result<R::BudgetState, R::BudgetError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::SpendDifferentialPrivacyBudget,
            |repository, _admission| async move {
                repository
                    .spend_differential_privacy_budget(principal, request)
                    .await
            },
        )
        .await
    }

    /// Lists budgets after application-level workflow admission.
    pub async fn list_differential_privacy_budgets(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: R::BudgetFilter,
    ) -> Result<Vec<R::BudgetState>, R::BudgetError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ListDifferentialPrivacyBudgets,
            |repository, _admission| async move {
                repository
                    .list_differential_privacy_budgets(principal, filter)
                    .await
            },
        )
        .await
    }

    /// Lists composition summaries after application-level workflow admission.
    pub async fn list_differential_privacy_composition_summaries(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: R::BudgetFilter,
    ) -> Result<Vec<R::CompositionSummaryState>, R::BudgetError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ListDifferentialPrivacyCompositionSummaries,
            |repository, _admission| async move {
                repository
                    .list_differential_privacy_composition_summaries(principal, filter)
                    .await
            },
        )
        .await
    }

    /// Releases a noised aggregate after application-level workflow admission.
    pub async fn release_differential_privacy_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        request: R::ReleaseRequest,
    ) -> Result<R::ReleaseState, R::ReleaseError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ReleaseDifferentialPrivacyAggregate,
            |repository, _admission| async move {
                repository
                    .release_differential_privacy_aggregate(principal, request)
                    .await
            },
        )
        .await
    }

    /// Releases a sampled noised aggregate after application-level workflow admission.
    pub async fn release_sampled_differential_privacy_aggregate(
        &self,
        principal: &AuthenticatedPrincipal,
        request: R::SampledReleaseRequest,
    ) -> Result<R::ReleaseState, R::ReleaseError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ReleaseSampledDifferentialPrivacyAggregate,
            |repository, _admission| async move {
                repository
                    .release_sampled_differential_privacy_aggregate(principal, request)
                    .await
            },
        )
        .await
    }

    /// Lists releases after application-level workflow admission.
    pub async fn list_differential_privacy_releases(
        &self,
        principal: &AuthenticatedPrincipal,
        filter: R::ReleaseFilter,
    ) -> Result<Vec<R::ReleaseState>, R::ReleaseError> {
        self.run_confidential_workflow(
            principal,
            ConfidentialWorkflowKind::ListDifferentialPrivacyReleases,
            |repository, _admission| async move {
                repository
                    .list_differential_privacy_releases(principal, filter)
                    .await
            },
        )
        .await
    }
}

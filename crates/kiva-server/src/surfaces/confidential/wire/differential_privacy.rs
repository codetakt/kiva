//! Differential privacy response projection.

use serde_json::Value;

use kiva_confidential::mechanism::differential_privacy::{
    DifferentialPrivacyAccountingScope, DifferentialPrivacyAdjacencyModel,
    DifferentialPrivacyNoiseMechanism,
};
use kiva_confidential_contract::{
    DifferentialPrivacyBudgetMetadata, DifferentialPrivacyBudgetMetadataList,
    DifferentialPrivacyCompositionSummary, DifferentialPrivacyCompositionSummaryList,
    DifferentialPrivacyReleaseMetadata, DifferentialPrivacyReleaseMetadataList,
};

use crate::{
    core::response::{hex_lower, rfc3339},
    surfaces::confidential::model::{
        ConfidentialDifferentialPrivacyBudgetState,
        ConfidentialDifferentialPrivacyCompositionSummaryState,
        ConfidentialDifferentialPrivacyReleaseState,
    },
};

use super::dto_value;

pub(crate) fn differential_privacy_budget_response(
    budget: &ConfidentialDifferentialPrivacyBudgetState,
) -> Result<Value, serde_json::Error> {
    dto_value(&differential_privacy_budget_metadata(budget))
}

pub(crate) fn differential_privacy_release_response(
    release: &ConfidentialDifferentialPrivacyReleaseState,
) -> Result<Value, serde_json::Error> {
    dto_value(&differential_privacy_release_metadata(release))
}

pub(crate) fn differential_privacy_budget_list_response(
    budgets: &[ConfidentialDifferentialPrivacyBudgetState],
) -> Result<Value, serde_json::Error> {
    dto_value(&DifferentialPrivacyBudgetMetadataList {
        budgets: budgets
            .iter()
            .map(differential_privacy_budget_metadata)
            .collect(),
    })
}

pub(crate) fn differential_privacy_composition_summary_list_response(
    summaries: &[ConfidentialDifferentialPrivacyCompositionSummaryState],
) -> Result<Value, serde_json::Error> {
    dto_value(&DifferentialPrivacyCompositionSummaryList {
        summaries: summaries
            .iter()
            .map(differential_privacy_composition_summary)
            .collect(),
    })
}

pub(crate) fn differential_privacy_release_list_response(
    releases: &[ConfidentialDifferentialPrivacyReleaseState],
) -> Result<Value, serde_json::Error> {
    dto_value(&DifferentialPrivacyReleaseMetadataList {
        releases: releases
            .iter()
            .map(differential_privacy_release_metadata)
            .collect(),
    })
}

fn differential_privacy_budget_metadata(
    budget: &ConfidentialDifferentialPrivacyBudgetState,
) -> DifferentialPrivacyBudgetMetadata {
    DifferentialPrivacyBudgetMetadata {
        budget_id: hex_lower(&budget.budget_id.bytes()),
        owner_subject: budget.owner_subject.clone(),
        total_epsilon_microunits: budget.ledger.total_epsilon_microunits().get(),
        spent_epsilon_microunits: budget.ledger.spent_epsilon_microunits(),
        remaining_epsilon_microunits: budget.ledger.remaining_epsilon_microunits(),
        created_at: rfc3339(budget.created_at),
        updated_at: rfc3339(budget.updated_at),
    }
}

fn differential_privacy_composition_summary(
    state: &ConfidentialDifferentialPrivacyCompositionSummaryState,
) -> DifferentialPrivacyCompositionSummary {
    let ledger = state.summary.ledger();
    DifferentialPrivacyCompositionSummary {
        budget_id: hex_lower(&state.budget_id.bytes()),
        owner_subject: state.owner_subject.clone(),
        accounting_scope: "subjectBudget".to_owned(),
        composition_rule: "basicSequential".to_owned(),
        advanced_composition_rule: "drvPureDp".to_owned(),
        advanced_composition_delta_microunits: state.advanced.delta_microunits().get(),
        advanced_composition_epsilon_microunits: state.advanced.epsilon_upper_bound_microunits(),
        total_epsilon_microunits: ledger.total_epsilon_microunits().get(),
        budget_spent_epsilon_microunits: ledger.spent_epsilon_microunits(),
        release_spent_epsilon_microunits: state.summary.release_spent_epsilon_microunits(),
        release_epsilon_square_sum_microunits2: state
            .summary
            .release_epsilon_square_sum_microunits2()
            .to_string(),
        unattributed_spent_epsilon_microunits: state
            .summary
            .unattributed_spent_epsilon_microunits(),
        remaining_epsilon_microunits: ledger.remaining_epsilon_microunits(),
        release_count: state.summary.release_count(),
        max_release_epsilon_microunits: state.summary.max_release_epsilon_microunits(),
    }
}

fn differential_privacy_release_metadata(
    release: &ConfidentialDifferentialPrivacyReleaseState,
) -> DifferentialPrivacyReleaseMetadata {
    let parameters = release.noised_aggregate.parameters();
    DifferentialPrivacyReleaseMetadata {
        release_id: hex_lower(&release.release_id.bytes()),
        budget_id: hex_lower(&release.budget_id.bytes()),
        transcript_id: hex_lower(&release.transcript_id.bytes()),
        owner_subject: release.owner_subject.clone(),
        noised_value: release.noised_aggregate.noised_value(),
        mechanism: differential_privacy_noise_mechanism_label(parameters.mechanism()).to_owned(),
        accounting_scope: differential_privacy_accounting_scope_label(
            parameters.accounting_scope(),
        )
        .to_owned(),
        adjacency_model: differential_privacy_adjacency_model_label(parameters.adjacency_model())
            .to_owned(),
        epsilon_microunits: parameters.epsilon_microunits().get(),
        l1_sensitivity: parameters.l1_sensitivity().get(),
        data_subject_count: release.data_subject_count.get(),
        minimum_population: release.minimum_population.get(),
        created_at: rfc3339(release.created_at),
    }
}

const fn differential_privacy_noise_mechanism_label(
    mechanism: DifferentialPrivacyNoiseMechanism,
) -> &'static str {
    match mechanism {
        DifferentialPrivacyNoiseMechanism::DiscreteLaplace => "discreteLaplace",
    }
}

const fn differential_privacy_accounting_scope_label(
    accounting_scope: DifferentialPrivacyAccountingScope,
) -> &'static str {
    match accounting_scope {
        DifferentialPrivacyAccountingScope::SubjectBudget => "subjectBudget",
    }
}

const fn differential_privacy_adjacency_model_label(
    adjacency_model: DifferentialPrivacyAdjacencyModel,
) -> &'static str {
    match adjacency_model {
        DifferentialPrivacyAdjacencyModel::AddRemoveOneSubject => "addRemoveOneSubject",
    }
}

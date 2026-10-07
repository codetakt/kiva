//! Differential-privacy `OpenAPI` request DTO conversion.

use kiva_confidential_contract as contract;

use crate::{core::response::HttpError, surfaces::confidential::model as domain};

use super::common::{field_element, transcript_id};

pub(crate) fn differential_privacy_budget_request_from_openapi(
    request: contract::DifferentialPrivacyBudget,
) -> Result<domain::ConfidentialDifferentialPrivacyBudgetRequest, HttpError> {
    Ok(domain::ConfidentialDifferentialPrivacyBudgetRequest {
        budget_id: transcript_id(request.budget_id, "budget_id")?,
        total_epsilon_microunits: request.total_epsilon_microunits,
    })
}

pub(crate) fn differential_privacy_budget_spend_request_from_openapi(
    request: contract::DifferentialPrivacyBudgetSpend,
) -> Result<domain::ConfidentialDifferentialPrivacyBudgetSpendRequest, HttpError> {
    Ok(domain::ConfidentialDifferentialPrivacyBudgetSpendRequest {
        budget_id: transcript_id(request.budget_id, "budget_id")?,
        epsilon_microunits: request.epsilon_microunits,
    })
}

pub(crate) fn differential_privacy_release_request_from_openapi(
    request: contract::DifferentialPrivacyRelease,
) -> Result<domain::ConfidentialDifferentialPrivacyReleaseRequest, HttpError> {
    Ok(domain::ConfidentialDifferentialPrivacyReleaseRequest {
        release_id: transcript_id(request.release_id, "release_id")?,
        budget_id: transcript_id(request.budget_id, "budget_id")?,
        transcript_id: transcript_id(request.transcript_id, "transcript_id")?,
        aggregate: field_element(request.aggregate, "aggregate")?,
        data_subject_count: request.data_subject_count,
        minimum_population: request.minimum_population,
        epsilon_microunits: request.epsilon_microunits,
        l1_sensitivity: request.l1_sensitivity,
        noise_value: request.noise_value,
    })
}

pub(crate) fn differential_privacy_sampled_release_request_from_openapi(
    request: contract::DifferentialPrivacySampledRelease,
) -> Result<domain::ConfidentialDifferentialPrivacySampledReleaseRequest, HttpError> {
    Ok(
        domain::ConfidentialDifferentialPrivacySampledReleaseRequest {
            release_id: transcript_id(request.release_id, "release_id")?,
            budget_id: transcript_id(request.budget_id, "budget_id")?,
            transcript_id: transcript_id(request.transcript_id, "transcript_id")?,
            data_subject_count: request.data_subject_count,
            minimum_population: request.minimum_population,
            epsilon_microunits: request.epsilon_microunits,
            l1_sensitivity: request.l1_sensitivity,
        },
    )
}

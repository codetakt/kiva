//! Differential privacy error response projection.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use kiva_confidential::mechanism::differential_privacy::{
    DifferentialPrivacyBudgetError, DifferentialPrivacyNoiseError, DifferentialPrivacyPolicyError,
    DifferentialPrivacyReleaseError,
};

use crate::{
    core::response::HttpError,
    surfaces::confidential::outcome::{
        ConfidentialDifferentialPrivacyBudgetError, ConfidentialDifferentialPrivacyReleaseError,
    },
};

use super::{
    confidential_application_admission_error_response, confidential_secure_error_response,
    confidential_store_error_response,
};

pub(crate) fn confidential_differential_privacy_budget_error_response(
    error: ConfidentialDifferentialPrivacyBudgetError,
) -> Response {
    match error {
        ConfidentialDifferentialPrivacyBudgetError::ApplicationAdmission(message) => {
            confidential_application_admission_error_response(&message)
        }
        ConfidentialDifferentialPrivacyBudgetError::Budget(
            DifferentialPrivacyBudgetError::SpendRequestMustBeNonZero,
        ) => HttpError::new(
            StatusCode::BAD_REQUEST,
            "confidential differential privacy budget spend must be non-zero",
        )
        .into_response(),
        ConfidentialDifferentialPrivacyBudgetError::Budget(
            DifferentialPrivacyBudgetError::SpendRequestExceedsRemaining,
        ) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential differential privacy budget spend exceeds remaining budget",
        )
        .into_response(),
        ConfidentialDifferentialPrivacyBudgetError::Budget(
            DifferentialPrivacyBudgetError::SpentBudgetExceedsTotal,
        ) => HttpError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "confidential differential privacy budget state is invalid",
        )
        .into_response(),
        ConfidentialDifferentialPrivacyBudgetError::Store(error) => {
            confidential_store_error_response(&error)
        }
        ConfidentialDifferentialPrivacyBudgetError::Composition(error) => {
            tracing::error!(
                error = ?error,
                "confidential differential privacy composition summary is inconsistent"
            );
            HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "confidential differential privacy composition summary is inconsistent",
            )
            .into_response()
        }
        ConfidentialDifferentialPrivacyBudgetError::NotFound => HttpError::new(
            StatusCode::NOT_FOUND,
            "confidential differential privacy budget was not found",
        )
        .into_response(),
        ConfidentialDifferentialPrivacyBudgetError::Forbidden => HttpError::new(
            StatusCode::FORBIDDEN,
            "confidential differential privacy budget belongs to a different subject",
        )
        .into_response(),
    }
}

pub(crate) fn confidential_differential_privacy_release_error_response(
    error: ConfidentialDifferentialPrivacyReleaseError,
) -> Response {
    match error {
        ConfidentialDifferentialPrivacyReleaseError::ApplicationAdmission(message) => {
            confidential_application_admission_error_response(&message)
        }
        ConfidentialDifferentialPrivacyReleaseError::Budget(error) => {
            confidential_differential_privacy_budget_error_response(
                ConfidentialDifferentialPrivacyBudgetError::Budget(error),
            )
        }
        ConfidentialDifferentialPrivacyReleaseError::Noise(error) => {
            confidential_differential_privacy_noise_error_response(error)
        }
        ConfidentialDifferentialPrivacyReleaseError::Policy(
            DifferentialPrivacyPolicyError::MinimumPopulationTooSmall,
        ) => HttpError::new(
            StatusCode::BAD_REQUEST,
            "confidential differential privacy minimum population must be at least two",
        )
        .into_response(),
        ConfidentialDifferentialPrivacyReleaseError::Release(
            DifferentialPrivacyReleaseError::MissingComputedAggregate,
        )
        | ConfidentialDifferentialPrivacyReleaseError::MissingComputedAggregate => {
            HttpError::new(
                StatusCode::CONFLICT,
                "confidential differential privacy release requires a matching computed aggregate result",
            )
            .into_response()
        }
        ConfidentialDifferentialPrivacyReleaseError::Release(
            DifferentialPrivacyReleaseError::PopulationBelowMinimum,
        ) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential differential privacy release population is below the minimum",
        )
        .into_response(),
        ConfidentialDifferentialPrivacyReleaseError::Release(
            DifferentialPrivacyReleaseError::EpsilonBudgetMustBeNonZero,
        ) => HttpError::new(
            StatusCode::BAD_REQUEST,
            "confidential differential privacy release epsilon must be non-zero",
        )
        .into_response(),
        ConfidentialDifferentialPrivacyReleaseError::Release(
            DifferentialPrivacyReleaseError::EpsilonBudgetExceeded,
        ) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential differential privacy release epsilon exceeds the release policy",
        )
        .into_response(),
        ConfidentialDifferentialPrivacyReleaseError::Release(
            DifferentialPrivacyReleaseError::NoiseNotApplied,
        ) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential differential privacy release requires applied noise evidence",
        )
        .into_response(),
        ConfidentialDifferentialPrivacyReleaseError::Secure(error) => {
            confidential_secure_error_response(error).into_response()
        }
        ConfidentialDifferentialPrivacyReleaseError::Store(error) => {
            confidential_store_error_response(&error)
        }
        ConfidentialDifferentialPrivacyReleaseError::NotFound => HttpError::new(
            StatusCode::NOT_FOUND,
            "confidential differential privacy budget was not found",
        )
        .into_response(),
        ConfidentialDifferentialPrivacyReleaseError::Forbidden => HttpError::new(
            StatusCode::FORBIDDEN,
            "confidential differential privacy release belongs to a different subject",
        )
        .into_response(),
        ConfidentialDifferentialPrivacyReleaseError::DataSubjectCountMismatch => {
            HttpError::new(
                StatusCode::CONFLICT,
                "confidential differential privacy data subject count does not match the computed aggregate",
            )
            .into_response()
        }
        ConfidentialDifferentialPrivacyReleaseError::RandomnessUnavailable => HttpError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "confidential differential privacy randomness source was unavailable",
        )
        .into_response(),
        ConfidentialDifferentialPrivacyReleaseError::Conflict => HttpError::new(
            StatusCode::CONFLICT,
            "confidential differential privacy release already exists with different data",
        )
        .into_response(),
    }
}

pub(crate) fn confidential_differential_privacy_noise_error_response(
    error: DifferentialPrivacyNoiseError,
) -> Response {
    match error {
        DifferentialPrivacyNoiseError::NoisedValueOverflow => HttpError::new(
            StatusCode::BAD_REQUEST,
            "confidential differential privacy noised value overflowed signed integer bounds",
        )
        .into_response(),
        DifferentialPrivacyNoiseError::NoiseParameterOutOfRange => HttpError::new(
            StatusCode::BAD_REQUEST,
            "confidential differential privacy noise parameters are outside the supported sampler range",
        )
        .into_response(),
        DifferentialPrivacyNoiseError::NoiseSampleOutOfRange => HttpError::new(
            StatusCode::CONFLICT,
            "confidential differential privacy sampled noise exceeded signed integer bounds",
        )
        .into_response(),
    }
}

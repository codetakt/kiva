#![allow(clippy::wildcard_imports)]

use super::*;

pub(crate) async fn post_confidential_differential_privacy_budget(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_write_endpoint(
        &state,
        CONFIDENTIAL_DIFFERENTIAL_PRIVACY_BUDGETS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request = match parse_confidential_openapi_request::<
        confidential_contract::DifferentialPrivacyBudget,
        _,
    >(
        &body,
        "confidential differential privacy budget body",
        wire::differential_privacy_budget_request_from_openapi,
    ) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .register_differential_privacy_budget(principal.as_principal(), request)
        .await
    {
        Ok(StoreOutcome::Stored | StoreOutcome::Unchanged) => {
            empty_response(StatusCode::NO_CONTENT)
        }
        Ok(StoreOutcome::Conflict) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential differential privacy budget already exists with different data",
        )
        .into_response(),
        Err(error) => confidential_differential_privacy_budget_error_response(error),
    }
}

pub(crate) async fn post_confidential_differential_privacy_budget_spend(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_write_endpoint(
        &state,
        CONFIDENTIAL_DIFFERENTIAL_PRIVACY_BUDGET_SPENDS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request = match parse_confidential_openapi_request::<
        confidential_contract::DifferentialPrivacyBudgetSpend,
        _,
    >(
        &body,
        "confidential differential privacy budget spend body",
        wire::differential_privacy_budget_spend_request_from_openapi,
    ) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .spend_differential_privacy_budget(principal.as_principal(), request)
        .await
    {
        Ok(budget) => json_projection_response(
            StatusCode::OK,
            wire::differential_privacy_budget_response(&budget),
        ),
        Err(error) => confidential_differential_privacy_budget_error_response(error),
    }
}

pub(crate) async fn post_confidential_differential_privacy_release(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_release_endpoint(
        &state,
        CONFIDENTIAL_DIFFERENTIAL_PRIVACY_RELEASES_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request = match parse_confidential_openapi_request::<
        confidential_contract::DifferentialPrivacyRelease,
        _,
    >(
        &body,
        "confidential differential privacy release body",
        wire::differential_privacy_release_request_from_openapi,
    ) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .release_differential_privacy_aggregate(principal.as_principal(), request)
        .await
    {
        Ok(release) => json_projection_response(
            StatusCode::OK,
            wire::differential_privacy_release_response(&release),
        ),
        Err(error) => confidential_differential_privacy_release_error_response(error),
    }
}

pub(crate) async fn post_confidential_differential_privacy_sampled_release(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_release_endpoint(
        &state,
        CONFIDENTIAL_DIFFERENTIAL_PRIVACY_SAMPLED_RELEASES_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request = match parse_confidential_openapi_request::<
        confidential_contract::DifferentialPrivacySampledRelease,
        _,
    >(
        &body,
        "confidential differential privacy sampled release body",
        wire::differential_privacy_sampled_release_request_from_openapi,
    ) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .release_sampled_differential_privacy_aggregate(principal.as_principal(), request)
        .await
    {
        Ok(release) => json_projection_response(
            StatusCode::OK,
            wire::differential_privacy_release_response(&release),
        ),
        Err(error) => confidential_differential_privacy_release_error_response(error),
    }
}

pub(crate) async fn get_confidential_differential_privacy_budgets(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    let principal = match authorize_confidential_read_endpoint(
        &state,
        CONFIDENTIAL_DIFFERENTIAL_PRIVACY_BUDGETS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let filter = match ConfidentialDifferentialPrivacyBudgetFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    match state
        .confidential_application()
        .list_differential_privacy_budgets(principal.as_principal(), filter)
        .await
    {
        Ok(budgets) => json_projection_response(
            StatusCode::OK,
            wire::differential_privacy_budget_list_response(&budgets),
        ),
        Err(error) => confidential_differential_privacy_budget_error_response(error),
    }
}

pub(crate) async fn get_confidential_differential_privacy_composition_summaries(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    let principal = match authorize_confidential_read_endpoint(
        &state,
        CONFIDENTIAL_DIFFERENTIAL_PRIVACY_COMPOSITION_SUMMARIES_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let filter = match ConfidentialDifferentialPrivacyBudgetFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    match state
        .confidential_application()
        .list_differential_privacy_composition_summaries(principal.as_principal(), filter)
        .await
    {
        Ok(summaries) => json_projection_response(
            StatusCode::OK,
            wire::differential_privacy_composition_summary_list_response(&summaries),
        ),
        Err(error) => confidential_differential_privacy_budget_error_response(error),
    }
}

pub(crate) async fn get_confidential_differential_privacy_releases(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    let principal = match authorize_confidential_read_endpoint(
        &state,
        CONFIDENTIAL_DIFFERENTIAL_PRIVACY_RELEASES_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let filter = match ConfidentialDifferentialPrivacyReleaseFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    match state
        .confidential_application()
        .list_differential_privacy_releases(principal.as_principal(), filter)
        .await
    {
        Ok(releases) => json_projection_response(
            StatusCode::OK,
            wire::differential_privacy_release_list_response(&releases),
        ),
        Err(error) => confidential_differential_privacy_release_error_response(error),
    }
}

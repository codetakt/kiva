#![allow(clippy::wildcard_imports)]

use super::*;

pub(crate) async fn post_confidential_additive_aggregate(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_compute_endpoint(
        &state,
        CONFIDENTIAL_ADDITIVE_AGGREGATES_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request = match parse_confidential_openapi_request::<
        confidential_contract::AdditiveAggregateRequest,
        _,
    >(
        &body,
        "confidential aggregate request body",
        wire::additive_aggregate_request_from_openapi,
    ) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .release_signed_additive_aggregate(principal.as_principal(), &request)
        .await
    {
        Ok(result) => json_projection_response(StatusCode::OK, wire::aggregate_response(&result)),
        Err(ConfidentialAggregateError::ApplicationAdmission(message)) => {
            confidential_application_admission_error_response(&message)
        }
        Err(ConfidentialAggregateError::Secure(error)) => {
            confidential_secure_error_response(error).into_response()
        }
        Err(ConfidentialAggregateError::Store(error)) => confidential_store_error_response(&error),
        Err(ConfidentialAggregateError::InactiveParticipant) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential participant is not active for this operation",
        )
        .into_response(),
    }
}

pub(crate) async fn post_confidential_additive_share_submission(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_write_endpoint(
        &state,
        CONFIDENTIAL_ADDITIVE_SHARE_SUBMISSIONS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request =
        match parse_confidential_openapi_request::<confidential_contract::ShareSubmission, _>(
            &body,
            "confidential additive share submission body",
            wire::share_submission_request_from_openapi,
        ) {
            Ok(request) => request,
            Err(response) => return response,
        };

    match state
        .confidential_application()
        .submit_additive_share(principal.as_principal(), request)
        .await
    {
        Ok(StoreOutcome::Stored | StoreOutcome::Unchanged) => {
            empty_response(StatusCode::NO_CONTENT)
        }
        Ok(StoreOutcome::Conflict) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential additive share submission already exists with different data",
        )
        .into_response(),
        Err(ConfidentialShareSubmissionError::Forbidden) => HttpError::new(
            StatusCode::FORBIDDEN,
            "confidential additive share signing key belongs to a different subject",
        )
        .into_response(),
        Err(ConfidentialShareSubmissionError::InactiveParticipant) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential participant is not active for share submission",
        )
        .into_response(),
        Err(ConfidentialShareSubmissionError::ApplicationAdmission(message)) => {
            confidential_application_admission_error_response(&message)
        }
        Err(ConfidentialShareSubmissionError::Secure(error)) => {
            confidential_secure_error_response(error).into_response()
        }
        Err(ConfidentialShareSubmissionError::Store(error)) => {
            confidential_store_error_response(&error)
        }
    }
}

pub(crate) async fn get_confidential_additive_share_submissions(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    let principal = match authorize_confidential_read_endpoint(
        &state,
        CONFIDENTIAL_ADDITIVE_SHARE_SUBMISSIONS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let transcript_id = match required_transcript_id_query(&query) {
        Ok(transcript_id) => transcript_id,
        Err(error) => return error.into_response(),
    };

    match state
        .confidential_application()
        .list_submitted_shares(principal.as_principal(), transcript_id)
        .await
    {
        Ok(submissions) => json_projection_response(
            StatusCode::OK,
            wire::submitted_share_list_response(&submissions),
        ),
        Err(ConfidentialAggregateError::ApplicationAdmission(message)) => {
            confidential_application_admission_error_response(&message)
        }
        Err(ConfidentialAggregateError::Secure(error)) => {
            confidential_secure_error_response(error).into_response()
        }
        Err(ConfidentialAggregateError::Store(error)) => confidential_store_error_response(&error),
        Err(ConfidentialAggregateError::InactiveParticipant) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential participant is not active for this operation",
        )
        .into_response(),
    }
}

pub(crate) async fn get_confidential_additive_share_replay_records(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    let principal = match authorize_confidential_read_endpoint(
        &state,
        CONFIDENTIAL_ADDITIVE_SHARE_REPLAY_RECORDS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let transcript_id = match required_transcript_id_query(&query) {
        Ok(transcript_id) => transcript_id,
        Err(error) => return error.into_response(),
    };

    match state
        .confidential_application()
        .list_replay_records(principal.as_principal(), transcript_id)
        .await
    {
        Ok(records) => {
            json_projection_response(StatusCode::OK, wire::replay_record_list_response(&records))
        }
        Err(ConfidentialAggregateError::ApplicationAdmission(message)) => {
            confidential_application_admission_error_response(&message)
        }
        Err(ConfidentialAggregateError::Secure(error)) => {
            confidential_secure_error_response(error).into_response()
        }
        Err(ConfidentialAggregateError::Store(error)) => confidential_store_error_response(&error),
        Err(ConfidentialAggregateError::InactiveParticipant) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential participant is not active for this operation",
        )
        .into_response(),
    }
}

pub(crate) async fn get_confidential_share_signing_keys(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    let principal = match authorize_confidential_read_endpoint(
        &state,
        CONFIDENTIAL_SHARE_SIGNING_KEYS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .list_share_signing_keys(principal.as_principal())
        .await
    {
        Ok(signing_keys) => json_projection_response(
            StatusCode::OK,
            wire::share_signing_key_list_response(&signing_keys),
        ),
        Err(ConfidentialAggregateError::ApplicationAdmission(message)) => {
            confidential_application_admission_error_response(&message)
        }
        Err(ConfidentialAggregateError::Secure(error)) => {
            confidential_secure_error_response(error).into_response()
        }
        Err(ConfidentialAggregateError::Store(error)) => confidential_store_error_response(&error),
        Err(ConfidentialAggregateError::InactiveParticipant) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential participant is not active for this operation",
        )
        .into_response(),
    }
}

pub(crate) async fn post_confidential_share_signing_key(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_write_endpoint(
        &state,
        CONFIDENTIAL_SHARE_SIGNING_KEYS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request = match parse_confidential_openapi_request::<
        confidential_contract::ShareSigningKeyRegistration,
        _,
    >(
        &body,
        "confidential share signing key request body",
        wire::share_signing_key_request_from_openapi,
    ) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .register_share_signing_key(principal.as_principal(), request)
        .await
    {
        Ok(StoreOutcome::Stored | StoreOutcome::Unchanged) => {
            empty_response(StatusCode::NO_CONTENT)
        }
        Ok(StoreOutcome::Conflict) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential share signing key already exists with different registration data",
        )
        .into_response(),
        Err(ConfidentialAggregateError::ApplicationAdmission(message)) => {
            confidential_application_admission_error_response(&message)
        }
        Err(ConfidentialAggregateError::Secure(error)) => {
            confidential_secure_error_response(error).into_response()
        }
        Err(ConfidentialAggregateError::Store(error)) => confidential_store_error_response(&error),
        Err(ConfidentialAggregateError::InactiveParticipant) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential participant is not active for this operation",
        )
        .into_response(),
    }
}

pub(crate) async fn post_confidential_share_signing_key_rotation(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_write_endpoint(
        &state,
        CONFIDENTIAL_SHARE_SIGNING_KEY_ROTATIONS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request = match parse_confidential_openapi_request::<
        confidential_contract::ShareSigningKeyRotation,
        _,
    >(
        &body,
        "confidential share signing key rotation body",
        wire::share_signing_key_rotation_request_from_openapi,
    ) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .rotate_share_signing_key(principal.as_principal(), request)
        .await
    {
        Ok(ConfidentialShareSigningKeyRotationOutcome::Rotated) => {
            empty_response(StatusCode::NO_CONTENT)
        }
        Ok(ConfidentialShareSigningKeyRotationOutcome::NotFound) => HttpError::new(
            StatusCode::NOT_FOUND,
            "confidential share signing key was not found",
        )
        .into_response(),
        Ok(ConfidentialShareSigningKeyRotationOutcome::Forbidden) => HttpError::new(
            StatusCode::FORBIDDEN,
            "confidential share signing key belongs to a different subject",
        )
        .into_response(),
        Ok(
            ConfidentialShareSigningKeyRotationOutcome::OldKeyRevoked
            | ConfidentialShareSigningKeyRotationOutcome::NewKeyConflict,
        ) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential share signing key rotation conflicts with current key state",
        )
        .into_response(),
        Err(ConfidentialAggregateError::ApplicationAdmission(message)) => {
            confidential_application_admission_error_response(&message)
        }
        Err(ConfidentialAggregateError::Secure(error)) => {
            confidential_secure_error_response(error).into_response()
        }
        Err(ConfidentialAggregateError::Store(error)) => confidential_store_error_response(&error),
        Err(ConfidentialAggregateError::InactiveParticipant) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential participant is not active for this operation",
        )
        .into_response(),
    }
}

pub(crate) async fn post_confidential_share_signing_key_revocation(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_write_endpoint(
        &state,
        CONFIDENTIAL_SHARE_SIGNING_KEY_REVOCATIONS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request = match parse_confidential_openapi_request::<
        confidential_contract::ShareSigningKeyRevocation,
        _,
    >(
        &body,
        "confidential share signing key revocation body",
        wire::share_signing_key_revocation_request_from_openapi,
    ) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .revoke_share_signing_key(principal.as_principal(), request)
        .await
    {
        Ok(
            ConfidentialShareSigningKeyRevocationOutcome::Revoked
            | ConfidentialShareSigningKeyRevocationOutcome::AlreadyRevoked,
        ) => empty_response(StatusCode::NO_CONTENT),
        Ok(ConfidentialShareSigningKeyRevocationOutcome::NotFound) => HttpError::new(
            StatusCode::NOT_FOUND,
            "confidential share signing key was not found",
        )
        .into_response(),
        Ok(ConfidentialShareSigningKeyRevocationOutcome::Forbidden) => HttpError::new(
            StatusCode::FORBIDDEN,
            "confidential share signing key belongs to a different subject",
        )
        .into_response(),
        Err(ConfidentialAggregateError::ApplicationAdmission(message)) => {
            confidential_application_admission_error_response(&message)
        }
        Err(ConfidentialAggregateError::Secure(error)) => {
            confidential_secure_error_response(error).into_response()
        }
        Err(ConfidentialAggregateError::Store(error)) => confidential_store_error_response(&error),
        Err(ConfidentialAggregateError::InactiveParticipant) => HttpError::new(
            StatusCode::CONFLICT,
            "confidential participant is not active for this operation",
        )
        .into_response(),
    }
}

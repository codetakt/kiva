#![allow(clippy::wildcard_imports)]

use super::*;

pub(crate) async fn post_confidential_participant(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_write_endpoint(
        &state,
        CONFIDENTIAL_PARTICIPANTS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request = match parse_confidential_openapi_request::<
        confidential_contract::ConfidentialParticipantRequest,
        _,
    >(&body, "confidential participant body", |request| {
        Ok(wire::confidential_participant_request_from_openapi(request))
    }) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .register_confidential_participant(principal.as_principal(), request)
        .await
    {
        Ok(
            ConfidentialParticipantRegistrationOutcome::Stored(participant)
            | ConfidentialParticipantRegistrationOutcome::Unchanged(participant),
        ) => json_projection_response(StatusCode::OK, wire::participant_response(&participant)),
        Ok(ConfidentialParticipantRegistrationOutcome::Conflict) => {
            confidential_participant_error_response(ConfidentialParticipantError::Conflict)
        }
        Err(error) => confidential_participant_error_response(error),
    }
}

pub(crate) async fn get_confidential_participants(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    let principal = match authorize_confidential_read_endpoint(
        &state,
        CONFIDENTIAL_PARTICIPANTS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let filter = match ConfidentialParticipantFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    match state
        .confidential_application()
        .list_confidential_participants(principal.as_principal(), filter)
        .await
    {
        Ok(participants) => json_projection_response(
            StatusCode::OK,
            wire::participant_list_response(&participants),
        ),
        Err(error) => confidential_participant_error_response(error),
    }
}

pub(crate) async fn post_confidential_participant_transition(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    let principal = match authorize_confidential_write_endpoint(
        &state,
        CONFIDENTIAL_PARTICIPANT_TRANSITIONS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let request = match parse_confidential_openapi_request::<
        confidential_contract::ConfidentialParticipantTransitionRequest,
        _,
    >(
        &body,
        "confidential participant transition body",
        |request| Ok(wire::confidential_participant_transition_request_from_openapi(request)),
    ) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match state
        .confidential_application()
        .transition_confidential_participant(principal.as_principal(), request)
        .await
    {
        Ok(
            ConfidentialParticipantTransitionOutcome::Updated(participant)
            | ConfidentialParticipantTransitionOutcome::Unchanged(participant),
        ) => json_projection_response(StatusCode::OK, wire::participant_response(&participant)),
        Err(error) => confidential_participant_error_response(error),
    }
}

pub(crate) async fn get_confidential_participant_audit_events(
    State(state): State<ConfidentialHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    let principal = match authorize_confidential_read_endpoint(
        &state,
        CONFIDENTIAL_PARTICIPANT_AUDIT_EVENTS_PATH,
        &headers,
        &query,
    )
    .await
    {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let filter = match ConfidentialParticipantAuditEventFilter::from_query(&query) {
        Ok(filter) => filter,
        Err(error) => return error.into_response(),
    };

    match state
        .confidential_application()
        .list_confidential_participant_audit_events(principal.as_principal(), filter)
        .await
    {
        Ok(events) => json_projection_response(
            StatusCode::OK,
            wire::participant_audit_event_list_response(&events),
        ),
        Err(error) => confidential_participant_error_response(error),
    }
}

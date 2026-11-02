//! xAPI document resource handlers.
#![allow(clippy::wildcard_imports)]

use super::common::*;
use crate::core::runtime::xapi_limits::BodyKind;
use crate::surfaces::xapi::request::{
    document_read_status, mixed_condition_response, undefined_condition_response,
};
use crate::surfaces::xapi::response::document_list_response;

pub(crate) async fn get_state_document(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    get_document(state, headers, query, DocumentKind::State).await
}

pub(crate) async fn put_state_document(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    put_document(state, headers, query, DocumentKind::State, body).await
}

pub(crate) async fn post_state_document(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    post_document(state, headers, query, DocumentKind::State, body).await
}

pub(crate) async fn delete_state_document(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    delete_document(state, headers, query, DocumentKind::State).await
}

pub(crate) async fn get_agent_profile_document(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    get_document(state, headers, query, DocumentKind::AgentProfile).await
}

pub(crate) async fn put_agent_profile_document(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    put_document(state, headers, query, DocumentKind::AgentProfile, body).await
}

pub(crate) async fn post_agent_profile_document(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    post_document(state, headers, query, DocumentKind::AgentProfile, body).await
}

pub(crate) async fn delete_agent_profile_document(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    delete_document(state, headers, query, DocumentKind::AgentProfile).await
}

pub(crate) async fn get_activity_profile_document(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    get_document(state, headers, query, DocumentKind::ActivityProfile).await
}

pub(crate) async fn put_activity_profile_document(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    put_document(state, headers, query, DocumentKind::ActivityProfile, body).await
}

pub(crate) async fn post_activity_profile_document(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    post_document(state, headers, query, DocumentKind::ActivityProfile, body).await
}

pub(crate) async fn delete_activity_profile_document(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    delete_document(state, headers, query, DocumentKind::ActivityProfile).await
}

async fn get_document(
    state: XapiHttpContext,
    headers: HeaderMap,
    query: BTreeMap<String, String>,
    kind: DocumentKind,
) -> Response {
    if let Err(error) = kind.validate_query(&query, true) {
        return error.into_response();
    }
    if let Err(error) = state.authorize_xapi_read_request(&headers, &query).await {
        return error.into_response();
    }

    let selection = match DocumentAddress::from_query(kind, &query, DocumentIdRequirement::Optional)
    {
        Ok(selection) => selection,
        Err(error) => return error.into_response(),
    };
    if let Some(response) = mixed_condition_response(&headers) {
        return response;
    }
    match selection {
        DocumentSelection::Single(address) => {
            match xapi_service::get_document(state.xapi_services(), &address).await {
                Ok(Some(record)) => {
                    match document_read_status(&record.etag, Some(record.updated_at), &headers) {
                        StatusCode::PRECONDITION_FAILED => HttpError::new(
                            StatusCode::PRECONDITION_FAILED,
                            "document precondition failed",
                        )
                        .into_response(),
                        status => document_response(status, &record),
                    }
                }
                Ok(None) => {
                    HttpError::new(StatusCode::NOT_FOUND, "document was not found").into_response()
                }
                Err(error) => document_store_error_response(&error),
            }
        }
        DocumentSelection::Scope(scope) => {
            match xapi_service::list_document_ids(state.xapi_services(), &scope).await {
                Ok(document_ids) => document_list_response(&document_ids, &headers),
                Err(error) => document_store_error_response(&error),
            }
        }
    }
}

async fn put_document(
    state: XapiHttpContext,
    headers: HeaderMap,
    query: BTreeMap<String, String>,
    kind: DocumentKind,
    body: Bytes,
) -> Response {
    if let Err(error) = kind.validate_query(&query, false) {
        return error.into_response();
    }
    let principal = match state.authorize_xapi_write_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    if let Err(error) = state.runtime_limits().xapi.body(
        principal
            .as_ref()
            .map(kiva_identity::AuthorizedPrincipal::as_principal),
        BodyKind::Document,
        body.len(),
    ) {
        return error.into_response();
    }
    let address = match required_document_address(kind, &query) {
        Ok(address) => address,
        Err(error) => return error.into_response(),
    };
    let content = document_content_from_request(&headers, &body);
    if let Some(response) = mixed_condition_response(&headers) {
        return response;
    }
    let preconditions = Preconditions::from_headers(&headers);

    match xapi_service::put_document(state.xapi_services(), address, content, preconditions).await {
        Ok(DocumentWriteOutcome::UndefinedPreconditions) => undefined_condition_response(),
        Ok(DocumentWriteOutcome::Stored) => empty_response(StatusCode::NO_CONTENT),
        Ok(DocumentWriteOutcome::Conflict) => HttpError::new(
            StatusCode::CONFLICT,
            "Check current state and use If-Match with the current ETag.",
        )
        .into_response(),
        Ok(DocumentWriteOutcome::PreconditionFailed) => HttpError::new(
            StatusCode::PRECONDITION_FAILED,
            "document precondition failed",
        )
        .into_response(),
        Ok(DocumentWriteOutcome::InvalidMerge) => {
            HttpError::new(StatusCode::BAD_REQUEST, "document merge is invalid").into_response()
        }
        Err(error) => document_store_error_response(&error),
    }
}

async fn post_document(
    state: XapiHttpContext,
    headers: HeaderMap,
    query: BTreeMap<String, String>,
    kind: DocumentKind,
    body: Bytes,
) -> Response {
    if let Err(error) = kind.validate_query(&query, false) {
        return error.into_response();
    }
    let principal = match state.authorize_xapi_write_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    if let Err(error) = state.runtime_limits().xapi.body(
        principal
            .as_ref()
            .map(kiva_identity::AuthorizedPrincipal::as_principal),
        BodyKind::Document,
        body.len(),
    ) {
        return error.into_response();
    }
    let address = match required_document_address(kind, &query) {
        Ok(address) => address,
        Err(error) => return error.into_response(),
    };
    let content = document_content_from_request(&headers, &body);
    if content.as_json_object().is_none() {
        return HttpError::new(
            StatusCode::BAD_REQUEST,
            "document merge requires an application/json object",
        )
        .into_response();
    }
    let preconditions = Preconditions::from_headers(&headers);

    match xapi_service::post_json_merge(state.xapi_services(), address, content, preconditions)
        .await
    {
        Ok(DocumentWriteOutcome::UndefinedPreconditions) => undefined_condition_response(),
        Ok(DocumentWriteOutcome::Stored) => empty_response(StatusCode::NO_CONTENT),
        Ok(DocumentWriteOutcome::Conflict) => HttpError::new(
            StatusCode::CONFLICT,
            "Check current state and use If-Match with the current ETag.",
        )
        .into_response(),
        Ok(DocumentWriteOutcome::PreconditionFailed) => HttpError::new(
            StatusCode::PRECONDITION_FAILED,
            "document precondition failed",
        )
        .into_response(),
        Ok(DocumentWriteOutcome::InvalidMerge) => {
            HttpError::new(StatusCode::BAD_REQUEST, "document merge is invalid").into_response()
        }
        Err(error) => document_store_error_response(&error),
    }
}

async fn delete_document(
    state: XapiHttpContext,
    headers: HeaderMap,
    query: BTreeMap<String, String>,
    kind: DocumentKind,
) -> Response {
    if let Err(error) = kind.validate_query(&query, false) {
        return error.into_response();
    }
    if let Err(error) = state.authorize_xapi_write_request(&headers, &query).await {
        return error.into_response();
    }
    let id_requirement = match kind {
        DocumentKind::State => DocumentIdRequirement::Optional,
        DocumentKind::AgentProfile | DocumentKind::ActivityProfile => {
            DocumentIdRequirement::Required
        }
    };
    let selection = match DocumentAddress::from_query(kind, &query, id_requirement) {
        Ok(selection) => selection,
        Err(error) => return error.into_response(),
    };
    if let Some(response) = mixed_condition_response(&headers) {
        return response;
    }
    let preconditions = Preconditions::from_headers(&headers);

    match xapi_service::delete_document(state.xapi_services(), selection, preconditions).await {
        Ok(DocumentDeleteOutcome::Deleted | DocumentDeleteOutcome::NotFound) => {
            empty_response(StatusCode::NO_CONTENT)
        }
        Ok(DocumentDeleteOutcome::PreconditionFailed) => HttpError::new(
            StatusCode::PRECONDITION_FAILED,
            "document precondition failed",
        )
        .into_response(),
        Err(error) => document_store_error_response(&error),
    }
}

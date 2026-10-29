//! xAPI Statement resource handlers.
#![allow(clippy::wildcard_imports)]

use super::common::*;
use crate::core::runtime::xapi_limits::BodyKind;
use crate::surfaces::xapi::lossless;
use crate::surfaces::xapi::request::{StatementCursor, statement_page};

pub(crate) async fn get_statements(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    statement_resource_response(get_statements_inner(state, headers, query, None).await)
}

pub(crate) async fn get_more_statements(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(mut query): Query<BTreeMap<String, String>>,
) -> Response {
    let cursor = match StatementCursor::from_query(&mut query) {
        Ok(cursor) => cursor,
        Err(error) => return statement_resource_response(error.into_response()),
    };
    statement_resource_response(get_statements_inner(state, headers, query, Some(cursor)).await)
}

#[allow(clippy::too_many_lines)]
async fn get_statements_inner(
    state: XapiHttpContext,
    headers: HeaderMap,
    query: BTreeMap<String, String>,
    cursor: Option<StatementCursor>,
) -> Response {
    let principal = match state.authorize_xapi_read_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    if let Err(error) = validate_statement_get_query(&query) {
        return error.into_response();
    }
    if query.contains_key("statementId") && query.contains_key("voidedStatementId") {
        return HttpError::new(
            StatusCode::BAD_REQUEST,
            "statementId and voidedStatementId must not be supplied together",
        )
        .into_response();
    }
    if let Some(statement_id) = query.get("statementId") {
        let statement_id = match parse_statement_id(statement_id) {
            Ok(statement_id) => statement_id,
            Err(error) => return error.into_response(),
        };
        if !state.may_read_statement(principal.as_ref(), statement_id) {
            return HttpError::new(StatusCode::NOT_FOUND, "statement was not found")
                .into_response();
        }
        let all_statements = match xapi_service::list_statements(state.xapi_services(), None).await
        {
            Ok(statements) => statements,
            Err(error) => return store_error_response(&error),
        };
        if kiva_xapi::statement_is_voided(&all_statements, statement_id) {
            return HttpError::new(StatusCode::NOT_FOUND, "statement was not found")
                .into_response();
        }
        return xapi_service::get_statement(state.xapi_services(), statement_id)
            .await
            .map_or_else(
                |error| store_error_response(&error),
                |statement| {
                    statement.map_or_else(
                        || {
                            HttpError::new(StatusCode::NOT_FOUND, "statement was not found")
                                .into_response()
                        },
                        |statement| {
                            let statement = formatted_single_statement(statement, &query, &headers);
                            single_statement_response(StatusCode::OK, &statement, &query)
                        },
                    )
                },
            );
    }
    if let Some(statement_id) = query.get("voidedStatementId") {
        let statement_id = match parse_statement_id(statement_id) {
            Ok(statement_id) => statement_id,
            Err(error) => return error.into_response(),
        };
        if !state.may_read_statement(principal.as_ref(), statement_id) {
            return HttpError::new(StatusCode::NOT_FOUND, "statement was not found")
                .into_response();
        }
        return xapi_service::get_statement(state.xapi_services(), statement_id)
            .await
            .map_or_else(
                |error| store_error_response(&error),
                |statement| {
                    statement.map_or_else(
                        || {
                            HttpError::new(StatusCode::NOT_FOUND, "statement was not found")
                                .into_response()
                        },
                        |statement| {
                            let statement = formatted_single_statement(statement, &query, &headers);
                            single_statement_response(StatusCode::OK, &statement, &query)
                        },
                    )
                },
            );
    }

    let limit = match optional_limit(&query) {
        Ok(limit) => limit,
        Err(error) => return error.into_response(),
    };
    let limit = state
        .runtime_limits()
        .max_statement_page_size
        .map_or(limit, |maximum| {
            Some(limit.unwrap_or_else(|| maximum.get()).min(maximum.get()))
        });
    let statements = match xapi_service::list_statements(state.xapi_services(), None).await {
        Ok(statements) => statements,
        Err(error) => return store_error_response(&error),
    };
    let mut statements = match filtered_statements(statements, &query, None) {
        Ok(statements) => statements,
        Err(error) => return error.into_response(),
    };
    // Resolve graph/voiding semantics on the complete repository, then apply
    // the verified subject's permission before page boundaries or attachments.
    // Every continuation request independently authenticates and re-evaluates
    // this policy; possession of a cursor never grants access.
    statements.retain(|statement| {
        statement
            .get("id")
            .and_then(serde_json::Value::as_str)
            .and_then(|id| uuid::Uuid::parse_str(id).ok())
            .is_some_and(|id| state.may_read_statement(principal.as_ref(), id))
    });
    let (statements, more) = statement_page(statements, &query, limit, cursor.as_ref());
    let statements = statements
        .into_iter()
        .map(|statement| formatted_statement(statement, &query, &headers))
        .collect::<Vec<_>>();
    let body = json!({"statements": statements, "more": more});
    let with_attachments = query.get("attachments").map(String::as_str) == Some("true");
    let mut attachments = BTreeMap::new();
    let mut opaque = lossless::OpaqueValues::new();
    let ids = body["statements"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|statement| {
            statement["id"]
                .as_str()
                .and_then(|id| uuid::Uuid::parse_str(id).ok())
        })
        .collect::<Vec<_>>();
    let raw_values = match xapi_service::statement_opaque_values(state.xapi_services(), &ids).await
    {
        Ok(values) => values,
        Err(error) => return store_error_response(&error),
    };
    for (index, statement) in body["statements"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let Some(statement_id) = statement
            .get("id")
            .and_then(serde_json::Value::as_str)
            .and_then(|id| uuid::Uuid::parse_str(id).ok())
        else {
            continue;
        };
        if let Some(values) = raw_values.get(&statement_id) {
            lossless::extend_prefixed(&mut opaque, values, &format!("/statements/{index}"));
        }
        if with_attachments {
            match xapi_service::get_statement(state.xapi_services(), statement_id).await {
                Ok(Some(record)) => attachments.extend(
                    record
                        .attachments
                        .into_iter()
                        .map(|(hash, attachment)| (hash.to_ascii_lowercase(), attachment)),
                ),
                Ok(None) => {}
                Err(error) => return store_error_response(&error),
            }
        }
    }
    if with_attachments {
        return statement_result_multipart_response(StatusCode::OK, body, attachments, &opaque);
    }
    statement_result_response(StatusCode::OK, body, &opaque)
}

pub(crate) async fn put_statement(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    statement_resource_response(put_statement_inner(state, headers, query, body).await)
}

async fn put_statement_inner(
    state: XapiHttpContext,
    headers: HeaderMap,
    query: BTreeMap<String, String>,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_query_parameters(&query, &["statementId"]) {
        return error.into_response();
    }
    if let Some(response) = statement_body_limit_response(state.runtime_limits(), body.len()) {
        return response;
    }
    let principal = match state.authorize_xapi_write_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let authenticated = principal
        .as_ref()
        .map(kiva_identity::AuthorizedPrincipal::as_principal);
    if let Err(error) =
        state
            .runtime_limits()
            .xapi
            .body(authenticated, BodyKind::Statement, body.len())
    {
        return error.into_response();
    }
    let authority = match statement_authority(
        principal
            .as_ref()
            .map(kiva_identity::AuthorizedPrincipal::as_principal),
    ) {
        Ok(authority) => authority,
        Err(error) => return error.into_response(),
    };
    let Some(statement_id) = query.get("statementId") else {
        return HttpError::new(StatusCode::BAD_REQUEST, "statementId is required").into_response();
    };
    let statement_id = match parse_statement_id(statement_id) {
        Ok(statement_id) => statement_id,
        Err(error) => return error.into_response(),
    };
    let payload = match statement_payload_from_request(&headers, &body) {
        Ok(payload) => payload,
        Err(error) => return error.into_response(),
    };
    for attachment in payload.attachments.values() {
        if let Err(error) = state.runtime_limits().xapi.body(
            authenticated,
            BodyKind::Attachment,
            attachment.body.len(),
        ) {
            return error.into_response();
        }
    }
    let stored_at = OffsetDateTime::now_utc();
    let mut statement = match normalized_statement_record(
        payload.statement,
        statement_id,
        stored_at,
        &authority,
        &payload.attachments,
        state.signature_policy(),
    ) {
        Ok(statement) => statement,
        Err(error) => return error.into_response(),
    };

    statement.opaque_values = payload.opaque_values;

    match xapi_service::put_statement(state.xapi_services(), statement_id, statement).await {
        Ok(StoreOutcome::Stored | StoreOutcome::Unchanged) => {
            empty_response(StatusCode::NO_CONTENT)
        }
        Ok(StoreOutcome::Conflict) => HttpError::new(
            StatusCode::CONFLICT,
            "statement id already exists with different content",
        )
        .into_response(),
        Err(error) => store_error_response(&error),
    }
}

pub(crate) async fn post_statements(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
    body: Bytes,
) -> Response {
    statement_resource_response(post_statements_inner(state, headers, query, body).await)
}

async fn post_statements_inner(
    state: XapiHttpContext,
    headers: HeaderMap,
    query: BTreeMap<String, String>,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_query_parameters(&query, &[]) {
        return error.into_response();
    }
    if let Some(response) = statement_body_limit_response(state.runtime_limits(), body.len()) {
        return response;
    }
    let principal = match state.authorize_xapi_write_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let authenticated = principal
        .as_ref()
        .map(kiva_identity::AuthorizedPrincipal::as_principal);
    if let Err(error) =
        state
            .runtime_limits()
            .xapi
            .body(authenticated, BodyKind::Statement, body.len())
    {
        return error.into_response();
    }
    let authority = match statement_authority(
        principal
            .as_ref()
            .map(kiva_identity::AuthorizedPrincipal::as_principal),
    ) {
        Ok(authority) => authority,
        Err(error) => return error.into_response(),
    };
    let payload = match statement_payload_from_request(&headers, &body) {
        Ok(payload) => payload,
        Err(error) => return error.into_response(),
    };
    for attachment in payload.attachments.values() {
        if let Err(error) = state.runtime_limits().xapi.body(
            authenticated,
            BodyKind::Attachment,
            attachment.body.len(),
        ) {
            return error.into_response();
        }
    }
    let is_array = payload.statement.is_array();
    let mut statements = match normalize_statement_batch(
        payload.statement,
        OffsetDateTime::now_utc(),
        &authority,
        &payload.attachments,
        state.signature_policy(),
    ) {
        Ok(statements) => statements,
        Err(error) => return error.into_response(),
    };
    for (index, (_, record)) in statements.iter_mut().enumerate() {
        record.opaque_values = if is_array {
            crate::surfaces::xapi::lossless::subtree(&payload.opaque_values, &format!("/{index}"))
        } else {
            payload.opaque_values.clone()
        };
    }
    let ids = statements
        .iter()
        .map(|(statement_id, _statement)| statement_id.to_string())
        .collect::<Vec<_>>();

    match xapi_service::put_statement_batch(state.xapi_services(), statements).await {
        Ok(()) => json_response(StatusCode::OK, json!(ids)),
        Err(PutBatchError::Conflict(statement_id)) => HttpError::new(
            StatusCode::CONFLICT,
            format!("statement id already exists with different content: {statement_id}"),
        )
        .into_response(),
        Err(PutBatchError::Store(error)) => store_error_response(&error),
    }
}

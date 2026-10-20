//! xAPI Statement resource handlers.
#![allow(clippy::wildcard_imports)]

use super::common::*;
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
    if let Err(error) = state.authorize_xapi_read_request(&headers, &query).await {
        return error.into_response();
    }
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
    let statements = match xapi_service::list_statements(state.xapi_services(), None).await {
        Ok(statements) => statements,
        Err(error) => return store_error_response(&error),
    };
    let statements = match filtered_statements(statements, &query, None) {
        Ok(statements) => statements,
        Err(error) => return error.into_response(),
    };
    let (statements, more) = statement_page(statements, &query, limit, cursor.as_ref());
    let statements = statements
        .into_iter()
        .map(|statement| formatted_statement(statement, &query, &headers))
        .collect::<Vec<_>>();
    let body = json!({"statements": statements, "more": more});
    if query.get("attachments").map(String::as_str) == Some("true") {
        let mut attachments = BTreeMap::new();
        for statement in body["statements"].as_array().into_iter().flatten() {
            let Some(statement_id) = statement
                .get("id")
                .and_then(serde_json::Value::as_str)
                .and_then(|id| uuid::Uuid::parse_str(id).ok())
            else {
                continue;
            };
            match xapi_service::get_statement(state.xapi_services(), statement_id).await {
                Ok(Some(record)) => attachments.extend(record.attachments),
                Ok(None) => {}
                Err(error) => return store_error_response(&error),
            }
        }
        return statement_result_multipart_response(StatusCode::OK, body, &attachments);
    }
    statement_result_response(StatusCode::OK, body)
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
    let stored_at = OffsetDateTime::now_utc();
    let statement = match normalized_statement_record(
        payload.statement,
        statement_id,
        stored_at,
        &authority,
        &payload.attachments,
    ) {
        Ok(statement) => statement,
        Err(error) => return error.into_response(),
    };

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
    let statements = match normalize_statement_batch(
        payload.statement,
        OffsetDateTime::now_utc(),
        &authority,
        &payload.attachments,
    ) {
        Ok(statements) => statements,
        Err(error) => return error.into_response(),
    };
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

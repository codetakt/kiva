//! xAPI Statement resource handlers.
#![allow(clippy::wildcard_imports)]

use super::common::*;
use crate::core::runtime::xapi_limits::BodyKind;
use crate::surfaces::xapi::lossless;
use crate::surfaces::xapi::request::{PageCursor, statement_page};

pub(crate) async fn get_statements(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    statement_resource_response(get_statements_inner(state, headers, query, false).await)
}

pub(crate) async fn get_more_statements(
    State(state): State<XapiHttpContext>,
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    statement_resource_response(get_statements_inner(state, headers, query, true).await)
}

#[allow(clippy::too_many_lines)]
async fn get_statements_inner(
    state: XapiHttpContext,
    headers: HeaderMap,
    mut query: BTreeMap<String, String>,
    continuation: bool,
) -> Response {
    let principal = match state.authorize_xapi_read_request(&headers, &query).await {
        Ok(principal) => principal,
        Err(error) => return error.into_response(),
    };
    let cursor = if continuation {
        match PageCursor::resolve(&state, &mut query).await {
            Ok(cursor) => Some(cursor),
            Err(error) => return error.into_response(),
        }
    } else {
        None
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
    let canonical = if query.get("format").map(String::as_str) == Some("canonical")
        && state.capabilities().config.canonical_inventory
    {
        match super::super::capabilities::canonical::Inventory::load(&state).await {
            Ok(inventory) => Some(inventory),
            Err(error) => return error.into_response(),
        }
    } else {
        None
    };
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
                        |mut statement| {
                            if let Some(inventory) = &canonical {
                                inventory
                                    .apply(&mut statement.statement, &mut statement.opaque_values);
                            }
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
                        |mut statement| {
                            if let Some(inventory) = &canonical {
                                inventory
                                    .apply(&mut statement.statement, &mut statement.opaque_values);
                            }
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
    let mut statements = match filtered_statements(
        statements,
        &query,
        None,
        state.capabilities().config.mailbox_hash_queries,
    ) {
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
    let (statements, more) =
        match statement_page(&state, statements, &query, limit, cursor.as_ref()).await {
            Ok(page) => page,
            Err(error) => return error.into_response(),
        };
    let mut body = json!({"statements": statements, "more": more});
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
        .as_array_mut()
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
        let mut values = raw_values.get(&statement_id).cloned().unwrap_or_default();
        if let Some(inventory) = &canonical {
            inventory.apply(statement, &mut values);
        }
        *statement = formatted_statement(std::mem::take(statement), &query, &headers);
        lossless::extend_prefixed(&mut opaque, &values, &format!("/statements/{index}"));
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
    if state.capabilities().config.detailed_interactions
        && let Err(error) = super::super::capabilities::interaction::validate(&payload.statement)
    {
        return error.into_response();
    }
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
    let supplied_authority = payload.statement.get("authority").cloned();
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
    state.capabilities().finish_admission(
        &mut statement,
        authenticated.map(|p| p.subject.as_str()),
        supplied_authority,
    );

    let encountered = vec![statement.statement.clone()];
    match xapi_service::put_statement(state.xapi_services(), statement_id, statement).await {
        Ok(StoreOutcome::Stored) => {
            super::super::capabilities::fetch::encountered(&state, &encountered).await;
            empty_response(StatusCode::NO_CONTENT)
        }
        Ok(StoreOutcome::Unchanged) => empty_response(StatusCode::NO_CONTENT),
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

#[allow(clippy::too_many_lines)]
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
    if state.capabilities().config.detailed_interactions
        && let Err(error) = super::super::capabilities::interaction::validate(&payload.statement)
    {
        return error.into_response();
    }
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
    let supplied_authorities = payload.statement.as_array().map_or_else(
        || vec![payload.statement.get("authority").cloned()],
        |statements| {
            statements
                .iter()
                .map(|s| s.get("authority").cloned())
                .collect::<Vec<_>>()
        },
    );
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
        state.capabilities().finish_admission(
            record,
            authenticated.map(|p| p.subject.as_str()),
            supplied_authorities[index].clone(),
        );
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

    let mut encountered = statements
        .iter()
        .map(|(id, record)| (*id, record.statement.clone()))
        .collect::<BTreeMap<_, _>>();
    match xapi_service::put_statement_batch(state.xapi_services(), statements).await {
        Ok(inserted) => {
            // Admission determines novelty under the same lock/transaction as
            // storage. A replay cannot schedule or retry metadata acquisition.
            let new_statements = inserted
                .into_iter()
                .filter_map(|id| encountered.remove(&id))
                .collect::<Vec<_>>();
            if !new_statements.is_empty() {
                super::super::capabilities::fetch::encountered(&state, &new_statements).await;
            }
            json_response(StatusCode::OK, json!(ids))
        }
        Err(PutBatchError::Conflict(statement_id)) => HttpError::new(
            StatusCode::CONFLICT,
            format!("statement id already exists with different content: {statement_id}"),
        )
        .into_response(),
        Err(PutBatchError::Store(error)) => store_error_response(&error),
    }
}

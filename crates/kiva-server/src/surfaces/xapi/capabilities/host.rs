//! Activity identifier hosting and shared extension write preconditions.

use std::collections::BTreeMap;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use super::{canonical::Definition, store::Record};
use crate::core::{
    response::{HttpError, new_etag},
    state::XapiHttpContext,
};
use crate::surfaces::xapi::{model::Preconditions, request::document_read_status};

fn enabled(state: &XapiHttpContext) -> Result<(), HttpError> {
    if !state.capabilities().config.activity_hosting {
        return Err(HttpError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "Activity hosting is not enabled",
        ));
    }
    Ok(())
}

pub(crate) fn precondition(
    headers: &HeaderMap,
    existing: Option<&Record>,
) -> Result<(), HttpError> {
    let condition = Preconditions::from_headers(headers);
    if condition.undefined_combination
        || headers.contains_key("if-modified-since")
        || headers.contains_key("if-unmodified-since")
    {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "metadata writes support a single If-Match or If-None-Match condition",
        ));
    }
    if !condition.has_any() && existing.is_some() {
        return Err(HttpError::new(
            StatusCode::CONFLICT,
            "GET the current resource and retry with its ETag in If-Match",
        ));
    }
    if !kiva_http_core::document_preconditions_pass(
        existing.map(|r| r.etag.as_str()),
        condition.if_match.as_deref(),
        condition.if_none_match.as_deref(),
    ) {
        return Err(HttpError::new(
            StatusCode::PRECONDITION_FAILED,
            "metadata write precondition failed",
        ));
    }
    Ok(())
}

pub(crate) fn representation(record: Record, headers: &HeaderMap) -> Response {
    let status = document_read_status(&record.etag, None, headers);
    let body = if status == StatusCode::OK {
        record.body
    } else {
        Vec::new()
    };
    (
        status,
        [
            (header::CONTENT_TYPE, "application/json".to_owned()),
            (header::ETAG, record.etag),
        ],
        body,
    )
        .into_response()
}

pub(crate) async fn writer(
    state: &XapiHttpContext,
    headers: &HeaderMap,
    directory: bool,
) -> Result<(), HttpError> {
    let principal = state
        .authorize_xapi_write_request(headers, &BTreeMap::new())
        .await?;
    let subjects = if directory {
        &state.capabilities().config.directory_writers
    } else {
        &state.capabilities().config.canonical_writers
    };
    if principal
        .as_ref()
        .is_none_or(|p| !subjects.contains(&p.as_principal().subject))
    {
        return Err(HttpError::new(
            StatusCode::FORBIDDEN,
            "this subject cannot manage the requested metadata",
        ));
    }
    Ok(())
}

pub(crate) async fn get(
    State(state): State<XapiHttpContext>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Response, HttpError> {
    // Only explicitly published metadata is served here. It is public identifier
    // metadata, not an unauthenticated path to Statements or private profiles.
    enabled(&state)?;
    let record = state
        .capabilities()
        .store
        .get(&format!("host:{id}"))
        .await?
        .ok_or_else(|| {
            HttpError::new(
                StatusCode::NOT_FOUND,
                "hosted Activity Definition was not found",
            )
        })?;
    Ok(representation(record, &headers))
}

pub(crate) async fn put(
    State(state): State<XapiHttpContext>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, HttpError> {
    writer(&state, &headers, false).await?;
    enabled(&state)?;
    if body.len() > state.capabilities().config.metadata_max_bytes {
        return Err(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Activity Definition exceeds the configured metadata limit",
        ));
    }
    let definition = Definition::parse(&body)?;
    if state.capabilities().config.detailed_interactions {
        super::interaction::definition(&definition.value)?;
    }
    let record = state
        .capabilities()
        .store
        .mutate("host:", move |records| {
            let key = format!("host:{id}");
            let existing = records.get(&key);
            precondition(&headers, existing)?;
            let definition = match existing {
                Some(old) => Definition::parse(&old.body)?.corrected(&definition)?,
                None => definition,
            };
            let record = Record {
                body: definition.bytes()?,
                etag: new_etag(),
            };
            records.insert(key, record.clone());
            Ok(record)
        })
        .await?;
    Ok((StatusCode::NO_CONTENT, [(header::ETAG, record.etag)]).into_response())
}

pub(crate) async fn delete(
    State(state): State<XapiHttpContext>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Response, HttpError> {
    writer(&state, &headers, false).await?;
    enabled(&state)?;
    state
        .capabilities()
        .store
        .mutate("host:", move |records| {
            let key = format!("host:{id}");
            precondition(&headers, records.get(&key))?;
            records.remove(&key);
            Ok(())
        })
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

//! Preserve query identities by checking UTF-8 before form decoding can replace it.

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;

use crate::core::response::HttpError;

pub(crate) struct Query<T>(pub T);

impl<S: Send + Sync, T: DeserializeOwned> FromRequestParts<S> for Query<T> {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        if !valid_query_utf8(parts.uri.query().unwrap_or_default()) {
            return Err(
                HttpError::new(StatusCode::BAD_REQUEST, "query must contain valid UTF-8")
                    .into_response(),
            );
        }
        axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Query(value)| Self(value))
            .map_err(IntoResponse::into_response)
    }
}

fn valid_query_utf8(query: &str) -> bool {
    let mut bytes = query.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    while let Some((&byte, tail)) = bytes.split_first() {
        // Match form_urlencoded's single percent-decoding pass. Literal '+'
        // and the separators are ASCII, so their form meaning cannot affect
        // UTF-8 validity. Malformed percent escapes retain existing handling.
        if byte == b'%'
            && let [a, b, rest @ ..] = tail
            && let (Some(a), Some(b)) = (hex(*a), hex(*b))
        {
            decoded.push(a * 16 + b);
            bytes = rest;
            continue;
        }
        decoded.push(byte);
        bytes = tail;
    }
    std::str::from_utf8(&decoded).is_ok()
}

const fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

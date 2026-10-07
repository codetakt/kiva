//! Multipart Statement request parsing.

use std::collections::BTreeMap;

use axum::http::StatusCode;
use sha2::{Digest, Sha256};

use crate::{
    core::{
        config::{DOCUMENT_JSON_CONTENT_TYPE, XAPI_HASH_HEADER},
        response::HttpError,
    },
    surfaces::xapi::model::{AttachmentRecord, MultipartPart, StatementPayload},
};

use super::{
    content_type_matches, json::parse_json_without_duplicate_properties,
    openapi::statement_body_from_openapi_contract,
};

pub(crate) fn multipart_statement_payload(
    content_type: &str,
    body: &[u8],
) -> Result<StatementPayload, HttpError> {
    let boundary = multipart_boundary(content_type)?;
    let parts = parse_multipart_parts(body, &boundary)?;
    let Some((first, attachments)) = parts.split_first() else {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "multipart Statement body must contain a JSON first part",
        ));
    };
    if !first
        .headers
        .get("content-type")
        .is_some_and(|value| content_type_matches(value, DOCUMENT_JSON_CONTENT_TYPE))
    {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "multipart Statement first part must be application/json",
        ));
    }

    let statement =
        parse_json_without_duplicate_properties(&first.body, "multipart Statement first part")
            .and_then(|body| {
                statement_body_from_openapi_contract(body, "multipart Statement first part")
            })?;
    let mut records = BTreeMap::new();
    for part in attachments {
        if part
            .headers
            .get("content-transfer-encoding")
            .is_none_or(|value| !value.eq_ignore_ascii_case("binary"))
        {
            return Err(HttpError::new(
                StatusCode::BAD_REQUEST,
                "multipart attachment part Content-Transfer-Encoding must be binary",
            ));
        }
        let hash = part
            .headers
            .get(&XAPI_HASH_HEADER.to_ascii_lowercase())
            .ok_or_else(|| {
                HttpError::new(
                    StatusCode::BAD_REQUEST,
                    "multipart attachment part must include X-Experience-API-Hash",
                )
            })?;
        let content_type = part
            .headers
            .get("content-type")
            .cloned()
            .unwrap_or_else(|| "application/octet-stream".to_owned());
        let computed_hash = sha256_hex(&part.body);
        if computed_hash != *hash {
            return Err(HttpError::new(
                StatusCode::BAD_REQUEST,
                "multipart attachment hash does not match attachment body",
            ));
        }
        records.insert(
            hash.clone(),
            AttachmentRecord {
                content_type,
                body: part.body.clone(),
            },
        );
    }

    Ok(StatementPayload {
        statement,
        attachments: records,
    })
}

pub(crate) fn multipart_boundary(content_type: &str) -> Result<String, HttpError> {
    content_type
        .split(';')
        .skip(1)
        .filter_map(|parameter| parameter.trim().split_once('='))
        .find_map(|(name, value)| {
            name.trim()
                .eq_ignore_ascii_case("boundary")
                .then(|| value.trim().trim_matches('"').to_owned())
        })
        .filter(|boundary| !boundary.is_empty())
        .ok_or_else(|| {
            HttpError::new(
                StatusCode::BAD_REQUEST,
                "multipart Statement content type must include a boundary",
            )
        })
}

pub(crate) fn parse_multipart_parts(
    body: &[u8],
    boundary: &str,
) -> Result<Vec<MultipartPart>, HttpError> {
    let delimiter = format!("--{boundary}");
    let delimiter = delimiter.as_bytes();
    let boundary_start = if body.starts_with(delimiter) {
        0
    } else {
        let prefixed_delimiter = [b"\r\n".as_slice(), delimiter].concat();
        body.windows(prefixed_delimiter.len())
            .position(|window| window == prefixed_delimiter)
            .map(|position| position + 2)
            .ok_or_else(|| {
                HttpError::new(
                    StatusCode::BAD_REQUEST,
                    "multipart body does not contain its opening boundary",
                )
            })?
    };

    let mut cursor = boundary_start + delimiter.len();
    let mut parts = Vec::new();
    loop {
        if body.get(cursor..cursor.saturating_add(2)) == Some(b"--") {
            break;
        }
        cursor = skip_line_ending(body, cursor)?;
        let (part_end, boundary_start) = next_multipart_boundary(body, cursor, delimiter)?;
        parts.push(parse_multipart_part(&body[cursor..part_end])?);
        cursor = boundary_start + delimiter.len();
    }
    Ok(parts)
}

pub(crate) fn skip_line_ending(body: &[u8], cursor: usize) -> Result<usize, HttpError> {
    if body.get(cursor..cursor.saturating_add(2)) == Some(b"\r\n") {
        return Ok(cursor + 2);
    }
    if body.get(cursor..cursor.saturating_add(1)) == Some(b"\n") {
        return Ok(cursor + 1);
    }
    Err(HttpError::new(
        StatusCode::BAD_REQUEST,
        "multipart boundary must be followed by a line ending",
    ))
}

pub(crate) fn next_multipart_boundary(
    body: &[u8],
    cursor: usize,
    delimiter: &[u8],
) -> Result<(usize, usize), HttpError> {
    let mut crlf_marker = b"\r\n".to_vec();
    crlf_marker.extend_from_slice(delimiter);
    if let Some(offset) = find_bytes(&body[cursor..], &crlf_marker) {
        return Ok((cursor + offset, cursor + offset + 2));
    }

    let mut lf_marker = b"\n".to_vec();
    lf_marker.extend_from_slice(delimiter);
    if let Some(offset) = find_bytes(&body[cursor..], &lf_marker) {
        return Ok((cursor + offset, cursor + offset + 1));
    }

    Err(HttpError::new(
        StatusCode::BAD_REQUEST,
        "multipart body is missing a closing boundary",
    ))
}

pub(crate) fn parse_multipart_part(body: &[u8]) -> Result<MultipartPart, HttpError> {
    let (headers, content) = split_multipart_part(body)?;
    let headers = parse_multipart_headers(headers)?;
    Ok(MultipartPart {
        headers,
        body: content.to_vec(),
    })
}

pub(crate) fn split_multipart_part(body: &[u8]) -> Result<(&[u8], &[u8]), HttpError> {
    if let Some(offset) = find_bytes(body, b"\r\n\r\n") {
        return Ok((&body[..offset], &body[offset + 4..]));
    }
    if let Some(offset) = find_bytes(body, b"\n\n") {
        return Ok((&body[..offset], &body[offset + 2..]));
    }
    Err(HttpError::new(
        StatusCode::BAD_REQUEST,
        "multipart part must separate headers from the body",
    ))
}

pub(crate) fn parse_multipart_headers(
    headers: &[u8],
) -> Result<BTreeMap<String, String>, HttpError> {
    let headers = std::str::from_utf8(headers).map_err(|_error| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            "multipart part headers must be valid UTF-8",
        )
    })?;
    headers
        .lines()
        .map(|line| {
            let (name, value) = line.split_once(':').ok_or_else(|| {
                HttpError::new(StatusCode::BAD_REQUEST, "multipart part header is invalid")
            })?;
            Ok((name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        })
        .collect()
}

pub(crate) fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub(crate) fn sha256_hex(body: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(body);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push(char::from(HEX[usize::from(byte >> 4)]));
        hex.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    hex
}

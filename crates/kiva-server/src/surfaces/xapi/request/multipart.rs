//! Multipart Statement request parsing.

use std::collections::BTreeMap;

use axum::http::StatusCode;

use crate::{
    core::{
        config::{DOCUMENT_JSON_CONTENT_TYPE, XAPI_HASH_HEADER},
        response::HttpError,
    },
    surfaces::xapi::model::{AttachmentRecord, MultipartPart, StatementPayload},
};

use super::{media, openapi::statement_body_from_openapi_contract};

mod digest;
use digest::attachment_hash_matches;

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
    if !first.headers.get("content-type").is_some_and(|value| {
        media::media_type(value, true)
            .is_ok_and(|value| value.essence == DOCUMENT_JSON_CONTENT_TYPE)
    }) {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "multipart Statement first part must be application/json",
        ));
    }

    let (statement, opaque_values) = crate::surfaces::xapi::lossless::parse_statement(
        &first.body,
        "multipart Statement first part",
    )?;
    let statement =
        statement_body_from_openapi_contract(statement, "multipart Statement first part")?;
    let mut records: BTreeMap<String, AttachmentRecord> = BTreeMap::new();
    for part in attachments {
        if let Some(length) = part.headers.get("content-length")
            && (length.is_empty()
                || !length.bytes().all(|b| b.is_ascii_digit())
                || length.parse::<usize>().ok() != Some(part.body.len()))
        {
            return Err(HttpError::new(
                StatusCode::BAD_REQUEST,
                "multipart attachment Content-Length does not match attachment body",
            ));
        }
        // The receiver follows the xAPI binary-default recommendation.
        // Responses always emit the mandatory binary header explicitly.
        if let Some(value) = part.headers.get("content-transfer-encoding") {
            media::transfer_encoding(value)?;
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
        let declared_type = part.headers.get("content-type");
        if let Some(content_type) = declared_type {
            media::media_type(content_type, true)?;
        }
        let content_type = declared_type.cloned().unwrap_or_default();
        if !attachment_hash_matches(&part.body, hash) {
            return Err(HttpError::new(
                StatusCode::BAD_REQUEST,
                "multipart attachment hash does not match attachment body",
            ));
        }
        let hash = hash.to_ascii_lowercase();
        if let Some(previous) = records.get_mut(&hash) {
            // Deduplication cannot hide an invalid earlier part. Validate
            // every occurrence before retaining one byte-identical copy.
            if previous.body != part.body
                || !previous.content_type.is_empty()
                    && !content_type.is_empty()
                    && !media::matching_types(&previous.content_type, &content_type)?
            {
                return Err(media::invalid());
            }
            if previous.content_type.is_empty() {
                previous.content_type = content_type;
            }
            continue;
        }
        records.insert(
            hash,
            AttachmentRecord {
                content_type,
                body: part.body.clone(),
            },
        );
    }

    Ok(StatementPayload {
        statement,
        opaque_values,
        attachments: records,
    })
}

pub(crate) fn multipart_boundary(content_type: &str) -> Result<String, HttpError> {
    let mut parsed = media::media_type(content_type, false)?;
    if parsed.essence != "multipart/mixed" {
        return Err(media::invalid());
    }
    let boundary = parsed
        .parameters
        .remove("boundary")
        .ok_or_else(media::invalid)?;
    if boundary.is_empty()
        || boundary.len() > 70
        || boundary.ends_with(' ')
        || !boundary
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"'()+_,-./:=? ".contains(&c))
    {
        return Err(media::invalid());
    }
    Ok(boundary)
}

pub(crate) fn parse_multipart_parts(
    body: &[u8],
    boundary: &str,
) -> Result<Vec<MultipartPart>, HttpError> {
    let delimiter = format!("--{boundary}");
    let delimiter = delimiter.as_bytes();
    let boundary_start =
        if body.starts_with(delimiter) && boundary_line_end(body, delimiter.len()).is_some() {
            0
        } else {
            next_multipart_boundary(body, 0, delimiter, true)?.1
        };

    let mut cursor = boundary_start + delimiter.len();
    let mut parts = Vec::new();
    loop {
        let Some((closing, next)) = boundary_line_end(body, cursor) else {
            return Err(HttpError::new(
                StatusCode::BAD_REQUEST,
                "invalid multipart boundary",
            ));
        };
        if closing {
            break;
        }
        cursor = next;
        let (part_end, boundary_start) = next_multipart_boundary(body, cursor, delimiter, false)?;
        parts.push(parse_multipart_part(&body[cursor..part_end])?);
        cursor = boundary_start + delimiter.len();
    }
    Ok(parts)
}

// RFC 2046 section 5.1.1: delimiters occupy a complete CRLF line. A body
// line beginning with the chosen prefix but trailing other text is invalid.
fn boundary_line_end(body: &[u8], mut cursor: usize) -> Option<(bool, usize)> {
    let closing = body.get(cursor..cursor.checked_add(2)?) == Some(b"--");
    if closing {
        cursor += 2;
    }
    while matches!(body.get(cursor), Some(b' ' | b'\t')) {
        cursor += 1;
    }
    if closing && cursor == body.len() {
        return Some((true, cursor));
    }
    skip_line_ending(body, cursor)
        .ok()
        .map(|next| (closing, next))
}

pub(crate) fn skip_line_ending(body: &[u8], cursor: usize) -> Result<usize, HttpError> {
    if body.get(cursor..cursor.saturating_add(2)) == Some(b"\r\n") {
        return Ok(cursor + 2);
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
    preamble: bool,
) -> Result<(usize, usize), HttpError> {
    let mut marker = b"\r\n".to_vec();
    marker.extend_from_slice(delimiter);
    for (offset, window) in body[cursor..].windows(marker.len()).enumerate() {
        let start = cursor + offset + 2;
        if window == marker {
            if boundary_line_end(body, start + delimiter.len()).is_some() {
                return Ok((cursor + offset, start));
            }
            if !preamble {
                return Err(media::invalid());
            }
        }
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
    // RFC 2046 permits an empty part header area. The xAPI first-part and
    // attachment header requirements are checked after framing is established.
    if let Some(content) = body.strip_prefix(b"\r\n") {
        return Ok((&[], content));
    }
    if let Some(offset) = find_bytes(body, b"\r\n\r\n") {
        return Ok((&body[..offset], &body[offset + 4..]));
    }
    Err(HttpError::new(
        StatusCode::BAD_REQUEST,
        "multipart part must separate headers from the body",
    ))
}

pub(crate) fn parse_multipart_headers(
    headers: &[u8],
) -> Result<BTreeMap<String, String>, HttpError> {
    if headers.is_empty() {
        return Ok(BTreeMap::new());
    }
    if !headers.is_ascii() {
        return Err(media::invalid());
    }
    let headers = std::str::from_utf8(headers).map_err(|_| media::invalid())?;
    let mut unfolded: Vec<String> = Vec::new();
    for line in headers.split("\r\n") {
        // Only CRLF delimits fields. RFC 822 quoted-pair can carry any ASCII
        // CHAR, and qtext/ctext can carry LF; their structured parsers decide
        // validity without deleting octets. Bare CR/LF outside that syntax
        // is rejected by the relevant field parser.
        if line.starts_with([' ', '\t']) {
            let previous = unfolded.last_mut().ok_or_else(|| {
                HttpError::new(
                    StatusCode::BAD_REQUEST,
                    "multipart header continuation has no field",
                )
            })?;
            // Unfold by removing only the line ending, preserving whitespace.
            previous.push_str(line);
        } else {
            unfolded.push(line.to_owned());
        }
    }
    let mut fields = BTreeMap::new();
    for line in unfolded {
        let (name, value) = line.split_once(':').ok_or_else(|| {
            HttpError::new(StatusCode::BAD_REQUEST, "multipart part header is invalid")
        })?;
        if name.is_empty() || !name.bytes().all(|c| c.is_ascii_graphic() && c != b':') {
            return Err(media::invalid());
        }
        if !name.eq_ignore_ascii_case("content-type")
            && !name.eq_ignore_ascii_case("content-transfer-encoding")
            && value.contains(['\r', '\n'])
        {
            return Err(media::invalid());
        }
        if fields
            .insert(
                name.to_ascii_lowercase(),
                value.trim_matches([' ', '\t']).to_owned(),
            )
            .is_some()
        {
            return Err(media::invalid());
        }
    }
    Ok(fields)
}

pub(crate) fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests;

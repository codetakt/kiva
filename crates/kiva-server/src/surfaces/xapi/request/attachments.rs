//! xAPI multipart attachment and compact JWS request validation.

use std::collections::BTreeMap;

use axum::http::StatusCode;
use kiva_xapi::statements_equivalent;
use serde_json::Value;

use crate::{
    core::{config::SIGNATURE_ATTACHMENT_USAGE_TYPE, response::HttpError},
    surfaces::xapi::model::{AttachmentRecord, CompactJwsSegments},
};

use super::json::parse_json_without_duplicate_properties;

pub(crate) fn attachments_for_statement(
    statement: &Value,
    attachments: &BTreeMap<String, AttachmentRecord>,
) -> Result<BTreeMap<String, AttachmentRecord>, HttpError> {
    let Some(metadata) = statement.get("attachments") else {
        if attachments.is_empty() {
            return Ok(BTreeMap::new());
        }
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "multipart attachment parts must be referenced by Statement attachment metadata",
        ));
    };
    let Some(metadata) = metadata.as_array() else {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "Statement attachments must be an array",
        ));
    };

    let mut referenced = BTreeMap::new();
    for entry in metadata {
        let sha2 = required_attachment_string(entry, "sha2")?;
        let content_type = required_attachment_string(entry, "contentType")?;
        let length = required_attachment_length(entry)?;
        let Some(record) = attachments.get(sha2) else {
            if entry.get("fileUrl").is_some()
                && entry.pointer("/usageType").and_then(Value::as_str)
                    != Some(SIGNATURE_ATTACHMENT_USAGE_TYPE)
            {
                continue;
            }
            return Err(HttpError::new(
                StatusCode::BAD_REQUEST,
                "Statement attachment metadata references a missing attachment part",
            ));
        };
        if record.content_type != content_type {
            return Err(HttpError::new(
                StatusCode::BAD_REQUEST,
                "Statement attachment contentType does not match its attachment part",
            ));
        }
        if record.body.len() != length {
            return Err(HttpError::new(
                StatusCode::BAD_REQUEST,
                "Statement attachment length does not match its attachment part",
            ));
        }
        if entry.pointer("/usageType").and_then(Value::as_str)
            == Some(SIGNATURE_ATTACHMENT_USAGE_TYPE)
        {
            if content_type != "application/octet-stream" {
                return Err(HttpError::new(
                    StatusCode::BAD_REQUEST,
                    "signed Statement attachment contentType must be application/octet-stream",
                ));
            }
            validate_signed_statement_attachment(statement, record)?;
        }
        referenced.insert(sha2.to_owned(), record.clone());
    }
    if referenced.len() != attachments.len() {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "multipart attachment parts must be referenced by Statement attachment metadata",
        ));
    }
    Ok(referenced)
}

fn validate_signed_statement_attachment(
    statement: &Value,
    attachment: &AttachmentRecord,
) -> Result<(), HttpError> {
    let jws = std::str::from_utf8(&attachment.body).map_err(|_error| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            "signed Statement attachment must be a UTF-8 compact JWS",
        )
    })?;
    let segments = compact_jws_segments(jws)?;
    let header = jws_json_segment(segments.header, "JWS protected header")?;
    if !matches!(
        header.pointer("/alg").and_then(Value::as_str),
        Some("RS256" | "RS384" | "RS512")
    ) {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "signed Statement JWS alg must be RS256, RS384, or RS512",
        ));
    }
    let payload = jws_json_segment(segments.payload, "JWS payload")?;
    let mut received = canonical_signed_statement(statement);
    let signed = canonical_signed_statement(&payload);
    if payload.get("version").is_none()
        && let Some(received) = received.as_object_mut()
    {
        received.remove("version");
    }
    if !statements_equivalent(&received, &signed) {
        return Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "signed Statement JWS payload must match the received Statement",
        ));
    }
    Ok(())
}

fn compact_jws_segments(value: &str) -> Result<CompactJwsSegments<'_>, HttpError> {
    let mut segments = value.split('.');
    let Some(header) = segments.next() else {
        return Err(malformed_jws_error());
    };
    let Some(payload) = segments.next() else {
        return Err(malformed_jws_error());
    };
    let Some(signature) = segments.next() else {
        return Err(malformed_jws_error());
    };
    if segments.next().is_some() || header.is_empty() || payload.is_empty() || signature.is_empty()
    {
        return Err(malformed_jws_error());
    }
    Ok(CompactJwsSegments { header, payload })
}

fn malformed_jws_error() -> HttpError {
    HttpError::new(
        StatusCode::BAD_REQUEST,
        "signed Statement attachment must be a compact JWS",
    )
}

fn jws_json_segment(segment: &str, label: &str) -> Result<Value, HttpError> {
    let decoded = base64_url_decode(segment).map_err(|_error| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            format!("{label} must be base64url encoded"),
        )
    })?;
    parse_json_without_duplicate_properties(&decoded, label)
}

fn base64_url_decode(value: &str) -> Result<Vec<u8>, ()> {
    let mut output = Vec::with_capacity(value.len() * 3 / 4);
    let mut accumulator = 0_u32;
    let mut bits = 0_u8;
    for byte in value.bytes() {
        let sextet = base64_url_sextet(byte).ok_or(())?;
        accumulator = (accumulator << 6) | u32::from(sextet);
        bits += 6;
        while bits >= 8 {
            bits -= 8;
            output.push(((accumulator >> bits) & 0xff) as u8);
        }
    }
    if bits > 0 && (accumulator & ((1_u32 << bits) - 1)) != 0 {
        return Err(());
    }
    Ok(output)
}

const fn base64_url_sextet(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    }
}

fn canonical_signed_statement(statement: &Value) -> Value {
    let mut statement = statement.clone();
    let Some(object) = statement.as_object_mut() else {
        return statement;
    };
    object.remove("attachments");
    object.remove("stored");
    object.remove("timestamp");
    object.remove("authority");
    statement
}

fn required_attachment_string<'a>(entry: &'a Value, name: &str) -> Result<&'a str, HttpError> {
    entry.get(name).and_then(Value::as_str).ok_or_else(|| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            format!("Statement attachment {name} must be a string"),
        )
    })
}

fn required_attachment_length(entry: &Value) -> Result<usize, HttpError> {
    let length = entry.get("length").and_then(Value::as_u64).ok_or_else(|| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            "Statement attachment length must be a non-negative integer",
        )
    })?;
    usize::try_from(length).map_err(|_error| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            "Statement attachment length is too large",
        )
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{canonical_signed_statement, statements_equivalent};

    #[test]
    fn signed_statement_comparison_ignores_sub_centisecond_duration_precision() {
        let received = canonical_signed_statement(&json!({
            "result": {"duration": "P1DT12H36M0.1237S"},
            "attachments": []
        }));
        let signed = canonical_signed_statement(&json!({
            "result": {"duration": "P1DT12H36M0.12S"}
        }));

        assert!(statements_equivalent(&received, &signed));
    }
}

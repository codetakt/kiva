//! xAPI HTTP response mapping.

use std::collections::BTreeMap;

use axum::{
    body::Body,
    http::{
        HeaderMap, StatusCode,
        header::{CONTENT_TYPE, ETAG, LAST_MODIFIED},
    },
    response::{IntoResponse, Response},
};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::core::{
    config::{MULTIPART_MIXED_CONTENT_TYPE, XAPI_HASH_HEADER},
    response::{
        HttpError, backend_error_response, http_date, insert_header, statement_result_headers,
        version_headers,
    },
    runtime::RuntimeLimits,
};
use crate::surfaces::xapi::{
    model::{
        DocumentAddress, DocumentContent, DocumentIdRequirement, DocumentKind, DocumentRecord,
        DocumentSelection, StoredStatement,
    },
    outcome::{DocumentStoreError, StatementStoreError},
    wire,
};

pub(crate) fn required_document_address(
    kind: DocumentKind,
    query: &BTreeMap<String, String>,
) -> Result<DocumentAddress, HttpError> {
    match DocumentAddress::from_query(kind, query, DocumentIdRequirement::Required)? {
        DocumentSelection::Single(address) => Ok(address),
        DocumentSelection::Scope(_scope) => Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            format!("{} is required", kind.id_param()),
        )),
    }
}

pub(crate) fn document_response(status: StatusCode, record: &DocumentRecord) -> Response {
    let headers = document_headers(record);
    match &record.content {
        DocumentContent::Json(value) => match serde_json::to_vec(value) {
            Ok(body) => (status, headers, Body::from(body)).into_response(),
            Err(_error) => HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "document body could not be serialized",
            )
            .into_response(),
        },
        DocumentContent::Bytes { body, .. } => {
            (status, headers, Body::from(body.clone())).into_response()
        }
    }
}

pub(crate) fn single_statement_response(
    status: StatusCode,
    record: &StoredStatement,
    query: &BTreeMap<String, String>,
) -> Response {
    if query.get("attachments").map(String::as_str) == Some("true") {
        return statement_multipart_response(status, record);
    }
    statement_json_response(status, record)
}

pub(crate) fn statement_json_response(status: StatusCode, record: &StoredStatement) -> Response {
    xapi_json_projection_response(
        status,
        single_statement_headers(record.stored_at),
        wire::statement_response(record.statement.clone()),
    )
}

pub(crate) fn statement_multipart_response(
    status: StatusCode,
    record: &StoredStatement,
) -> Response {
    let statement = match wire::statement_response(record.statement.clone())
        .and_then(|statement| serde_json::to_string(&statement))
    {
        Ok(statement) => statement,
        Err(error) => {
            return backend_error_response(
                &error,
                "Statement response projection failed",
                "Statement response projection failed",
            );
        }
    };
    let boundary = format!("kiva-statement-attachments-{}", Uuid::new_v4());
    let mut body = Vec::new();
    append_multipart_line(&mut body, &format!("--{boundary}"));
    append_multipart_line(&mut body, "Content-Type: application/json");
    append_multipart_line(&mut body, "");
    append_multipart_line(&mut body, &statement);
    for (sha2, attachment) in &record.attachments {
        append_multipart_line(&mut body, &format!("--{boundary}"));
        append_multipart_line(
            &mut body,
            &format!("Content-Type: {}", attachment.content_type),
        );
        append_multipart_line(&mut body, "Content-Transfer-Encoding: binary");
        append_multipart_line(&mut body, &format!("{XAPI_HASH_HEADER}: {sha2}"));
        append_multipart_line(&mut body, "");
        body.extend_from_slice(&attachment.body);
        append_multipart_line(&mut body, "");
    }
    append_multipart_line(&mut body, &format!("--{boundary}--"));

    let mut headers = single_statement_headers(record.stored_at);
    insert_header(
        &mut headers,
        CONTENT_TYPE,
        &format!("{MULTIPART_MIXED_CONTENT_TYPE}; boundary={boundary}"),
    );
    (status, headers, Body::from(body)).into_response()
}

pub(crate) fn append_multipart_line(body: &mut Vec<u8>, line: &str) {
    body.extend_from_slice(line.as_bytes());
    body.extend_from_slice(b"\r\n");
}

pub(crate) fn document_headers(record: &DocumentRecord) -> HeaderMap {
    let mut headers = version_headers();
    insert_header(&mut headers, ETAG, &record.etag);
    insert_header(&mut headers, LAST_MODIFIED, &http_date(record.updated_at));
    insert_header(
        &mut headers,
        ::http::header::CONTENT_TYPE,
        &record.content_type,
    );
    headers
}

pub(crate) fn single_statement_headers(stored_at: OffsetDateTime) -> HeaderMap {
    let mut headers = statement_result_headers();
    insert_header(&mut headers, LAST_MODIFIED, &http_date(stored_at));
    headers
}

pub(crate) fn store_error_response(error: &StatementStoreError) -> Response {
    backend_error_response(
        &error.source,
        "statement store operation failed",
        "statement store operation failed",
    )
}

pub(crate) fn document_store_error_response(error: &DocumentStoreError) -> Response {
    backend_error_response(
        &error.source,
        "document store operation failed",
        "document store operation failed",
    )
}

pub(crate) fn statement_body_limit_response(
    limits: &RuntimeLimits,
    body_len: usize,
) -> Option<Response> {
    if limits
        .max_statement_body_bytes()
        .is_some_and(|limit| body_len > limit)
    {
        return Some(
            HttpError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Statement request body is too large",
            )
            .into_response(),
        );
    }
    None
}

pub(crate) fn statement_result_response(status: StatusCode, body: Value) -> Response {
    let last_modified = body
        .get("statements")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(kiva_xapi::statement_stored_at)
        .max();
    let mut headers = statement_result_headers();
    if let Some(last_modified) = last_modified {
        insert_header(&mut headers, LAST_MODIFIED, &http_date(last_modified));
    }
    xapi_json_projection_response(status, headers, wire::statement_result_response(body))
}

pub(crate) fn statement_result_multipart_response(
    status: StatusCode,
    body: Value,
    attachments: &BTreeMap<String, crate::surfaces::xapi::model::AttachmentRecord>,
) -> Response {
    let body = match wire::statement_result_response(body) {
        Ok(body) => body,
        Err(error) => {
            return backend_error_response(
                &error,
                "StatementResult response projection failed",
                "StatementResult response projection failed",
            );
        }
    };
    let last_modified = body
        .get("statements")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(kiva_xapi::statement_stored_at)
        .max();
    let boundary = format!("kiva-statement-attachments-{}", Uuid::new_v4());
    let mut multipart = Vec::new();
    append_multipart_line(&mut multipart, &format!("--{boundary}"));
    append_multipart_line(&mut multipart, "Content-Type: application/json");
    append_multipart_line(&mut multipart, "");
    match serde_json::to_string(&body) {
        Ok(body) => append_multipart_line(&mut multipart, &body),
        Err(error) => {
            return backend_error_response(
                &error,
                "StatementResult response serialization failed",
                "StatementResult response serialization failed",
            );
        }
    }
    for (sha2, attachment) in attachments {
        append_multipart_line(&mut multipart, &format!("--{boundary}"));
        append_multipart_line(
            &mut multipart,
            &format!("Content-Type: {}", attachment.content_type),
        );
        append_multipart_line(&mut multipart, "Content-Transfer-Encoding: binary");
        append_multipart_line(&mut multipart, &format!("{XAPI_HASH_HEADER}: {sha2}"));
        append_multipart_line(&mut multipart, "");
        multipart.extend_from_slice(&attachment.body);
        append_multipart_line(&mut multipart, "");
    }
    append_multipart_line(&mut multipart, &format!("--{boundary}--"));

    let mut headers = statement_result_headers();
    if let Some(last_modified) = last_modified {
        insert_header(&mut headers, LAST_MODIFIED, &http_date(last_modified));
    }
    insert_header(
        &mut headers,
        CONTENT_TYPE,
        &format!("{MULTIPART_MIXED_CONTENT_TYPE}; boundary={boundary}"),
    );
    (status, headers, Body::from(multipart)).into_response()
}

fn xapi_json_projection_response(
    status: StatusCode,
    mut headers: HeaderMap,
    body: Result<Value, serde_json::Error>,
) -> Response {
    match body.and_then(|body| serde_json::to_vec(&body)) {
        Ok(body) => {
            insert_header(&mut headers, CONTENT_TYPE, "application/json");
            (status, headers, Body::from(body)).into_response()
        }
        Err(error) => backend_error_response(
            &error,
            "xAPI response projection failed",
            "xAPI response projection failed",
        ),
    }
}

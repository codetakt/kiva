//! xAPI statement and document server model types.

mod document_list;

pub(crate) use document_list::DocumentList;

use std::collections::BTreeMap;

use axum::http::{
    HeaderMap, StatusCode,
    header::{IF_MATCH, IF_NONE_MATCH},
};
use kiva_http_core::DocumentContentShape;
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::core::{
    config::DOCUMENT_JSON_CONTENT_TYPE,
    request_util::{header_text, key_component},
    response::{HttpError, new_etag, rfc3339},
};
use crate::surfaces::xapi::request::{
    content_type_matches, document_request_content_type, optional_registration,
    required_document_actor, required_iri, validate_query_parameters,
};
use kiva_xapi::canonical_actor_ifi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DocumentKind {
    State,
    AgentProfile,
    ActivityProfile,
}

impl DocumentKind {
    pub(crate) fn validate_query(
        self,
        query: &BTreeMap<String, String>,
        allow_since: bool,
    ) -> Result<(), HttpError> {
        let mut allowed = match self {
            Self::State => vec!["activityId", "agent", "registration", "stateId"],
            Self::AgentProfile => vec!["agent", "profileId"],
            Self::ActivityProfile => vec!["activityId", "profileId"],
        };
        // `since` filters the multiple-document operation. A present ID
        // selects the single-document table, where that parameter is absent.
        if allow_since && !query.contains_key(self.id_param()) {
            allowed.push("since");
        }
        validate_query_parameters(query, &allowed)
    }

    pub(crate) const fn as_resource_kind(self) -> &'static str {
        match self {
            Self::State => "state",
            Self::AgentProfile => "agent-profile",
            Self::ActivityProfile => "activity-profile",
        }
    }

    pub(crate) const fn id_param(self) -> &'static str {
        match self {
            Self::State => "stateId",
            Self::AgentProfile | Self::ActivityProfile => "profileId",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum DocumentSelection {
    Single(DocumentAddress),
    Scope(DocumentScope),
}

#[derive(Debug, Clone)]
pub(crate) struct DocumentAddress {
    pub(crate) kind: DocumentKind,
    pub(crate) document_key: String,
    pub(crate) document_id: String,
    pub(crate) activity_id: Option<String>,
    pub(crate) agent: Option<Value>,
    pub(crate) registration: Option<Uuid>,
}

impl DocumentAddress {
    pub(crate) fn from_query(
        kind: DocumentKind,
        query: &BTreeMap<String, String>,
        id_requirement: DocumentIdRequirement,
    ) -> Result<DocumentSelection, HttpError> {
        let scope = DocumentScope::from_query(kind, query)?;
        let Some(document_id) = query.get(kind.id_param()).cloned() else {
            return match id_requirement {
                DocumentIdRequirement::Required => Err(HttpError::new(
                    StatusCode::BAD_REQUEST,
                    format!("{} is required", kind.id_param()),
                )),
                DocumentIdRequirement::Optional => Ok(DocumentSelection::Scope(scope)),
            };
        };
        // Presence selects the document, including the empty String. The
        // Document/selector tables do not impose a nonempty identifier rule.
        let document_key = format!(
            "{}{}",
            scope.document_key_prefix,
            key_component("document_id", &document_id)
        );
        Ok(DocumentSelection::Single(Self {
            kind,
            document_key,
            document_id,
            activity_id: scope.activity_id,
            agent: scope.agent,
            registration: scope.registration,
        }))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DocumentScope {
    pub(crate) kind: DocumentKind,
    pub(crate) document_key_prefix: String,
    pub(crate) activity_id: Option<String>,
    pub(crate) agent: Option<Value>,
    pub(crate) registration: Option<Uuid>,
    pub(crate) since: Option<OffsetDateTime>,
}

impl DocumentScope {
    pub(crate) fn from_query(
        kind: DocumentKind,
        query: &BTreeMap<String, String>,
    ) -> Result<Self, HttpError> {
        let since = query
            .get("since")
            .map(|since| {
                OffsetDateTime::parse(since, &Rfc3339).map_err(|_error| {
                    HttpError::new(
                        StatusCode::BAD_REQUEST,
                        "since must be an RFC3339 timestamp",
                    )
                })
            })
            .transpose()?;
        let mut prefix = key_component("kind", kind.as_resource_kind());
        let mut activity_id = None;
        let mut agent = None;
        let mut registration = None;

        match kind {
            DocumentKind::State => {
                let activity = required_iri(query, "activityId")?;
                let parsed_agent = required_document_actor(query)?;
                let agent_key = canonical_actor_ifi(&parsed_agent).ok_or_else(|| {
                    HttpError::new(
                        StatusCode::BAD_REQUEST,
                        "agent must be a valid Agent object",
                    )
                })?;
                registration = optional_registration(query)?;
                prefix.push_str(&key_component("activity_id", activity));
                prefix.push_str(&key_component("agent", &agent_key));
                if registration.is_some() || query.contains_key("stateId") {
                    prefix.push_str(&key_component(
                        "registration",
                        &registration.map_or_else(|| "none".to_owned(), |value| value.to_string()),
                    ));
                }
                activity_id = Some(activity.to_owned());
                agent = Some(parsed_agent);
            }
            DocumentKind::AgentProfile => {
                let parsed_agent = required_document_actor(query)?;
                let agent_key = canonical_actor_ifi(&parsed_agent).ok_or_else(|| {
                    HttpError::new(
                        StatusCode::BAD_REQUEST,
                        "agent must be a valid Agent object",
                    )
                })?;
                prefix.push_str(&key_component("agent", &agent_key));
                agent = Some(parsed_agent);
            }
            DocumentKind::ActivityProfile => {
                let activity = required_iri(query, "activityId")?;
                prefix.push_str(&key_component("activity_id", activity));
                activity_id = Some(activity.to_owned());
            }
        }

        Ok(Self {
            kind,
            document_key_prefix: prefix,
            activity_id,
            agent,
            registration,
            since,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum DocumentIdRequirement {
    Required,
    Optional,
}

#[derive(Debug, Clone)]
pub(crate) struct DocumentRecord {
    pub(crate) document_id: String,
    pub(crate) content: DocumentContent,
    pub(crate) content_type: String,
    pub(crate) etag: String,
    pub(crate) updated_at: OffsetDateTime,
}

impl DocumentRecord {
    pub(crate) fn new(
        document_id: String,
        content: DocumentContent,
        updated_at: OffsetDateTime,
    ) -> Self {
        let content_type = content.content_type().to_owned();
        Self {
            document_id,
            content,
            content_type,
            etag: new_etag(),
            updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DocumentContent {
    Json(Value),
    Bytes { content_type: String, body: Vec<u8> },
}

impl DocumentContent {
    pub(crate) fn from_request(headers: &HeaderMap, body: &[u8]) -> Self {
        let content_type = document_request_content_type(headers);
        // A Document PUT stores the HTTP entity itself. JSON object parsing
        // belongs to POST merge, not to arbitrary document representation.
        Self::Bytes {
            content_type,
            body: body.to_vec(),
        }
    }

    pub(crate) fn content_type(&self) -> &str {
        match self {
            Self::Json(_value) => DOCUMENT_JSON_CONTENT_TYPE,
            Self::Bytes { content_type, .. } => content_type,
        }
    }

    pub(crate) fn as_json_object(&self) -> Option<super::lossless::Members> {
        match self {
            Self::Json(value) => serde_json::from_slice(&serde_json::to_vec(value).ok()?).ok(),
            Self::Bytes { content_type, body }
                if content_type_matches(content_type, DOCUMENT_JSON_CONTENT_TYPE) =>
            {
                serde_json::from_slice(body).ok()
            }
            Self::Bytes { .. } => None,
        }
    }

    pub(crate) fn shape(&self) -> DocumentContentShape {
        match self {
            Self::Json(value) if value.is_object() => DocumentContentShape::JsonObject,
            Self::Json(_value) => DocumentContentShape::JsonNonObject,
            Self::Bytes { content_type, .. }
                if content_type_matches(content_type, DOCUMENT_JSON_CONTENT_TYPE) =>
            {
                if self.as_json_object().is_some() {
                    DocumentContentShape::JsonObject
                } else {
                    DocumentContentShape::JsonNonObject
                }
            }
            Self::Bytes { .. } => DocumentContentShape::Bytes,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StoredStatement {
    pub(crate) statement: Value,
    pub(crate) opaque_values: super::lossless::OpaqueValues,
    pub(crate) stored_at: OffsetDateTime,
    pub(crate) attachments: BTreeMap<String, AttachmentRecord>,
    // Admission-only provenance: an omitted incoming timestamp can reuse the
    // existing value on retry. Persisted records do not need this flag.
    pub(crate) defaulted_substatement_timestamp: bool,
    pub(crate) defaulted_timestamp: bool,
}

impl StoredStatement {
    pub(crate) const fn new(
        statement: Value,
        stored_at: OffsetDateTime,
        attachments: BTreeMap<String, AttachmentRecord>,
    ) -> Self {
        Self {
            statement,
            opaque_values: BTreeMap::new(),
            stored_at,
            attachments,
            defaulted_substatement_timestamp: false,
            defaulted_timestamp: false,
        }
    }

    pub(crate) fn record_at(&mut self, instant: OffsetDateTime) {
        self.stored_at = instant;
        let stored = Value::String(rfc3339(instant));
        self.statement["stored"] = stored.clone();
        if self.defaulted_timestamp {
            self.statement["timestamp"] = stored.clone();
        }
        if self.defaulted_substatement_timestamp {
            self.statement["object"]["timestamp"] = stored;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredStatementMetadata {
    pub(crate) statement_id: Uuid,
    pub(crate) actor_ifi: String,
    pub(crate) verb_id: Option<String>,
    pub(crate) object_id: Option<String>,
    pub(crate) stored_at: OffsetDateTime,
    pub(crate) version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StoredPortabilityStatement {
    pub(crate) statement_id: Uuid,
    pub(crate) statement: Value,
    pub(crate) opaque_values: super::lossless::OpaqueValues,
    pub(crate) stored_at: OffsetDateTime,
    pub(crate) voided: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttachmentRecord {
    // Empty records an absent optional MIME-part Content-Type. Do not invent
    // application/octet-stream and compare it against the metadata claim.
    // Persistence retains this distinction; output omits that optional field.
    pub(crate) content_type: String,
    pub(crate) body: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct StatementPayload {
    pub(crate) statement: Value,
    pub(crate) opaque_values: super::lossless::OpaqueValues,
    pub(crate) attachments: BTreeMap<String, AttachmentRecord>,
}

impl StatementPayload {
    pub(crate) const fn json(statement: Value) -> Self {
        Self {
            statement,
            opaque_values: BTreeMap::new(),
            attachments: BTreeMap::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct MultipartPart {
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct CompactJwsSegments<'a> {
    pub(crate) header: &'a str,
    pub(crate) payload: &'a str,
    pub(crate) signature: &'a str,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Preconditions {
    pub(crate) undefined_combination: bool,
    pub(crate) if_match: Option<String>,
    pub(crate) if_none_match: Option<String>,
    pub(crate) if_unmodified_since: Option<OffsetDateTime>,
    pub(crate) if_modified_since: Option<OffsetDateTime>,
}

impl Preconditions {
    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            undefined_combination: [
                ("if-match", "if-none-match"),
                ("if-match", "if-modified-since"),
                ("if-none-match", "if-unmodified-since"),
                ("if-unmodified-since", "if-modified-since"),
            ]
            .iter()
            .any(|(a, b)| headers.contains_key(*a) && headers.contains_key(*b)),
            // Repeated list-valued fields are one ordered comma list (RFC2616
            // 4.2); do not silently discard a later matching candidate.
            if_match: joined_header(headers, IF_MATCH),
            if_none_match: joined_header(headers, IF_NONE_MATCH),
            if_unmodified_since: http_date_header(headers, "if-unmodified-since"),
            if_modified_since: http_date_header(headers, "if-modified-since")
                .filter(|instant| *instant <= OffsetDateTime::now_utc()),
        }
    }

    pub(crate) const fn has_any(&self) -> bool {
        self.if_match.is_some() || self.if_none_match.is_some()
    }
}

fn joined_header(headers: &HeaderMap, name: axum::http::HeaderName) -> Option<String> {
    let values = headers
        .get_all(name)
        .iter()
        .map(header_text)
        .collect::<Vec<_>>();
    (!values.is_empty()).then(|| values.join(","))
}

fn http_date_header(headers: &HeaderMap, name: &'static str) -> Option<OffsetDateTime> {
    httpdate::parse_http_date(headers.get(name)?.to_str().ok()?)
        .ok()
        .map(OffsetDateTime::from)
}

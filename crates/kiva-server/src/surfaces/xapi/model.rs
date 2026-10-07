//! xAPI statement and document server model types.

use std::collections::BTreeMap;

use axum::http::{
    HeaderMap, StatusCode,
    header::{IF_MATCH, IF_NONE_MATCH},
};
use kiva_http_core::DocumentContentShape;
use serde_json::{Map, Value};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::core::{
    config::DOCUMENT_JSON_CONTENT_TYPE,
    request_util::{header_string, key_component, required_query},
    response::{HttpError, new_etag},
};
use crate::surfaces::xapi::request::{
    content_type_matches, document_request_content_type, optional_registration, required_agent,
};
use kiva_xapi::canonical_actor_ifi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DocumentKind {
    State,
    AgentProfile,
    ActivityProfile,
}

impl DocumentKind {
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
        if document_id.is_empty() {
            return Err(HttpError::new(
                StatusCode::BAD_REQUEST,
                format!("{} must not be empty", kind.id_param()),
            ));
        }
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
}

impl DocumentScope {
    pub(crate) fn from_query(
        kind: DocumentKind,
        query: &BTreeMap<String, String>,
    ) -> Result<Self, HttpError> {
        if query
            .get("since")
            .is_some_and(|since| OffsetDateTime::parse(since, &Rfc3339).is_err())
        {
            return Err(HttpError::new(
                StatusCode::BAD_REQUEST,
                "since must be an RFC3339 timestamp",
            ));
        }
        let mut prefix = key_component("kind", kind.as_resource_kind());
        let mut activity_id = None;
        let mut agent = None;
        let mut registration = None;

        match kind {
            DocumentKind::State => {
                let activity = required_query(query, "activityId")?;
                let parsed_agent = required_agent(query)?;
                let agent_key = canonical_actor_ifi(&parsed_agent).ok_or_else(|| {
                    HttpError::new(
                        StatusCode::BAD_REQUEST,
                        "agent must be a valid Agent object",
                    )
                })?;
                registration = optional_registration(query)?;
                prefix.push_str(&key_component("activity_id", activity));
                prefix.push_str(&key_component("agent", &agent_key));
                prefix.push_str(&key_component(
                    "registration",
                    &registration.map_or_else(|| "none".to_owned(), |value| value.to_string()),
                ));
                activity_id = Some(activity.to_owned());
                agent = Some(parsed_agent);
            }
            DocumentKind::AgentProfile => {
                let parsed_agent = required_agent(query)?;
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
                let activity = required_query(query, "activityId")?;
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
    pub(crate) fn new(document_id: String, content: DocumentContent) -> Self {
        let content_type = content.content_type().to_owned();
        Self {
            document_id,
            content,
            content_type,
            etag: new_etag(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DocumentContent {
    Json(Value),
    Bytes { content_type: String, body: Vec<u8> },
}

impl DocumentContent {
    pub(crate) fn from_request(headers: &HeaderMap, body: &[u8]) -> Result<Self, HttpError> {
        let content_type = document_request_content_type(headers);
        if content_type_matches(&content_type, DOCUMENT_JSON_CONTENT_TYPE) {
            return serde_json::from_slice::<Value>(body)
                .map(Self::Json)
                .map_err(|_error| {
                    HttpError::new(StatusCode::BAD_REQUEST, "document body must be valid JSON")
                });
        }

        Ok(Self::Bytes {
            content_type,
            body: body.to_vec(),
        })
    }

    pub(crate) fn content_type(&self) -> &str {
        match self {
            Self::Json(_value) => DOCUMENT_JSON_CONTENT_TYPE,
            Self::Bytes { content_type, .. } => content_type,
        }
    }

    pub(crate) fn as_json_object(&self) -> Option<&Map<String, Value>> {
        match self {
            Self::Json(value) => value.as_object(),
            Self::Bytes { .. } => None,
        }
    }

    pub(crate) fn shape(&self) -> DocumentContentShape {
        match self {
            Self::Json(value) if value.is_object() => DocumentContentShape::JsonObject,
            Self::Json(_value) => DocumentContentShape::JsonNonObject,
            Self::Bytes { .. } => DocumentContentShape::Bytes,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StoredStatement {
    pub(crate) statement: Value,
    pub(crate) stored_at: OffsetDateTime,
    pub(crate) attachments: BTreeMap<String, AttachmentRecord>,
}

impl StoredStatement {
    pub(crate) const fn new(
        statement: Value,
        stored_at: OffsetDateTime,
        attachments: BTreeMap<String, AttachmentRecord>,
    ) -> Self {
        Self {
            statement,
            stored_at,
            attachments,
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
    pub(crate) stored_at: OffsetDateTime,
    pub(crate) voided: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttachmentRecord {
    pub(crate) content_type: String,
    pub(crate) body: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct StatementPayload {
    pub(crate) statement: Value,
    pub(crate) attachments: BTreeMap<String, AttachmentRecord>,
}

impl StatementPayload {
    pub(crate) const fn json(statement: Value) -> Self {
        Self {
            statement,
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
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Preconditions {
    pub(crate) if_match: Option<String>,
    pub(crate) if_none_match: Option<String>,
}

impl Preconditions {
    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            if_match: header_string(headers, IF_MATCH),
            if_none_match: header_string(headers, IF_NONE_MATCH),
        }
    }

    pub(crate) const fn has_any(&self) -> bool {
        self.if_match.is_some() || self.if_none_match.is_some()
    }
}

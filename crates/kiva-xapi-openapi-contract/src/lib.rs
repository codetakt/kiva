//! xAPI `OpenAPI` contract DTO slice.
//!
//! This crate is intentionally wire-only and generated from
//! `spec/openapi/kiva-xapi.yaml`. Server handlers parse and project
//! Statement-family payloads through these DTOs before entering handwritten
//! xAPI normalization, persistence, or HTTP semantic helpers.

#![allow(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Repository-relative `OpenAPI` contract that owns this crate's generated DTOs.
pub const XAPI_OPENAPI_CONTRACT: &str = "spec/openapi/kiva-xapi.yaml";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Iri(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Timestamp(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Uuid(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LanguageMap(pub BTreeMap<String, String>);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Document(pub Value);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct StringIdArray(pub Vec<String>);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Actor(pub Value);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct AgentOrIdentifiedGroup(pub Value);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct AgentStatementObject(pub Value);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct ActorStatementObject(pub Value);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct IdentifiedGroup(pub Value);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct NonSubStatementObject(pub Value);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct StatementObject(pub Value);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Account {
    pub home_page: Iri,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: Iri,
    pub object_type: Option<String>,
    pub definition: Option<Value>,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Agent {
    #[serde(rename = "objectType")]
    pub object_type: Option<String>,
    pub name: Option<String>,
    pub mbox: Option<Iri>,
    pub mbox_sha1sum: Option<String>,
    pub openid: Option<Iri>,
    pub account: Option<Account>,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Group {
    #[serde(rename = "objectType")]
    pub object_type: String,
    pub name: Option<String>,
    pub member: Option<Vec<Agent>>,
    pub mbox: Option<Iri>,
    pub mbox_sha1sum: Option<String>,
    pub openid: Option<Iri>,
    pub account: Option<Account>,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Attachment {
    pub usage_type: Iri,
    pub display: LanguageMap,
    pub description: Option<LanguageMap>,
    pub content_type: String,
    pub length: u64,
    pub sha2: String,
    pub file_url: Option<Iri>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Verb {
    pub id: Iri,
    pub display: Option<LanguageMap>,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Statement {
    pub id: Option<Uuid>,
    pub actor: Actor,
    pub verb: Verb,
    pub object: StatementObject,
    pub result: Option<Value>,
    pub context: Option<Value>,
    pub timestamp: Option<Timestamp>,
    pub stored: Option<Timestamp>,
    pub authority: Option<Actor>,
    pub version: Option<String>,
    pub attachments: Option<Vec<Attachment>>,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatementRef {
    pub object_type: String,
    pub id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubStatement {
    pub object_type: String,
    pub actor: Actor,
    pub verb: Verb,
    pub object: NonSubStatementObject,
    pub result: Option<Value>,
    pub context: Option<Value>,
    pub timestamp: Option<Timestamp>,
    pub attachments: Option<Vec<Attachment>>,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatementResult {
    pub statements: Vec<Statement>,
    pub more: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Person {
    #[serde(rename = "objectType")]
    pub object_type: String,
    pub name: Option<Vec<String>>,
    pub mbox: Option<Vec<Iri>>,
    pub mbox_sha1sum: Option<Vec<String>>,
    pub openid: Option<Vec<String>>,
    pub account: Option<Vec<Account>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct About {
    pub version: Vec<String>,
    pub extensions: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ErrorResponse {
    pub error: String,
    pub detail: Option<String>,
}

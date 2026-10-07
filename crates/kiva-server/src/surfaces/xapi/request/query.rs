//! xAPI query parameter parsing and support-resource helpers.

use std::collections::BTreeMap;

use axum::http::StatusCode;
use kiva_http_core::{
    StatementBooleanParameter, StatementFormatParameter, StatementGetQueryDecision,
    StatementGetQueryParameter, StatementGetQueryParameterSet, StatementGetQueryParameters,
    StatementLimitParameter, StatementTimestampParameter, statement_get_query_decision,
};
use kiva_xapi::{filter_statements, validate_agent};
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::core::{request_util::required_query, response::HttpError};

pub(crate) fn optional_limit(query: &BTreeMap<String, String>) -> Result<Option<usize>, HttpError> {
    query
        .get("limit")
        .map(|limit| {
            let limit = limit.parse::<usize>().map_err(|_error| {
                HttpError::new(
                    StatusCode::BAD_REQUEST,
                    "limit must be a non-negative integer",
                )
            })?;
            if limit == 0 {
                return Ok(None);
            }
            Ok(Some(limit))
        })
        .transpose()
        .map(Option::flatten)
}

pub(crate) fn validate_statement_get_query(
    query: &BTreeMap<String, String>,
) -> Result<(), HttpError> {
    match statement_get_query_decision(statement_get_query_parameters(query)) {
        StatementGetQueryDecision::Allow => Ok(()),
        StatementGetQueryDecision::UnknownParameter => {
            let name = unknown_statement_get_query_parameter(query).unwrap_or("unknown");
            Err(HttpError::new(
                StatusCode::BAD_REQUEST,
                format!("unknown Statement query parameter: {name}"),
            ))
        }
        StatementGetQueryDecision::ConflictingStatementIds => Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "statementId and voidedStatementId must not be supplied together",
        )),
        StatementGetQueryDecision::SingleStatementParameterConflict => Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "single Statement retrieval only allows format and attachments as additional parameters",
        )),
        StatementGetQueryDecision::InvalidFormat => Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "format has an unsupported value",
        )),
        StatementGetQueryDecision::InvalidBoolean => {
            let name = invalid_statement_boolean_parameter(query).unwrap_or("boolean parameter");
            Err(HttpError::new(
                StatusCode::BAD_REQUEST,
                format!("{name} has an unsupported value"),
            ))
        }
        StatementGetQueryDecision::InvalidTimestamp => {
            let name = invalid_statement_timestamp_parameter(query).unwrap_or("timestamp");
            Err(HttpError::new(
                StatusCode::BAD_REQUEST,
                format!("{name} must be an RFC3339 timestamp"),
            ))
        }
        StatementGetQueryDecision::InvalidLimit => Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "limit must be a non-negative integer",
        )),
    }
}

pub(crate) fn statement_get_query_parameters(
    query: &BTreeMap<String, String>,
) -> StatementGetQueryParameters {
    let mut parameters = StatementGetQueryParameterSet::EMPTY;
    for name in query.keys() {
        parameters = match statement_get_query_parameter(name) {
            Some(parameter) => parameters.with(parameter),
            None if is_scalar_statement_get_query_parameter(name) => parameters,
            None => parameters.with(StatementGetQueryParameter::Unknown),
        };
    }

    StatementGetQueryParameters {
        parameters,
        related_activities: statement_boolean_parameter(
            query.get("related_activities").map(String::as_str),
        ),
        related_agents: statement_boolean_parameter(
            query.get("related_agents").map(String::as_str),
        ),
        format: statement_format_parameter(query.get("format").map(String::as_str)),
        attachments: statement_boolean_parameter(query.get("attachments").map(String::as_str)),
        ascending: statement_boolean_parameter(query.get("ascending").map(String::as_str)),
        since: statement_timestamp_parameter(query.get("since").map(String::as_str)),
        until: statement_timestamp_parameter(query.get("until").map(String::as_str)),
        limit: statement_limit_parameter(query.get("limit").map(String::as_str)),
    }
}

pub(crate) fn statement_get_query_parameter(name: &str) -> Option<StatementGetQueryParameter> {
    match name {
        "statementId" => Some(StatementGetQueryParameter::StatementId),
        "voidedStatementId" => Some(StatementGetQueryParameter::VoidedStatementId),
        "agent" => Some(StatementGetQueryParameter::Agent),
        "verb" => Some(StatementGetQueryParameter::Verb),
        "activity" => Some(StatementGetQueryParameter::Activity),
        "registration" => Some(StatementGetQueryParameter::Registration),
        _ => None,
    }
}

pub(crate) fn is_scalar_statement_get_query_parameter(name: &str) -> bool {
    matches!(
        name,
        "related_activities"
            | "related_agents"
            | "since"
            | "until"
            | "limit"
            | "format"
            | "attachments"
            | "ascending"
    )
}

pub(crate) fn statement_format_parameter(value: Option<&str>) -> StatementFormatParameter {
    match value {
        None => StatementFormatParameter::Missing,
        Some("exact") => StatementFormatParameter::Exact,
        Some("canonical") => StatementFormatParameter::Canonical,
        Some("ids") => StatementFormatParameter::Ids,
        Some(_) => StatementFormatParameter::Invalid,
    }
}

pub(crate) fn statement_boolean_parameter(value: Option<&str>) -> StatementBooleanParameter {
    match value {
        None => StatementBooleanParameter::Missing,
        Some("true") => StatementBooleanParameter::True,
        Some("false") => StatementBooleanParameter::False,
        Some(_) => StatementBooleanParameter::Invalid,
    }
}

pub(crate) fn statement_timestamp_parameter(value: Option<&str>) -> StatementTimestampParameter {
    match value {
        None => StatementTimestampParameter::Missing,
        Some(value) if OffsetDateTime::parse(value, &Rfc3339).is_ok() => {
            StatementTimestampParameter::Valid
        }
        Some(_) => StatementTimestampParameter::Invalid,
    }
}

pub(crate) fn statement_limit_parameter(value: Option<&str>) -> StatementLimitParameter {
    match value {
        None => StatementLimitParameter::Missing,
        Some(value) if value.parse::<usize>().is_ok() => StatementLimitParameter::Valid,
        Some(_) => StatementLimitParameter::Invalid,
    }
}

pub(crate) fn unknown_statement_get_query_parameter(
    query: &BTreeMap<String, String>,
) -> Option<&str> {
    query
        .keys()
        .find(|name| {
            statement_get_query_parameter(name).is_none()
                && !is_scalar_statement_get_query_parameter(name)
        })
        .map(String::as_str)
}

pub(crate) fn invalid_statement_boolean_parameter(
    query: &BTreeMap<String, String>,
) -> Option<&'static str> {
    [
        "attachments",
        "related_activities",
        "related_agents",
        "ascending",
    ]
    .into_iter()
    .find(|name| {
        query
            .get(*name)
            .is_some_and(|value| !matches!(value.as_str(), "true" | "false"))
    })
}

pub(crate) fn invalid_statement_timestamp_parameter(
    query: &BTreeMap<String, String>,
) -> Option<&'static str> {
    ["since", "until"].into_iter().find(|name| {
        query
            .get(*name)
            .is_some_and(|value| OffsetDateTime::parse(value, &Rfc3339).is_err())
    })
}

pub(crate) fn filtered_statements(
    statements: Vec<Value>,
    query: &BTreeMap<String, String>,
    limit: Option<usize>,
) -> Result<Vec<Value>, HttpError> {
    filter_statements(statements, query, limit)
        .map_err(|error| HttpError::new(StatusCode::BAD_REQUEST, error.message()))
}

pub(crate) fn parse_statement_id(value: &str) -> Result<Uuid, HttpError> {
    Uuid::parse_str(value)
        .map_err(|_error| HttpError::new(StatusCode::BAD_REQUEST, "statement id must be a UUID"))
}

pub(crate) fn required_agent(query: &BTreeMap<String, String>) -> Result<Value, HttpError> {
    let agent = required_query(query, "agent")?;
    match serde_json::from_str::<Value>(agent) {
        Ok(value) if validate_agent(&value).is_ok() => Ok(value),
        Ok(_value) => Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "agent must be a valid Agent object",
        )),
        Err(_error) => Err(HttpError::new(
            StatusCode::BAD_REQUEST,
            "agent must be valid JSON",
        )),
    }
}

pub(crate) fn optional_registration(
    query: &BTreeMap<String, String>,
) -> Result<Option<Uuid>, HttpError> {
    query
        .get("registration")
        .map(|value| {
            Uuid::parse_str(value).map_err(|_error| {
                HttpError::new(StatusCode::BAD_REQUEST, "registration must be a UUID")
            })
        })
        .transpose()
}

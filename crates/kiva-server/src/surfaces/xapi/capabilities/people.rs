//! Explicit, authorized Person associations and reversible Statement metadata.

use std::collections::{BTreeMap, BTreeSet};

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use uuid::Uuid;

use super::{host, store::Record};
use crate::core::{
    response::{HttpError, new_etag},
    state::XapiHttpContext,
};
use crate::surfaces::xapi::{request::person_from_agent, service};

const IFIS: [&str; 4] = ["mbox", "mbox_sha1sum", "openid", "account"];

fn invalid(message: &str) -> HttpError {
    HttpError::new(StatusCode::BAD_REQUEST, message)
}

fn parse(bytes: &[u8]) -> Result<(Value, BTreeSet<String>), HttpError> {
    kiva_xapi::validate_unique_json_properties(bytes)
        .map_err(|_| invalid("Person must have unique JSON properties"))?;
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| invalid("Person must be valid JSON"))?;
    let object = value
        .as_object()
        .ok_or_else(|| invalid("Person must be an object"))?;
    if value["objectType"] != "Person"
        || object.keys().any(|key| {
            ![
                "objectType",
                "name",
                "mbox",
                "mbox_sha1sum",
                "openid",
                "account",
            ]
            .contains(&key.as_str())
        })
    {
        return Err(invalid(
            "Person must have objectType Person and supported Person properties",
        ));
    }
    if let Some(names) = value.get("name")
        && names
            .as_array()
            .is_none_or(|names| names.iter().any(|name| !name.is_string()))
    {
        return Err(invalid("Person.name must be an array of strings"));
    }
    let mut identities = BTreeSet::new();
    for key in IFIS {
        let Some(values) = value.get(key) else {
            continue;
        };
        for identifier in values
            .as_array()
            .ok_or_else(|| invalid("Person identifiers must be arrays"))?
        {
            let mut agent = json!({"objectType":"Agent"});
            agent[key] = identifier.clone();
            kiva_xapi::validate_agent(&agent).map_err(|_| invalid("invalid Person identifier"))?;
            let ifi = kiva_xapi::canonical_actor_ifi(&agent)
                .ok_or_else(|| invalid("missing Person identifier"))?;
            if !identities.insert(ifi) {
                return Err(invalid("duplicate Person identifier"));
            }
        }
    }
    if identities.is_empty() {
        return Err(invalid(
            "an association requires at least one Person identifier",
        ));
    }
    Ok((value, identities))
}

pub(crate) async fn person(state: &XapiHttpContext, agent: &Value) -> Result<Value, HttpError> {
    let requested = kiva_xapi::canonical_actor_ifi(agent)
        .ok_or_else(|| invalid("Agent must have an identifier"))?;
    let mut identities = BTreeSet::from([requested.clone()]);
    let mut result =
        person_from_agent(agent).ok_or_else(|| invalid("Agent must have an identifier"))?;
    let mut directory = false;
    for record in state.capabilities().store.list("person:").await?.values() {
        let (known, keys) = parse(&record.body)?;
        if keys.contains(&requested) {
            result = known;
            identities = keys;
            directory = true;
            break;
        }
    }
    let all = service::list_statements(state.xapi_services(), None)
        .await
        .map_err(|_| {
            HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Person metadata could not be read",
            )
        })?;
    let mut names = BTreeSet::new();
    if directory {
        names.extend(
            result
                .get("name")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned),
        );
    }
    let mut seen = false;
    for statement in &all {
        if state.capabilities().config.rollback_voided_metadata
            && statement["id"]
                .as_str()
                .and_then(|id| Uuid::parse_str(id).ok())
                .is_some_and(|id| kiva_xapi::statement_is_voided(&all, id))
        {
            continue;
        }
        let mut agents = Vec::new();
        collect_agents(statement, &mut agents);
        for known in agents {
            if kiva_xapi::canonical_actor_ifi(known).is_some_and(|ifi| identities.contains(&ifi)) {
                seen = true;
                if let Some(name) = known.get("name").and_then(Value::as_str) {
                    names.insert(name.to_owned());
                }
            }
        }
    }
    if directory || seen {
        if let Some(object) = result.as_object_mut() {
            object.remove("name");
        }
        if !names.is_empty() {
            result["name"] = json!(names);
        }
    }
    Ok(result)
}

fn agent_members<'a>(value: &'a Value, result: &mut Vec<&'a Value>) {
    if value["objectType"] != "Group"
        && (value.get("mbox").is_some()
            || value.get("mbox_sha1sum").is_some()
            || value.get("openid").is_some()
            || value.get("account").is_some())
    {
        result.push(value);
    }
    for member in value
        .get("member")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        agent_members(member, result);
    }
}

fn collect_agents<'a>(statement: &'a Value, result: &mut Vec<&'a Value>) {
    for path in [
        "/actor",
        "/authority",
        "/context/instructor",
        "/context/team",
    ] {
        if let Some(value) = statement.pointer(path) {
            agent_members(value, result);
        }
    }
    if let Some(object) = statement.get("object") {
        if object["objectType"] == "SubStatement" {
            collect_agents(object, result);
        } else if object["objectType"] == "Agent" || object["objectType"] == "Group" {
            agent_members(object, result);
        }
    }
    for (path, key) in [
        ("/context/contextAgents", "agent"),
        ("/context/contextGroups", "group"),
    ] {
        for relation in statement
            .pointer(path)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(value) = relation.get(key) {
                agent_members(value, result);
            }
        }
    }
}

fn enabled(state: &XapiHttpContext) -> Result<(), HttpError> {
    if !state.capabilities().config.person_enrichment {
        return Err(HttpError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "Person enrichment is not enabled",
        ));
    }
    Ok(())
}

pub(crate) async fn get(
    State(state): State<XapiHttpContext>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Response, HttpError> {
    state
        .authorize_xapi_read_request(&headers, &BTreeMap::new())
        .await?;
    enabled(&state)?;
    let record = state
        .capabilities()
        .store
        .get(&format!("person:{id}"))
        .await?
        .ok_or_else(|| HttpError::new(StatusCode::NOT_FOUND, "Person association was not found"))?;
    Ok(host::representation(record, &headers))
}

pub(crate) async fn put(
    State(state): State<XapiHttpContext>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, HttpError> {
    host::writer(&state, &headers, true).await?;
    enabled(&state)?;
    if body.len() > state.capabilities().config.metadata_max_bytes {
        return Err(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Person exceeds the configured metadata limit",
        ));
    }
    let (_, identities) = parse(&body)?;
    let record = state
        .capabilities()
        .store
        .mutate("person:", move |records| {
            let key = format!("person:{id}");
            host::precondition(&headers, records.get(&key))?;
            for (other_key, record) in records
                .iter()
                .filter(|(k, _)| k.starts_with("person:") && **k != key)
            {
                let (_, other) = parse(&record.body)?;
                if !identities.is_disjoint(&other) {
                    let _ = other_key;
                    return Err(HttpError::new(
                        StatusCode::CONFLICT,
                        "an identifier already belongs to another Person association",
                    ));
                }
            }
            let record = Record {
                body: body.to_vec(),
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
    host::writer(&state, &headers, true).await?;
    enabled(&state)?;
    state
        .capabilities()
        .store
        .mutate("person:", move |records| {
            let key = format!("person:{id}");
            host::precondition(&headers, records.get(&key))?;
            records.remove(&key);
            Ok(())
        })
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

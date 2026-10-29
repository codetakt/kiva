//! xAPI HTTP response mapping.

use kiva_xapi_openapi_contract as xapi_contract;
use serde::Serialize;
use serde_json::{Value, json};

use crate::core::config::XAPI_VERSION;

pub(crate) fn about_response() -> Result<Value, serde_json::Error> {
    serde_json::to_value(xapi_contract::About {
        version: vec![XAPI_VERSION.to_owned()],
        extensions: Some(json!(AboutExtensions {
            name: "Kiva",
            standard: "IEEE/ISO/IEC 39274-1-1-2025",
            status: "validated xAPI LRS HTTP boundary",
            resources: kiva_xapi::REQUIRED_LRS_RESOURCES,
        })),
    })
}

pub(crate) fn activity_response(
    activity_id: &str,
    definition: Option<Value>,
) -> Result<Value, serde_json::Error> {
    let mut body = json!({"id":activity_id,"objectType":"Activity"});
    if let Some(definition) = definition {
        body["definition"] = definition;
    }
    let checked = super::lossless::contract_view(&json!({"object":body.clone()}));
    serde_json::from_value::<xapi_contract::Activity>(checked["object"].clone())?;
    Ok(body)
}

pub(crate) fn activity_definition_sources<'a>(
    statements: &'a [Value],
    activity_id: &str,
) -> Vec<(usize, String, &'a serde_json::Map<String, Value>)> {
    let mut sources = Vec::new();
    for (index, statement) in statements.iter().enumerate() {
        collect_activity_definitions(statement, "", activity_id, index, &mut sources);
    }
    sources
}

fn collect_activity_definitions<'a>(
    statement: &'a Value,
    path: &str,
    id: &str,
    index: usize,
    sources: &mut Vec<(usize, String, &'a serde_json::Map<String, Value>)>,
) {
    if let Some(object) = statement.get("object") {
        if object["objectType"].as_str() == Some("SubStatement") {
            collect_activity_definitions(object, &format!("{path}/object"), id, index, sources);
        } else {
            push_activity_definition(object, &format!("{path}/object"), id, index, sources);
        }
    }
    for role in ["parent", "grouping", "category", "other"] {
        let location = format!("/context/contextActivities/{role}");
        if let Some(activity) = statement.pointer(&location) {
            let location = format!("{path}{location}");
            if let Some(array) = activity.as_array() {
                for (offset, activity) in array.iter().enumerate() {
                    push_activity_definition(
                        activity,
                        &format!("{location}/{offset}"),
                        id,
                        index,
                        sources,
                    );
                }
            } else {
                push_activity_definition(activity, &location, id, index, sources);
            }
        }
    }
}

fn push_activity_definition<'a>(
    activity: &'a Value,
    path: &str,
    id: &str,
    index: usize,
    sources: &mut Vec<(usize, String, &'a serde_json::Map<String, Value>)>,
) {
    if activity["id"].as_str() == Some(id)
        && activity["objectType"]
            .as_str()
            .is_none_or(|kind| kind == "Activity")
        && let Some(definition) = activity.get("definition").and_then(Value::as_object)
    {
        sources.push((index, format!("{path}/definition"), definition));
    }
}

pub(crate) fn merged_activity_definition(statements: &[Value], activity_id: &str) -> Option<Value> {
    let mut merged = serde_json::Map::new();
    for (_, _, definition) in activity_definition_sources(statements, activity_id) {
        merge_json_objects(&mut merged, definition);
    }
    (!merged.is_empty()).then_some(Value::Object(merged))
}

fn merge_json_objects(
    target: &mut serde_json::Map<String, Value>,
    source: &serde_json::Map<String, Value>,
) {
    for (name, value) in source {
        match (target.get_mut(name), value) {
            (Some(Value::Object(target)), Value::Object(source)) => {
                if name == "extensions" {
                    // Each IRI's owner value is indivisible. Merging inside it
                    // would synthesize data that no producer ever supplied.
                    for (key, value) in source {
                        target.entry(key.clone()).or_insert_with(|| value.clone());
                    }
                } else {
                    merge_json_objects(target, source);
                }
            }
            (None, value) => {
                target.insert(name.clone(), value.clone());
            }
            (Some(_), _) => {}
        }
    }
}

pub(crate) fn person_response(person: Value) -> Result<Value, serde_json::Error> {
    serde_json::from_value::<xapi_contract::Person>(person.clone())?;
    Ok(person)
}

pub(crate) fn statement_response(statement: Value) -> Result<Value, serde_json::Error> {
    let mut statement = statement;
    project_context_activities(&mut statement);
    serde_json::from_value::<xapi_contract::Statement>(super::lossless::contract_view(&statement))?;
    Ok(statement)
}

pub(crate) fn statement_result_response(body: Value) -> Result<Value, serde_json::Error> {
    let mut body = body;
    if let Some(statements) = body.get_mut("statements").and_then(Value::as_array_mut) {
        for statement in statements {
            project_context_activities(statement);
        }
    }
    let mut checked = body.clone();
    if let Some(statements) = checked.get_mut("statements") {
        *statements = super::lossless::contract_view(statements);
    }
    serde_json::from_value::<xapi_contract::StatementResult>(checked)?;
    Ok(body)
}

fn project_context_activities(statement: &mut Value) {
    let Some(statement) = statement.as_object_mut() else {
        return;
    };
    if let Some(context_activities) = statement
        .get_mut("context")
        .and_then(Value::as_object_mut)
        .and_then(|context| context.get_mut("contextActivities"))
        .and_then(Value::as_object_mut)
    {
        for key in ["parent", "grouping", "category", "other"] {
            if let Some(activity) = context_activities.get_mut(key)
                && activity.is_object()
            {
                *activity = Value::Array(vec![activity.clone()]);
            }
        }
    }
    if statement
        .get("object")
        .and_then(Value::as_object)
        .and_then(|object| object.get("objectType"))
        .and_then(Value::as_str)
        == Some("SubStatement")
        && let Some(substatement) = statement.get_mut("object")
    {
        project_context_activities(substatement);
    }
}

#[derive(Serialize)]
struct AboutExtensions<'a> {
    name: &'a str,
    standard: &'a str,
    status: &'a str,
    resources: &'a [&'a str],
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{merged_activity_definition, project_context_activities};

    #[test]
    fn response_projection_normalizes_top_level_and_substatement_context_activities() {
        let mut statement = json!({
            "context": {"contextActivities": {"parent": {"id": "https://example.org/top"}}},
            "object": {
                "objectType": "SubStatement",
                "context": {"contextActivities": {"other": {"id": "https://example.org/sub"}}}
            }
        });

        project_context_activities(&mut statement);

        assert!(
            statement
                .pointer("/context/contextActivities/parent")
                .is_some_and(Value::is_array)
        );
        assert!(
            statement
                .pointer("/object/context/contextActivities/other")
                .is_some_and(Value::is_array)
        );
    }

    #[test]
    fn activity_definition_projection_merges_available_language_maps() {
        let statements = [
            json!({"object": {"id": "https://example.org/a", "definition": {"name": {"en": "A"}}}}),
            json!({"object": {"objectType": "Activity", "id": "https://example.org/a", "definition": {"name": {"ja": "A-ja"}}}}),
        ];
        let definition = merged_activity_definition(&statements, "https://example.org/a")
            .expect("definition should be available");
        assert_eq!(definition.pointer("/name/en"), Some(&json!("A")));
        assert_eq!(definition.pointer("/name/ja"), Some(&json!("A-ja")));
    }
}

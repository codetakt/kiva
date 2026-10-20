//! xAPI HTTP response mapping.

use std::collections::BTreeMap;

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
    let definition = definition.map(serde_json::from_value).transpose()?;
    serde_json::to_value(xapi_contract::Activity {
        id: xapi_contract::Iri(activity_id.to_owned()),
        object_type: Some("Activity".to_owned()),
        definition,
        extensions: BTreeMap::new(),
    })
}

pub(crate) fn merged_activity_definition(statements: &[Value], activity_id: &str) -> Option<Value> {
    let mut merged = serde_json::Map::new();
    for definition in statements.iter().filter_map(|statement| {
        let object = statement.get("object")?.as_object()?;
        if object.get("id").and_then(Value::as_str) != Some(activity_id)
            || object
                .get("objectType")
                .and_then(Value::as_str)
                .is_some_and(|object_type| object_type != "Activity")
        {
            return None;
        }
        object.get("definition")?.as_object()
    }) {
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
                merge_json_objects(target, source);
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
    serde_json::from_value::<xapi_contract::Statement>(statement.clone())?;
    Ok(statement)
}

pub(crate) fn statement_result_response(body: Value) -> Result<Value, serde_json::Error> {
    let mut body = body;
    if let Some(statements) = body.get_mut("statements").and_then(Value::as_array_mut) {
        for statement in statements {
            project_context_activities(statement);
        }
    }
    serde_json::from_value::<xapi_contract::StatementResult>(body.clone())?;
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

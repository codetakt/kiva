#![allow(clippy::wildcard_imports)]

use super::*;

/// Parses an `Accept-Language` header value into preferred language ranges.
#[must_use]
pub fn accepted_language_ranges(header: Option<&str>) -> Vec<String> {
    header
        .map(|value| {
            value
                .split(',')
                .filter_map(|language_range| {
                    let language = language_range
                        .split_once(';')
                        .map_or(language_range, |(language, _quality)| language)
                        .trim();
                    (!language.is_empty() && language != "*").then(|| language.to_owned())
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Retains preferred entries in xAPI language maps for a canonical response.
#[must_use]
pub fn canonicalize_statement_language_maps(mut statement: Value, languages: &[String]) -> Value {
    canonicalize_statement_value(&mut statement, languages);
    statement
}

fn canonicalize_statement_value(statement: &mut Value, languages: &[String]) {
    retain_preferred_language_map(statement, "/verb/display", languages);
    retain_preferred_language_map(statement, "/object/definition/name", languages);
    retain_preferred_language_map(statement, "/object/definition/description", languages);
    canonicalize_context_activity_language_maps(statement, languages);
    for component_list in ["choices", "scale", "source", "target", "steps"] {
        let pointer = format!("/object/definition/{component_list}");
        if let Some(components) = statement
            .pointer_mut(&pointer)
            .and_then(Value::as_array_mut)
        {
            for component in components {
                retain_preferred_language_map(component, "/description", languages);
            }
        }
    }
    if let Some(attachments) = statement
        .get_mut("attachments")
        .and_then(Value::as_array_mut)
    {
        for attachment in attachments {
            retain_preferred_language_map(attachment, "/display", languages);
            retain_preferred_language_map(attachment, "/description", languages);
        }
    }
    if statement
        .pointer("/object/objectType")
        .and_then(Value::as_str)
        == Some("SubStatement")
        && let Some(substatement) = statement.get_mut("object")
    {
        canonicalize_statement_value(substatement, languages);
    }
}

fn canonicalize_context_activity_language_maps(statement: &mut Value, languages: &[String]) {
    let Some(context_activities) = statement
        .pointer_mut("/context/contextActivities")
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    for activities in context_activities.values_mut() {
        match activities {
            Value::Array(activities) => {
                for activity in activities {
                    canonicalize_activity_language_maps(activity, languages);
                }
            }
            Value::Object(_) => canonicalize_activity_language_maps(activities, languages),
            _ => {}
        }
    }
}

fn canonicalize_activity_language_maps(activity: &mut Value, languages: &[String]) {
    retain_preferred_language_map(activity, "/definition/name", languages);
    retain_preferred_language_map(activity, "/definition/description", languages);
    for component_list in ["choices", "scale", "source", "target", "steps"] {
        let pointer = format!("/definition/{component_list}");
        if let Some(components) = activity.pointer_mut(&pointer).and_then(Value::as_array_mut) {
            for component in components {
                retain_preferred_language_map(component, "/description", languages);
            }
        }
    }
}

/// Projects a Statement into the `ids` response format without changing stored JSON.
#[must_use]
pub fn statement_ids_format(mut statement: Value) -> Value {
    project_statement_ids(&mut statement);
    statement
}

fn project_statement_ids(statement: &mut Value) {
    project_actor_ids(statement.get_mut("actor"));
    project_actor_ids(statement.get_mut("authority"));
    project_context_ids(statement.get_mut("context"));
    if let Some(verb) = statement.get_mut("verb").and_then(Value::as_object_mut) {
        verb.retain(|name, _value| name == "id");
    }
    let Some(object) = statement.get_mut("object") else {
        return;
    };
    match object.get("objectType").and_then(Value::as_str) {
        Some("Agent" | "Group") => project_actor_ids(Some(object)),
        Some("StatementRef") => {
            if let Some(object) = object.as_object_mut() {
                object.retain(|name, _value| matches!(name.as_str(), "objectType" | "id"));
            }
        }
        Some("SubStatement") => project_statement_ids(object),
        _ => {
            if let Some(object) = object.as_object_mut() {
                object.retain(|name, _value| name == "id");
            }
        }
    }
}

fn project_context_ids(context: Option<&mut Value>) {
    let Some(context) = context.and_then(Value::as_object_mut) else {
        return;
    };
    for name in ["instructor", "team"] {
        project_actor_ids(context.get_mut(name));
    }
    for name in ["contextAgents", "contextGroups"] {
        if let Some(actors) = context.get_mut(name).and_then(Value::as_array_mut) {
            for actor in actors {
                project_actor_ids(Some(actor));
            }
        }
    }
    if let Some(context_activities) = context
        .get_mut("contextActivities")
        .and_then(Value::as_object_mut)
    {
        for activities in context_activities.values_mut() {
            match activities {
                Value::Array(activities) => {
                    for activity in activities {
                        project_activity_ids(activity);
                    }
                }
                Value::Object(_) => project_activity_ids(activities),
                _ => {}
            }
        }
    }
}

fn project_activity_ids(activity: &mut Value) {
    if let Some(activity) = activity.as_object_mut() {
        activity.retain(|name, _value| matches!(name.as_str(), "objectType" | "id"));
    }
}

fn project_actor_ids(actor: Option<&mut Value>) {
    let Some(actor) = actor.and_then(Value::as_object_mut) else {
        return;
    };
    if let Some(members) = actor.get_mut("member").and_then(Value::as_array_mut) {
        for member in members {
            project_actor_ids(Some(member));
        }
    }
    actor.retain(|name, _value| {
        matches!(
            name.as_str(),
            "objectType" | "mbox" | "mbox_sha1sum" | "openid" | "account" | "member"
        )
    });
}
fn retain_preferred_language_map(statement: &mut Value, pointer: &str, languages: &[String]) {
    let Some(language_map) = statement
        .pointer_mut(pointer)
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    let Some((language, text)) = preferred_language_entry(language_map, languages) else {
        return;
    };

    let mut retained = Map::new();
    retained.insert(language, text);
    *language_map = retained;
}

fn preferred_language_entry(
    language_map: &Map<String, Value>,
    languages: &[String],
) -> Option<(String, Value)> {
    for language in languages {
        if let Some((key, value)) = language_map
            .iter()
            .find(|(key, _value)| key.eq_ignore_ascii_case(language))
        {
            return Some((key.clone(), value.clone()));
        }
    }
    language_map
        .iter()
        .next()
        .map(|(key, value)| (key.clone(), value.clone()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{canonicalize_statement_language_maps, statement_ids_format};

    #[test]
    fn canonicalizes_context_activity_language_maps_without_changing_other_fields() {
        let statement = json!({
            "context": {
                "contextActivities": {
                    "category": {
                        "id": "https://example.org/categories/one",
                        "definition": {
                            "name": {"en-US": "US name", "en-GB": "UK name"},
                            "description": {"en-US": "US text", "en-GB": "UK text"}
                        }
                    }
                }
            }
        });

        let projected = canonicalize_statement_language_maps(statement, &["en-GB".to_owned()]);

        assert_eq!(
            projected.pointer("/context/contextActivities/category/id"),
            Some(&json!("https://example.org/categories/one"))
        );
        assert_eq!(
            projected.pointer("/context/contextActivities/category/definition/name"),
            Some(&json!({"en-GB": "UK name"}))
        );
        assert_eq!(
            projected.pointer("/context/contextActivities/category/definition/description"),
            Some(&json!({"en-GB": "UK text"}))
        );
    }

    #[test]
    fn ids_format_projects_nested_context_identifiers() {
        let statement = json!({
            "actor": {"objectType": "Agent", "name": "Learner", "mbox": "mailto:learner@example.org"},
            "verb": {"id": "https://example.org/verbs/experienced", "display": {"en": "experienced"}},
            "object": {"id": "https://example.org/activity", "definition": {"name": {"en": "Activity"}}},
            "context": {
                "instructor": {"objectType": "Agent", "name": "Instructor", "mbox": "mailto:instructor@example.org"},
                "contextAgents": [{"objectType": "Agent", "name": "Observer", "mbox": "mailto:observer@example.org"}],
                "contextActivities": {
                    "parent": [{"id": "https://example.org/parent", "definition": {"name": {"en": "Parent"}}}]
                }
            }
        });

        let projected = statement_ids_format(statement);

        assert!(projected.pointer("/actor/name").is_none());
        assert!(projected.pointer("/verb/display").is_none());
        assert!(projected.pointer("/object/definition").is_none());
        assert!(projected.pointer("/context/instructor/name").is_none());
        assert!(projected.pointer("/context/contextAgents/0/name").is_none());
        assert!(
            projected
                .pointer("/context/contextActivities/parent/0/definition")
                .is_none()
        );
    }
}

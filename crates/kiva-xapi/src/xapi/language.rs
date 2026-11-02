#![allow(clippy::wildcard_imports)]

use super::*;

/// An HTTP language range with its original quality and relative order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguagePreference {
    range: String,
    quality: u16,
}

/// Parses language ranges, retaining exclusions and the sender's order.
#[must_use]
pub fn accepted_language_ranges(header: Option<&str>) -> Vec<LanguagePreference> {
    header
        .unwrap_or_default()
        .split(',')
        .filter_map(|entry| {
            let mut fields = entry.split(';');
            let range = fields.next()?.trim();
            if range != "*"
                && !range.split('-').enumerate().all(|(index, part)| {
                    (1..=8).contains(&part.len())
                        && part.bytes().all(|byte| {
                            if index == 0 {
                                byte.is_ascii_alphabetic()
                            } else {
                                byte.is_ascii_alphanumeric()
                            }
                        })
                })
            {
                return None;
            }
            let weight = match fields.next() {
                None => 1000,
                Some(parameter) => {
                    let (name, value) = parameter.trim().split_once('=')?;
                    // RFC 2616 permits SP/HTAB between tokens and separators.
                    let name = name.trim_matches([' ', '\t']);
                    let value = value.trim_matches([' ', '\t']);
                    if !name.eq_ignore_ascii_case("q") {
                        return None;
                    }
                    language_quality(value)?
                }
            };
            if fields.next().is_some() {
                return None;
            }
            Some(LanguagePreference {
                quality: weight,
                range: range.to_owned(),
            })
        })
        .collect()
}

fn language_quality(value: &str) -> Option<u16> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if fraction.len() > 3 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    match whole {
        "1" if fraction.bytes().all(|byte| byte == b'0') => Some(1000),
        "0" => {
            let digits = format!("{fraction:0<3}");
            digits.parse().ok()
        }
        _ => None,
    }
}

/// Retains preferred entries in xAPI language maps for a canonical response.
#[must_use]
pub fn canonicalize_statement_language_maps(
    mut statement: Value,
    languages: &[LanguagePreference],
) -> Value {
    canonicalize_statement_value(&mut statement, languages);
    statement
}

fn canonicalize_statement_value(statement: &mut Value, languages: &[LanguagePreference]) {
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

fn canonicalize_context_activity_language_maps(
    statement: &mut Value,
    languages: &[LanguagePreference],
) {
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

fn canonicalize_activity_language_maps(activity: &mut Value, languages: &[LanguagePreference]) {
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
    for (name, actor_property) in [("contextAgents", "agent"), ("contextGroups", "group")] {
        if let Some(actors) = context.get_mut(name).and_then(Value::as_array_mut) {
            for relation in actors {
                project_actor_ids(relation.get_mut(actor_property));
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
    let identified = ["mbox", "mbox_sha1sum", "openid", "account"]
        .iter()
        .any(|name| actor.contains_key(*name));
    if identified {
        actor.remove("member");
    }
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
fn retain_preferred_language_map(
    statement: &mut Value,
    pointer: &str,
    languages: &[LanguagePreference],
) {
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
    languages: &[LanguagePreference],
) -> Option<(String, Value)> {
    // The longest matching range supplies a tag's quality, even when its
    // quality is zero. Only then compare available tags by quality and order.
    language_map.iter().filter_map(|(key, value)| {
        let preference = languages.iter().enumerate().filter(|(_, preference)| {
            let range = &preference.range;
            range == "*" || key.eq_ignore_ascii_case(range)
                || (key.len() > range.len()
                    && key.get(..range.len()).is_some_and(|prefix| prefix.eq_ignore_ascii_case(range))
                    && key.as_bytes().get(range.len()) == Some(&b'-'))
        }).max_by_key(|(index, preference)| (
            if preference.range == "*" { 0 } else { preference.range.len() },
            std::cmp::Reverse(*index),
        ));
        let (index, preference) = preference?;
        (preference.quality > 0).then_some(((preference.quality, std::cmp::Reverse(index), std::cmp::Reverse(key)), (key, value)))
    }).max_by_key(|(rank, _)| *rank).map(|(_, entry)| entry)
        // Kiva's adopted fallback applies when no available language is acceptable.
        .or_else(|| language_map.iter().next())
        .map(|(key, value)| (key.clone(), value.clone()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        accepted_language_ranges, canonicalize_statement_language_maps, statement_ids_format,
    };

    #[test]
    fn negotiates_weighted_ranges_in_every_nested_language_map() {
        let maps = json!({"en": "English", "ja-JP": "Japanese"});
        let statement = json!({
            "verb": {"display": maps},
            "object": {"objectType": "SubStatement", "object": {"definition": {
                "name": maps, "choices": [{"description": maps}]
            }}},
            "attachments": [{"display": maps}]
        });
        for header in [
            "en;q=0.1, ja-JP;q=0.9",
            "ja;q=1, en;q=0.5",
            "en;q=0, JA;q=1",
            "en; q = 0, JA; q = 1",
            "en;q\t=\t0, ja-JP; Q\t=\t1",
            "*;q = 0, ja;q = 1",
        ] {
            let projected = canonicalize_statement_language_maps(
                statement.clone(),
                &accepted_language_ranges(Some(header)),
            );
            for pointer in [
                "/verb/display",
                "/object/object/definition/name",
                "/object/object/definition/choices/0/description",
                "/attachments/0/display",
            ] {
                assert_eq!(
                    projected.pointer(pointer),
                    Some(&json!({"ja-JP": "Japanese"})),
                    "{header} {pointer}"
                );
            }
        }
        assert_eq!(
            accepted_language_ranges(Some("de;q=0.6, fr;q=0.6, *;q=0.2, en;q=0, es;q=2")),
            vec![
                super::LanguagePreference {
                    range: "de".into(),
                    quality: 600
                },
                super::LanguagePreference {
                    range: "fr".into(),
                    quality: 600
                },
                super::LanguagePreference {
                    range: "*".into(),
                    quality: 200
                },
                super::LanguagePreference {
                    range: "en".into(),
                    quality: 0
                },
            ]
        );
    }

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

        let projected = canonicalize_statement_language_maps(
            statement,
            &accepted_language_ranges(Some("en-GB")),
        );

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
                "contextAgents": [{"objectType": "contextAgent", "agent": {"name": "Observer", "mbox": "mailto:observer@example.org"}, "relevantTypes": ["https://example.org/observer"]}],
                "contextGroups": [{"objectType": "contextGroup", "group": {"objectType": "Group", "mbox": "mailto:group@example.org", "member": [{"mbox": "mailto:member@example.org"}]}}],
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
        assert!(
            projected
                .pointer("/context/contextAgents/0/agent/name")
                .is_none()
        );
        assert_eq!(
            projected.pointer("/context/contextAgents/0/agent/mbox"),
            Some(&json!("mailto:observer@example.org"))
        );
        assert_eq!(
            projected.pointer("/context/contextAgents/0/relevantTypes"),
            Some(&json!(["https://example.org/observer"]))
        );
        assert_eq!(
            projected.pointer("/context/contextGroups/0/group/mbox"),
            Some(&json!("mailto:group@example.org"))
        );
        assert!(
            projected
                .pointer("/context/contextGroups/0/group/member")
                .is_none()
        );
        assert!(
            projected
                .pointer("/context/contextActivities/parent/0/definition")
                .is_none()
        );
    }
}

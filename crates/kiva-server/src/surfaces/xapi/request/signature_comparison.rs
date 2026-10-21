//! Logical equivalence for signed Statement payloads.

use serde_json::Value;

pub(super) fn canonical_signed_statement(statement: &Value) -> Value {
    let mut statement = statement.clone();
    normalize_signed_statement(&mut statement);
    statement
}

// Apply the Statement immutability exclusions only at their xAPI locations.
// Extension payloads and other immutable values must remain significant.
fn normalize_signed_statement(statement: &mut Value) {
    let Some(object) = statement.as_object_mut() else {
        return;
    };
    for field in [
        "id",
        "version",
        "attachments",
        "stored",
        "timestamp",
        "authority",
    ] {
        object.remove(field);
    }
    if let Some(verb) = object.get_mut("verb").and_then(Value::as_object_mut) {
        verb.remove("display");
    }
    if let Some(actor) = object.get_mut("actor") {
        normalize_signed_actor(actor);
    }
    if let Some(target) = object.get_mut("object") {
        match target.get("objectType").and_then(Value::as_str) {
            Some("SubStatement") => normalize_signed_statement(target),
            Some("Group") => normalize_signed_actor(target),
            None | Some("Activity") => remove_activity_definition(target),
            _ => {}
        }
    }
    if let Some(context) = object.get_mut("context").and_then(Value::as_object_mut) {
        for field in ["instructor", "team"] {
            if let Some(actor) = context.get_mut(field) {
                normalize_signed_actor(actor);
            }
        }
        for (field, property) in [("contextAgents", "agent"), ("contextGroups", "group")] {
            if let Some(relations) = context.get_mut(field).and_then(Value::as_array_mut) {
                for relation in relations {
                    if let Some(actor) = relation.get_mut(property) {
                        normalize_signed_actor(actor);
                    }
                }
            }
        }
        if let Some(activities) = context
            .get_mut("contextActivities")
            .and_then(Value::as_object_mut)
        {
            for value in activities.values_mut() {
                if let Some(values) = value.as_array_mut() {
                    for activity in values {
                        remove_activity_definition(activity);
                    }
                } else {
                    remove_activity_definition(value);
                }
            }
        }
    }
}

fn normalize_signed_actor(actor: &mut Value) {
    if actor.get("objectType").and_then(Value::as_str) == Some("Group")
        && let Some(members) = actor.get_mut("member").and_then(Value::as_array_mut)
    {
        members.sort_by_cached_key(Value::to_string);
    }
}

fn remove_activity_definition(activity: &mut Value) {
    if let Some(activity) = activity.as_object_mut() {
        activity.remove("definition");
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::canonical_signed_statement;
    use kiva_xapi::statements_equivalent;

    #[test]
    fn signed_statement_comparison_ignores_sub_centisecond_duration_precision() {
        let received = canonical_signed_statement(&json!({
            "result": {"duration": "P1DT12H36M0.1237S"},
            "attachments": []
        }));
        let signed = canonical_signed_statement(&json!({
            "result": {"duration": "P1DT12H36M0.12S"}
        }));

        assert!(statements_equivalent(&received, &signed));
    }

    #[test]
    fn signed_statement_exclusions_preserve_immutable_and_extension_data() {
        let original = json!({
            "id": "old", "version": "2.0.0",
            "actor": {"objectType": "Group", "member": [{"mbox": "mailto:a@example.org"}, {"mbox": "mailto:b@example.org"}]},
            "verb": {"id": "https://example.org/v", "display": {"en": "old"}},
            "object": {"id": "https://example.org/a", "definition": {"name": {"en": "old"}}},
            "context": {"extensions": {"https://example.org/e": {"id": "kept", "definition": "kept"}}}
        });
        let mut changed = original.clone();
        changed["id"] = json!("new");
        changed["version"] = json!("1.0.3");
        changed["actor"]["member"] =
            json!([{"mbox": "mailto:b@example.org"}, {"mbox": "mailto:a@example.org"}]);
        changed["verb"]["display"] = json!({"en": "new"});
        changed["object"]["definition"] = json!({"name": {"en": "new"}});
        assert_eq!(
            canonical_signed_statement(&original),
            canonical_signed_statement(&changed)
        );
        for pointer in [
            "/verb/id",
            "/object/id",
            "/actor/member/0/mbox",
            "/context/extensions/https:~1~1example.org~1e/id",
        ] {
            let mut different = changed.clone();
            if let Some(value) = different.pointer_mut(pointer) {
                *value = json!("different");
            }
            assert_ne!(
                canonical_signed_statement(&original),
                canonical_signed_statement(&different),
                "{pointer}"
            );
        }
    }
}

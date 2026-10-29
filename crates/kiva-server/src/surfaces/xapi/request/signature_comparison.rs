//! Logical equivalence for signed Statement payloads.

use serde_json::Value;

pub(super) fn statements_logically_equivalent(left: &Value, right: &Value) -> bool {
    let equivalent = kiva_xapi::statements_equivalent(
        &canonical_signed_statement(left),
        &canonical_signed_statement(right),
    );
    #[cfg(test)]
    LOGICAL_OBSERVATIONS.with(|values| values.borrow_mut().push(serde_json::json!({
        "left":left,"right":right,"equivalent":equivalent,
        "adapter":"request::attachments::validate_signed_statement_attachment -> signature_comparison::statements_logically_equivalent"
    })));
    equivalent
}

#[cfg(test)]
std::thread_local! {
    // This records the production comparator, after original-byte signature
    // verification. Current-thread native HTTP tests consume their own calls.
    static LOGICAL_OBSERVATIONS: std::cell::RefCell<Vec<Value>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
pub(crate) fn take_logical_observations() -> Vec<Value> {
    LOGICAL_OBSERVATIONS.with(|values| std::mem::take(&mut *values.borrow_mut()))
}

fn canonical_signed_statement(statement: &Value) -> Value {
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
    // IEEE 4.2 / OPEN-EQUALITY-01 defines the invariant core as Actor, Verb,
    // Object and Duration. Context and other Result fields are outside this
    // relation. Replay preservation still compares the entire stored object in
    // statement_queries; these exclusions never authorize a storage mutation.
    object.retain(|field, _| {
        matches!(
            field.as_str(),
            "objectType" | "actor" | "verb" | "object" | "result"
        )
    });
    if let Some(result) = object.get_mut("result").and_then(Value::as_object_mut) {
        result.retain(|field, _| field == "duration");
    }
    if object
        .get("result")
        .and_then(Value::as_object)
        .is_some_and(serde_json::Map::is_empty)
    {
        object.remove("result");
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
            Some("Agent" | "Group") => normalize_signed_actor(target),
            None | Some("Activity") => remove_activity_definition(target),
            _ => {}
        }
    }
}

fn normalize_signed_actor(actor: &mut Value) {
    if let Some(actor) = actor.as_object_mut() {
        actor
            .entry("objectType")
            .or_insert_with(|| Value::String("Agent".to_owned()));
    }
    if actor.get("objectType").and_then(Value::as_str) == Some("Group")
        && let Some(members) = actor.get_mut("member").and_then(Value::as_array_mut)
    {
        for member in members.iter_mut() {
            normalize_signed_actor(member);
        }
        members.sort_by_cached_key(Value::to_string);
    }
}

fn remove_activity_definition(activity: &mut Value) {
    if let Some(activity) = activity.as_object_mut() {
        activity.remove("definition");
        activity
            .entry("objectType")
            .or_insert_with(|| Value::String("Activity".to_owned()));
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{canonical_signed_statement, statements_logically_equivalent};
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
        for pointer in ["/verb/id", "/object/id", "/actor/member/0/mbox"] {
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
        let mut context_changed = original.clone();
        context_changed["context"] =
            json!({"extensions":{"https://example.org/e":{"id":"different"}}});
        context_changed["result"] = json!({"score":{"raw":3},"response":"different"});
        assert!(statements_logically_equivalent(&original, &context_changed));
    }

    #[test]
    fn actual_signature_comparator_obeys_the_issued_core_and_exclusions() {
        let original = json!({
            "id":"00000000-0000-4000-8000-000000000001",
            "actor":{"objectType":"Group","mbox":"mailto:group@example.org","member":[{"mbox":"mailto:a@example.org"},{"mbox":"mailto:b@example.org"}]},
            "verb":{"id":"https://example.org/Verb","display":{"en":"original"}},
            "object":{"id":"https://example.org/Activity","definition":{"name":{"en":"original"}}},
            "result":{"duration":"PT1.011S"}
        });
        for (pointer, replacement) in [
            ("/id", json!("00000000-0000-4000-8000-000000000002")),
            ("/verb/display", json!({"ja":"変更"})),
            (
                "/object/definition",
                json!({"extensions":{"https://example.org/e":{"duration":"opaque"}}}),
            ),
            (
                "/actor/member",
                json!([{"mbox":"mailto:b@example.org"},{"mbox":"mailto:a@example.org"}]),
            ),
            ("/result/duration", json!("PT1.019S")),
            ("/result/duration", json!("PT1.01S")),
        ] {
            let mut changed = original.clone();
            *changed
                .pointer_mut(pointer)
                .expect("comparison fixture pointer") = replacement;
            assert!(
                statements_logically_equivalent(&original, &changed),
                "{pointer}"
            );
        }
        for (field, value) in [
            ("stored", json!("2020-01-01T00:00:01Z")),
            ("timestamp", json!("2020-01-01T00:00:00Z")),
            ("authority", json!({"mbox":"mailto:authority@example.org"})),
            ("version", json!("1.0.3")),
            ("attachments", json!([])),
            ("context", json!({"language":"ja"})),
        ] {
            let mut changed = original.clone();
            changed[field] = value;
            assert!(
                kiva_xapi::validate_statement_shape(&changed).is_ok(),
                "{field}"
            );
            assert!(
                kiva_xapi::validate_statement_actors(&changed).is_ok(),
                "{field}"
            );
            assert!(
                statements_logically_equivalent(&original, &changed),
                "{field}"
            );
        }
        for (pointer, replacement) in [
            ("/actor/mbox", json!("mailto:Group@example.org")),
            ("/actor/member", json!([{"mbox":"mailto:a@example.org"}])),
            ("/verb/id", json!("https://example.org/verb")),
            ("/object/id", json!("https://example.org/activity")),
            ("/result/duration", json!("PT1.02S")),
        ] {
            let mut changed = original.clone();
            *changed
                .pointer_mut(pointer)
                .expect("comparison fixture pointer") = replacement;
            assert!(
                !statements_logically_equivalent(&original, &changed),
                "{pointer}"
            );
        }
    }
}

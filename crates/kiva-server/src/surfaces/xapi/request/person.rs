//! xAPI Agent-to-Person response projection helpers.

use serde_json::{Map, Value, json};

pub(crate) fn person_from_agent(agent: &Value) -> Option<Value> {
    let agent = agent.as_object()?;
    let mut person = Map::new();
    person.insert("objectType".to_owned(), Value::String("Person".to_owned()));

    insert_person_string_array(&mut person, agent, "name");
    insert_person_string_array(&mut person, agent, "mbox");
    insert_person_string_array(&mut person, agent, "mbox_sha1sum");
    insert_person_string_array(&mut person, agent, "openid");
    insert_person_account_array(&mut person, agent);

    if person.len() == 1 {
        None
    } else {
        Some(Value::Object(person))
    }
}

fn insert_person_string_array(
    person: &mut Map<String, Value>,
    agent: &Map<String, Value>,
    key: &str,
) {
    if let Some(value) = agent.get(key).and_then(Value::as_str) {
        person.insert(key.to_owned(), json!([value]));
    }
}

fn insert_person_account_array(person: &mut Map<String, Value>, agent: &Map<String, Value>) {
    if let Some(account) = agent.get("account").filter(|value| value.is_object()) {
        person.insert("account".to_owned(), json!([account]));
    }
}

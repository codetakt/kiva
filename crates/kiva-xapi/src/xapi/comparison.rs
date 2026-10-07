#![allow(clippy::wildcard_imports)]

use super::*;

/// Returns true when two Statement JSON values are equivalent after removing
/// LRS-managed fields and normalizing duration precision used by conformance
/// fixtures.
#[must_use]
pub fn statements_equivalent(left: &Value, right: &Value) -> bool {
    canonical_statement_for_comparison(left) == canonical_statement_for_comparison(right)
}

fn canonical_statement_for_comparison(statement: &Value) -> Value {
    let mut statement = statement.clone();
    let Some(object) = statement.as_object_mut() else {
        return statement;
    };

    object.remove("stored");
    object.remove("timestamp");
    object.remove("authority");
    normalize_result_duration_for_comparison(object);
    statement
}

fn normalize_result_duration_for_comparison(statement: &mut Map<String, Value>) {
    let Some(duration) = statement
        .get("result")
        .and_then(Value::as_object)
        .and_then(|result| result.get("duration"))
        .and_then(Value::as_str)
        .and_then(centisecond_duration)
    else {
        return;
    };
    if let Some(result) = statement.get_mut("result").and_then(Value::as_object_mut) {
        result.insert("duration".to_owned(), Value::String(duration));
    }
}

fn centisecond_duration(duration: &str) -> Option<String> {
    let duration = duration.strip_suffix('S')?;
    let (prefix, fractional_seconds) = duration.rsplit_once('.')?;
    if !prefix.contains('T')
        || !prefix
            .chars()
            .next_back()
            .is_some_and(|character| character.is_ascii_digit())
        || !fractional_seconds
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return None;
    }

    let mut centiseconds = fractional_seconds.chars().take(2).collect::<String>();
    while centiseconds.len() < 2 {
        centiseconds.push('0');
    }
    Some(format!("{prefix}.{centiseconds}S"))
}

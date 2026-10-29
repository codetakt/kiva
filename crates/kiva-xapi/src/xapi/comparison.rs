#![allow(clippy::wildcard_imports)]

use super::*;
use num_bigint::BigUint;

/// Returns true when two Statement JSON values are equivalent after removing
/// LRS-managed fields and normalizing duration precision used by conformance
/// fixtures.
#[must_use]
pub fn statements_equivalent(left: &Value, right: &Value) -> bool {
    valid_durations(left)
        && valid_durations(right)
        && super::number::json_equal(
            &canonical_statement_for_comparison(left),
            &canonical_statement_for_comparison(right),
        )
}

fn valid_durations(statement: &Value) -> bool {
    if let Some(duration) = statement.pointer("/result/duration")
        && !duration
            .as_str()
            .is_some_and(super::validation::is_duration)
    {
        return false;
    }
    statement.get("object").is_none_or(|object| {
        object.get("objectType").and_then(Value::as_str) != Some("SubStatement")
            || valid_durations(object)
    })
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
    if let Some(nested) = object.get_mut("object").and_then(Value::as_object_mut)
        && nested.get("objectType").and_then(Value::as_str) == Some("SubStatement")
    {
        normalize_result_duration_for_comparison(nested);
    }
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
    if !super::validation::is_duration(duration) {
        return None;
    }
    let (date, clock) = duration[1..]
        .split_once('T')
        .unwrap_or_else(|| (&duration[1..], ""));
    // Calendar units are retained separately. No assumed reference date, fixed
    // year/month length, or civil-day-to-second conversion enters this relation.
    let mut calendar = Vec::new();
    let mut start = 0;
    for (end, unit) in date.char_indices().filter(|(_, c)| c.is_ascii_alphabetic()) {
        let value = &date[start..end];
        let (whole, fraction) = value.split_once(['.', ',']).unwrap_or((value, ""));
        let whole = whole.trim_start_matches('0');
        let fraction = fraction.trim_end_matches('0');
        if !whole.is_empty() || !fraction.is_empty() {
            calendar.push(format!(
                "{}.{fraction}{unit}",
                if whole.is_empty() { "0" } else { whole }
            ));
        }
        start = end + 1;
    }
    // Arbitrary precision avoids collapsing neighboring huge components or
    // dropping sub-centisecond fractions before hours/minutes become seconds.
    let mut total = BigUint::from(0_u8);
    let mut scale = 2_u32;
    start = 0;
    for (end, unit) in clock
        .char_indices()
        .filter(|(_, c)| c.is_ascii_alphabetic())
    {
        let value = &clock[start..end];
        let (whole, fraction) = value.split_once(['.', ',']).unwrap_or((value, ""));
        let digits = format!("{whole}{fraction}");
        let value_scale = u32::try_from(fraction.len()).ok()?;
        let mut coefficient = BigUint::parse_bytes(digits.as_bytes(), 10)?;
        coefficient *= match unit {
            'H' => 3600_u32,
            'M' => 60_u32,
            'S' => 1_u32,
            _ => return None,
        };
        if value_scale > scale {
            total *= BigUint::from(10_u8).pow(value_scale - scale);
            scale = value_scale;
        } else {
            coefficient *= BigUint::from(10_u8).pow(scale - value_scale);
        }
        total += coefficient;
        start = end + 1;
    }
    total /= BigUint::from(10_u8).pow(scale - 2);
    Some(format!("{}|{total}cs", calendar.join("")))
}

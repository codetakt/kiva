//! Schema-scoped Statement readback comparison, separate from 409 equivalence.

#![expect(
    clippy::redundant_pub_crate,
    reason = "Only sibling assertion code uses this oracle"
)]

use super::lossless_json::Json;
use serde_json::Value;

pub(super) fn compare_statement_bytes(expected: &Json, bytes: &[u8]) -> Result<(), String> {
    let actual: Json = serde_json::from_slice(bytes)
        .map_err(|error| format!("preservation requires JSON: {error}"))?;
    unique_core_members(expected, "", false)?;
    unique_core_members(&actual, "", false)?;
    compare(expected, &actual, "", false)
}

#[cfg(test)]
fn compare_statement(expected: &Value, actual: &Value) -> Result<(), String> {
    compare(&expected.clone().into(), &actual.clone().into(), "", false)
}

fn unique_core_members(value: &Json, path: &str, opaque: bool) -> Result<(), String> {
    match value {
        Json::Object(pairs) => {
            let mut names = std::collections::BTreeSet::new();
            for (key, child) in pairs {
                if !opaque && !names.insert(key) {
                    return Err(format!("duplicate property {key} at {path}"));
                }
                unique_core_members(
                    child,
                    &format!("{path}/{}", escape_key(key)),
                    opaque || path.ends_with("/extensions"),
                )?;
            }
        }
        Json::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                unique_core_members(item, &format!("{path}/{index}"), opaque)?;
            }
        }
        Json::Scalar(_) => {}
    }
    Ok(())
}

fn escape_key(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// Authority determinism concerns Agent/Group identity, not display members.
/// Anonymous Group members form a multiset of Agent identities. Identified
/// Group membership and names do not replace its identifying IFI.
#[expect(
    clippy::too_many_lines,
    reason = "Keep structural validation and known/possible identity matching in one ordered oracle"
)]
fn agent_identity_core(left: &Value, right: &Value) -> Result<bool, String> {
    fn kind(v: &Value) -> &str {
        v.get("objectType")
            .and_then(Value::as_str)
            .unwrap_or("Agent")
    }
    fn valid(v: &Value, member: bool) -> bool {
        if !v.is_object()
            || v.get("objectType").is_some_and(|k| !k.is_string())
            || !matches!(kind(v), "Agent" | "Group")
            || member && kind(v) != "Agent"
        {
            return false;
        }
        if v.get("name").is_some_and(|n| !n.is_string()) {
            return false;
        }
        let keys: Vec<_> = ["mbox", "mbox_sha1sum", "openid", "account"]
            .into_iter()
            .filter(|k| v.get(*k).is_some())
            .collect();
        if keys.len() > 1 || kind(v) == "Agent" && keys.len() != 1 {
            return false;
        }
        if let Some(key) = keys.first() {
            if *key == "account" {
                if !v[*key]["homePage"].is_string() || !v[*key]["name"].is_string() {
                    return false;
                }
            } else if !v[*key].is_string() {
                return false;
            }
        }
        if kind(v) == "Group" {
            (match v.get("member") {
                Some(Value::Array(_)) => true,
                None => !keys.is_empty(),
                _ => false,
            }) && v
                .get("member")
                .and_then(Value::as_array)
                .is_none_or(|items| items.iter().all(|a| valid(a, true)))
        } else {
            true
        }
    }
    if !valid(left, false) || !valid(right, false) || kind(left) != kind(right) {
        return Ok(false);
    }
    if !matches!(kind(left), "Agent" | "Group") {
        return Ok(false);
    }
    let keys = ["mbox", "mbox_sha1sum", "openid", "account"];
    let a: Vec<_> = keys
        .iter()
        .copied()
        .filter(|k| left.get(*k).is_some())
        .collect();
    let b: Vec<_> = keys
        .iter()
        .copied()
        .filter(|k| right.get(*k).is_some())
        .collect();
    if a != b || a.len() > 1 {
        return Ok(false);
    }
    if let Some(key) = a.first() {
        if *key == "account" {
            return super::assertions::all_comparisons([
                super::assertions::json_equal(&left[*key]["name"], &right[*key]["name"]),
                identifier_equal(
                    &left[*key]["homePage"],
                    &right[*key]["homePage"],
                    "homePage",
                ),
            ]);
        }
        return identifier_equal(&left[*key], &right[*key], key);
    }
    if kind(left) != "Group" {
        return Ok(false);
    }
    let (Some(a), Some(b)) = (
        left.get("member").and_then(Value::as_array),
        right.get("member").and_then(Value::as_array),
    ) else {
        return Ok(false);
    };
    if a.len() != b.len() || a.iter().chain(b).any(|v| kind(v) != "Agent") {
        return Ok(false);
    }
    let relations: Vec<Vec<_>> = a
        .iter()
        .map(|x| b.iter().map(|y| agent_identity_equal(x, y)).collect())
        .collect();
    let known: Vec<Vec<bool>> = relations
        .iter()
        .map(|r| r.iter().map(|v| matches!(v, Ok(true))).collect())
        .collect();
    if perfect_matching(&known) {
        return Ok(true);
    }
    let possible: Vec<Vec<bool>> = relations
        .iter()
        .map(|r| r.iter().map(|v| !matches!(v, Ok(false))).collect())
        .collect();
    if !perfect_matching(&possible) {
        return Ok(false);
    }
    Err("ORACLE-UNAVAILABLE: Agent identity matching needs the full identifier relation".into())
}

/// Bounded authority comparison; display changes require the full mapping
/// contract instead of being silently ignored. This is not signature equality.
pub(super) fn agent_identity_equal(left: &Value, right: &Value) -> Result<bool, String> {
    let identity = agent_identity_core(left, right);
    if matches!(identity, Ok(false)) {
        return identity;
    }
    let ignored = [
        "objectType",
        "mbox",
        "mbox_sha1sum",
        "openid",
        "account",
        "member",
    ];
    let (Some(left), Some(right)) = (left.as_object(), right.as_object()) else {
        return Ok(false);
    };
    let a: serde_json::Map<String, Value> = left
        .iter()
        .filter(|(k, _)| !ignored.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let b: serde_json::Map<String, Value> = right
        .iter()
        .filter(|(k, _)| !ignored.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let mut results = vec![identity];
    results.push(if a == b {
        Ok(true)
    } else {
        Err(
            "ORACLE-UNAVAILABLE: changed authority display facts require declared mapping contract"
                .into(),
        )
    });
    match (left.get("member"),right.get("member")) {
        (None,None) => {},
        (Some(Value::Array(a)),Some(Value::Array(b))) => {
            let relations: Vec<Vec<_>> = a.iter().map(|x|b.iter().map(|y|agent_identity_equal(x,y)).collect()).collect();
            let known: Vec<Vec<bool>> = relations.iter().map(|r|r.iter().map(|v| matches!(v,Ok(true))).collect()).collect();
            if a.len()==b.len() && perfect_matching(&known) { results.push(Ok(true)); }
            else { results.push(Err("ORACLE-UNAVAILABLE: authority display membership requires full mapping contract".into())); }
        },
        _ => results.push(Err("ORACLE-UNAVAILABLE: changed authority membership presence requires full mapping contract".into())),
    }
    super::assertions::all_comparisons(results)
}

fn identifier_equal(left: &Value, right: &Value, field: &str) -> Result<bool, String> {
    let (Some(a), Some(b)) = (left.as_str(), right.as_str()) else {
        return Ok(false);
    };
    if a == b {
        return Ok(true);
    }
    if field == "mbox_sha1sum" {
        return Ok(a.eq_ignore_ascii_case(b));
    }
    if field == "mbox" {
        let simple = |s: &str| -> Option<(String, String)> {
            let (scheme, rest) = s.split_once(':')?;
            if !scheme.eq_ignore_ascii_case("mailto")
                || !rest.is_ascii()
                || rest.contains(['%', '?', ',', '#'])
            {
                return None;
            }
            let (local, host) = rest.rsplit_once('@')?;
            if local.is_empty()
                || host.is_empty()
                || !local
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".!$'*+_~-".contains(&b))
                || !host
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
            {
                return None;
            }
            Some((local.to_owned(), host.to_ascii_lowercase()))
        };
        if let (Some(a), Some(b)) = (simple(a), simple(b)) {
            return Ok(a == b);
        }
    }
    Err(
        "ORACLE-UNAVAILABLE: identifier equivalence requires the defined RFC3987/DC-MAILTO relation"
            .into(),
    )
}

fn compare(expected: &Json, actual: &Json, path: &str, opaque: bool) -> Result<(), String> {
    if matches!(path, "/stored" | "/authority") {
        // Checked separately by managed-field/time/credential assertions.
        return Ok(());
    }
    if !opaque
        && matches!(
            path,
            "/timestamp" | "/object/timestamp" | "/result/duration" | "/object/result/duration"
        )
    {
        let (Json::Scalar(left), Json::Scalar(right)) = (expected, actual) else {
            return Err(format!("time value must be a string at {path}"));
        };
        return if path.ends_with("/timestamp") {
            compare_timestamp(left, right, path)
        } else {
            compare_duration(left, right, path)
        };
    }
    match (expected, actual) {
        (Json::Object(left), Json::Object(right))
            if opaque && (expected.has_repeated_members() || actual.has_repeated_members()) =>
        {
            compare_object_multiset(left, right, path)
        }
        (Json::Object(left), Json::Object(right)) => {
            let mut unavailable = None;
            for (key, value) in left {
                let child = format!("{path}/{}", escape_key(key));
                let returned = actual.get(key).ok_or_else(|| format!("missing {child}"))?;
                defer_unknown(
                    compare(
                        value,
                        returned,
                        &child,
                        opaque || path.ends_with("/extensions"),
                    ),
                    &mut unavailable,
                )?;
            }
            for (key, _) in right.iter().filter(|(key, _)| expected.get(key).is_none()) {
                if opaque || !permitted_addition(expected, path, key) {
                    return Err(format!("unexpected returned property {key} at {path}"));
                }
            }
            unavailable.map_or(Ok(()), Err)
        }
        (Json::Object(_), Json::Array(right))
            if !opaque && context_activity_value(path) && right.len() == 1 =>
        {
            compare(expected, &right[0], &format!("{path}/0"), false)
        }
        (Json::Array(left), Json::Array(right)) if left.len() == right.len() => {
            if !opaque
                && (path.ends_with("/member")
                    || matches!(path, "/attachments" | "/object/attachments"))
            {
                compare_multiset(left, right, path)
            } else {
                let mut unavailable = None;
                for (index, (value, returned)) in left.iter().zip(right).enumerate() {
                    defer_unknown(
                        compare(value, returned, &format!("{path}/{index}"), opaque),
                        &mut unavailable,
                    )?;
                }
                unavailable.map_or(Ok(()), Err)
            }
        }
        (Json::Scalar(left), Json::Scalar(right)) => {
            if left.is_number() && right.is_number() {
                if numeric_precision(left, right)? {
                    Ok(())
                } else {
                    Err(format!(
                        "submitted number lost binary32 precision at {path}"
                    ))
                }
            } else if left == right {
                Ok(())
            } else {
                Err(format!("submitted value changed at {path}"))
            }
        }
        _ => Err(format!("submitted value changed at {path}")),
    }
}

fn compare_object_multiset(
    left: &[(String, Json)],
    right: &[(String, Json)],
    path: &str,
) -> Result<(), String> {
    if left.len() != right.len() {
        return Err(format!("extension member multiplicity changed at {path}"));
    }
    // Equal names may carry different values: find a full matching,
    // retaining definite mismatches even when another pair is delegated.
    let results: Vec<Vec<_>> = left
        .iter()
        .map(|(key, value)| {
            right
                .iter()
                .map(|(name, returned)| {
                    if key == name {
                        compare(
                            value,
                            returned,
                            &format!("{path}/{}", escape_key(key)),
                            true,
                        )
                    } else {
                        Err(format!("extension member name changed at {path}"))
                    }
                })
                .collect()
        })
        .collect();
    matching_result(&results, path)
}

fn permitted_addition(expected: &Json, path: &str, key: &str) -> bool {
    // Presence allowance only: shapes, values and credential/time relationships
    // are checked independently. An implicit object type is not an exact-output
    // property-addition permission. Never apply these exceptions in extensions.
    if path.is_empty() {
        return matches!(key, "id" | "stored" | "authority" | "timestamp" | "version");
    }
    if path == "/object"
        && expected.get("objectType").and_then(Json::as_str) == Some("SubStatement")
    {
        return key == "timestamp";
    }
    key == "fileUrl"
        && ["/attachments/", "/object/attachments/"]
            .iter()
            .any(|prefix| {
                path.strip_prefix(prefix).is_some_and(|index| {
                    !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
                })
            })
}

fn defer_unknown(result: Result<(), String>, pending: &mut Option<String>) -> Result<(), String> {
    match result {
        Err(error) if error.starts_with("ORACLE-UNAVAILABLE:") => {
            pending.get_or_insert(error);
            Ok(())
        }
        other => other,
    }
}

// Distinguish a definite perfect matching from a merely possible one. Greedily
// consuming an unknown candidate can hide an unmatched member or invent a FAIL.
fn compare_multiset(left: &[Json], right: &[Json], path: &str) -> Result<(), String> {
    let results: Vec<Vec<_>> = left
        .iter()
        .enumerate()
        .map(|(i, value)| {
            right
                .iter()
                .map(|item| compare(value, item, &format!("{path}/{i}"), false))
                .collect()
        })
        .collect();
    matching_result(&results, path)
}

fn matching_result(results: &[Vec<Result<(), String>>], path: &str) -> Result<(), String> {
    let known: Vec<Vec<bool>> = results
        .iter()
        .map(|row| row.iter().map(Result::is_ok).collect())
        .collect();
    if perfect_matching(&known) {
        return Ok(());
    }
    let possible: Vec<Vec<bool>> = results
        .iter()
        .map(|row| {
            row.iter()
                .map(|r| {
                    r.is_ok()
                        || r.as_ref()
                            .err()
                            .is_some_and(|e| e.starts_with("ORACLE-UNAVAILABLE:"))
                })
                .collect()
        })
        .collect();
    if perfect_matching(&possible) {
        Err(format!(
            "ORACLE-UNAVAILABLE: multiset matching at {path} needs delegated primitive"
        ))
    } else {
        Err(format!("submitted multiset changed at {path}"))
    }
}

fn perfect_matching(edges: &[Vec<bool>]) -> bool {
    fn augment(
        row: usize,
        edges: &[Vec<bool>],
        seen: &mut [bool],
        owners: &mut [Option<usize>],
    ) -> bool {
        for column in 0..owners.len() {
            if !edges[row][column] || seen[column] {
                continue;
            }
            seen[column] = true;
            let previous = owners[column];
            if previous.is_none_or(|owner| augment(owner, edges, seen, owners)) {
                owners[column] = Some(row);
                return true;
            }
        }
        false
    }
    let mut owners = vec![None; edges.len()];
    (0..edges.len()).all(|row| augment(row, edges, &mut vec![false; edges.len()], &mut owners))
}

fn context_activity_value(path: &str) -> bool {
    let path = path.strip_prefix("/object").unwrap_or(path);
    path.strip_prefix("/context/contextActivities/")
        .is_some_and(|key| matches!(key, "parent" | "grouping" | "category" | "other"))
}

/// Only integer JSON representations retain exact input values in this parser.
/// Decimal/exponent representations require a lossless provider, even when their
/// already-rounded binary64 values compare equal. Never infer a decimal PASS.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "Binary32 bounds are integral for integral inputs; i128 contains all u64/i64 values and their adjacent binary32 bounds"
)]
pub(super) fn numeric_precision(expected: &Value, actual: &Value) -> Result<bool, String> {
    if !expected.is_number() || !actual.is_number() {
        return Ok(false);
    }
    let integer = |value: &Value| {
        value
            .as_i64()
            .map(i128::from)
            .or_else(|| value.as_u64().map(i128::from))
    };
    let (Some(expected), Some(actual)) = (integer(expected), integer(actual)) else {
        return Err("ORACLE-UNAVAILABLE: decimal/exponent JSON input needs lossless DC-NUMBER provider before numeric equality or precision grading".into());
    };
    if expected == actual {
        return Ok(true);
    }
    let rounded = expected as f32;
    let center = rounded as i128;
    if center == expected {
        return Ok(binary32_integer_spelling(rounded, actual));
    }
    let adjacent = if center < expected {
        rounded.next_up()
    } else {
        rounded.next_down()
    };
    let other = adjacent as i128;
    Ok((actual >= center.min(other) && actual <= center.max(other))
        || binary32_integer_spelling(rounded, actual)
        || binary32_integer_spelling(adjacent, actual))
}

/// Recognize directed decimal rounding of an integral binary32 endpoint at
/// each significant precision. Floor/ceiling bracket nearest/tie choices;
/// sharing a binary32 rounding bin alone is not a serialization witness.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "Caller supplies integral f32 bounds of i64/u64 input; i128 arithmetic stays within 10^20 and the f32 cast only checks endpoint restoration"
)]
const fn binary32_integer_spelling(endpoint: f32, actual: i128) -> bool {
    if (actual as f32).to_bits() != endpoint.to_bits() {
        return false;
    }
    let exact = endpoint as i128;
    // Units and finer precision yield the exact integer, already handled by
    // the singleton/enclosing interval. Coarser decimal places are powers of 10.
    let mut quantum = 10;
    while quantum <= exact.abs() {
        let lower = exact.div_euclid(quantum) * quantum;
        // Exact multiples need no rounding: their floor and ceiling coincide.
        if actual == lower || (exact != lower && actual == lower + quantum) {
            return true;
        }
        quantum *= 10;
    }
    false
}

fn compare_timestamp(expected: &Value, actual: &Value, path: &str) -> Result<(), String> {
    let (Some(left), Some(right)) = (expected.as_str(), actual.as_str()) else {
        return Err(format!("timestamp type changed at {path}"));
    };
    if super::assertions::has_explicit_non_utc_offset(right) {
        return Err(format!("timestamp is not UTC at {path}"));
    }
    // The comparison below validates both operands before deferring an unknown
    // instant. Do not let this independent UTC check short-circuit that work.
    if super::assertions::parse_time(right).is_ok_and(|time| !time.offset().is_utc()) {
        return Err(format!("timestamp is not UTC at {path}"));
    }
    if super::assertions::timestamp_preserved(right, left, true)? {
        return Ok(());
    }
    Err(format!(
        "timestamp changed beyond permitted precision at {path}"
    ))
}

fn compare_duration(expected: &Value, actual: &Value, path: &str) -> Result<(), String> {
    let (Some(left), Some(right)) = (expected.as_str(), actual.as_str()) else {
        return Err(format!("duration type changed at {path}"));
    };
    if !duration_lexically_valid(left) || !duration_lexically_valid(right) {
        return Err(format!("invalid duration characters at {path}"));
    }
    if left == right {
        return Ok(());
    }
    // Preserve all calendar/time components. Only extra fractional seconds may
    // be truncated here; no conversion of months or years to seconds is made.
    if let Some(seconds) = left.strip_suffix('S')
        && left.contains('T')
        && let Some((prefix, fraction)) = seconds.rsplit_once(['.', ','])
        && fraction.len() > 2
        && fraction.bytes().all(|b| b.is_ascii_digit())
        && let Some(first_two) = fraction.get(..2)
    {
        for separator in ['.', ','] {
            if right == format!("{prefix}{separator}{first_two}S") {
                return Ok(());
            }
        }
        if first_two == "00" && right == format!("{prefix}S") {
            return Ok(());
        }
        if first_two.ends_with('0') && right == format!("{prefix}.{}S", &first_two[..1]) {
            return Ok(());
        }
    }
    // Decimal spelling/zero padding are not duration changes. Compare seconds
    // exactly when the preceding calendar/time component spelling is equal.
    if let (Some((prefix, seconds, fraction)), Some((other_prefix, other_seconds, other_fraction))) =
        (duration_seconds(left), duration_seconds(right))
        && prefix == other_prefix
    {
        let allowed_fraction = &fraction[..fraction.len().min(2)];
        if seconds == other_seconds
            && (fraction == other_fraction
                || allowed_fraction.trim_end_matches('0') == other_fraction)
        {
            return Ok(());
        }
        return Err(format!(
            "duration changed beyond permitted truncation at {path}"
        ));
    }
    Err(format!(
        "ORACLE-UNAVAILABLE: ISO duration relation at {path} needs DC-DURATION provider; no fixed calendar-to-seconds conversion"
    ))
}

fn duration_lexically_valid(text: &str) -> bool {
    text.starts_with('P')
        && text.bytes().enumerate().all(|(index, byte)| {
            b"0123456789PYMDWTHS.,".contains(&byte)
                && (!matches!(byte, b'.' | b',')
                    || text
                        .as_bytes()
                        .get(index + 1)
                        .is_some_and(u8::is_ascii_digit))
        })
}

fn duration_seconds(text: &str) -> Option<(&str, &str, &str)> {
    // Only ASCII designators/digits can enter byte-indexed component slicing.
    // Other legal calendar relations still require the duration provider.
    if !duration_lexically_valid(text) {
        return None;
    }
    let text = text.strip_suffix('S')?;
    if !text.starts_with('P') || !text.contains('T') {
        return None;
    }
    let start = text.rfind(|c: char| !c.is_ascii_digit() && !matches!(c, '.' | ','))? + 1;
    let (whole, fraction) = text[start..]
        .split_once(['.', ','])
        .unwrap_or_else(|| (&text[start..], ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    Some((
        &text[..start],
        whole.trim_start_matches('0'),
        fraction.trim_end_matches('0'),
    ))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn duration_lexical_defects_do_not_reach_component_slices() {
        for invalid in [
            "PTé1S", "PT🙂1S", "PT１S", "PT1éS", "PT1.éS", "PT1\0S", "PT1e3S", "PT1.S", "PT1,S",
            "PT1.H", "P1,D", "PT1.",
        ] {
            assert!(super::duration_seconds(invalid).is_none());
            for other in ["PT1.239S", "P1M", invalid] {
                for (sent, got) in [(other, invalid), (invalid, other)] {
                    let error =
                        super::compare_duration(&json!(sent), &json!(got), "/result/duration")
                            .expect_err("lexical violation");
                    assert!(!error.starts_with("ORACLE-UNAVAILABLE:"), "{error}");
                }
            }
        }
        for (sent, got) in [
            ("PT1.239S", "PT1.23S"),
            ("PT1.239S", "PT1,23S"),
            ("PT1S", "PT1.00S"),
            ("PT1S", "PT1,00S"),
            ("P1W", "P1W"),
        ] {
            assert!(super::compare_duration(&json!(sent), &json!(got), "duration").is_ok());
        }
        assert!(
            super::compare_duration(&json!("P1M"), &json!("P30D"), "duration")
                .expect_err("calendar provider required")
                .starts_with("ORACLE-UNAVAILABLE:")
        );
    }

    #[test]
    fn precision_allowances_and_extension_boundaries() {
        let sent = json!({"timestamp":"2025-01-01T09:00:00.123456+09:00", "result":{"score":{"raw":16_777_217}, "duration":"PT1.239S", "extensions":{"urn:example:data":{"timestamp":"custom", "member":[1,2]}}}});
        let mut got = sent.clone();
        got["timestamp"] = json!("2025-01-01T00:00:00.123Z");
        got["result"]["score"]["raw"] = json!(16_777_216);
        got["result"]["duration"] = json!("PT1.23S");
        assert!(super::compare_statement(&sent, &got).is_ok());
        got["timestamp"] = json!("2025-01-01T00:00:00.124Z");
        assert!(super::compare_statement(&sent, &got).is_err());
        got = sent.clone();
        got["timestamp"] = json!("2025-01-01T00:00:00.123456Z");
        got["result"]["extensions"]["urn:example:data"]["member"] = json!([2, 1]);
        assert!(super::compare_statement(&sent, &got).is_err());
    }

    #[test]
    fn only_group_members_are_unordered_and_context_singletons_expand() {
        let a = json!({"mbox":"mailto:a@example.org"});
        let b = json!({"mbox":"mailto:b@example.org"});
        let sent = json!({"actor":{"objectType":"Group", "member":[a,b]}, "context":{"contextActivities":{"parent":{"id":"urn:example:a"}}}});
        let got = json!({"actor":{"objectType":"Group", "member":[b,a]}, "context":{"contextActivities":{"parent":[{"id":"urn:example:a"}]}}});
        assert!(super::compare_statement(&sent, &got).is_ok());
        let mut duplicate = got;
        duplicate["actor"]["member"] = json!([a, a]);
        assert!(super::compare_statement(&sent, &duplicate).is_err());
    }
}

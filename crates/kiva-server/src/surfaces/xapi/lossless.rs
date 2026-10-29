//! Retain owner-defined extension JSON separately from the typed core view.
//! Core and extensions-map names are unique; owner values may repeat names.
//! No magic JSON property or last-wins conversion represents their raw data.

use crate::core::response::HttpError;
use axum::http::StatusCode;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{MapAccess, Visitor},
    ser::{SerializeMap, SerializeSeq},
};
use serde_json::{Value, value::RawValue};
use std::collections::{BTreeMap, BTreeSet};

/// JSON-pointer -> valid raw owner value. Strings preserve member sequences
/// even when the database or application port uses an ordinary JSON codec.
pub(crate) type OpaqueValues = BTreeMap<String, String>;

/// Database JSON decoding must not interpret user keys as serde's private
/// arbitrary-number/raw-value transport tokens.
pub(crate) struct StoredJson(pub(crate) Value);
impl<'de> Deserialize<'de> for StoredJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = Box::<RawValue>::deserialize(deserializer)?;
        typed_view(&raw, 0)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

fn typed_view(raw: &RawValue, depth: usize) -> Result<Value, serde_json::Error> {
    if depth > 127 {
        return Err(<serde_json::Error as serde::de::Error>::custom(
            "JSON recursion limit exceeded",
        ));
    }
    match raw.get().as_bytes().first() {
        Some(b'{') => {
            let members: Members = serde_json::from_str(raw.get())?;
            let mut object = serde_json::Map::new();
            for (key, value) in members.0 {
                object.insert(key, typed_view(&value, depth + 1)?);
            }
            Ok(Value::Object(object))
        }
        Some(b'[') => {
            let items: Vec<Box<RawValue>> = serde_json::from_str(raw.get())?;
            items
                .iter()
                .map(|raw| typed_view(raw, depth + 1))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
        }
        _ => serde_json::from_str(raw.get()),
    }
}

/// DTO validation owns the structural extension map, not its arbitrary values.
/// Validate a copy with owner values masked; retain the original typed/raw data.
pub(crate) fn contract_view(value: &Value) -> Value {
    let mut view = value.clone();
    let mut maps = BTreeSet::new();
    if let Some(array) = value.as_array() {
        for (index, statement) in array.iter().enumerate() {
            extension_maps(statement, &format!("/{index}"), &mut maps);
        }
    } else {
        extension_maps(value, "", &mut maps);
    }
    for path in maps {
        if let Some(object) = view.pointer_mut(&path).and_then(Value::as_object_mut) {
            for value in object.values_mut() {
                *value = Value::Null;
            }
        }
    }
    view
}

pub(crate) fn pointer(parent: &str, key: &str) -> String {
    format!("{parent}/{}", key.replace('~', "~0").replace('/', "~1"))
}

pub(crate) fn parse_statement(
    body: &[u8],
    label: &str,
) -> Result<(Value, OpaqueValues), HttpError> {
    let invalid = |error: serde_json::Error| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            format!("{label} must be valid JSON: {error}"),
        )
    };
    // Enforce parser depth/numeric contracts, then check ORIGINAL member
    // sequences before trusting this typed view or admitting any Statement.
    let raw: &RawValue = serde_json::from_slice(body).map_err(invalid)?;
    let value = typed_view(raw, 0).map_err(invalid)?;
    let mut maps = BTreeSet::new();
    if let Some(batch) = value.as_array() {
        for (index, statement) in batch.iter().enumerate() {
            extension_maps(statement, &format!("/{index}"), &mut maps);
        }
    } else {
        extension_maps(&value, "", &mut maps);
    }
    let mut opaque = OpaqueValues::new();
    walk(raw, "", &maps, &mut opaque)
        .map_err(|error| HttpError::new(StatusCode::BAD_REQUEST, format!("{label}: {error}")))?;
    Ok((value, opaque))
}

fn extension_maps(statement: &Value, path: &str, maps: &mut BTreeSet<String>) {
    for name in ["result", "context"] {
        maps.insert(format!("{path}/{name}/extensions"));
    }
    if let Some(object) = statement.get("object") {
        if object.get("objectType").and_then(Value::as_str) == Some("SubStatement") {
            extension_maps(object, &format!("{path}/object"), maps);
        } else if object
            .get("objectType")
            .and_then(Value::as_str)
            .is_none_or(|kind| kind == "Activity")
        {
            maps.insert(format!("{path}/object/definition/extensions"));
        }
    }
    if let Some(activities) = statement
        .pointer("/context/contextActivities")
        .and_then(Value::as_object)
    {
        for key in ["parent", "grouping", "category", "other"] {
            if let Some(array) = activities.get(key).and_then(Value::as_array) {
                for index in 0..array.len() {
                    maps.insert(format!(
                        "{path}/context/contextActivities/{key}/{index}/definition/extensions"
                    ));
                }
            } else {
                maps.insert(format!(
                    "{path}/context/contextActivities/{key}/definition/extensions"
                ));
            }
        }
    }
}

pub(crate) struct Members(pub(crate) Vec<(String, Box<RawValue>)>);
impl Serialize for Members {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (name, value) in &self.0 {
            map.serialize_entry(name, value)?;
        }
        map.end()
    }
}
impl<'de> Deserialize<'de> for Members {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = Members;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON object with its original member sequence")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Members, A::Error> {
                let mut members = Vec::new();
                while let Some(key) = access.next_key::<String>()? {
                    members.push((key, access.next_value::<Box<RawValue>>()?));
                }
                Ok(Members(members))
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}

fn walk(
    raw: &RawValue,
    path: &str,
    maps: &BTreeSet<String>,
    opaque: &mut OpaqueValues,
) -> Result<(), String> {
    match raw.get().as_bytes().first() {
        Some(b'{') => {
            let members: Members = serde_json::from_str(raw.get()).map_err(|e| e.to_string())?;
            let mut seen = BTreeSet::new();
            for (key, value) in members.0 {
                if !seen.insert(key.clone()) {
                    return Err(format!(
                        "duplicate core or extensions-map member at {}",
                        pointer(path, &key)
                    ));
                }
                let child = pointer(path, &key);
                if maps.contains(path) {
                    opaque.insert(child, value.get().to_owned());
                } else {
                    walk(&value, &child, maps, opaque)?;
                }
            }
        }
        Some(b'[') => {
            let array: Vec<Box<RawValue>> =
                serde_json::from_str(raw.get()).map_err(|e| e.to_string())?;
            for (index, value) in array.iter().enumerate() {
                walk(value, &format!("{path}/{index}"), maps, opaque)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn subtree(values: &OpaqueValues, prefix: &str) -> OpaqueValues {
    values
        .iter()
        .filter_map(|(path, raw)| {
            path.strip_prefix(prefix)
                .filter(|suffix| suffix.starts_with('/'))
                .map(|suffix| (suffix.to_owned(), raw.clone()))
        })
        .collect()
}

pub(crate) fn extend_prefixed(target: &mut OpaqueValues, values: &OpaqueValues, prefix: &str) {
    target.extend(
        values
            .iter()
            .map(|(path, raw)| (format!("{prefix}{path}"), raw.clone())),
    );
}

/// Compare owner values before masking their lossy typed views for the core
/// replay comparator. Object order is irrelevant; duplicate multiplicity and
/// array order remain significant. Missing legacy sidecars use their JSON view.
pub(crate) fn mask_equivalent_values(
    left: &mut Value,
    left_raw: &OpaqueValues,
    right: &mut Value,
    right_raw: &OpaqueValues,
) -> bool {
    let paths = left_raw
        .keys()
        .chain(right_raw.keys())
        .collect::<BTreeSet<_>>();
    for path in paths {
        let (Some(a), Some(b)) = (left.pointer_mut(path), right.pointer_mut(path)) else {
            return false;
        };
        let a_text = left_raw.get(path).cloned().unwrap_or_else(|| a.to_string());
        let b_text = right_raw
            .get(path)
            .cloned()
            .unwrap_or_else(|| b.to_string());
        let (Ok(a_node), Ok(b_node)) = (canonical(&a_text), canonical(&b_text)) else {
            return false;
        };
        if a_node != b_node {
            return false;
        }
        *a = Value::Null;
        *b = Value::Null;
    }
    true
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Canonical {
    Null,
    Boolean(bool),
    Number(bool, String, num_bigint::BigInt),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

fn canonical(raw: &str) -> Result<Canonical, serde_json::Error> {
    let raw: &RawValue = serde_json::from_str(raw)?;
    match raw.get().as_bytes().first() {
        Some(b'{') => {
            let members: Members = serde_json::from_str(raw.get())?;
            let mut members = members
                .0
                .into_iter()
                .map(|(key, value)| canonical(value.get()).map(|value| (key, value)))
                .collect::<Result<Vec<_>, _>>()?;
            members.sort();
            Ok(Canonical::Object(members))
        }
        Some(b'[') => {
            let items: Vec<Box<RawValue>> = serde_json::from_str(raw.get())?;
            Ok(Canonical::Array(
                items
                    .iter()
                    .map(|item| canonical(item.get()))
                    .collect::<Result<_, _>>()?,
            ))
        }
        Some(b'"') => serde_json::from_str(raw.get()).map(Canonical::String),
        Some(b't' | b'f') => serde_json::from_str(raw.get()).map(Canonical::Boolean),
        Some(b'n') => Ok(Canonical::Null),
        _ => canonical_number(raw.get()),
    }
}

fn canonical_number(raw: &str) -> Result<Canonical, serde_json::Error> {
    let error = || <serde_json::Error as serde::de::Error>::custom("invalid exact JSON number");
    let negative = raw.starts_with('-');
    let unsigned = raw.strip_prefix('-').unwrap_or(raw);
    let (coefficient, exponent) = unsigned.split_once(['e', 'E']).unwrap_or((unsigned, "0"));
    let mut exponent: num_bigint::BigInt = exponent.parse().map_err(|_| error())?;
    if let Some((_, fraction)) = coefficient.split_once('.') {
        exponent -= num_bigint::BigInt::from(fraction.len());
    }
    let digits = coefficient.replace('.', "");
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Ok(Canonical::Number(false, "0".to_owned(), 0.into()));
    }
    let significant = digits.trim_end_matches('0');
    exponent += num_bigint::BigInt::from(digits.len() - significant.len());
    Ok(Canonical::Number(
        negative,
        significant.to_owned(),
        exponent,
    ))
}

pub(crate) fn render(value: &Value, opaque: &OpaqueValues) -> Result<Vec<u8>, serde_json::Error> {
    if opaque.is_empty() {
        return serde_json::to_vec(value);
    }
    // The issued output contract wraps a singleton Context Activity in an
    // array. Move only that structural pointer; never traverse/rewrite an
    // extension-owned value because it happens to contain xAPI-like names.
    let mut projected = OpaqueValues::new();
    for (path, raw) in opaque {
        let mut path = path.clone();
        if value.pointer(&path).is_none() {
            let mut segments = path.split('/').collect::<Vec<_>>();
            if let Some(at) = segments.windows(4).position(|s| {
                s[0] == "context"
                    && s[1] == "contextActivities"
                    && matches!(s[2], "parent" | "grouping" | "category" | "other")
                    && s[3] == "definition"
            }) {
                segments.insert(at + 3, "0");
                let wrapped = segments.join("/");
                if value.pointer(&wrapped).is_some() {
                    path = wrapped;
                }
            }
        }
        projected.insert(path, raw.clone());
    }
    serde_json::to_vec(&WireValue {
        value,
        opaque: &projected,
        path: String::new(),
    })
}

struct WireValue<'a> {
    value: &'a Value,
    opaque: &'a OpaqueValues,
    path: String,
}
impl Serialize for WireValue<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if let Some(raw) = self.opaque.get(&self.path) {
            let raw: &RawValue = serde_json::from_str(raw).map_err(serde::ser::Error::custom)?;
            let parsed = typed_view(raw, 0).map_err(serde::ser::Error::custom)?;
            if &parsed != self.value {
                return Err(serde::ser::Error::custom(
                    "opaque value changed in a typed projection",
                ));
            }
            return raw.serialize(serializer);
        }
        match self.value {
            Value::Object(object) => {
                let mut output = serializer.serialize_map(Some(object.len()))?;
                for (key, value) in object {
                    output.serialize_entry(
                        key,
                        &WireValue {
                            value,
                            opaque: self.opaque,
                            path: pointer(&self.path, key),
                        },
                    )?;
                }
                output.end()
            }
            Value::Array(array) => {
                let mut output = serializer.serialize_seq(Some(array.len()))?;
                for (index, value) in array.iter().enumerate() {
                    output.serialize_element(&WireValue {
                        value,
                        opaque: self.opaque,
                        path: format!("{}/{index}", self.path),
                    })?;
                }
                output.end()
            }
            value => value.serialize(serializer),
        }
    }
}

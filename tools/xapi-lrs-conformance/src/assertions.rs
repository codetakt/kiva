//! Response relationships and table-shape assertions for neutral HTTP cases.

#![expect(
    clippy::redundant_pub_crate,
    reason = "Assertion types are shared only with the containing runner module"
)]

use std::collections::{BTreeMap, BTreeSet};

use reqwest::header::HeaderMap;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256, Sha384, Sha512};
use time::{
    OffsetDateTime,
    format_description::well_known::{Iso8601, Rfc3339},
};

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum Check {
    StatusBody {
        status: u16,
        non_empty: bool,
    },
    HeaderTime {
        header: String,
        #[serde(default)]
        equals_pointer: Option<String>,
        #[serde(default)]
        max_pointer: Option<String>,
        #[serde(default)]
        optional_if_empty: bool,
    },
    JsonShape {
        #[serde(default)]
        status: Option<u16>,
        #[serde(default)]
        pointer: String,
        shape: Box<Shape>,
    },
    JsonCaptureEqual {
        pointer: String,
        capture: String,
        #[serde(default)]
        not_equal: bool,
    },
    JsonOrder {
        pointer: String,
        by: String,
        #[serde(default)]
        descending: bool,
    },
    JsonTime {
        pointer: String,
        #[serde(default)]
        equals_capture: Option<String>,
        #[serde(default)]
        equals_pointer: Option<String>,
        #[serde(default)]
        expected: Option<String>,
        #[serde(default)]
        utc: bool,
        #[serde(default)]
        allow_truncation_or_rounding: bool,
    },
    Bytes {
        hex: String,
    },
    NumericPrecision {
        pointer: String,
        expected: f64,
    },
    LanguageMaps {
        #[serde(default)]
        pointer: String,
    },
    Multipart {
        #[serde(default)]
        expected_data: Vec<String>,
        #[serde(default)]
        expected_data_hex: Vec<String>,
    },
}

/// A deliberately bounded, language-neutral JSON table-shape vocabulary.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Shape {
    #[serde(default, rename = "type")]
    value_type: Option<String>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default, rename = "enum")]
    enum_values: Option<Vec<Value>>,
    #[serde(default)]
    required: Vec<String>,
    #[serde(default)]
    properties: BTreeMap<String, Self>,
    #[serde(default)]
    additional_properties: Option<bool>,
    #[serde(default)]
    items: Option<Box<Self>>,
    #[serde(default)]
    min_items: Option<usize>,
    #[serde(default)]
    max_items: Option<usize>,
    #[serde(default)]
    min_properties: Option<usize>,
    #[serde(default)]
    max_properties: Option<usize>,
    #[serde(default)]
    minimum: Option<f64>,
    #[serde(default)]
    maximum: Option<f64>,
    #[serde(default)]
    any_of: Vec<Self>,
}

impl Check {
    pub(super) fn validate_captures(&self, names: &BTreeSet<String>) -> Result<(), String> {
        let name = match self {
            Self::JsonTime {
                equals_capture: Some(name),
                ..
            }
            | Self::JsonCaptureEqual { capture: name, .. } => Some(name),
            _ => None,
        };
        if let Some(name) = name
            && !names.contains(name)
        {
            return Err(format!("undefined check capture {name}"));
        }
        Ok(())
    }
    pub(super) fn validate(&self) -> Result<(), String> {
        match self {
            Self::HeaderTime {
                header,
                equals_pointer,
                max_pointer,
                ..
            } => {
                reqwest::header::HeaderName::from_bytes(header.as_bytes())
                    .map_err(|error| error.to_string())?;
                if equals_pointer.is_some() && max_pointer.is_some() {
                    return Err("header-time accepts one JSON timestamp source".into());
                }
            }
            Self::JsonShape {
                status,
                pointer,
                shape,
            } => {
                if status.is_some_and(|status| !(100..=599).contains(&status)) {
                    return Err("json-shape requires a valid HTTP status".into());
                }
                validate_pointer(pointer)?;
                shape.validate()?;
            }
            Self::JsonCaptureEqual { pointer, .. } | Self::LanguageMaps { pointer } => {
                validate_pointer(pointer)?;
            }
            Self::JsonTime {
                pointer,
                equals_pointer,
                expected,
                allow_truncation_or_rounding,
                ..
            } => {
                validate_pointer(pointer)?;
                if let Some(pointer) = equals_pointer {
                    validate_pointer(pointer)?;
                }
                if let Some(expected) = expected {
                    parse_time(expected)?;
                } else if *allow_truncation_or_rounding {
                    return Err("timestamp rounding requires an expected timestamp".into());
                }
            }
            Self::JsonOrder { pointer, by, .. } => {
                validate_pointer(pointer)?;
                if by.is_empty() {
                    return Err("json-order requires a field name".into());
                }
            }
            Self::NumericPrecision { pointer, expected } => {
                validate_pointer(pointer)?;
                if !expected.is_finite() {
                    return Err("expected number must be finite".into());
                }
            }
            Self::Bytes { hex } => {
                decode_hex(hex)?;
            }
            Self::Multipart {
                expected_data_hex, ..
            } => {
                for hex in expected_data_hex {
                    decode_hex(hex)?;
                }
            }
            Self::StatusBody { status, .. } if !(100..=599).contains(status) => {
                return Err("status-body requires a valid HTTP status".into());
            }
            Self::StatusBody { .. } => {}
        }
        Ok(())
    }

    pub(super) fn evaluate(
        &self,
        status: u16,
        headers: &HeaderMap,
        bytes: &[u8],
        captures: &BTreeMap<String, String>,
    ) -> Vec<String> {
        if let Self::JsonShape {
            status: Some(expected),
            ..
        } = self
            && status != *expected
        {
            return Vec::new();
        }
        if let Self::StatusBody {
            status: expected,
            non_empty,
        } = self
        {
            return if status == *expected && (bytes.is_empty() == *non_empty) {
                vec![format!("status {status} response violates body.non_empty")]
            } else {
                Vec::new()
            };
        }
        self.evaluate_inner(headers, bytes, captures)
            .err()
            .into_iter()
            .collect()
    }

    #[expect(
        clippy::too_many_lines,
        reason = "One dispatch preserves a single JSON parse for all assertion kinds"
    )]
    fn evaluate_inner(
        &self,
        headers: &HeaderMap,
        bytes: &[u8],
        captures: &BTreeMap<String, String>,
    ) -> Result<(), String> {
        if let Self::Multipart {
            expected_data,
            expected_data_hex,
        } = self
        {
            let mut expected: Vec<_> = expected_data
                .iter()
                .map(|text| text.as_bytes().to_vec())
                .collect();
            expected.extend(
                expected_data_hex
                    .iter()
                    .map(|hex| decode_hex(hex))
                    .collect::<Result<Vec<_>, _>>()?,
            );
            return verify_multipart(headers, bytes, &expected);
        }
        if let Self::Bytes { hex } = self {
            if bytes != decode_hex(hex)? {
                return Err("response bytes differ from expected content".into());
            }
            return Ok(());
        }
        if let Self::HeaderTime {
            header,
            equals_pointer: None,
            max_pointer: None,
            ..
        } = self
        {
            return header_time(headers, header).map(|_| ());
        }
        let json: Value = serde_json::from_slice(bytes)
            .map_err(|error| format!("response relationship requires JSON: {error}"))?;
        match self {
            Self::HeaderTime {
                header,
                equals_pointer,
                max_pointer,
                optional_if_empty,
            } => {
                let expected = if let Some(pointer) = equals_pointer {
                    json_time(json.pointer(pointer))?
                } else if let Some(pointer) = max_pointer {
                    let array = json
                        .pointer(pointer)
                        .and_then(Value::as_array)
                        .ok_or_else(|| format!("{pointer} is not an array"))?;
                    if array.is_empty() && *optional_if_empty {
                        return Ok(());
                    }
                    let times = array
                        .iter()
                        .map(|item| json_time(item.get("stored")))
                        .collect::<Result<Vec<_>, _>>()?;
                    *times
                        .iter()
                        .max()
                        .ok_or("cannot compare an empty timestamp collection")?
                } else {
                    return Err("header-time requires a timestamp source".into());
                };
                let actual = header_time(headers, header)?;
                if actual.unix_timestamp() != expected.unix_timestamp() {
                    return Err(format!(
                        "{header} does not equal the returned JSON timestamp at HTTP-date second precision"
                    ));
                }
            }
            Self::JsonShape { pointer, shape, .. } => {
                shape.check(
                    json.pointer(pointer)
                        .ok_or_else(|| format!("missing {pointer}"))?,
                    pointer,
                )?;
            }
            Self::JsonCaptureEqual {
                pointer,
                capture,
                not_equal,
            } => {
                let serialized_value = captures
                    .get(capture)
                    .ok_or_else(|| format!("missing capture {capture}"))?;
                let expected: Value =
                    serde_json::from_str(serialized_value).map_err(|error| error.to_string())?;
                let actual = json
                    .pointer(pointer)
                    .ok_or_else(|| format!("missing {pointer}"))?;
                if json_equal(actual, &expected) == *not_equal {
                    return Err(format!("{pointer} differs from JSON capture {capture}"));
                }
            }
            Self::JsonOrder {
                pointer,
                by,
                descending,
            } => {
                let array = json
                    .pointer(pointer)
                    .and_then(Value::as_array)
                    .ok_or_else(|| format!("{pointer} is not an array"))?;
                let times = array
                    .iter()
                    .map(|item| json_time(item.get(by)))
                    .collect::<Result<Vec<_>, _>>()?;
                if times.windows(2).any(|pair| {
                    if *descending {
                        pair[0] < pair[1]
                    } else {
                        pair[0] > pair[1]
                    }
                }) {
                    return Err(format!("{pointer} is not ordered by {by}"));
                }
            }
            Self::JsonTime {
                pointer,
                equals_capture,
                equals_pointer,
                expected,
                utc,
                allow_truncation_or_rounding,
            } => {
                let value = json.pointer(pointer);
                let actual = json_time(value)?;
                if let Some(expected) = expected
                    && !timestamp_preserved(
                        actual,
                        parse_time(expected)?,
                        *allow_truncation_or_rounding,
                    )
                {
                    return Err(format!("{pointer} changed the submitted timestamp"));
                }
                // Unknown local offset syntax must not survive the required UTC conversion.
                if *utc
                    && (!actual.offset().is_utc()
                        || value
                            .and_then(Value::as_str)
                            .is_some_and(|text| text.ends_with("-00:00")))
                {
                    return Err(format!("{pointer} is not returned in UTC"));
                }
                if let Some(name) = equals_capture {
                    let value = captures
                        .get(name)
                        .ok_or_else(|| format!("missing capture {name}"))?;
                    let expected = parse_time(value)?;
                    if actual != expected {
                        return Err(format!("{pointer} does not equal capture {name}"));
                    }
                }
                if let Some(pointer) = equals_pointer
                    && actual != json_time(json.pointer(pointer))?
                {
                    return Err(format!("timestamp does not equal {pointer}"));
                }
            }
            Self::NumericPrecision { pointer, expected } => {
                let actual = json
                    .pointer(pointer)
                    .and_then(Value::as_f64)
                    .ok_or_else(|| format!("{pointer} is not numeric"))?;
                let tolerance =
                    expected.abs().max(f64::from(f32::MIN_POSITIVE)) * f64::from(f32::EPSILON);
                if (actual - expected).abs() > tolerance {
                    return Err(format!("{pointer} loses IEEE 754 binary32 precision"));
                }
            }
            Self::LanguageMaps { pointer } => {
                activity_language_maps(
                    json.pointer(pointer)
                        .ok_or_else(|| format!("missing {pointer}"))?,
                )?;
            }
            Self::Multipart { .. } | Self::Bytes { .. } | Self::StatusBody { .. } => {
                return Err("unexpected binary check".into());
            }
        }
        Ok(())
    }
}

impl Shape {
    fn validate(&self) -> Result<(), String> {
        if self.format.as_deref().is_some_and(|kind| {
            !matches!(
                kind,
                "uuid" | "timestamp" | "iri" | "uri" | "mailto-iri" | "sha1"
            )
        }) {
            return Err("unsupported JSON shape format".into());
        }
        if self.value_type.as_deref().is_some_and(|kind| {
            !matches!(
                kind,
                "object" | "array" | "string" | "number" | "integer" | "boolean" | "null"
            )
        }) {
            return Err("unknown JSON shape type".into());
        }
        if self
            .min_items
            .zip(self.max_items)
            .is_some_and(|(min, max)| min > max)
        {
            return Err("JSON shape has inconsistent array bounds".into());
        }
        if self
            .min_properties
            .zip(self.max_properties)
            .is_some_and(|(min, max)| min > max)
            || self
                .minimum
                .zip(self.maximum)
                .is_some_and(|(min, max)| min > max)
        {
            return Err("JSON shape has inconsistent property or numeric bounds".into());
        }
        for shape in self
            .properties
            .values()
            .chain(self.items.iter().map(Box::as_ref))
            .chain(self.any_of.iter())
        {
            shape.validate()?;
        }
        Ok(())
    }

    fn check(&self, value: &Value, pointer: &str) -> Result<(), String> {
        if let Some(format) = self.format.as_deref() {
            let text = value.as_str().ok_or("formatted value is not a string")?;
            check_string_format(format, text, pointer)?;
        }
        if !self.any_of.is_empty()
            && !self
                .any_of
                .iter()
                .any(|shape| shape.check(value, pointer).is_ok())
        {
            return Err(format!("{pointer} does not satisfy any allowed JSON shape"));
        }
        let correct_type = self.value_type.as_deref().is_none_or(|kind| match kind {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "number" => value.is_number(),
            "integer" => {
                value.is_i64()
                    || value.is_u64()
                    || value.as_f64().is_some_and(|number| {
                        number.is_finite()
                            && number.fract().classify() == std::num::FpCategory::Zero
                    })
            }
            "boolean" => value.is_boolean(),
            "null" => value.is_null(),
            _ => false,
        });
        if !correct_type
            || self
                .enum_values
                .as_ref()
                .is_some_and(|allowed| !allowed.iter().any(|item| json_equal(item, value)))
        {
            return Err(format!("{pointer} has an invalid type or enumerated value"));
        }
        if let Some(object) = value.as_object() {
            if self.required.iter().any(|name| !object.contains_key(name))
                || self.min_properties.is_some_and(|min| object.len() < min)
                || self.max_properties.is_some_and(|max| object.len() > max)
                || self.additional_properties == Some(false)
                    && object
                        .keys()
                        .any(|name| !self.properties.contains_key(name))
            {
                return Err(format!(
                    "{pointer} violates required, allowed, or counted object properties"
                ));
            }
            for (name, shape) in &self.properties {
                if let Some(item) = object.get(name) {
                    shape.check(item, &format!("{pointer}/{name}"))?;
                }
            }
        }
        if let Some(array) = value.as_array() {
            if self.min_items.is_some_and(|min| array.len() < min)
                || self.max_items.is_some_and(|max| array.len() > max)
            {
                return Err(format!("{pointer} violates array length bounds"));
            }
            if let Some(shape) = &self.items {
                for (index, item) in array.iter().enumerate() {
                    shape.check(item, &format!("{pointer}/{index}"))?;
                }
            }
        }
        if let Some(number) = value.as_f64()
            && (self.minimum.is_some_and(|min| number < min)
                || self.maximum.is_some_and(|max| number > max))
        {
            return Err(format!("{pointer} violates numeric bounds"));
        }
        Ok(())
    }
}

fn check_string_format(format: &str, text: &str, pointer: &str) -> Result<(), String> {
    let valid = match format {
        "uuid" => standard_uuid(text),
        "timestamp" => return parse_time(text).map(|_| ()),
        "iri" => crate::identifier_has_valid_syntax(text, false),
        "uri" => crate::identifier_has_valid_syntax(text, true),
        "mailto-iri" => {
            crate::identifier_has_valid_syntax(text, false)
                && text.split_once(':').is_some_and(|(scheme, rest)| {
                    scheme.eq_ignore_ascii_case("mailto") && !rest.is_empty()
                })
        }
        "sha1" => text.len() == 40 && text.bytes().all(|byte| byte.is_ascii_hexdigit()),
        _ => return Err(format!("unknown string format {format}")),
    };
    if valid {
        Ok(())
    } else {
        Err(format!("{pointer} is not valid {format} direct syntax"))
    }
}

pub(super) fn parse_time(value: &str) -> Result<OffsetDateTime, String> {
    OffsetDateTime::parse(value, &Rfc3339)
        .or_else(|_| OffsetDateTime::parse(value, &Iso8601::DEFAULT))
        .map_err(|error| format!("invalid JSON timestamp: {error}"))
}

fn standard_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

fn json_time(value: Option<&Value>) -> Result<OffsetDateTime, String> {
    parse_time(
        value
            .and_then(Value::as_str)
            .ok_or("missing JSON timestamp string")?,
    )
}

fn header_time(headers: &HeaderMap, name: &str) -> Result<OffsetDateTime, String> {
    let value = headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| format!("missing {name}"))?;
    let format = time::format_description::parse_borrowed::<2>(
        "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT",
    )
    .map_err(|error| error.to_string())?;
    time::PrimitiveDateTime::parse(value, &format)
        .map(time::PrimitiveDateTime::assume_utc)
        .map_err(|error| format!("{name} is not a valid IMF-fixdate: {error}"))
}

fn activity_language_maps(value: &Value) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            for (key, item) in object {
                match key.as_str() {
                    "definition" => one_language(item)?,
                    "extensions" | "attachments" => {}
                    _ => activity_language_maps(item)?,
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                activity_language_maps(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn one_language(value: &Value) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            for (key, item) in object {
                if key == "extensions" {
                    continue;
                }
                if matches!(key.as_str(), "name" | "description")
                    && item.is_object()
                    && item.as_object().is_some_and(|map| map.len() > 1)
                {
                    return Err(format!(
                        "canonical language map {key} has more than one language"
                    ));
                }
                one_language(item)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                one_language(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn mime_header<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(key, _)| key.trim().eq_ignore_ascii_case(name))
        .map(|(_, value)| value.trim())
}

fn unfold_mime_headers(headers: &str) -> String {
    let mut result = String::new();
    for line in headers.lines() {
        if line.starts_with([' ', '\t']) && !result.is_empty() {
            result.push(' ');
            result.push_str(line.trim());
        } else {
            if !result.is_empty() {
                result.push_str("\r\n");
            }
            result.push_str(line);
        }
    }
    result
}

#[expect(
    clippy::too_many_lines,
    reason = "MIME framing, hashes, and metadata are checked as a single ordered transmission"
)]
fn verify_multipart(
    headers: &HeaderMap,
    bytes: &[u8],
    expected_data: &[Vec<u8>],
) -> Result<(), String> {
    let content_type = headers
        .get("Content-Type")
        .and_then(|value| value.to_str().ok())
        .ok_or("missing multipart Content-Type")?;
    if !content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("multipart/mixed"))
    {
        return Err("response is not multipart/mixed".into());
    }
    let boundary = content_type
        .split(';')
        .filter_map(|part| part.split_once('='))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case("boundary"))
        .map(|(_, value)| value.trim().trim_matches('"'))
        .ok_or("missing MIME boundary")?;
    if boundary.is_empty() || boundary.len() > 70 {
        return Err("invalid MIME boundary length".into());
    }
    let delimiter = format!("--{boundary}").into_bytes();
    let mut positions = Vec::new();
    for (index, window) in bytes.windows(delimiter.len()).enumerate() {
        if window != delimiter
            || index != 0 && bytes.get(index.saturating_sub(2)..index) != Some(b"\r\n")
        {
            continue;
        }
        let mut end = index + delimiter.len();
        let closing = bytes[end..].starts_with(b"--");
        if closing {
            end += 2;
        }
        while bytes
            .get(end)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
        {
            end += 1;
        }
        if bytes[end..].starts_with(b"\r\n") {
            end += 2;
        } else if !closing || end != bytes.len() {
            continue;
        }
        positions.push((index, end, closing));
        if closing {
            break;
        }
    }
    if !positions.last().ok_or("missing MIME delimiters")?.2 {
        return Err("missing closing MIME delimiter".into());
    }
    let mut data_parts = Vec::new();
    let mut metadata = Vec::new();
    for (number, pair) in positions.windows(2).enumerate() {
        let part = &bytes[pair[0].1..pair[1].0];
        let split = part
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or("missing MIME part headers")?;
        let raw_headers = std::str::from_utf8(&part[..split]).map_err(|error| error.to_string())?;
        let unfolded = unfold_mime_headers(raw_headers);
        let part_headers = unfolded.as_str();
        let data = part[split + 4..]
            .strip_suffix(b"\r\n")
            .ok_or("invalid MIME part termination")?;
        if number == 0 {
            if !mime_header(part_headers, "Content-Type").is_some_and(|value| {
                value
                    .split(';')
                    .next()
                    .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("application/json"))
            }) {
                return Err("first MIME part is not application/json".into());
            }
            let json: Value = serde_json::from_slice(data)
                .map_err(|error| format!("invalid first MIME JSON part: {error}"))?;
            let statements: Vec<&Value> = json
                .as_array()
                .or_else(|| json.get("statements").and_then(Value::as_array))
                .map_or_else(|| vec![&json], |array| array.iter().collect());
            for statement in statements {
                if let Some(array) = statement.get("attachments").and_then(Value::as_array) {
                    metadata.extend(array.iter().cloned());
                }
                if let Some(array) = statement
                    .pointer("/object/attachments")
                    .and_then(Value::as_array)
                {
                    metadata.extend(array.iter().cloned());
                }
            }
        } else {
            let hash = mime_header(part_headers, "X-Experience-API-Hash")
                .ok_or("missing MIME attachment hash")?;
            let digest = match hash.len() {
                64 => format!("{:x}", Sha256::digest(data)),
                96 => format!("{:x}", Sha384::digest(data)),
                128 => format!("{:x}", Sha512::digest(data)),
                _ => return Err("unsupported SHA-2 representation".into()),
            };
            if !digest.eq_ignore_ascii_case(hash) {
                return Err("MIME attachment hash does not match raw data".into());
            }
            if !mime_header(part_headers, "Content-Transfer-Encoding")
                .is_some_and(|value| value.eq_ignore_ascii_case("binary"))
            {
                return Err("MIME attachment is not binary".into());
            }
            let matching: Vec<_> = metadata
                .iter()
                .filter(|item| {
                    item.get("sha2")
                        .and_then(Value::as_str)
                        .is_some_and(|value| value.eq_ignore_ascii_case(hash))
                })
                .collect();
            if matching.is_empty() {
                return Err("MIME part has no matching attachment metadata".into());
            }
            for item in matching {
                if !item
                    .get("length")
                    .is_some_and(|value| json_equal(value, &serde_json::json!(data.len())))
                {
                    return Err("MIME data length differs from attachment metadata".into());
                }
                if let Some(kind) = mime_header(part_headers, "Content-Type")
                    && item.get("contentType").and_then(Value::as_str) != Some(kind)
                {
                    return Err("MIME content type differs from attachment metadata".into());
                }
            }
            if let Some(length) = mime_header(part_headers, "Content-Length")
                && length.parse::<usize>().ok() != Some(data.len())
            {
                return Err("MIME Content-Length differs from raw data".into());
            }
            data_parts.push((hash.to_owned(), data));
        }
    }
    if positions.len() < 2 {
        return Err("incomplete multipart response".into());
    }
    for item in &metadata {
        if item.get("fileUrl").is_none()
            && !data_parts.iter().any(|(hash, _)| {
                item.get("sha2")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value.eq_ignore_ascii_case(hash))
            })
        {
            return Err("response omits attachment data without a fileUrl".into());
        }
    }
    for expected in expected_data {
        if !data_parts
            .iter()
            .any(|(_, part)| *part == expected.as_slice())
        {
            return Err("multipart response omitted or altered expected attachment data".into());
        }
    }
    Ok(())
}

fn timestamp_preserved(actual: OffsetDateTime, expected: OffsetDateTime, rounding: bool) -> bool {
    if actual == expected {
        return true;
    }
    if !rounding {
        return false;
    }
    // Try every decimal precision permitted by the source, never below milliseconds.
    let actual = actual.unix_timestamp_nanos();
    let expected = expected.unix_timestamp_nanos();
    [1_000_000, 100_000, 10_000, 1_000, 100, 10, 1]
        .into_iter()
        .any(|resolution| {
            let truncated = expected.div_euclid(resolution) * resolution;
            let remainder = expected.rem_euclid(resolution);
            actual == truncated || remainder * 2 >= resolution && actual == truncated + resolution
        })
}

pub(super) fn decode_hex(hex: &str) -> Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) || !hex.is_ascii() {
        return Err("hex byte string must contain pairs of ASCII digits".into());
    }
    (0..hex.len())
        .step_by(2)
        .map(|offset| {
            u8::from_str_radix(&hex[offset..offset + 2], 16).map_err(|error| error.to_string())
        })
        .collect()
}

pub(super) fn json_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => normalized_number(&a.to_string())
            .zip(normalized_number(&b.to_string()))
            .is_some_and(|(a, b)| a == b),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| json_equal(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| json_equal(value, other)))
        }
        _ => left == right,
    }
}

// Compare JSON numeric values without collapsing neighboring large integers
// through a binary64 conversion. Integer and decimal spellings may differ.
fn normalized_number(text: &str) -> Option<(bool, String, i64)> {
    let (mantissa, exponent) =
        text.split_once(['e', 'E'])
            .map_or(Some((text, 0)), |(value, exponent)| {
                exponent
                    .parse::<i64>()
                    .ok()
                    .map(|exponent| (value, exponent))
            })?;
    let negative = mantissa.starts_with('-');
    let unsigned = mantissa.trim_start_matches('-');
    let fraction = unsigned.split_once('.').map_or(0, |(_, part)| part.len());
    let digits: String = unsigned.chars().filter(|value| *value != '.').collect();
    let leading_trimmed = digits.trim_start_matches('0');
    if leading_trimmed.is_empty() {
        return Some((false, "0".into(), 0));
    }
    let coefficient = leading_trimmed.trim_end_matches('0');
    let scale = exponent
        .checked_sub(i64::try_from(fraction).ok()?)?
        .checked_add(i64::try_from(leading_trimmed.len() - coefficient.len()).ok()?)?;
    Some((negative, coefficient.into(), scale))
}

fn validate_pointer(pointer: &str) -> Result<(), String> {
    if !pointer.is_empty() && !pointer.starts_with('/') {
        return Err("JSON pointer must start with /".into());
    }
    for token in pointer.split('/') {
        let mut characters = token.chars();
        while let Some(character) = characters.next() {
            if character == '~' && !matches!(characters.next(), Some('0' | '1')) {
                return Err("invalid JSON pointer escape".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_iri_grammar_accepts_legal_alternatives_and_rejects_single_defects() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/conformance/fixtures/identifier-grammar.json"
        ))
        .expect("independent RFC grammar vectors");
        for row in fixture["cases"].as_array().expect("grammar cases") {
            let text = row["value"].as_str().expect("identifier string");
            for format in ["uri", "iri"] {
                assert_eq!(
                    check_string_format(format, text, "/identifier").is_ok(),
                    row[format].as_bool().expect("source-derived verdict"),
                    "{} as {format}",
                    row["id"]
                );
            }
        }
    }

    #[test]
    fn person_identifiers_check_every_enrichment_entry() {
        let check: super::Check = serde_yaml::from_str("kind: json-shape\nshape:\n  type: object\n  properties:\n    mbox: {type: array, items: {type: string, format: mailto-iri}}\n    openid: {type: array, items: {type: string, format: uri}}\n    mbox_sha1sum: {type: array, items: {type: string, format: sha1}}")
            .expect("Person identifier shape");
        let valid = serde_json::json!({"mbox": ["mailto:a@example.org", "MAILTO:b@example.org"], "openid": ["urn:example:person", "scheme:"], "mbox_sha1sum": ["0123456789abcdef0123456789abcdef01234567"]});
        assert!(
            check
                .evaluate(
                    200,
                    &reqwest::header::HeaderMap::new(),
                    valid.to_string().as_bytes(),
                    &std::collections::BTreeMap::new()
                )
                .is_empty()
        );
        let invalid = serde_json::json!({"mbox": ["mailto:a@example.org", "not-a-mailto-iri"]});
        let errors = check.evaluate(
            200,
            &reqwest::header::HeaderMap::new(),
            invalid.to_string().as_bytes(),
            &std::collections::BTreeMap::new(),
        );
        assert!(errors.iter().any(|error| error.contains("/mbox/1")));
        let invalid = serde_json::json!({"openid": ["https://[::1]/person", "https://["]});
        let errors = check.evaluate(
            200,
            &reqwest::header::HeaderMap::new(),
            invalid.to_string().as_bytes(),
            &std::collections::BTreeMap::new(),
        );
        assert!(errors.iter().any(|error| error.contains("/openid/1")));
    }

    #[test]
    fn utc_output_distinguishes_unknown_offset_from_normalized_zero() {
        let check: super::Check = serde_yaml::from_str(
            "kind: json-time\npointer: /timestamp\nexpected: 2026-07-29T09:00:00Z\nutc: true",
        )
        .expect("UTC check");
        for timestamp in ["2026-07-29T09:00:00Z", "2026-07-29T09:00:00+00:00"] {
            let body = serde_json::json!({"timestamp": timestamp}).to_string();
            assert!(
                check
                    .evaluate(
                        200,
                        &reqwest::header::HeaderMap::new(),
                        body.as_bytes(),
                        &std::collections::BTreeMap::new()
                    )
                    .is_empty()
            );
        }
        for timestamp in ["2026-07-29T09:00:00-00:00", "2026-07-29T10:00:00+01:00"] {
            let body = serde_json::json!({"timestamp": timestamp}).to_string();
            assert_eq!(
                check.evaluate(
                    200,
                    &reqwest::header::HeaderMap::new(),
                    body.as_bytes(),
                    &std::collections::BTreeMap::new()
                ),
                vec!["/timestamp is not returned in UTC"]
            );
        }
    }

    #[test]
    fn successful_alternative_requires_its_json_body() -> Result<(), Box<dyn std::error::Error>> {
        let check: Check = serde_yaml::from_str(
            "kind: json-shape\nstatus: 200\nshape:\n  type: array\n  min_items: 1\n  max_items: 1\n  items: {type: string, format: uuid}",
        )?;
        check.validate()?;
        let headers = HeaderMap::new();
        let captures = BTreeMap::new();
        for status in [204, 409] {
            assert!(check.evaluate(status, &headers, b"", &captures).is_empty());
        }
        assert!(
            check
                .evaluate(
                    200,
                    &headers,
                    br#"["00000000-0000-4000-8000-000000000001"]"#,
                    &captures
                )
                .is_empty()
        );
        for bytes in [b"".as_slice(), b"[]", br#"["wrong"]"#, b"null"] {
            assert!(!check.evaluate(200, &headers, bytes, &captures).is_empty());
        }
        Ok(())
    }

    #[test]
    fn timestamp_precision_allows_only_permitted_rounding() -> Result<(), Box<dyn std::error::Error>>
    {
        let expected = parse_time("2025-01-01T00:00:00.123456789Z")?;
        for text in ["2025-01-01T00:00:00.123Z", "2025-01-01T00:00:00.1235Z"] {
            assert!(timestamp_preserved(parse_time(text)?, expected, true));
        }
        for text in ["2025-01-01T00:00:00.12Z", "2025-01-01T00:00:00.124Z"] {
            assert!(!timestamp_preserved(parse_time(text)?, expected, true));
        }
        assert!(!timestamp_preserved(
            parse_time("2025-01-01T00:00:00.123Z")?,
            expected,
            false
        ));
        let expected = parse_time("2025-01-01T00:00:00.999987Z")?;
        assert!(timestamp_preserved(
            parse_time("2025-01-01T00:00:01Z")?,
            expected,
            true
        ));
        Ok(())
    }

    #[test]
    fn numeric_spellings_compare_without_losing_large_integer_distinctions()
    -> Result<(), serde_json::Error> {
        let integer: Value = serde_json::from_str("{\"number\":1}")?;
        let decimal: Value = serde_json::from_str("{\"number\":1.0}")?;
        assert!(json_equal(&integer, &decimal));
        let lower: Value = serde_json::from_str("9007199254740992")?;
        let upper: Value = serde_json::from_str("9007199254740993")?;
        assert!(!json_equal(&lower, &upper));
        Ok(())
    }

    #[test]
    fn iso_dates_and_multipart_binary_framing_are_validated()
    -> Result<(), Box<dyn std::error::Error>> {
        assert!(parse_time("20251005T010203+0900").is_ok());
        let data = b"data --boundary-like text\0\r\n--boundary--not-a-delimiter\r\nend";
        let hash = format!("{:x}", Sha256::digest(data));
        let metadata = serde_json::json!({"attachments":[{"sha2":hash,"length":data.len(),"contentType":"text/plain"}]});
        let mut bytes = format!("preamble\r\n--boundary \t\r\nContent-Type:\r\n application/json\r\n\r\n{metadata}\r\n--boundary\t\r\nContent-Type: text/plain\r\nContent-Transfer-Encoding: binary\r\nX-Experience-API-Hash: {hash}\r\n\r\n").into_bytes();
        bytes.extend(data);
        bytes.extend(b"\r\n--boundary--\t \r\nepilogue");
        let mut headers = HeaderMap::new();
        headers.insert(
            "Content-Type",
            "multipart/mixed; BOUNDARY=\"boundary\"".parse()?,
        );
        let check: Check = serde_yaml::from_str("kind: multipart")?;
        assert!(
            check
                .evaluate(200, &headers, &bytes, &BTreeMap::new())
                .is_empty()
        );
        let offset = bytes
            .windows(data.len())
            .position(|window| window == data)
            .ok_or("missing test data")?;
        bytes[offset] = b'X';
        assert!(
            !check
                .evaluate(200, &headers, &bytes, &BTreeMap::new())
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn last_modified_compares_the_maximum_stored_time() -> Result<(), Box<dyn std::error::Error>> {
        let check: Check = serde_yaml::from_str(
            "kind: header-time\nheader: Last-Modified\nmax_pointer: /statements",
        )?;
        let body = br#"{"statements":[{"stored":"2026-10-05T00:00:01.123Z"},{"stored":"2026-10-05T00:00:02.456Z"}]}"#;
        let mut headers = HeaderMap::new();
        headers.insert("Last-Modified", "Mon, 05 Oct 2026 00:00:02 GMT".parse()?);
        assert!(
            check
                .evaluate(200, &headers, body, &BTreeMap::new())
                .is_empty()
        );
        headers.insert("Last-Modified", "Tue, 01 Jan 1980 00:00:00 GMT".parse()?);
        assert!(
            !check
                .evaluate(200, &headers, body, &BTreeMap::new())
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn shape_rejects_missing_required_and_wrongly_typed_array_members()
    -> Result<(), Box<dyn std::error::Error>> {
        let check: Check = serde_yaml::from_str(
            "kind: json-shape\nshape:\n  type: object\n  required: [version]\n  properties:\n    version:\n      type: array\n      items: {type: string}",
        )?;
        for bytes in [br"{}".as_slice(), br#"{"version":[1]}"#.as_slice()] {
            assert!(
                !check
                    .evaluate(200, &HeaderMap::new(), bytes, &BTreeMap::new())
                    .is_empty()
            );
        }
        assert!(
            check
                .evaluate(
                    200,
                    &HeaderMap::new(),
                    br#"{"version":["2.0.0"]}"#,
                    &BTreeMap::new()
                )
                .is_empty()
        );
        Ok(())
    }
}

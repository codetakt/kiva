//! Response relationships and table-shape assertions for neutral HTTP cases.

#![expect(
    clippy::redundant_pub_crate,
    reason = "Assertion types are shared only with the containing runner module"
)]

use std::collections::{BTreeMap, BTreeSet};

use reqwest::header::HeaderMap;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha224, Sha256, Sha384, Sha512};
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
        #[serde(default)]
        agent_identity: bool,
    },
    JsonPreservation {
        expected: Value,
    },
    StatementQuery {
        stored_captures: BTreeMap<String, String>,
        #[serde(default)]
        since_capture: Option<String>,
        #[serde(default)]
        until_capture: Option<String>,
        #[serde(default)]
        descending: bool,
        #[serde(default)]
        limit: usize,
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
        expected: Value,
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
        if let Self::StatementQuery {
            stored_captures,
            since_capture,
            until_capture,
            ..
        } = self
        {
            for name in stored_captures
                .values()
                .chain(since_capture.iter())
                .chain(until_capture.iter())
            {
                if !names.contains(name) {
                    return Err(format!("undefined check capture {name}"));
                }
            }
        }
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
            Self::StatementQuery {
                stored_captures, ..
            } => {
                if stored_captures.is_empty() {
                    return Err(
                        "statement-query needs independently retrieved fixture timestamps".into(),
                    );
                }
            }
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
                if !expected.is_number() {
                    return Err("expected value must be a JSON number".into());
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
            Self::JsonPreservation { expected } => {
                if !expected.is_object()
                    || ["actor", "verb", "object"]
                        .iter()
                        .any(|key| expected.get(key).is_none())
                {
                    return Err("json-preservation requires a submitted Statement object".into());
                }
            }
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

    /// Preserve obligations decidable from received headers when the body is unavailable.
    pub(super) fn evaluate_without_body(&self, headers: &HeaderMap) -> Vec<String> {
        let Self::HeaderTime {
            header,
            max_pointer,
            optional_if_empty,
            ..
        } = self
        else {
            return Vec::new();
        };
        // Missing optional max(empty) is not a definite violation without the
        // array. A supplied header must still have valid HTTP-date syntax.
        if max_pointer.is_some() && *optional_if_empty && !headers.contains_key(header.as_str()) {
            return Vec::new();
        }
        header_time(headers, header).err().into_iter().collect()
    }

    /// A declared `StatementResult` order also constrains the observed page prefix.
    pub(super) fn evaluate_collected_order(&self, statements: &[Value]) -> Vec<String> {
        if let Self::JsonOrder {
            pointer,
            by,
            descending,
        } = self
            && pointer == "/statements"
        {
            return json_order(statements, pointer, by, *descending)
                .err()
                .into_iter()
                .collect();
        }
        Vec::new()
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
            Self::StatementQuery {
                stored_captures,
                since_capture,
                until_capture,
                descending,
                limit,
            } => {
                // Structure, membership and limits are independent of the ISO adapter.
                let array = json
                    .get("statements")
                    .and_then(Value::as_array)
                    .ok_or("statement-query needs statements array")?;
                if *limit != 0 && array.len() > *limit {
                    return Err("statement-query exceeds requested limit".into());
                }
                let mut unavailable = None;
                let captured_time = |name: &String| -> Result<ExactTime, String> {
                    exact_time(
                        captures
                            .get(name)
                            .ok_or_else(|| format!("missing capture {name}"))?,
                    )
                };
                let since = match since_capture {
                    Some(name) => defer_unavailable(captured_time(name), &mut unavailable)?,
                    None => None,
                };
                let until = match until_capture {
                    Some(name) => defer_unavailable(captured_time(name), &mut unavailable)?,
                    None => None,
                };
                // Keep possibly eligible entries as well as definitely eligible ones.
                // A missing bound and an unavailable bound are different facts.
                let mut eligible = BTreeMap::new();
                for (id, capture) in stored_captures {
                    let time = defer_unavailable(captured_time(capture), &mut unavailable)?;
                    let excluded = time.as_ref().is_some_and(|time| {
                        since.as_ref().is_some_and(|bound| time <= bound)
                            || until.as_ref().is_some_and(|bound| time > bound)
                    });
                    if !excluded {
                        let definite = (since_capture.is_none()
                            || time
                                .as_ref()
                                .zip(since.as_ref())
                                .is_some_and(|(t, b)| t > b))
                            && (until_capture.is_none()
                                || time
                                    .as_ref()
                                    .zip(until.as_ref())
                                    .is_some_and(|(t, b)| t <= b));
                        eligible.insert(id.as_str(), (time, definite));
                    }
                }
                let bounded_count = |count: usize| {
                    if *limit == 0 {
                        count
                    } else {
                        count.min(*limit)
                    }
                };
                let maximum_count = bounded_count(eligible.len());
                let minimum_count =
                    bounded_count(eligible.values().filter(|(_, definite)| *definite).count());
                if array.len() > maximum_count {
                    return Err("statement-query exceeds eligible set or requested limit".into());
                }
                // Fewer results are legal per page, with a more IRL for known omissions.
                if array.len() < minimum_count
                    && json
                        .get("more")
                        .and_then(Value::as_str)
                        .is_none_or(str::is_empty)
                {
                    return Err("statement-query omits eligible results without pagination".into());
                }
                let mut previous_returned = None;
                for statement in array {
                    let id = statement
                        .get("id")
                        .and_then(Value::as_str)
                        .ok_or("statement-query missing id")?;
                    let (time, _) = eligible
                        .get(id)
                        .ok_or(
                            "statement-query returned duplicate, unrelated or boundary-excluded id",
                        )?
                        .clone();
                    let returned =
                        defer_unavailable(json_time(statement.get("stored")), &mut unavailable)?;
                    if time
                        .as_ref()
                        .zip(returned.as_ref())
                        .is_some_and(|(a, b)| a != b)
                    {
                        return Err("statement-query changed captured stored value".into());
                    }
                    if returned.as_ref().is_some_and(|time| {
                        since.as_ref().is_some_and(|bound| time <= bound)
                            || until.as_ref().is_some_and(|bound| time > bound)
                    }) {
                        return Err(
                            "statement-query returned stored outside the requested boundary".into(),
                        );
                    }
                    // A compliant result must preserve captured stored. After a known
                    // inequality has failed above, either known side supplies a safe
                    // ordering witness: otherwise preservation itself would be violated.
                    let known_time = time.as_ref().or(returned.as_ref());
                    if previous_returned
                        .as_ref()
                        .zip(known_time)
                        .is_some_and(|(a, b)| if *descending { a < b } else { a > b })
                    {
                        return Err("statement-query is not stored-ordered".into());
                    }
                    if known_time.is_some_and(|time| {
                        eligible.values().any(|(other, definite)| {
                            *definite
                                && other.as_ref().is_some_and(|other| {
                                    if *descending {
                                        other > time
                                    } else {
                                        other < time
                                    }
                                })
                        })
                    }) {
                        return Err("statement-query is not a stored-ordered prefix".into());
                    }
                    if let Some(time) = known_time {
                        previous_returned = Some(time.clone());
                    }
                    eligible.remove(id);
                }
                if let Some(error) = unavailable {
                    return Err(error);
                }
            }
            Self::JsonPreservation { expected } => {
                super::preservation::compare_statement(expected, &json)?;
            }
            Self::HeaderTime {
                header,
                equals_pointer,
                max_pointer,
                optional_if_empty,
            } => {
                let sources: Vec<Option<&Value>> = if let Some(pointer) = equals_pointer {
                    vec![json.pointer(pointer)]
                } else if let Some(pointer) = max_pointer {
                    let array = json
                        .pointer(pointer)
                        .and_then(Value::as_array)
                        .ok_or_else(|| format!("{pointer} is not an array"))?;
                    if array.is_empty() && *optional_if_empty {
                        // No max(empty) exists. A supplied header still has HTTP-date syntax.
                        if headers.contains_key(header.as_str()) {
                            header_time(headers, header)?;
                        }
                        return Ok(());
                    }
                    array.iter().map(|item| item.get("stored")).collect()
                } else {
                    return Err("header-time requires a timestamp source".into());
                };
                // Presence and HTTP-date syntax do not depend on the ISO
                // provider's ability to compare the returned stored values.
                let actual = header_time(headers, header)?;
                let (times, unavailable) = json_times(sources.into_iter())?;
                let expected = times.iter().max();
                // Even with unknown values, the maximum cannot precede a
                // known member. Preserve that definite failure before deferral.
                if expected.is_some_and(|time| time.seconds > actual.unix_timestamp()) {
                    return Err(format!(
                        "{header} precedes a returned JSON timestamp at HTTP-date second precision"
                    ));
                }
                if let Some(error) = unavailable {
                    return Err(error);
                }
                let expected = expected.ok_or("cannot compare an empty timestamp collection")?;
                if actual.unix_timestamp() != expected.seconds {
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
                agent_identity,
            } => {
                let serialized_value = captures
                    .get(capture)
                    .ok_or_else(|| format!("missing capture {capture}"))?;
                let expected: Value =
                    serde_json::from_str(serialized_value).map_err(|error| error.to_string())?;
                let actual = json
                    .pointer(pointer)
                    .ok_or_else(|| format!("missing {pointer}"))?;
                let equal = if *agent_identity {
                    super::preservation::agent_identity_equal(actual, &expected)?
                } else {
                    json_equal(actual, &expected)?
                };
                if equal == *not_equal {
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
                json_order(array, pointer, by, *descending)?;
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
                let actual_text = value
                    .and_then(Value::as_str)
                    .ok_or("timestamp must be a string")?;
                let mut unavailable = None;
                let actual = defer_unavailable(parse_time(actual_text), &mut unavailable)?;
                // These UTC violations are decidable without comparing calendar instants.
                // The narrow suffix check does not claim a complete ISO grammar validator.
                if *utc
                    && (has_explicit_non_utc_offset(actual_text)
                        || actual.as_ref().is_some_and(|time| !time.offset().is_utc()))
                {
                    return Err(format!("{pointer} is not returned in UTC"));
                }
                if let Some(expected) = expected
                    && defer_unavailable(
                        timestamp_preserved(actual_text, expected, *allow_truncation_or_rounding),
                        &mut unavailable,
                    )? == Some(false)
                {
                    return Err(format!("{pointer} changed the submitted timestamp"));
                }
                if let Some(name) = equals_capture {
                    let value = captures
                        .get(name)
                        .ok_or_else(|| format!("missing capture {name}"))?;
                    if defer_unavailable(
                        timestamp_preserved(actual_text, value, false),
                        &mut unavailable,
                    )? == Some(false)
                    {
                        return Err(format!("{pointer} does not equal capture {name}"));
                    }
                }
                if let Some(pointer) = equals_pointer {
                    let expected = json
                        .pointer(pointer)
                        .and_then(Value::as_str)
                        .ok_or("compared timestamp must be a string")?;
                    if defer_unavailable(
                        timestamp_preserved(actual_text, expected, false),
                        &mut unavailable,
                    )? == Some(false)
                    {
                        return Err(format!("timestamp does not equal {pointer}"));
                    }
                }
                if let Some(error) = unavailable {
                    return Err(error);
                }
            }
            Self::NumericPrecision { pointer, expected } => {
                let actual = json
                    .pointer(pointer)
                    .ok_or_else(|| format!("{pointer} is not numeric"))?;
                if !super::preservation::numeric_precision(expected, actual)? {
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
                "uuid" | "timestamp" | "iri" | "irl" | "uri" | "mailto-iri" | "sha1"
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

    #[expect(
        clippy::too_many_lines,
        reason = "Ordered shape checks retain determinate violations while collecting unavailable comparisons"
    )]
    fn check(&self, value: &Value, pointer: &str) -> Result<(), String> {
        let mut unavailable = None;
        // Collect capability limits, but still inspect independently decidable
        // required fields and siblings. An unavailable IRL cannot hide a
        // missing name or a malformed second Person identifier.
        let mut retain = |result: Result<(), String>| -> Result<(), String> {
            match result {
                Err(error) if error.starts_with("ORACLE-UNAVAILABLE:") => {
                    unavailable = Some(error);
                    Ok(())
                }
                result => result,
            }
        };
        if let Some(format) = self.format.as_deref() {
            let text = value.as_str().ok_or("formatted value is not a string")?;
            retain(check_string_format(format, text, pointer))?;
        }
        if !self.any_of.is_empty() {
            let result =
                any_comparison(
                    self.any_of
                        .iter()
                        .map(|shape| match shape.check(value, pointer) {
                            Ok(()) => Ok(true),
                            Err(error) if error.starts_with("ORACLE-UNAVAILABLE:") => Err(error),
                            Err(_) => Ok(false),
                        }),
                );
            retain(result.and_then(|matches| {
                if matches {
                    Ok(())
                } else {
                    Err(format!("{pointer} does not satisfy any allowed JSON shape"))
                }
            }))?;
        }
        let correct_type = self.value_type.as_deref().is_none_or(|kind| match kind {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "number" | "integer" => value.is_number(),
            "boolean" => value.is_boolean(),
            "null" => value.is_null(),
            _ => false,
        });
        if !correct_type {
            return Err(format!("{pointer} has an invalid type"));
        }
        if self.value_type.as_deref() == Some("integer") {
            if value
                .as_f64()
                .is_some_and(|number| number.fract().abs() > 0.0)
            {
                return Err(format!("{pointer} is not an integer"));
            }
            // Decimal spelling may be integral; it needs the original token.
            retain(require_exact_numbers(value))?;
        }
        if let Some(allowed) = &self.enum_values {
            retain(
                any_comparison(allowed.iter().map(|item| json_equal(item, value))).and_then(
                    |matches| {
                        if matches {
                            Ok(())
                        } else {
                            Err(format!("{pointer} has an invalid enumerated value"))
                        }
                    },
                ),
            )?;
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
                    retain(shape.check(item, &format!("{pointer}/{name}")))?;
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
                    retain(shape.check(item, &format!("{pointer}/{index}")))?;
                }
            }
        }
        if self.minimum.is_some() || self.maximum.is_some() {
            match require_exact_numbers(value) {
                Err(error) => retain(Err(error))?,
                Ok(()) => {
                    if let Some(number) = value.as_f64()
                        && (self.minimum.is_some_and(|min| number < min)
                            || self.maximum.is_some_and(|max| number > max))
                    {
                        return Err(format!("{pointer} violates numeric bounds"));
                    }
                }
            }
        }
        unavailable.map_or(Ok(()), Err)
    }
}

fn check_string_format(format: &str, text: &str, pointer: &str) -> Result<(), String> {
    let valid = match format {
        "uuid" => standard_uuid(text),
        "timestamp" => return parse_time(text).map(|_| ()),
        "iri" => crate::identifier_has_valid_syntax(text, false),
        "uri" => crate::identifier_has_valid_syntax(text, true),
        "irl" => {
            if !crate::identifier_has_valid_syntax(text, false) {
                return Err(format!("{pointer} is not valid IRI syntax"));
            }
            return Err(format!(
                "ORACLE-UNAVAILABLE: {pointer} requires DC-IRL locator/access-mechanism evidence; IRI syntax alone is insufficient"
            ));
        }
        "mailto-iri" => {
            if !crate::identifier_has_valid_syntax(text, false) {
                false
            } else if let Some((scheme, rest)) = text.split_once(':') {
                scheme.eq_ignore_ascii_case("mailto") && mailbox_list(rest)?
            } else {
                false
            }
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

// Bounded RFC 6068 adapter. Encoded/quoted/internationalized addresses and
// hfields need the complete DC-MAILTO contract, never a generic-IRI PASS.
fn mailbox_list(rest: &str) -> Result<bool, String> {
    if rest.contains('#') {
        return Ok(false);
    }
    if rest.contains('%') || rest.contains('?') || !rest.is_ascii() {
        return Err(
            "ORACLE-UNAVAILABLE: DC-MAILTO percent/quoted/domain/hfield adapter required".into(),
        );
    }
    let atom = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!$'*+-_~".contains(&b))
    };
    Ok(!rest.is_empty()
        && rest.split(',').all(|address| {
            let Some((local, domain)) = address.rsplit_once('@') else {
                return false;
            };
            // Raw comments, whitespace, extra @, and non-atext fail this supported
            // ASCII dot-atom partition. No DNS lookup or delivery test is required.
            !local.is_empty()
                && !domain.is_empty()
                && local.split('.').all(&atom)
                && domain.split('.').all(&atom)
        }))
}

// Classify definite violations in ordinary calendar/clock spellings before an
// unsupported representation (leap seconds, end-of-day, etc.) is delegated.
// This bounded check is not a replacement for a complete ISO date/time provider.
fn check_common_time_components(value: &str) -> Result<(), String> {
    let Some((date, clock)) = value.split_once(['T', 't', ' ']) else {
        return Ok(());
    };
    let date_bytes = date.as_bytes();
    if !(date_bytes.len() == 8 && date_bytes.iter().all(u8::is_ascii_digit)
        || date_bytes.len() == 10
            && date_bytes[4] == b'-'
            && date_bytes[7] == b'-'
            && date_bytes
                .iter()
                .enumerate()
                .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit()))
    {
        return Ok(());
    }
    let date = date.replace('-', "");
    let year = date[..4].parse::<i32>().map_err(|e| e.to_string())?;
    let month = date[4..6].parse::<u8>().map_err(|e| e.to_string())?;
    let day = date[6..].parse::<u8>().map_err(|e| e.to_string())?;
    let month = time::Month::try_from(month).map_err(|e| e.to_string())?;
    time::Date::from_calendar_date(year, month, day).map_err(|e| e.to_string())?;

    let (local, zone) = clock
        .find(['Z', 'z', '+', '-'])
        .map_or((clock, ""), |index| clock.split_at(index));
    check_common_offset(zone)?;
    let (local, fraction) = local.split_once(['.', ',']).unwrap_or((local, ""));
    let clock_bytes = local.as_bytes();
    if !(matches!(clock_bytes.len(), 2 | 4 | 6) && clock_bytes.iter().all(u8::is_ascii_digit)
        || matches!(clock_bytes.len(), 5 | 8)
            && clock_bytes[2] == b':'
            && (clock_bytes.len() == 5 || clock_bytes[5] == b':')
            && clock_bytes
                .iter()
                .enumerate()
                .all(|(index, byte)| matches!(index, 2 | 5) || byte.is_ascii_digit()))
    {
        return Ok(());
    }
    let digits = local.replace(':', "");
    let hour = digits[..2].parse::<u8>().map_err(|e| e.to_string())?;
    let minute = digits
        .get(2..4)
        .unwrap_or("0")
        .parse::<u8>()
        .map_err(|e| e.to_string())?;
    let second = digits
        .get(4..6)
        .unwrap_or("0")
        .parse::<u8>()
        .map_err(|e| e.to_string())?;
    if hour > 24 || minute > 59 || second > 60 {
        return Err("invalid calendar/time component".into());
    }
    if hour == 24 && (minute != 0 || second != 0 || fraction.bytes().any(|digit| digit != b'0')) {
        return Err("end-of-day requires zero smaller components".into());
    }
    if hour == 24 {
        return Err("ORACLE-UNAVAILABLE: ISO end-of-day provider required".into());
    }
    Ok(())
}

fn check_common_offset(zone: &str) -> Result<(), String> {
    let Some(offset) = zone.strip_prefix(['+', '-']) else {
        return Ok(());
    };
    let bytes = offset.as_bytes();
    let ordinary = matches!(bytes.len(), 2 | 4) && bytes.iter().all(u8::is_ascii_digit)
        || bytes.len() == 5
            && bytes[2] == b':'
            && bytes
                .iter()
                .enumerate()
                .all(|(index, byte)| index == 2 || byte.is_ascii_digit());
    if ordinary {
        let digits = offset.replace(':', "");
        let hour = digits[..2].parse::<u8>().map_err(|e| e.to_string())?;
        let minute = digits
            .get(2..)
            .filter(|part| !part.is_empty())
            .unwrap_or("0")
            .parse::<u8>()
            .map_err(|e| e.to_string())?;
        if hour > 23 || minute > 59 {
            return Err("invalid timezone offset component".into());
        }
    }
    Ok(())
}

pub(super) fn parse_time(value: &str) -> Result<OffsetDateTime, String> {
    if !value.as_bytes().windows(3).any(|w| {
        matches!(w[0], b'T' | b't' | b' ') && w[1].is_ascii_digit() && w[2].is_ascii_digit()
    }) {
        return Err("timestamp requires explicit combined date and time".into());
    }
    check_common_time_components(value)?;
    let clock = value.rsplit(['T', 't', ' ']).next().unwrap_or("");
    let seconds_digits: String = clock
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ':')
        .filter(char::is_ascii_digit)
        .collect();
    if seconds_digits.len() == 6 && &seconds_digits[4..] == "60" {
        return Err("ORACLE-UNAVAILABLE: leap-second identity needs delegated ISO provider; no clamp to second 59".into());
    }
    // The lossless fraction comparator below represents fractions of seconds.
    // Fractional hours/minutes require a richer ISO provider, not this adapter.
    if let Some((prefix, _)) = value.split_once(['.', ',']) {
        let clock = prefix.rsplit(['T', 't', ' ']).next().unwrap_or("");
        let digits = clock.bytes().filter(u8::is_ascii_digit).count();
        if digits != 6 {
            return Err(
                "ORACLE-UNAVAILABLE: fractional hours/minutes require delegated ISO provider"
                    .into(),
            );
        }
    }
    OffsetDateTime::parse(value, &Rfc3339)
        .or_else(|_| OffsetDateTime::parse(value, &Iso8601::DEFAULT))
        .map_err(|error| format!("ORACLE-UNAVAILABLE: ISO date/time provider cannot classify this representation: {error}"))
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ExactTime {
    seconds: i64,
    fraction: String,
}

fn exact_time(value: &str) -> Result<ExactTime, String> {
    Ok(ExactTime {
        seconds: parse_time(value)?.unix_timestamp(),
        // Trailing zeros are insignificant. Lexical ordering of these decimal
        // fractions is exact, including proper prefixes and negative epochs.
        fraction: timestamp_fraction(value),
    })
}

pub(super) fn compare_timestamps(left: &str, right: &str) -> Result<std::cmp::Ordering, String> {
    Ok(exact_time(left)?.cmp(&exact_time(right)?))
}

pub(super) fn standard_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

fn json_time(value: Option<&Value>) -> Result<ExactTime, String> {
    exact_time(
        value
            .and_then(Value::as_str)
            .ok_or("missing JSON timestamp string")?,
    )
}

/// Only provider uncertainty is deferred; definite malformed values still fail.
fn defer_unavailable<T>(
    result: Result<T, String>,
    unavailable: &mut Option<String>,
) -> Result<Option<T>, String> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.starts_with("ORACLE-UNAVAILABLE:") => {
            unavailable.get_or_insert(error);
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

pub(super) fn has_explicit_non_utc_offset(text: &str) -> bool {
    let Some((_, clock)) = text.rsplit_once(['T', 't', ' ']) else {
        return false;
    };
    if !clock
        .as_bytes()
        .get(..2)
        .is_some_and(|digits| digits.iter().all(u8::is_ascii_digit))
    {
        return false;
    }
    let Some(start) = clock.rfind(['+', '-']) else {
        return false;
    };
    let suffix = &clock.as_bytes()[start..];
    let digits = match suffix {
        [_, h1, h2] => vec![*h1, *h2],
        [_, h1, h2, m1, m2] | [_, h1, h2, b':', m1, m2] => vec![*h1, *h2, *m1, *m2],
        _ => return false,
    };
    // Recognize extended/basic/reduced offsets independently of ISO parsing.
    // Unknown-local-offset negative zero does not satisfy returned UTC.
    digits.iter().all(u8::is_ascii_digit)
        && (suffix[0] == b'-' || digits.iter().any(|b| *b != b'0'))
}

fn json_times<'a>(
    values: impl Iterator<Item = Option<&'a Value>>,
) -> Result<(Vec<ExactTime>, Option<String>), String> {
    let mut times = Vec::new();
    let mut unavailable = None;
    for value in values {
        match json_time(value) {
            Ok(time) => times.push(time),
            Err(error) if error.starts_with("ORACLE-UNAVAILABLE:") => {
                unavailable.get_or_insert(error);
            }
            Err(error) => return Err(error),
        }
    }
    Ok((times, unavailable))
}

fn json_order(array: &[Value], pointer: &str, by: &str, descending: bool) -> Result<(), String> {
    let (times, unavailable) = json_times(array.iter().map(|item| item.get(by)))?;
    // A monotonic sequence has a monotonic known subsequence, even across
    // page boundaries or legal timestamps that require provider support.
    if times.windows(2).any(|pair| {
        if descending {
            pair[0] < pair[1]
        } else {
            pair[0] > pair[1]
        }
    }) {
        return Err(format!("{pointer} is not ordered by {by}"));
    }
    if let Some(error) = unavailable {
        return Err(error);
    }
    Ok(())
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
        .split("\r\n")
        .filter_map(|line| line.split_once(':'))
        .find(|(key, _)| key.trim().eq_ignore_ascii_case(name))
        .map(|(_, value)| value.trim_matches([' ', '\t']))
}

fn unfold_mime_headers(headers: &str) -> Result<String, String> {
    if !headers.is_ascii() {
        return Err("MIME part headers must be US-ASCII".into());
    }
    let mut result = String::new();
    for line in headers.split("\r\n") {
        // Only CRLF delimits a physical field line. Quoted CR and bare LF
        // can be RFC 822 field data; the structured field parser checks them.
        if line.starts_with([' ', '\t']) {
            if result.is_empty() {
                return Err("MIME continuation has no preceding field".into());
            }
            // RFC 822 unfolding removes CRLF, preserving following LWSP.
        } else {
            let (name, _) = line.split_once(':').ok_or("missing MIME field colon")?;
            if name.is_empty() || !name.bytes().all(|b| (33..=126).contains(&b) && b != b':') {
                return Err("invalid MIME field name".into());
            }
            if !result.is_empty() {
                result.push_str("\r\n");
            }
        }
        result.push_str(line);
    }
    Ok(result)
}

fn verify_multipart(
    headers: &HeaderMap,
    bytes: &[u8],
    expected_data: &[Vec<u8>],
) -> Result<(), String> {
    multipart_document(headers, bytes, expected_data).map(|_| ())
}

#[expect(
    clippy::too_many_lines,
    reason = "MIME framing, hashes, and metadata are checked as a single ordered transmission"
)]
pub(super) fn multipart_document(
    headers: &HeaderMap,
    bytes: &[u8],
    expected_data: &[Vec<u8>],
) -> Result<Value, String> {
    let content_type = headers
        .get("Content-Type")
        .and_then(|value| value.to_str().ok())
        .ok_or("missing multipart Content-Type")?;
    let parsed_type = super::media_type::parse_http(content_type)?;
    if parsed_type.essence != "multipart/mixed" {
        return Err("response is not multipart/mixed".into());
    }
    let boundary = parsed_type
        .parameters
        .get("boundary")
        .ok_or("missing MIME boundary")?;
    if parsed_type.ambiguous.contains("boundary") {
        return Err(
            "ORACLE-UNAVAILABLE: ambiguous MIME boundary prevents safe part framing".into(),
        );
    }
    if boundary.is_empty()
        || boundary.len() > 70
        || boundary.ends_with(' ')
        || !boundary
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"'()+_,-./:=? ".contains(&b))
    {
        return Err("invalid MIME boundary bchars/length/trailing space".into());
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
            if !positions.is_empty() {
                return Err("MIME boundary prefix collision inside a part".into());
            }
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
    let mut document = None;
    // A local numeric or media-type limit must not hide an independent violation.
    // Keep it pending until later metadata, parts and required raw data are checked.
    let mut unavailable = parsed_type.unavailable();
    for (number, pair) in positions.windows(2).enumerate() {
        let part = &bytes[pair[0].1..pair[1].0];
        let split = part
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or("missing MIME part headers")?;
        let raw_headers = std::str::from_utf8(&part[..split]).map_err(|error| error.to_string())?;
        let unfolded = unfold_mime_headers(raw_headers)?;
        let part_headers = unfolded.as_str();
        let data = part[split + 4..]
            .strip_suffix(b"\r\n")
            .ok_or("invalid MIME part termination")?;
        if number == 0 {
            // Absent Content-Type defaults to text/plain under RFC2045, so
            // it cannot meet xAPI's first-part application/json requirement.
            match super::media_type::parse(
                mime_header(part_headers, "Content-Type").unwrap_or("text/plain; charset=us-ascii"),
            ) {
                Ok(kind) if kind.essence == "application/json" => {
                    if let Some(error) = kind.unavailable() {
                        unavailable.get_or_insert(error);
                    }
                }
                Ok(_) => return Err("first MIME part is not application/json".into()),
                Err(error) if error.starts_with("ORACLE-UNAVAILABLE:") => {
                    unavailable.get_or_insert(error);
                }
                Err(error) => return Err(error),
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
            document = Some(json);
        } else {
            let hash = mime_header(part_headers, "X-Experience-API-Hash")
                .ok_or("missing MIME attachment hash")?;
            let digest = match hash.len() {
                56 => format!("{:x}", Sha224::digest(data)),
                64 => format!("{:x}", Sha256::digest(data)),
                96 => format!("{:x}", Sha384::digest(data)),
                128 => format!("{:x}", Sha512::digest(data)),
                _ => return Err("unsupported SHA-2 representation".into()),
            };
            if !digest.eq_ignore_ascii_case(hash) {
                return Err("MIME attachment hash does not match raw data".into());
            }
            let encoding = mime_header(part_headers, "Content-Transfer-Encoding")
                .ok_or("missing MIME attachment transfer encoding")?;
            if !super::media_type::transfer_encoding(encoding)?.eq_ignore_ascii_case("binary") {
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
                let length = item.get("length").ok_or("missing attachment length")?;
                match json_equal(length, &serde_json::json!(data.len())) {
                    Ok(true) => {}
                    Ok(false) => {
                        return Err("MIME data length differs from attachment metadata".into());
                    }
                    Err(error) if error.starts_with("ORACLE-UNAVAILABLE:") => {
                        unavailable.get_or_insert(error);
                    }
                    Err(error) => return Err(error),
                }
                if let Some(kind) = mime_header(part_headers, "Content-Type") {
                    match super::media_type::equivalent(
                        item.get("contentType")
                            .and_then(Value::as_str)
                            .ok_or("missing attachment contentType")?,
                        kind,
                    ) {
                        Ok(true) => {}
                        Ok(false) => {
                            return Err("MIME content type differs from attachment metadata".into());
                        }
                        Err(error) if error.starts_with("ORACLE-UNAVAILABLE:") => {
                            unavailable.get_or_insert(error);
                        }
                        Err(error) => return Err(error),
                    }
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
    if let Some(error) = unavailable {
        return Err(error);
    }
    document.ok_or_else(|| "missing first MIME JSON part".into())
}

pub(super) fn timestamp_preserved(
    actual: &str,
    expected: &str,
    rounding: bool,
) -> Result<bool, String> {
    let mut unavailable = None;
    let actual_time = defer_unavailable(parse_time(actual), &mut unavailable)?;
    let expected_time = defer_unavailable(parse_time(expected), &mut unavailable)?;
    let (Some(actual_time), Some(expected_time)) = (actual_time, expected_time) else {
        return Err(unavailable.unwrap_or_else(|| {
            "ORACLE-UNAVAILABLE: timestamp comparison requires both instants".into()
        }));
    };
    let actual_second = actual_time.unix_timestamp();
    let expected_second = expected_time.unix_timestamp();
    let actual_fraction = timestamp_fraction(actual);
    let expected_fraction = timestamp_fraction(expected);
    if actual_second == expected_second && actual_fraction == expected_fraction {
        return Ok(true);
    }
    if rounding {
        // Decimal digits remain strings: neither binary64 nor nanosecond
        // truncation may invent equality for finer input/output precision.
        for precision in 3..=expected_fraction.len() {
            let prefix = &expected_fraction[..precision];
            if actual_second == expected_second && actual_fraction == prefix.trim_end_matches('0') {
                return Ok(true);
            }
            if expected_fraction
                .as_bytes()
                .get(precision)
                .is_some_and(|digit| *digit >= b'5')
            {
                let (carry, rounded) = increment_fraction(prefix);
                if actual_second == expected_second + i64::from(carry)
                    && actual_fraction == rounded.trim_end_matches('0')
                {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn timestamp_fraction(text: &str) -> String {
    text.split_once(['.', ','])
        .map_or_else(String::new, |(_, suffix)| {
            suffix
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .trim_end_matches('0')
                .to_owned()
        })
}

fn increment_fraction(text: &str) -> (bool, String) {
    let mut digits = text.as_bytes().to_vec();
    let mut carry = true;
    for digit in digits.iter_mut().rev() {
        if !carry {
            break;
        }
        if *digit == b'9' {
            *digit = b'0';
        } else {
            *digit += 1;
            carry = false;
        }
    }
    (carry, digits.into_iter().map(char::from).collect())
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

/// A binary64 Value is not evidence of the original JSON number token.
/// Until a lossless decimal adapter is supplied, never turn it into PASS/FAIL
/// by serializing the rounded Value back to text. Integer Values remain exact.
pub(super) fn require_exact_numbers(value: &Value) -> Result<(), String> {
    match value {
        Value::Number(number) if number.is_f64() => Err(
            "ORACLE-UNAVAILABLE: original decimal/exponent token required for exact JSON comparison/capture".into(),
        ),
        Value::Array(values) => values.iter().try_for_each(require_exact_numbers),
        Value::Object(values) => values.values().try_for_each(require_exact_numbers),
        _ => Ok(()),
    }
}

/// For conjunction, a known difference dominates an unavailable comparison.
pub(super) fn all_comparisons(
    values: impl IntoIterator<Item = Result<bool, String>>,
) -> Result<bool, String> {
    let mut unavailable = None;
    for value in values {
        match value {
            Ok(false) => return Ok(false),
            Ok(true) => {}
            Err(error) => unavailable = Some(error),
        }
    }
    unavailable.map_or(Ok(true), Err)
}

/// For membership, a known match dominates an unavailable alternative.
pub(super) fn any_comparison(
    values: impl IntoIterator<Item = Result<bool, String>>,
) -> Result<bool, String> {
    let mut unavailable = None;
    for value in values {
        match value {
            Ok(true) => return Ok(true),
            Ok(false) => {}
            Err(error) => unavailable = Some(error),
        }
    }
    unavailable.map_or(Ok(false), Err)
}

pub(super) fn json_equal(left: &Value, right: &Value) -> Result<bool, String> {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => {
            require_exact_numbers(left)?;
            require_exact_numbers(right)?;
            let integer = |value: &serde_json::Number| {
                value
                    .as_i64()
                    .map(i128::from)
                    .or_else(|| value.as_u64().map(i128::from))
            };
            Ok(integer(a) == integer(b))
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Ok(false);
            }
            all_comparisons(a.iter().zip(b).map(|(a, b)| json_equal(a, b)))
        }
        (Value::Object(a), Value::Object(b)) => {
            if a.len() != b.len() {
                return Ok(false);
            }
            all_comparisons(a.iter().map(|(key, value)| {
                b.get(key)
                    .map_or(Ok(false), |other| json_equal(value, other))
            }))
        }
        _ => Ok(left == right),
    }
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
    fn preservation_definition_requires_the_submitted_statement() {
        let submitted = serde_json::json!({
            "actor": {"mbox": "mailto:test@example.org"},
            "verb": {"id": "https://example.org/verb"},
            "object": {"id": "https://example.org/activity"},
            "result": {"duration": "PT1S"}
        });
        assert!(
            Check::JsonPreservation {
                expected: submitted.clone()
            }
            .validate()
            .is_ok()
        );
        for key in ["actor", "verb", "object"] {
            let mut partial = submitted.clone();
            partial.as_object_mut().expect("Statement").remove(key);
            assert!(
                Check::JsonPreservation { expected: partial }
                    .validate()
                    .is_err()
            );
        }
        assert!(
            Check::JsonPreservation {
                expected: serde_json::json!({"result": {"duration": "PT1S"}})
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn exact_preservation_checks_additions_and_managed_field_boundaries()
    -> Result<(), serde_json::Error> {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/conformance/fixtures/preservation-properties.json"
        ))?;
        for row in fixture["cases"].as_array().expect("property controls") {
            let check = Check::JsonPreservation {
                expected: row["submitted"].clone(),
            };
            let errors = check.evaluate(
                200,
                &HeaderMap::new(),
                row["returned"].to_string().as_bytes(),
                &BTreeMap::new(),
            );
            let valid = row["preserved"].as_bool().expect("control result");
            assert_eq!(errors.is_empty(), valid, "{}: {errors:?}", row["id"]);
            if !valid {
                assert!(
                    errors.iter().any(|e| !e.starts_with("ORACLE-UNAVAILABLE:")),
                    "{}: addition must be a definite failure: {errors:?}",
                    row["id"]
                );
            }
        }
        Ok(())
    }

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
        for address in [
            "mailto:directory",
            "mailto:@example.org",
            "mailto:user@",
            "mailto:user..name@example.org",
        ] {
            assert!(super::check_string_format("mailto-iri", address, "/mbox/0").is_err());
        }
        for address in ["mailto:a+b@example.org", "MAILTO:user@example.org"] {
            assert!(super::check_string_format("mailto-iri", address, "/mbox/0").is_ok());
        }
        assert!(
            super::check_string_format(
                "mailto-iri",
                "mailto:%22not%20me%22@example.org",
                "/mbox/0"
            )
            .unwrap_err()
            .starts_with("ORACLE-UNAVAILABLE:")
        );
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
        let expected = "2025-01-01T00:00:00.123456789Z";
        for text in ["2025-01-01T00:00:00.123Z", "2025-01-01T00:00:00.1235Z"] {
            assert!(timestamp_preserved(text, expected, true)?);
        }
        for text in ["2025-01-01T00:00:00.12Z", "2025-01-01T00:00:00.124Z"] {
            assert!(!timestamp_preserved(text, expected, true)?);
        }
        assert!(!timestamp_preserved(
            "2025-01-01T00:00:00.123Z",
            expected,
            false
        )?);
        let expected = "2025-01-01T00:00:00.999987Z";
        assert!(timestamp_preserved("2025-01-01T00:00:01Z", expected, true)?);
        assert!(!timestamp_preserved(
            "2025-01-01T00:00:00.123000001Z",
            "2025-01-01T00:00:00.123Z",
            true
        )?);
        assert!(timestamp_preserved(
            "2025-01-01T00:00:00.123456782Z",
            "2025-01-01T00:00:00.1234567819Z",
            true
        )?);
        assert!(!timestamp_preserved(
            "2025-01-01T00:00:00.1234567811Z",
            "2025-01-01T00:00:00.1234567819Z",
            true
        )?);
        Ok(())
    }

    #[test]
    fn timestamp_cross_field_equality_keeps_subnanosecond_digits() -> Result<(), serde_json::Error>
    {
        let earlier = "2025-01-01T00:00:00.1234567811Z";
        let later = "2025-01-01T00:00:00.1234567819Z";
        let captures = std::collections::BTreeMap::from([("stored".into(), later.into())]);
        let headers = reqwest::header::HeaderMap::new();
        for field in ["equals_pointer", "equals_capture"] {
            let mut definition = serde_json::json!({"kind":"json-time", "pointer":"/timestamp"});
            definition[field] = serde_json::json!(if field == "equals_pointer" {
                "/stored"
            } else {
                "stored"
            });
            let check: Check = serde_json::from_value(definition)?;
            for (timestamp, valid) in [(earlier, false), (later, true)] {
                let body = serde_json::json!({"stored":later, "timestamp":timestamp}).to_string();
                assert_eq!(
                    check
                        .evaluate(200, &headers, body.as_bytes(), &captures)
                        .is_empty(),
                    valid
                );
            }
        }
        Ok(())
    }

    #[test]
    fn integer_endpoint_serializations_agree_across_numeric_and_preservation_checks()
    -> Result<(), serde_json::Error> {
        for (expected, actual, valid) in [
            ("100000008", "100000010", true),
            ("-100000008", "-100000010", true),
            ("100000007", "100000010", true),
            ("100000009", "100000020", true),
            ("-100000009", "-100000020", true),
            ("-9223372036854775808", "-9223372000000000000", true),
            ("18446744073709551615", "18446742970000000000", true),
            ("100000008", "100000008", true),
            ("16777217", "16777216", true),
            ("16777217", "16777218", true),
            ("0", "0", true),
            ("100000008", "100000009", false),
            ("-100000008", "-100000009", false),
            ("100000008", "100000000", false),
            ("100000008", "100000020", false),
            ("100000008", "0", false),
            ("100000008", "-100000010", false),
            ("9007199254740992", "9007199254740993", false),
            ("-9007199254740992", "-9007199254740993", false),
            ("1", "2", false),
            ("0", "1", false),
            // Exact decimal multiples have coincident floor and ceiling bounds.
            ("1000000000", "1000000000", true),
            ("-1000000000", "-1000000000", true),
            ("1000000000", "1000000010", false),
            ("1000000000", "999999990", false),
            ("-1000000000", "-999999990", false),
            ("-1000000000", "-1000000010", false),
            ("10000000000", "10000000010", false),
            ("-10000000000", "-9999999990", false),
            ("10000000000", "10000000100", false),
            ("-10000000000", "-9999999900", false),
            ("999999999", "1000000010", false),
            ("-1000000001", "-999999990", false),
        ] {
            let source: Value = serde_json::from_str(expected)?;
            let returned: Value = serde_json::from_str(actual)?;
            let submitted = serde_json::json!({"result":{"score":{"raw":source}}});
            let body = serde_json::json!({"result":{"score":{"raw":returned}}}).to_string();
            let failures = [
                Check::NumericPrecision {
                    pointer: "/result/score/raw".into(),
                    expected: source,
                },
                Check::JsonPreservation {
                    expected: submitted,
                },
            ]
            .map(|check| check.evaluate(200, &HeaderMap::new(), body.as_bytes(), &BTreeMap::new()));
            assert_eq!(
                failures.each_ref().map(Vec::is_empty),
                [valid; 2],
                "{expected} -> {actual} [numeric-precision, json-preservation]: {failures:?}"
            );
            for failures in failures {
                assert!(
                    failures
                        .iter()
                        .all(|error| !error.starts_with("ORACLE-UNAVAILABLE:"))
                );
            }
        }
        // Lost decimal/exponent tokens still require a lossless provider.
        for body in [
            br#"{"result":{"score":{"raw":100000010.0}}}"#.as_slice(),
            br#"{"result":{"score":{"raw":1.0000001e8}}}"#,
        ] {
            for check in [
                Check::NumericPrecision {
                    pointer: "/result/score/raw".into(),
                    expected: serde_json::json!(100_000_008),
                },
                Check::JsonPreservation {
                    expected: serde_json::json!({"result":{"score":{"raw":100_000_008}}}),
                },
            ] {
                let failures = check.evaluate(200, &HeaderMap::new(), body, &BTreeMap::new());
                assert_eq!(failures.len(), 1);
                assert!(failures[0].starts_with("ORACLE-UNAVAILABLE:"));
            }
        }
        Ok(())
    }

    #[test]
    fn numeric_spellings_compare_without_losing_large_integer_distinctions()
    -> Result<(), serde_json::Error> {
        let integer: Value = serde_json::from_str("{\"number\":1}")?;
        let decimal: Value = serde_json::from_str("{\"number\":1.0}")?;
        assert!(json_equal(&integer, &decimal).is_err());
        let lower: Value = serde_json::from_str("9007199254740992")?;
        let upper: Value = serde_json::from_str("9007199254740993")?;
        assert_eq!(json_equal(&lower, &upper), Ok(false));
        Ok(())
    }

    #[test]
    fn iso_dates_and_multipart_binary_framing_are_validated()
    -> Result<(), Box<dyn std::error::Error>> {
        assert!(parse_time("20251005T010203+0900").is_ok());
        let data = b"data --boundary-like text\0\r\n--boundary--not-a-delimiter\r\nend";
        let hash = format!("{:x}", Sha256::digest(data));
        let metadata = serde_json::json!({"attachments":[{"sha2":hash,"length":data.len(),"contentType":"text/plain"}]});
        let mut bytes = format!("preamble\r\n--outer-boundary \t\r\nContent-Type:\r\n application/json\r\n\r\n{metadata}\r\n--outer-boundary\t\r\nContent-Type: text/plain\r\nContent-Transfer-Encoding: binary\r\nX-Experience-API-Hash: {hash}\r\n\r\n").into_bytes();
        bytes.extend(data);
        bytes.extend(b"\r\n--outer-boundary--\t \r\nepilogue");
        let mut headers = HeaderMap::new();
        headers.insert(
            "Content-Type",
            "multipart/mixed; BOUNDARY=\"outer-boundary\"".parse()?,
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
    fn integer_shape_rejects_known_fractions_without_guessing_lost_tokens()
    -> Result<(), Box<dyn std::error::Error>> {
        let shape: super::Shape = serde_yaml::from_str("type: integer")?;
        for value in [serde_json::json!(1.5), serde_json::json!(-1.5)] {
            assert!(
                matches!(shape.check(&value, ""), Err(error) if !error.starts_with("ORACLE-UNAVAILABLE:"))
            );
        }
        assert!(shape.check(&serde_json::json!(1), "").is_ok());
        assert!(
            matches!(shape.check(&serde_json::json!(1.0), ""), Err(error) if error.starts_with("ORACLE-UNAVAILABLE:"))
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

//! JSON request helpers for xAPI payloads.

use std::collections::BTreeSet;

use axum::http::StatusCode;
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;

use crate::core::response::HttpError;

pub(crate) fn parse_json_without_duplicate_properties(
    body: &[u8],
    label: &str,
) -> Result<Value, HttpError> {
    let mut detector = serde_json::Deserializer::from_slice(body);
    NoDuplicateJsonProperties
        .deserialize(&mut detector)
        .map_err(|_error| {
            HttpError::new(
                StatusCode::BAD_REQUEST,
                format!("{label} must not contain duplicate JSON object properties"),
            )
        })?;
    detector.end().map_err(|_error| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            format!("{label} must be valid JSON"),
        )
    })?;
    serde_json::from_slice::<Value>(body).map_err(|_error| {
        HttpError::new(
            StatusCode::BAD_REQUEST,
            format!("{label} must be valid JSON"),
        )
    })
}

pub(crate) struct NoDuplicateJsonProperties;

impl<'de> DeserializeSeed<'de> for NoDuplicateJsonProperties {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(NoDuplicateJsonPropertiesVisitor)
    }
}

pub(crate) struct NoDuplicateJsonPropertiesVisitor;

impl<'de> Visitor<'de> for NoDuplicateJsonPropertiesVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        NoDuplicateJsonProperties.deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence
            .next_element_seed(NoDuplicateJsonProperties)?
            .is_some()
        {}
        Ok(())
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate JSON object property: {key}"
                )));
            }
            map.next_value_seed(NoDuplicateJsonProperties)?;
        }
        Ok(())
    }
}

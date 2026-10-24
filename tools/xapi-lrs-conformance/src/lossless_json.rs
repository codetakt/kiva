//! Decoded JSON objects retain every member, including repeated names.

#![expect(
    clippy::redundant_pub_crate,
    reason = "Lossless JSON is internal to the runner oracles"
)]

use serde::Deserialize;
use serde::de::{Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;

#[derive(Debug)]
pub(super) enum Json {
    Object(Vec<(String, Self)>),
    Array(Vec<Self>),
    Scalar(Value),
}

impl Json {
    pub(super) const fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    pub(super) fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(pairs) => pairs
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub(super) fn as_str(&self) -> Option<&str> {
        match self {
            Self::Scalar(value) => value.as_str(),
            _ => None,
        }
    }

    pub(super) fn has_repeated_members(&self) -> bool {
        match self {
            Self::Object(pairs) => {
                let mut names = std::collections::BTreeSet::new();
                pairs.iter().any(|(name, _)| !names.insert(name))
            }
            _ => false,
        }
    }
}

impl From<Value> for Json {
    fn from(value: Value) -> Self {
        match value {
            Value::Object(object) => Self::Object(
                object
                    .into_iter()
                    .map(|(key, value)| (key, value.into()))
                    .collect(),
            ),
            Value::Array(array) => Self::Array(array.into_iter().map(Self::from).collect()),
            scalar => Self::Scalar(scalar),
        }
    }
}

impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct JsonVisitor;

        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = Json;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON value retaining all object members")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Json, M::Error> {
                let mut pairs = Vec::new();
                while let Some(pair) = map.next_entry()? {
                    pairs.push(pair);
                }
                Ok(Json::Object(pairs))
            }

            fn visit_seq<S: SeqAccess<'de>>(self, mut sequence: S) -> Result<Json, S::Error> {
                let mut items = Vec::new();
                while let Some(item) = sequence.next_element()? {
                    items.push(item);
                }
                Ok(Json::Array(items))
            }

            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Json, E> {
                Ok(Json::Scalar(value.into()))
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Json, E> {
                Ok(Json::Scalar(value.into()))
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Json, E> {
                Ok(Json::Scalar(value.into()))
            }

            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Json, E> {
                serde_json::Number::from_f64(value)
                    .map(|number| Json::Scalar(Value::Number(number)))
                    .ok_or_else(|| E::custom("JSON numbers must be finite"))
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Json, E> {
                Ok(Json::Scalar(value.into()))
            }

            fn visit_unit<E: serde::de::Error>(self) -> Result<Json, E> {
                Ok(Json::Scalar(Value::Null))
            }
        }
        deserializer.deserialize_any(JsonVisitor)
    }
}

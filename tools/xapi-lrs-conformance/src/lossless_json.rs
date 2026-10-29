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
    fn into_value(self) -> Value {
        match self {
            Self::Scalar(value) => value,
            Self::Array(items) => Value::Array(items.into_iter().map(Self::into_value).collect()),
            Self::Object(pairs) => Value::Object(
                pairs
                    .into_iter()
                    .map(|(key, value)| (key, value.into_value()))
                    .collect(),
            ),
        }
    }
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
                if let [(key, Json::Scalar(Value::String(token)))] = pairs.as_slice()
                    && key == "$serde_json::private::Number"
                {
                    return token
                        .parse::<serde_json::Number>()
                        .map(|n| Json::Scalar(Value::Number(n)))
                        .map_err(serde::de::Error::custom);
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

/// Decode raw responses without confusing an actual object key with Serde's
/// private arbitrary-precision number transport. Retain duplicate members.
pub(super) fn from_slice(bytes: &[u8]) -> Result<Json, serde_json::Error> {
    let raw: Box<serde_json::value::RawValue> = serde_json::from_slice(bytes)?;
    from_raw(&raw)
}

pub(super) fn value_from_slice(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    from_slice(bytes).map(Json::into_value)
}

fn from_raw(raw: &serde_json::value::RawValue) -> Result<Json, serde_json::Error> {
    match raw.get().as_bytes().first() {
        Some(b'{') => {
            struct Members(Vec<(String, Box<serde_json::value::RawValue>)>);
            impl<'de> Deserialize<'de> for Members {
                fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                    struct MemberVisitor;
                    impl<'de> Visitor<'de> for MemberVisitor {
                        type Value = Members;
                        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                            f.write_str("JSON object")
                        }
                        fn visit_map<M: MapAccess<'de>>(
                            self,
                            mut m: M,
                        ) -> Result<Members, M::Error> {
                            let mut pairs = Vec::new();
                            while let Some(pair) = m.next_entry()? {
                                pairs.push(pair);
                            }
                            Ok(Members(pairs))
                        }
                    }
                    d.deserialize_map(MemberVisitor)
                }
            }
            let Members(members) = serde_json::from_str(raw.get())?;
            Ok(Json::Object(
                members
                    .into_iter()
                    .map(|(key, value)| Ok((key, from_raw(&value)?)))
                    .collect::<Result<_, serde_json::Error>>()?,
            ))
        }
        Some(b'[') => {
            let elements: Vec<Box<serde_json::value::RawValue>> = serde_json::from_str(raw.get())?;
            Ok(Json::Array(
                elements
                    .iter()
                    .map(|value| from_raw(value))
                    .collect::<Result<_, _>>()?,
            ))
        }
        _ => serde_json::from_str(raw.get()).map(Json::Scalar),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn raw_objects_cannot_impersonate_numbers() -> Result<(), serde_json::Error> {
        let bytes = br#"{"$serde_json::private::Number":"1.2","x":0.10000000000000000001,"x":2}"#;
        let decoded = super::from_slice(bytes)?;
        assert!(decoded.is_object());
        assert!(decoded.has_repeated_members());
        assert_eq!(
            decoded
                .get("$serde_json::private::Number")
                .and_then(super::Json::as_str),
            Some("1.2")
        );
        let value = super::value_from_slice(bytes)?;
        assert!(value.is_object());
        assert_eq!(value["x"], 2);
        Ok(())
    }
}

//! Duplicate-aware JSON syntax validation for structured protocol objects.

use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use std::collections::BTreeSet;

/// Rejects duplicate properties at every object depth before JSON map decoding.
/// Opaque extension and document content must use their separate parsing contract.
pub fn validate_unique_json_properties(body: &[u8]) -> Result<(), serde_json::Error> {
    let mut detector = serde_json::Deserializer::from_slice(body);
    NoDuplicateJsonProperties.deserialize(&mut detector)?;
    detector.end()
}

struct NoDuplicateJsonProperties;

impl<'de> DeserializeSeed<'de> for NoDuplicateJsonProperties {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(NoDuplicateJsonPropertiesVisitor)
    }
}

struct NoDuplicateJsonPropertiesVisitor;

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

//! Load suite YAML numbers from their lexical representation, before binary64.

#![expect(
    clippy::redundant_pub_crate,
    reason = "Provider helpers are internal to the runner"
)]

use serde::de::DeserializeOwned;
use serde_json::{Map, Number, Value};
use yaml_rust2::{Yaml, YamlLoader};

pub(super) fn from_slice<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    let mut documents = YamlLoader::load_from_str(text).map_err(|e| e.to_string())?;
    if documents.len() != 1 {
        return Err("suite requires exactly one YAML document".into());
    }
    let document = documents.pop().ok_or("missing YAML document")?;
    let json = value(document)?;
    // JSON's arbitrary-precision deserializer retains decimal/exponent tokens.
    serde_json::from_str(&json.to_string()).map_err(|e| e.to_string())
}

fn value(yaml: Yaml) -> Result<Value, String> {
    Ok(match yaml {
        Yaml::Real(text) => {
            let text = text.strip_prefix('+').unwrap_or(&text);
            let (mantissa, exponent) = text.split_once(['e', 'E']).unwrap_or((text, "0"));
            let mantissa = mantissa.strip_prefix("-.").map_or_else(
                || {
                    if mantissa.starts_with('.') {
                        format!("0{mantissa}")
                    } else if mantissa.ends_with('.') {
                        format!("{mantissa}0")
                    } else {
                        mantissa.to_owned()
                    }
                },
                |rest| format!("-0.{rest}"),
            );
            let number = if text.contains(['e', 'E']) {
                format!("{mantissa}e{exponent}")
            } else {
                mantissa
            };
            Value::Number(number.parse::<Number>().map_err(|e| e.to_string())?)
        }
        Yaml::Integer(number) => number.into(),
        Yaml::String(text) => Value::String(text),
        Yaml::Boolean(value) => Value::Bool(value),
        Yaml::Null => Value::Null,
        Yaml::Array(items) => Value::Array(items.into_iter().map(value).collect::<Result<_, _>>()?),
        Yaml::Hash(items) => {
            let mut map = Map::new();
            for (key, item) in items {
                let Yaml::String(key) = key else {
                    return Err("JSON mapping key must be a string".into());
                };
                // Fail closed before generic Serde decoding could interpret this
                // literal object member as its private number transport.
                if key == "$serde_json::private::Number" {
                    return Err("suite mapping uses the reserved Serde number transport key".into());
                }
                if map.insert(key, value(item)?).is_some() {
                    return Err("duplicate mapping key".into());
                }
            }
            Value::Object(map)
        }
        Yaml::Alias(_) | Yaml::BadValue => return Err("unresolved or invalid YAML value".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_and_aliases_keep_original_values() -> Result<(), String> {
        let document: Value =
            from_slice(b"x: &v [0.10000000000000000001, 18446744073709551616, '1.00']\ny: *v\n")?;
        assert_eq!(document["x"][0].to_string(), "0.10000000000000000001");
        assert_eq!(document["x"][1].to_string(), "18446744073709551616");
        assert_eq!(document["x"], document["y"]);
        assert!(document["x"][2].is_string());
        assert!(from_slice::<Value>(b"x: 1\nx: 2\n").is_err());
        assert!(from_slice::<Value>(b"x: {'$serde_json::private::Number': '1.2'}").is_err());
        Ok(())
    }
}

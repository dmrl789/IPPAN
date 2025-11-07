//! Canonical JSON serialization helpers.
//!
//! Provides utilities to serialize structures with deterministically sorted
//! object keys and stable formatting so that model artifacts can be hashed and
//! compared reliably across architectures.

use serde::{ser::Error as SerdeSerError, Serialize};
use serde_json::{self, map::Map, ser::PrettyFormatter, Serializer, Value};
use std::io::Write;

/// Recursively sort JSON object keys to obtain a canonical representation.
fn canonicalize(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));

            let mut sorted = Map::with_capacity(entries.len());
            for (key, val) in entries {
                sorted.insert(key, canonicalize(val));
            }

            Value::Object(sorted)
        }
        Value::Array(elements) => Value::Array(elements.into_iter().map(canonicalize).collect()),
        other => other,
    }
}

/// Serialize a value into canonical JSON and write it to the provided writer.
pub fn write_canonical_json<T, W>(mut writer: W, value: &T) -> Result<(), serde_json::Error>
where
    T: Serialize,
    W: Write,
{
    let canonical_value = canonicalize(serde_json::to_value(value)?);
    let formatter = PrettyFormatter::with_indent(b"  ");
    let mut serializer = Serializer::with_formatter(&mut writer, formatter);
    canonical_value.serialize(&mut serializer)?;
    Ok(())
}

/// Serialize a value into canonical JSON and return it as a String.
pub fn canonical_json_string<T>(value: &T) -> Result<String, serde_json::Error>
where
    T: Serialize,
{
    let mut buffer = Vec::new();
    write_canonical_json(&mut buffer, value)?;
    String::from_utf8(buffer).map_err(|err| SerdeSerError::custom(err.to_string()))
}

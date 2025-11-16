//! Canonical JSON serialization helpers.
//!
//! Provides utilities to serialize structures with deterministically sorted
//! object keys and stable formatting so that model artifacts can be hashed and
//! compared reliably across architectures.

use crate::errors::AiCoreError;
use crate::gbdt::Model;
use serde::{ser::Error as SerdeSerError, Serialize};
use serde_json::{self, map::Map, ser::PrettyFormatter, Serializer, Value};
use std::io::Write;
use std::path::Path;

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

/// Produce canonical JSON for a deterministic GBDT model.
pub fn canonical_model_json(model: &Model) -> Result<String, AiCoreError> {
    Ok(model.to_canonical_json().map_err(AiCoreError::from)?)
}

/// Load a deterministic GBDT model from disk, validating its structure.
pub fn load_model_from_path<P: AsRef<Path>>(path: P) -> Result<Model, AiCoreError> {
    Ok(Model::load_json(path).map_err(AiCoreError::from)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gbdt::tree::Node;
    use crate::gbdt::{Model, Tree, SCALE};
    use tempfile::NamedTempFile;

    fn sample_model() -> Model {
        let tree = Tree::new(
            vec![
                Node::internal(0, 0, 50 * SCALE, 1, 2),
                Node::leaf(1, 100 * SCALE),
                Node::leaf(2, 200 * SCALE),
            ],
            SCALE,
        );
        Model::new(vec![tree], 0)
    }

    #[test]
    fn test_canonical_model_json_wrapper() {
        let model = sample_model();
        let via_model = model.to_canonical_json().unwrap();
        let via_helper = canonical_model_json(&model).unwrap();
        assert_eq!(via_model, via_helper);
    }

    #[test]
    fn test_load_model_from_path_wrapper() {
        let model = sample_model();
        let tmp = NamedTempFile::new().unwrap();
        model.save_json(tmp.path()).unwrap();

        let loaded = load_model_from_path(tmp.path()).unwrap();
        assert_eq!(loaded, model);
    }
}

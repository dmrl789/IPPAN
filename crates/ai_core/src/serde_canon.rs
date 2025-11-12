//! Canonical JSON serialization for deterministic hashing
//!
//! Provides canonical JSON serialization with:
//! - Sorted map keys for determinism
//! - No whitespace or pretty-printing
//! - Consistent number formatting
//! - Blake3 hashing for model verification

use serde::Serialize;
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CanonicalError {
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Invalid JSON structure: {0}")]
    InvalidStructure(String),
}

/// Serialize a value to canonical JSON (sorted keys, no whitespace)
pub fn to_canonical_json<T: Serialize>(value: &T) -> Result<String, CanonicalError> {
    // First serialize to a serde_json::Value to normalize structure
    let json_value = serde_json::to_value(value)
        .map_err(|e| CanonicalError::SerializationError(e.to_string()))?;
    
    // Then canonicalize and serialize
    let canonical = canonicalize_value(&json_value);
    serde_json::to_string(&canonical)
        .map_err(|e| CanonicalError::SerializationError(e.to_string()))
}

/// Canonicalize a JSON value by sorting all object keys recursively
fn canonicalize_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut btree = BTreeMap::new();
            for (k, v) in map {
                btree.insert(k.clone(), canonicalize_value(v));
            }
            serde_json::Value::Object(btree.into_iter().collect())
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(canonicalize_value).collect())
        }
        other => other.clone(),
    }
}

/// Compute Blake3 hash of canonical JSON representation
pub fn hash_canonical<T: Serialize>(value: &T) -> Result<[u8; 32], CanonicalError> {
    let json = to_canonical_json(value)?;
    let hash = blake3::hash(json.as_bytes());
    Ok(*hash.as_bytes())
}

/// Compute Blake3 hash and return as hex string
pub fn hash_canonical_hex<T: Serialize>(value: &T) -> Result<String, CanonicalError> {
    let hash = hash_canonical(value)?;
    Ok(hex::encode(hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestStruct {
        b_field: i64,
        a_field: i64,
        z_field: String,
    }

    #[test]
    fn test_canonical_json_sorts_keys() {
        let data = TestStruct {
            b_field: 2,
            a_field: 1,
            z_field: "test".to_string(),
        };
        
        let json = to_canonical_json(&data).unwrap();
        // Keys should be sorted alphabetically
        assert!(json.contains(r#""a_field":1"#));
        assert!(json.contains(r#""b_field":2"#));
        assert!(json.contains(r#""z_field":"test""#));
        
        // Check order: a comes before b comes before z
        let a_pos = json.find("a_field").unwrap();
        let b_pos = json.find("b_field").unwrap();
        let z_pos = json.find("z_field").unwrap();
        assert!(a_pos < b_pos);
        assert!(b_pos < z_pos);
    }

    #[test]
    fn test_canonical_json_no_whitespace() {
        let data = TestStruct {
            b_field: 2,
            a_field: 1,
            z_field: "test".to_string(),
        };
        
        let json = to_canonical_json(&data).unwrap();
        // No newlines or extra spaces
        assert!(!json.contains('\n'));
        assert!(!json.contains("  "));
    }

    #[test]
    fn test_hash_deterministic() {
        let data1 = TestStruct {
            b_field: 2,
            a_field: 1,
            z_field: "test".to_string(),
        };
        let data2 = TestStruct {
            a_field: 1,
            b_field: 2,
            z_field: "test".to_string(),
        };
        
        let hash1 = hash_canonical_hex(&data1).unwrap();
        let hash2 = hash_canonical_hex(&data2).unwrap();
        
        // Same data, different field order -> same hash
        assert_eq!(hash1, hash2);
        
        // Hash should be 64 hex chars (32 bytes)
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_hash_changes_with_data() {
        let data1 = TestStruct {
            b_field: 2,
            a_field: 1,
            z_field: "test".to_string(),
        };
        let data2 = TestStruct {
            a_field: 1,
            b_field: 3, // Different value
            z_field: "test".to_string(),
        };
        
        let hash1 = hash_canonical_hex(&data1).unwrap();
        let hash2 = hash_canonical_hex(&data2).unwrap();
        
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_nested_objects_sorted() {
        #[derive(Serialize)]
        struct Nested {
            outer_b: Inner,
            outer_a: Inner,
        }
        
        #[derive(Serialize)]
        struct Inner {
            inner_z: i64,
            inner_a: i64,
        }
        
        let data = Nested {
            outer_b: Inner { inner_z: 2, inner_a: 1 },
            outer_a: Inner { inner_z: 4, inner_a: 3 },
        };
        
        let json = to_canonical_json(&data).unwrap();
        
        // Check outer keys are sorted
        let outer_a_pos = json.find("outer_a").unwrap();
        let outer_b_pos = json.find("outer_b").unwrap();
        assert!(outer_a_pos < outer_b_pos);
    }
}

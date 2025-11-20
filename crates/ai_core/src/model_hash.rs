//! Canonical hashing helpers for deterministic D-GBDT models.

use crate::errors::AiCoreError;
use crate::gbdt::Model;

/// Compute the canonical BLAKE3 hash (hex) for a deterministic GBDT model.
///
/// Internally this serializes the model using canonical JSON (sorted keys,
/// no whitespace) and feeds it to BLAKE3. The result is a 64-character hex
/// string that is stable across platforms.
pub fn model_hash_hex(model: &Model) -> Result<String, AiCoreError> {
    model.hash_hex().map_err(AiCoreError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gbdt::{tree::Node, Model, Tree, SCALE};

    fn sample_model_with_bias(bias: i64) -> Model {
        let tree = Tree::new(vec![Node::leaf(0, 100 * SCALE)], SCALE);
        Model::new(vec![tree], bias)
    }

    #[test]
    fn model_hash_hex_is_stable_for_identical_models() {
        let model = sample_model_with_bias(0);
        let hash_a = model_hash_hex(&model).expect("hash a");
        let hash_b = model_hash_hex(&model).expect("hash b");
        assert_eq!(hash_a, hash_b);
        assert_eq!(hash_a.len(), 64);
    }

    #[test]
    fn model_hash_hex_changes_when_model_changes() {
        let model_a = sample_model_with_bias(0);
        let model_b = sample_model_with_bias(25);
        let hash_a = model_hash_hex(&model_a).expect("hash a");
        let hash_b = model_hash_hex(&model_b).expect("hash b");
        assert_ne!(hash_a, hash_b);
    }
}

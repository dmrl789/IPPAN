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

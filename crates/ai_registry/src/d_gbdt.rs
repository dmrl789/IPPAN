//! D-GBDT Model Registry and Storage
//!
//! This module provides functionality to:
//! - Load and validate D-GBDT models from disk
//! - Store active models in the Sled database with BLAKE3 hashing
//! - Maintain version history with a configurable ring buffer
//! - Retrieve active models with their verification hashes
//!
//! ## Example
//! ```no_run
//! use ippan_ai_registry::d_gbdt::{DGBDTRegistry, load_model_from_path};
//! use std::path::Path;
//!
//! # fn main() -> anyhow::Result<()> {
//! let db = sled::open("data/registry")?;
//! let mut registry = DGBDTRegistry::new(db);
//!
//! // Load model from disk
//! let (model, hash) = load_model_from_path(Path::new("config/models/d_gbdt_v1.json"))?;
//!
//! // Store as active model
//! registry.store_active_model(model, hash)?;
//!
//! // Retrieve active model
//! let (model, hash) = registry.get_active_model()?;
//! println!("Active model hash: {}", hash);
//! # Ok(())
//! # }
//! ```

use crate::errors::{RegistryError, Result};
use ippan_ai_core::gbdt::Model;
use ippan_ai_core::{
    canonical_model_json, load_model_from_path as load_ai_model_from_path, model_hash_hex,
};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Maximum number of model versions to keep in history
pub const MAX_HISTORY_VERSIONS: usize = 5;

/// Key for storing the active model in the database
const ACTIVE_MODEL_KEY: &[u8] = b"d_gbdt/active";

/// Key for storing model version history
const HISTORY_KEY: &[u8] = b"d_gbdt/history";

/// Stored model data with hash
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredModel {
    pub model: Model,
    pub hash_hex: String,
    pub stored_at: u64,
}

/// Model version history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub hash_hex: String,
    pub stored_at: u64,
}

/// D-GBDT Model Registry
///
/// Manages loading, storing, and retrieving D-GBDT models with cryptographic
/// verification using BLAKE3 hashing and canonical JSON serialization.
pub struct DGBDTRegistry {
    db: Db,
}

impl DGBDTRegistry {
    /// Create a new registry instance
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Store the active model with its hash
    ///
    /// This will:
    /// 1. Store the model and hash under "d_gbdt/active"
    /// 2. Add the hash to the version history ring buffer
    pub fn store_active_model(&mut self, model: Model, hash_hex: String) -> Result<()> {
        let stored = StoredModel {
            model,
            hash_hex: hash_hex.clone(),
            stored_at: current_timestamp(),
        };

        let serialized = bincode::serialize(&stored).map_err(|err| {
            RegistryError::Internal(format!("Failed to serialize model for storage: {err}"))
        })?;

        self.db
            .insert(ACTIVE_MODEL_KEY, serialized)
            .map_err(|err| {
                RegistryError::Database(format!("Failed to store active model in database: {err}"))
            })?;

        info!("Stored active D-GBDT model with hash: {}", hash_hex);

        // Update history
        self.add_to_history(hash_hex)?;

        self.db
            .flush()
            .map_err(|err| RegistryError::Database(format!("Failed to flush database: {err}")))?;

        Ok(())
    }

    /// Load a model referenced in the provided config file, activate it, and
    /// return the `(Model, hash)` tuple.
    pub fn load_and_activate_from_config<P: AsRef<Path>>(
        &mut self,
        config_path: P,
    ) -> Result<(Model, String)> {
        let (model, hash) = load_model_from_config(config_path.as_ref())?;
        let activated_model = model.clone();
        self.store_active_model(model, hash.clone())?;
        Ok((activated_model, hash))
    }

    /// Ensure an active model exists, activating from a config file if needed.
    ///
    /// Returns the active `(Model, hash)` pair either from the registry or by
    /// loading and activating the model specified in the provided config file.
    pub fn ensure_active_model_from_config<P: AsRef<Path>>(
        &mut self,
        config_path: P,
    ) -> Result<(Model, String)> {
        let config_path = config_path.as_ref();
        let config = read_config(config_path)?;
        let (model_path, expected_hash) = extract_model_entry(&config)?;
        let resolved_path = resolve_model_path(config_path, Path::new(&model_path));

        match self.get_active_model() {
            Ok((active_model, _stored_hash)) => {
                let computed_active_hash = compute_model_hash(&active_model)?;

                if let Some(expected) = expected_hash.as_ref() {
                    if computed_active_hash.eq_ignore_ascii_case(expected) {
                        return Ok((active_model, computed_active_hash));
                    }

                    info!(
                        "Configured expected hash {} does not match active hash {}; reloading",
                        expected, computed_active_hash
                    );

                    let (configured_model, configured_hash) = load_model_from_path(&resolved_path)?;
                    if !configured_hash.eq_ignore_ascii_case(expected) {
                        return Err(RegistryError::InvalidInput(format!(
                            "Configured expected hash {} does not match model hash {}",
                            expected, configured_hash
                        )));
                    }

                    let activated_model = configured_model.clone();
                    self.store_active_model(configured_model, configured_hash.clone())?;
                    return Ok((activated_model, configured_hash));
                }

                let (configured_model, configured_hash) = load_model_from_path(&resolved_path)?;
                if computed_active_hash.eq_ignore_ascii_case(&configured_hash) {
                    return Ok((active_model, computed_active_hash));
                }

                info!(
                    "Configured model hash {} differs from active hash {}; updating active model",
                    configured_hash, computed_active_hash
                );

                let updated_model = configured_model.clone();
                self.store_active_model(configured_model, configured_hash.clone())?;
                Ok((updated_model, configured_hash))
            }
            Err(RegistryError::ModelNotFound(_)) => self.load_and_activate_from_config(config_path),
            Err(err) => Err(err),
        }
    }

    /// Get the currently active model with its hash
    pub fn get_active_model(&self) -> Result<(Model, String)> {
        let data = self.db.get(ACTIVE_MODEL_KEY).map_err(|err| {
            RegistryError::Database(format!("Failed to read active model: {err}"))
        })?;

        let data = data.ok_or_else(|| {
            debug!("No active D-GBDT model found in registry");
            RegistryError::ModelNotFound("No active D-GBDT model".to_string())
        })?;

        let stored: StoredModel = bincode::deserialize(&data).map_err(|err| {
            RegistryError::Internal(format!("Failed to deserialize stored model: {err}"))
        })?;

        debug!("Retrieved active model with hash: {}", stored.hash_hex);

        Ok((stored.model, stored.hash_hex))
    }

    /// Get the model version history
    pub fn get_history(&self) -> Result<Vec<HistoryEntry>> {
        let data = match self
            .db
            .get(HISTORY_KEY)
            .map_err(|err| RegistryError::Database(format!("Failed to read history: {err}")))?
        {
            Some(data) => data,
            None => return Ok(Vec::new()),
        };

        let history: Vec<HistoryEntry> = bincode::deserialize(&data).map_err(|err| {
            RegistryError::Internal(format!("Failed to deserialize history: {err}"))
        })?;

        Ok(history)
    }

    /// Add a hash to the version history, maintaining a ring buffer
    fn add_to_history(&mut self, hash_hex: String) -> Result<()> {
        let mut history = self.get_history()?;

        // Add new entry
        history.push(HistoryEntry {
            hash_hex,
            stored_at: current_timestamp(),
        });

        // Keep only the last N entries (ring buffer)
        if history.len() > MAX_HISTORY_VERSIONS {
            history.drain(0..(history.len() - MAX_HISTORY_VERSIONS));
        }

        let serialized = bincode::serialize(&history).map_err(|err| {
            RegistryError::Internal(format!("Failed to serialize history: {err}"))
        })?;

        self.db.insert(HISTORY_KEY, serialized).map_err(|err| {
            RegistryError::Database(format!("Failed to store history in database: {err}"))
        })?;

        debug!(
            "Updated history. Current size: {}/{}",
            history.len(),
            MAX_HISTORY_VERSIONS
        );

        Ok(())
    }

    /// Clear all stored models and history
    #[cfg(test)]
    pub fn clear(&mut self) -> Result<()> {
        self.db.remove(ACTIVE_MODEL_KEY).map_err(|err| {
            RegistryError::Database(format!("Failed to clear active model: {err}"))
        })?;
        self.db
            .remove(HISTORY_KEY)
            .map_err(|err| RegistryError::Database(format!("Failed to clear history: {err}")))?;
        self.db
            .flush()
            .map_err(|err| RegistryError::Database(format!("Failed to flush database: {err}")))?;
        Ok(())
    }
}

/// Load a D-GBDT model from a file path using the shared ai_core loader.
///
/// Returns the validated model and its canonical BLAKE3 hash.
pub fn load_model_from_path(path: &Path) -> Result<(Model, String)> {
    info!("Loading D-GBDT model from: {}", path.display());

    let model = load_ai_model_from_path(path).map_err(|err| {
        RegistryError::InvalidInput(format!("Failed to load model {}: {err}", path.display()))
    })?;

    let canonical_len = canonical_model_json(&model)
        .map(|json| json.len())
        .map_err(|err| {
            RegistryError::InvalidInput(format!(
                "Failed to canonicalize model {}: {err}",
                path.display()
            ))
        })?;

    let hash_hex = model_hash_hex(&model).map_err(|err| {
        RegistryError::InvalidInput(format!("Failed to hash model {}: {err}", path.display()))
    })?;

    info!(
        "Successfully loaded model: version={}, scale={}, trees={}, canonical_bytes={}, hash={}",
        model.version,
        model.scale,
        model.trees.len(),
        canonical_len,
        hash_hex
    );

    Ok((model, hash_hex))
}

/// Compute the BLAKE3 hash of a model using canonical JSON serialization
///
/// This ensures that the hash is deterministic and identical across all nodes
pub fn compute_model_hash(model: &Model) -> Result<String> {
    model_hash_hex(model)
        .map_err(|err| RegistryError::InvalidInput(format!("Failed to compute model hash: {err}")))
}

/// Get current Unix timestamp in seconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("System time before Unix epoch")
        .as_secs()
}

/// Load D-GBDT model from config file path
///
/// Reads the `d_gbdt_model_path` from the config and loads the model
pub fn load_model_from_config(config_path: &Path) -> Result<(Model, String)> {
    let config = read_config(config_path)?;

    let (model_path_str, expected_hash) = extract_model_entry(&config)?;

    let full_path = resolve_model_path(config_path, Path::new(&model_path_str));
    let (model, hash) = load_model_from_path(&full_path)?;

    if let Some(expected) = expected_hash {
        if !hash.eq_ignore_ascii_case(&expected) {
            return Err(RegistryError::InvalidInput(format!(
                "Configured expected hash {} does not match model hash {}",
                expected, hash
            )));
        }
    }

    Ok((model, hash))
}

/// Strict loader: Load model with mandatory hash verification (fail-fast)
///
/// This function enforces that the model file exists, is valid JSON, and matches
/// the expected BLAKE3 hash. If any check fails, it returns an error immediately.
/// This is used at node startup to ensure all nodes load the exact same model.
///
/// # Arguments
/// * `model_path` - Path to the model JSON file
/// * `expected_hash` - Expected BLAKE3 hash (64 hex characters)
///
/// # Returns
/// * `Ok((Model, String))` - Model and computed hash if verification passes
/// * `Err(RegistryError)` - If file missing, invalid JSON, or hash mismatch
///
/// # Example
/// ```no_run
/// use ippan_ai_registry::d_gbdt::load_fairness_model_strict;
/// use std::path::Path;
///
/// let (model, hash) = load_fairness_model_strict(
///     Path::new("crates/ai_registry/models/ippan_d_gbdt_v2.json"),
///     "ac5234082ce1de0c52ae29fab9a43e9c52c0ea184f24a1e830f12f2412c5cb0d"
/// )?;
/// ```
pub fn load_fairness_model_strict(
    model_path: &Path,
    expected_hash: &str,
) -> Result<(Model, String)> {
    info!(
        "Loading fairness model with strict hash verification: {} (expected: {})",
        model_path.display(),
        expected_hash
    );

    // Load model and compute hash
    let (model, computed_hash) = load_model_from_path(model_path)?;

    // Verify hash matches (case-insensitive comparison)
    if !computed_hash.eq_ignore_ascii_case(expected_hash) {
        return Err(RegistryError::InvalidInput(format!(
            "Model hash mismatch: expected {}, computed {}",
            expected_hash, computed_hash
        )));
    }

    info!(
        "Strict hash verification passed: {}",
        computed_hash
    );

    Ok((model, computed_hash))
}

fn read_config(config_path: &Path) -> Result<toml::Value> {
    let config_content = std::fs::read_to_string(config_path).map_err(|err| {
        RegistryError::Internal(format!(
            "Failed to read config file {}: {}",
            config_path.display(),
            err
        ))
    })?;

    toml::from_str(&config_content).map_err(|err| {
        RegistryError::Internal(format!(
            "Failed to parse config TOML {}: {}",
            config_path.display(),
            err
        ))
    })
}

/// Convenience helper to activate a model using an existing registry handle.
pub fn load_and_activate_from_config<P: AsRef<Path>>(
    registry: &mut DGBDTRegistry,
    config_path: P,
) -> Result<(Model, String)> {
    registry.load_and_activate_from_config(config_path)
}

fn extract_model_entry(config: &toml::Value) -> Result<(String, Option<String>)> {
    let dgbdt_table = config.get("dgbdt").ok_or_else(|| {
        RegistryError::InvalidInput("Missing 'dgbdt' section in config".to_string())
    })?;

    if let Some(model_table) = dgbdt_table.get("model") {
        let path = model_table
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                RegistryError::InvalidInput("Missing 'dgbdt.model.path' in config".to_string())
            })?
            .to_string();
        let expected_hash = model_table
            .get("expected_hash")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        return Ok((path, expected_hash));
    }

    // Fallback to legacy flat field for backwards compatibility
    let path = dgbdt_table
        .get("d_gbdt_model_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            RegistryError::InvalidInput(
                "Missing 'dgbdt.model.path' or legacy 'dgbdt.d_gbdt_model_path' in config"
                    .to_string(),
            )
        })?
        .to_string();

    Ok((path, None))
}

fn resolve_model_path(config_path: &Path, model_path: &Path) -> PathBuf {
    if model_path.is_absolute() {
        return model_path.to_path_buf();
    }

    config_path
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or_else(|| Path::new("."))
        .join(model_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::RegistryError;
    use tempfile::TempDir;

    fn create_test_model() -> Model {
        use ippan_ai_core::gbdt::{Node, Tree, SCALE};

        let tree1 = Tree::new(
            vec![
                Node::internal(0, 0, 5000, 1, 2),
                Node::leaf(1, 100),
                Node::leaf(2, 200),
            ],
            SCALE,
        );

        let tree2 = Tree::new(vec![Node::leaf(0, 50)], SCALE);

        Model::new(vec![tree1, tree2], 0)
    }

    fn create_test_registry() -> (DGBDTRegistry, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db = sled::open(temp_dir.path()).unwrap();
        (DGBDTRegistry::new(db), temp_dir)
    }

    #[test]
    fn test_compute_model_hash() {
        let model = create_test_model();
        let hash = compute_model_hash(&model).unwrap();

        // Hash should be 64 hex characters (32 bytes)
        assert_eq!(hash.len(), 64);

        // Hash should be deterministic
        let hash2 = compute_model_hash(&model).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_hash_determinism() {
        let model1 = create_test_model();
        let model2 = create_test_model();

        let hash1 = compute_model_hash(&model1).unwrap();
        let hash2 = compute_model_hash(&model2).unwrap();

        assert_eq!(
            hash1, hash2,
            "Identical models should have identical hashes"
        );
    }

    #[test]
    fn test_store_and_retrieve_model() {
        let (mut registry, _temp_dir) = create_test_registry();
        let model = create_test_model();
        let hash = compute_model_hash(&model).unwrap();

        // Store model
        registry
            .store_active_model(model.clone(), hash.clone())
            .unwrap();

        // Retrieve model
        let (retrieved_model, retrieved_hash) = registry.get_active_model().unwrap();
        assert_eq!(retrieved_hash, hash);
        assert_eq!(retrieved_model.version, model.version);
        assert_eq!(retrieved_model.bias, model.bias);
        assert_eq!(retrieved_model.scale, model.scale);
        assert_eq!(retrieved_model.trees.len(), model.trees.len());
    }

    #[test]
    fn test_no_active_model() {
        let (registry, _temp_dir) = create_test_registry();
        let err = registry.get_active_model().unwrap_err();
        assert!(matches!(err, RegistryError::ModelNotFound(_)));
    }

    #[test]
    fn test_version_history_ring_buffer() {
        use ippan_ai_core::gbdt::{Node, Tree, SCALE};

        let (mut registry, _temp_dir) = create_test_registry();

        // Store more than MAX_HISTORY_VERSIONS models
        for i in 0..(MAX_HISTORY_VERSIONS + 3) {
            let tree = Tree::new(vec![Node::leaf(0, (i as i64) * 1000)], SCALE);
            let test_model = Model::new(vec![tree], i as i64);
            let hash = compute_model_hash(&test_model).unwrap();
            registry.store_active_model(test_model, hash).unwrap();
        }

        // History should only contain MAX_HISTORY_VERSIONS entries
        let history = registry.get_history().unwrap();
        assert_eq!(history.len(), MAX_HISTORY_VERSIONS);
    }

    #[test]
    fn test_history_ordering() {
        use ippan_ai_core::gbdt::{Node, Tree, SCALE};

        let (mut registry, _temp_dir) = create_test_registry();

        let mut hashes = Vec::new();
        for i in 0..3 {
            let tree = Tree::new(vec![Node::leaf(0, (i as i64) * 1000)], SCALE);
            let test_model = Model::new(vec![tree], i as i64);
            let hash = compute_model_hash(&test_model).unwrap();
            hashes.push(hash.clone());
            registry.store_active_model(test_model, hash).unwrap();

            // Sleep briefly to ensure different timestamps
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let history = registry.get_history().unwrap();
        assert_eq!(history.len(), 3);

        // Check that hashes are in the order they were added
        for (i, entry) in history.iter().enumerate() {
            assert_eq!(entry.hash_hex, hashes[i]);
        }
    }

    #[test]
    fn test_load_model_from_path() {
        let model = create_test_model();
        let json = serde_json::to_string_pretty(&model).unwrap();

        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("test_model.json");
        std::fs::write(&model_path, json).unwrap();

        let (loaded_model, hash) = load_model_from_path(&model_path).unwrap();

        assert_eq!(loaded_model.version, model.version);
        assert_eq!(loaded_model.scale, model.scale);
        assert_eq!(hash.len(), 64); // BLAKE3 hash hex length
    }

    #[test]
    fn test_ensure_active_model_respects_expected_hash() {
        let (mut registry, temp_dir) = create_test_registry();

        // Seed registry with an outdated model
        let initial_model = create_test_model();
        let initial_hash = compute_model_hash(&initial_model).unwrap();
        registry
            .store_active_model(initial_model.clone(), initial_hash.clone())
            .unwrap();

        // Create a newer model referenced in config with an expected hash
        let newer_model = Model::new(initial_model.trees.clone(), 42);
        let newer_hash = compute_model_hash(&newer_model).unwrap();
        let model_path = temp_dir.path().join("dgbdt_new.json");
        std::fs::write(
            &model_path,
            serde_json::to_string_pretty(&newer_model).unwrap(),
        )
        .unwrap();

        let config_path = temp_dir.path().join("config_expected.toml");
        std::fs::write(
            &config_path,
            format!(
                "[dgbdt.model]\npath = \"{}\"\nexpected_hash = \"{}\"\n",
                model_path.display(),
                newer_hash
            ),
        )
        .unwrap();

        let (active_model, active_hash) = registry
            .ensure_active_model_from_config(&config_path)
            .unwrap();

        assert_eq!(active_hash, newer_hash);
        assert_eq!(active_model.bias, 42);

        let (_, stored_hash) = registry.get_active_model().unwrap();
        assert_eq!(stored_hash, newer_hash);
        assert_ne!(stored_hash, initial_hash);
    }

    #[test]
    fn test_ensure_active_model_reloads_when_expected_missing() {
        let (mut registry, temp_dir) = create_test_registry();

        // Seed registry with an outdated model
        let initial_model = create_test_model();
        let initial_hash = compute_model_hash(&initial_model).unwrap();
        registry
            .store_active_model(initial_model.clone(), initial_hash.clone())
            .unwrap();

        // Create a newer model referenced in config without an expected hash
        let newer_model = Model::new(initial_model.trees.clone(), 99);
        let newer_hash = compute_model_hash(&newer_model).unwrap();
        let model_path = temp_dir.path().join("dgbdt_latest.json");
        std::fs::write(
            &model_path,
            serde_json::to_string_pretty(&newer_model).unwrap(),
        )
        .unwrap();

        let config_path = temp_dir.path().join("config_no_expected.toml");
        std::fs::write(
            &config_path,
            format!("[dgbdt.model]\npath = \"{}\"\n", model_path.display()),
        )
        .unwrap();

        let (active_model, active_hash) = registry
            .ensure_active_model_from_config(&config_path)
            .unwrap();

        assert_eq!(active_hash, newer_hash);
        assert_eq!(active_model.bias, 99);

        let (_, stored_hash) = registry.get_active_model().unwrap();
        assert_eq!(stored_hash, newer_hash);
    }

    #[test]
    fn test_model_validation() {
        use ippan_ai_core::gbdt::{Node, Tree, SCALE};

        // Create an invalid tree (invalid node references)
        let invalid_tree = Tree::new(
            vec![
                Node::internal(0, 0, 5000, 10, 20), // Invalid child indices
            ],
            SCALE,
        );

        let invalid_model = Model::new(vec![invalid_tree], 0);
        let json = serde_json::to_string(&invalid_model).unwrap();

        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("invalid_model.json");
        std::fs::write(&model_path, json).unwrap();

        let result = load_model_from_path(&model_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_changes_with_model() {
        let model1 = create_test_model();
        let mut model2 = create_test_model();
        model2.bias = 200; // Change bias

        let hash1 = compute_model_hash(&model1).unwrap();
        let hash2 = compute_model_hash(&model2).unwrap();

        assert_ne!(
            hash1, hash2,
            "Different models should have different hashes"
        );
    }

    #[test]
    fn test_load_and_activate_from_config() {
        let (mut registry, temp_dir) = create_test_registry();
        let root = temp_dir.path();

        let config_dir = root.join("config");
        let models_dir = root.join("models");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::create_dir_all(&models_dir).unwrap();

        let model = create_test_model();
        let model_path = models_dir.join("model.json");
        std::fs::write(&model_path, serde_json::to_string(&model).unwrap()).unwrap();

        let hash = compute_model_hash(&model).unwrap();
        let config_path = config_dir.join("dlc.toml");
        let config_contents = format!(
            r#"
[dgbdt]
  [dgbdt.model]
  path = "models/model.json"
  expected_hash = "{}"
"#,
            hash
        );
        std::fs::write(&config_path, config_contents).unwrap();

        let (activated_model, hash) = registry
            .load_and_activate_from_config(&config_path)
            .unwrap();
        assert_eq!(activated_model.trees.len(), model.trees.len());

        let stored = registry.get_active_model().unwrap();
        assert_eq!(stored.1, hash);
        assert_eq!(stored.0.trees.len(), model.trees.len());
    }

    #[test]
    fn test_load_and_activate_records_history_and_active_model() {
        let (mut registry, temp_dir) = create_test_registry();
        let root = temp_dir.path();

        let config_dir = root.join("config");
        let models_dir = root.join("models");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::create_dir_all(&models_dir).unwrap();

        let model = create_test_model();
        let model_path = models_dir.join("model.json");
        std::fs::write(&model_path, serde_json::to_string(&model).unwrap()).unwrap();
        let hash = compute_model_hash(&model).unwrap();

        let config_path = config_dir.join("dlc.toml");
        let config_contents = format!(
            r#"
[dgbdt]
  [dgbdt.model]
  path = "models/model.json"
  expected_hash = "{}"
"#,
            hash
        );
        std::fs::write(&config_path, config_contents).unwrap();

        registry
            .load_and_activate_from_config(&config_path)
            .expect("activation");

        let history = registry.get_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].hash_hex, hash);

        let (active_model, active_hash) = registry.get_active_model().unwrap();
        assert_eq!(active_hash, hash);
        assert_eq!(active_model.trees.len(), model.trees.len());
    }

    #[test]
    fn test_expected_hash_mismatch() {
        let (_registry, temp_dir) = create_test_registry();
        let root = temp_dir.path();

        let config_dir = root.join("config");
        let models_dir = root.join("models");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::create_dir_all(&models_dir).unwrap();

        let model = create_test_model();
        let model_path = models_dir.join("model.json");
        std::fs::write(&model_path, serde_json::to_string(&model).unwrap()).unwrap();

        let config_path = config_dir.join("dlc.toml");
        let config_contents = r#"
[dgbdt]
  [dgbdt.model]
  path = "models/model.json"
  expected_hash = "deadbeef"
"#;
        std::fs::write(&config_path, config_contents).unwrap();

        let result = load_model_from_config(&config_path);
        assert!(matches!(result, Err(RegistryError::InvalidInput(_))));
    }

    #[test]
    fn test_clear_registry() {
        let (mut registry, _temp_dir) = create_test_registry();
        let model = create_test_model();
        let hash = compute_model_hash(&model).unwrap();

        registry.store_active_model(model, hash).unwrap();
        assert!(registry.get_active_model().is_ok());

        registry.clear().unwrap();
        assert!(registry.get_active_model().is_err());
        assert_eq!(registry.get_history().unwrap().len(), 0);
    }

    #[test]
    fn test_load_fairness_model_strict() {
        let (_registry, temp_dir) = create_test_registry();
        let root = temp_dir.path();
        let models_dir = root.join("models");
        std::fs::create_dir_all(&models_dir).unwrap();

        let model = create_test_model();
        let model_path = models_dir.join("model.json");
        std::fs::write(&model_path, serde_json::to_string(&model).unwrap()).unwrap();
        let hash = compute_model_hash(&model).unwrap();

        // Test successful load with correct hash
        let (loaded_model, loaded_hash) =
            load_fairness_model_strict(&model_path, &hash).unwrap();
        assert_eq!(loaded_model.trees.len(), model.trees.len());
        assert_eq!(loaded_hash, hash);

        // Test failure with wrong hash
        let result = load_fairness_model_strict(&model_path, "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
        assert!(matches!(result, Err(RegistryError::InvalidInput(_))));
        if let Err(RegistryError::InvalidInput(msg)) = result {
            assert!(msg.contains("hash mismatch"));
        }
    }
}

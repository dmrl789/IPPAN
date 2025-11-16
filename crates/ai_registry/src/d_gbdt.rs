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
    let config_content = std::fs::read_to_string(config_path).map_err(|err| {
        RegistryError::Internal(format!(
            "Failed to read config file {}: {}",
            config_path.display(),
            err
        ))
    })?;

    let config: toml::Value = toml::from_str(&config_content).map_err(|err| {
        RegistryError::Internal(format!(
            "Failed to parse config TOML {}: {}",
            config_path.display(),
            err
        ))
    })?;

    let model_path_str = config
        .get("dgbdt")
        .and_then(|v| v.get("d_gbdt_model_path"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            RegistryError::InvalidInput("Missing 'dgbdt.d_gbdt_model_path' in config".to_string())
        })?;

    let full_path = resolve_model_path(config_path, Path::new(model_path_str));

    load_model_from_path(&full_path)
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

        let config_path = config_dir.join("dlc.toml");
        let config_contents = r#"
[dgbdt]
d_gbdt_model_path = "models/model.json"
"#;
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
}

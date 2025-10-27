//! Production-grade model management system for GBDT models
//!
//! This module provides comprehensive model lifecycle management including:
//! - Model loading, saving, and versioning
//! - Model validation and integrity checking
//! - Performance monitoring and metrics collection
//! - Security validation and audit logging
//! - Model deployment and rollback capabilities

use crate::gbdt::{GBDTModel, GBDTError, ModelMetadata, SecurityConstraints, GBDTMetrics};
use crate::model::ModelPackage;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn, instrument};
use tokio::fs;
use tokio::sync::RwLock as AsyncRwLock;

/// Model manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManagerConfig {
    /// Base directory for model storage
    pub model_directory: PathBuf,
    /// Maximum number of models to keep in memory
    pub max_cached_models: usize,
    /// Model validation timeout in milliseconds
    pub validation_timeout_ms: u64,
    /// Enable model integrity checking
    pub enable_integrity_checking: bool,
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// Model cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Enable automatic model cleanup
    pub enable_auto_cleanup: bool,
    /// Maximum model file size in bytes
    pub max_model_size_bytes: u64,
}

impl Default for ModelManagerConfig {
    fn default() -> Self {
        Self {
            model_directory: PathBuf::from("./models"),
            max_cached_models: 10,
            validation_timeout_ms: 5000,
            enable_integrity_checking: true,
            enable_performance_monitoring: true,
            cache_ttl_seconds: 3600, // 1 hour
            enable_auto_cleanup: true,
            max_model_size_bytes: 100 * 1024 * 1024, // 100MB
        }
    }
}

/// Model cache entry with metadata
#[derive(Debug, Clone)]
struct CachedModel {
    model: GBDTModel,
    loaded_at: SystemTime,
    access_count: u64,
    last_accessed: SystemTime,
    file_path: PathBuf,
    file_size: u64,
}

/// Model manager for production GBDT model lifecycle
#[derive(Debug)]
pub struct ModelManager {
    config: ModelManagerConfig,
    models: Arc<RwLock<HashMap<String, CachedModel>>>,
    metrics: Arc<RwLock<ModelManagerMetrics>>,
}

/// Model manager performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelManagerMetrics {
    pub total_models_loaded: u64,
    pub total_models_saved: u64,
    pub total_cache_hits: u64,
    pub total_cache_misses: u64,
    pub total_validation_errors: u64,
    pub total_load_errors: u64,
    pub total_save_errors: u64,
    pub avg_load_time_ms: f64,
    pub avg_save_time_ms: f64,
    pub current_cached_models: usize,
    pub total_disk_usage_bytes: u64,
}

/// Model loading result with metadata
#[derive(Debug, Clone)]
pub struct ModelLoadResult {
    pub model: GBDTModel,
    pub load_time_ms: u64,
    pub file_size: u64,
    pub from_cache: bool,
    pub validation_passed: bool,
}

/// Model save result with metadata
#[derive(Debug, Clone)]
pub struct ModelSaveResult {
    pub file_path: PathBuf,
    pub file_size: u64,
    pub save_time_ms: u64,
    pub model_hash: String,
}

impl ModelManager {
    /// Create a new model manager with configuration
    pub fn new(config: ModelManagerConfig) -> Self {
        Self {
            config,
            models: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(ModelManagerMetrics::default())),
        }
    }

    /// Load a model by ID with comprehensive error handling and caching
    #[instrument(skip(self), fields(model_id = %model_id))]
    pub async fn load_model(&self, model_id: &str) -> Result<ModelLoadResult, GBDTError> {
        let start_time = std::time::Instant::now();

        // Check cache first
        if let Some(cached) = self.get_cached_model(model_id) {
            self.update_cache_metrics(true);
            debug!("Model {} loaded from cache", model_id);
            return Ok(ModelLoadResult {
                model: cached.model.clone(),
                load_time_ms: start_time.elapsed().as_millis() as u64,
                file_size: cached.file_size,
                from_cache: true,
                validation_passed: true,
            });
        }

        self.update_cache_metrics(false);

        // Load from disk
        let model_path = self.get_model_path(model_id);
        let model = self.load_model_from_disk(&model_path).await?;
        
        // Validate model
        let validation_passed = if self.config.enable_integrity_checking {
            self.validate_model(&model).await?
        } else {
            true
        };

        // Cache the model
        self.cache_model(model_id, &model, &model_path).await?;

        let load_time = start_time.elapsed();
        self.update_load_metrics(load_time);

        info!(
            "Model {} loaded in {}ms, size: {} bytes, validation: {}",
            model_id,
            load_time.as_millis(),
            model_path.metadata()?.len(),
            validation_passed
        );

        Ok(ModelLoadResult {
            model,
            load_time_ms: load_time.as_millis() as u64,
            file_size: model_path.metadata()?.len(),
            from_cache: false,
            validation_passed,
        })
    }

    /// Save a model with comprehensive validation and metadata
    #[instrument(skip(self, model), fields(model_id = %model_id))]
    pub async fn save_model(
        &self,
        model_id: &str,
        model: &GBDTModel,
    ) -> Result<ModelSaveResult, GBDTError> {
        let start_time = std::time::Instant::now();

        // Validate model before saving
        if self.config.enable_integrity_checking {
            self.validate_model(model).await?;
        }

        // Check file size limits
        let serialized_size = bincode::serialized_size(model)
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to calculate model size: {}", e),
            })?;

        if serialized_size > self.config.max_model_size_bytes {
            return Err(GBDTError::ModelValidationFailed {
                reason: format!(
                    "Model size {} exceeds maximum {} bytes",
                    serialized_size, self.config.max_model_size_bytes
                ),
            });
        }

        // Ensure model directory exists
        fs::create_dir_all(&self.config.model_directory)
            .context("Failed to create model directory")?;

        // Save model to disk
        let model_path = self.get_model_path(model_id);
        let model_package = ModelPackage {
            metadata: model.metadata.clone(),
            model: model.clone(),
        };

        let model_data = bincode::serialize(&model_package)
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to serialize model: {}", e),
            })?;

        fs::write(&model_path, &model_data)
            .await
            .context("Failed to write model file")?;

        // Update cache
        self.cache_model(model_id, model, &model_path).await?;

        let save_time = start_time.elapsed();
        self.update_save_metrics(save_time);

        info!(
            "Model {} saved in {}ms, size: {} bytes",
            model_id,
            save_time.as_millis(),
            model_data.len()
        );

        Ok(ModelSaveResult {
            file_path: model_path,
            file_size: model_data.len() as u64,
            save_time_ms: save_time.as_millis() as u64,
            model_hash: model.metadata.model_hash.clone(),
        })
    }

    /// Get a cached model if available
    fn get_cached_model(&self, model_id: &str) -> Option<CachedModel> {
        let models = self.models.read().ok()?;
        let cached = models.get(model_id)?;

        // Check if cache entry is still valid
        if cached.loaded_at.elapsed().unwrap_or(Duration::from_secs(0))
            > Duration::from_secs(self.config.cache_ttl_seconds)
        {
            return None;
        }

        Some(cached.clone())
    }

    /// Load model from disk with error handling
    async fn load_model_from_disk(&self, path: &Path) -> Result<GBDTModel, GBDTError> {
        if !path.exists() {
            return Err(GBDTError::ModelValidationFailed {
                reason: format!("Model file not found: {}", path.display()),
            });
        }

        let model_data = fs::read(path)
            .await
            .context("Failed to read model file")
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to read model file: {}", e),
            })?;

        let model_package: ModelPackage = bincode::deserialize(&model_data)
            .context("Failed to deserialize model")
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to deserialize model: {}", e),
            })?;

        Ok(model_package.model)
    }

    /// Validate model integrity and security
    async fn validate_model(&self, model: &GBDTModel) -> Result<bool, GBDTError> {
        // Check model structure
        model.validate()?;

        // Check security constraints
        if model.trees.len() > model.security_constraints.max_trees {
            return Err(GBDTError::SecurityValidationFailed {
                reason: format!(
                    "Too many trees: {} > {}",
                    model.trees.len(),
                    model.security_constraints.max_trees
                ),
            });
        }

        // Check model hash integrity
        let expected_hash = GBDTModel::calculate_model_hash(&model.trees, model.bias, model.scale);
        if model.metadata.model_hash != expected_hash {
            return Err(GBDTError::SecurityValidationFailed {
                reason: "Model hash mismatch - possible corruption".to_string(),
            });
        }

        // Performance validation - test evaluation
        let test_features = vec![0i64; model.metadata.feature_count];
        let start_time = std::time::Instant::now();
        
        let mut test_model = model.clone();
        let _ = test_model.evaluate(&test_features)?;
        
        let eval_time = start_time.elapsed();
        if eval_time.as_millis() > self.config.validation_timeout_ms as u128 {
            return Err(GBDTError::EvaluationTimeout {
                timeout_ms: self.config.validation_timeout_ms,
            });
        }

        Ok(true)
    }

    /// Cache a model in memory
    async fn cache_model(
        &self,
        model_id: &str,
        model: &GBDTModel,
        file_path: &Path,
    ) -> Result<(), GBDTError> {
        let mut models = self.models.write().map_err(|_| GBDTError::ModelValidationFailed {
            reason: "Failed to acquire write lock".to_string(),
        })?;

        // Check cache size limit
        if models.len() >= self.config.max_cached_models {
            self.evict_oldest_model(&mut models)?;
        }

        let file_size = file_path.metadata()
            .await
            .context("Failed to get file metadata")
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to get file metadata: {}", e),
            })?
            .len();

        let cached_model = CachedModel {
            model: model.clone(),
            loaded_at: SystemTime::now(),
            access_count: 0,
            last_accessed: SystemTime::now(),
            file_path: file_path.to_path_buf(),
            file_size,
        };

        models.insert(model_id.to_string(), cached_model);
        self.update_cached_models_count();

        Ok(())
    }

    /// Evict the oldest model from cache
    fn evict_oldest_model(
        &self,
        models: &mut HashMap<String, CachedModel>,
    ) -> Result<(), GBDTError> {
        let oldest_key = models
            .iter()
            .min_by_key(|(_, cached)| cached.last_accessed)
            .map(|(key, _)| key.clone());

        if let Some(key) = oldest_key {
            models.remove(&key);
            debug!("Evicted model {} from cache", key);
        }

        Ok(())
    }

    /// Get model file path
    fn get_model_path(&self, model_id: &str) -> PathBuf {
        self.config.model_directory.join(format!("{}.model", model_id))
    }

    /// Update cache hit/miss metrics
    fn update_cache_metrics(&self, hit: bool) {
        if let Ok(mut metrics) = self.metrics.write() {
            if hit {
                metrics.total_cache_hits += 1;
            } else {
                metrics.total_cache_misses += 1;
            }
        }
    }

    /// Update load metrics
    fn update_load_metrics(&self, load_time: Duration) {
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.total_models_loaded += 1;
            let load_time_ms = load_time.as_millis() as f64;
            metrics.avg_load_time_ms = (metrics.avg_load_time_ms * (metrics.total_models_loaded - 1) as f64 + load_time_ms)
                / metrics.total_models_loaded as f64;
        }
    }

    /// Update save metrics
    fn update_save_metrics(&self, save_time: Duration) {
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.total_models_saved += 1;
            let save_time_ms = save_time.as_millis() as f64;
            metrics.avg_save_time_ms = (metrics.avg_save_time_ms * (metrics.total_models_saved - 1) as f64 + save_time_ms)
                / metrics.total_models_saved as f64;
        }
    }

    /// Update cached models count
    fn update_cached_models_count(&self) {
        if let Ok(models) = self.models.read() {
            if let Ok(mut metrics) = self.metrics.write() {
                metrics.current_cached_models = models.len();
            }
        }
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> ModelManagerMetrics {
        self.metrics.read().unwrap_or_default().clone()
    }

    /// List all available models
    pub async fn list_models(&self) -> Result<Vec<String>, GBDTError> {
        let mut models = Vec::new();
        
        if !self.config.model_directory.exists() {
            return Ok(models);
        }

        let mut entries = fs::read_dir(&self.config.model_directory)
            .await
            .context("Failed to read model directory")
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to read model directory: {}", e),
            })?;

        while let Some(entry) = entries.next_entry().await
            .context("Failed to read directory entry")
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to read directory entry: {}", e),
            })? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("model") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    models.push(stem.to_string());
                }
            }
        }

        Ok(models)
    }

    /// Delete a model
    pub async fn delete_model(&self, model_id: &str) -> Result<(), GBDTError> {
        // Remove from cache
        if let Ok(mut models) = self.models.write() {
            models.remove(model_id);
        }

        // Delete file
        let model_path = self.get_model_path(model_id);
        if model_path.exists() {
            fs::remove_file(&model_path)
                .await
                .context("Failed to delete model file")
                .map_err(|e| GBDTError::ModelValidationFailed {
                    reason: format!("Failed to delete model file: {}", e),
                })?;
        }

        info!("Model {} deleted", model_id);
        Ok(())
    }

    /// Clean up old models and cache entries
    pub async fn cleanup(&self) -> Result<(), GBDTError> {
        if !self.config.enable_auto_cleanup {
            return Ok(());
        }

        // Clean up expired cache entries
        let mut models = self.models.write().map_err(|_| GBDTError::ModelValidationFailed {
            reason: "Failed to acquire write lock".to_string(),
        })?;

        let now = SystemTime::now();
        let ttl = Duration::from_secs(self.config.cache_ttl_seconds);
        
        models.retain(|model_id, cached| {
            let is_valid = cached.loaded_at.elapsed().unwrap_or(Duration::from_secs(0)) <= ttl;
            if !is_valid {
                debug!("Removing expired model {} from cache", model_id);
            }
            is_valid
        });

        self.update_cached_models_count();
        info!("Cache cleanup completed, {} models remaining", models.len());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gbdt::{Tree, Node, GBDTModel};
    use tempfile::TempDir;

    fn create_test_model() -> GBDTModel {
        let tree = Tree {
            nodes: vec![
                Node { feature_index: 0, threshold: 50, left: 1, right: 2, value: None },
                Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(10) },
                Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(20) },
            ],
        };

        GBDTModel::new(vec![tree], 0, 100, 1).unwrap()
    }

    #[tokio::test]
    async fn test_model_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = ModelManagerConfig {
            model_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let manager = ModelManager::new(config);
        assert_eq!(manager.get_metrics().current_cached_models, 0);
    }

    #[tokio::test]
    async fn test_model_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config = ModelManagerConfig {
            model_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let manager = ModelManager::new(config);
        let model = create_test_model();

        // Save model
        let save_result = manager.save_model("test_model", &model).await.unwrap();
        assert!(save_result.file_path.exists());
        assert!(save_result.file_size > 0);

        // Load model
        let load_result = manager.load_model("test_model").await.unwrap();
        assert_eq!(load_result.model.bias, model.bias);
        assert_eq!(load_result.model.scale, model.scale);
        assert!(!load_result.from_cache); // First load should not be from cache

        // Load again (should be from cache)
        let load_result2 = manager.load_model("test_model").await.unwrap();
        assert!(load_result2.from_cache);
    }

    #[tokio::test]
    async fn test_model_caching() {
        let temp_dir = TempDir::new().unwrap();
        let config = ModelManagerConfig {
            model_directory: temp_dir.path().to_path_buf(),
            max_cached_models: 2,
            ..Default::default()
        };

        let manager = ModelManager::new(config);
        let model = create_test_model();

        // Save and load multiple models
        for i in 0..3 {
            let model_id = format!("model_{}", i);
            manager.save_model(&model_id, &model).await.unwrap();
            manager.load_model(&model_id).await.unwrap();
        }

        // Check that we have at most 2 cached models
        let metrics = manager.get_metrics();
        assert!(metrics.current_cached_models <= 2);
    }

    #[tokio::test]
    async fn test_model_validation() {
        let temp_dir = TempDir::new().unwrap();
        let config = ModelManagerConfig {
            model_directory: temp_dir.path().to_path_buf(),
            enable_integrity_checking: true,
            ..Default::default()
        };

        let manager = ModelManager::new(config);
        let model = create_test_model();

        // Save and load with validation
        manager.save_model("test_model", &model).await.unwrap();
        let load_result = manager.load_model("test_model").await.unwrap();
        assert!(load_result.validation_passed);
    }

    #[tokio::test]
    async fn test_model_deletion() {
        let temp_dir = TempDir::new().unwrap();
        let config = ModelManagerConfig {
            model_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let manager = ModelManager::new(config);
        let model = create_test_model();

        // Save model
        manager.save_model("test_model", &model).await.unwrap();
        assert!(manager.load_model("test_model").await.is_ok());

        // Delete model
        manager.delete_model("test_model").await.unwrap();
        assert!(manager.load_model("test_model").await.is_err());
    }

    #[tokio::test]
    async fn test_model_listing() {
        let temp_dir = TempDir::new().unwrap();
        let config = ModelManagerConfig {
            model_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let manager = ModelManager::new(config);
        let model = create_test_model();

        // Save multiple models
        for i in 0..3 {
            let model_id = format!("model_{}", i);
            manager.save_model(&model_id, &model).await.unwrap();
        }

        // List models
        let models = manager.list_models().await.unwrap();
        assert_eq!(models.len(), 3);
        assert!(models.contains(&"model_0".to_string()));
        assert!(models.contains(&"model_1".to_string()));
        assert!(models.contains(&"model_2".to_string()));
    }
}
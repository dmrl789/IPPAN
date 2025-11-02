//! Hot-reload functionality for GBDT models
//!
//! Allows updating AI models without restarting the consensus engine

use anyhow::Result;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::time::interval;
use tracing::{error, info, warn};

#[cfg(feature = "ai_l1")]
use ippan_ai_core::gbdt::GBDTModel;

/// Model reloader watches for model file changes and hot-reloads them
#[cfg(feature = "ai_l1")]
#[derive(Clone)]
pub struct ModelReloader {
    validator_model_path: Option<PathBuf>,
    fee_model_path: Option<PathBuf>,
    health_model_path: Option<PathBuf>,
    ordering_model_path: Option<PathBuf>,
    last_validator_mtime: Arc<RwLock<Option<SystemTime>>>,
    last_fee_mtime: Arc<RwLock<Option<SystemTime>>>,
    last_health_mtime: Arc<RwLock<Option<SystemTime>>>,
    last_ordering_mtime: Arc<RwLock<Option<SystemTime>>>,
    reload_callback: Arc<dyn Fn(ModelUpdate) -> Result<()> + Send + Sync>,
}

/// Model update event
#[cfg(feature = "ai_l1")]
#[derive(Debug)]
pub enum ModelUpdate {
    Validator(GBDTModel),
    Fee(GBDTModel),
    Health(GBDTModel),
    Ordering(GBDTModel),
}

#[cfg(feature = "ai_l1")]
impl ModelReloader {
    /// Create a new model reloader
    ///
    /// # Arguments
    /// * `reload_callback` - Function to call when a model is reloaded
    pub fn new<F>(reload_callback: F) -> Self
    where
        F: Fn(ModelUpdate) -> Result<()> + Send + Sync + 'static,
    {
        Self {
            validator_model_path: None,
            fee_model_path: None,
            health_model_path: None,
            ordering_model_path: None,
            last_validator_mtime: Arc::new(RwLock::new(None)),
            last_fee_mtime: Arc::new(RwLock::new(None)),
            last_health_mtime: Arc::new(RwLock::new(None)),
            last_ordering_mtime: Arc::new(RwLock::new(None)),
            reload_callback: Arc::new(reload_callback),
        }
    }

    /// Set the validator model path to watch
    pub fn set_validator_model_path<P: AsRef<Path>>(&mut self, path: P) {
        self.validator_model_path = Some(path.as_ref().to_path_buf());
        info!("Watching validator model: {}", path.as_ref().display());
    }

    /// Set the fee model path to watch
    pub fn set_fee_model_path<P: AsRef<Path>>(&mut self, path: P) {
        self.fee_model_path = Some(path.as_ref().to_path_buf());
        info!("Watching fee model: {}", path.as_ref().display());
    }

    /// Set the health model path to watch
    pub fn set_health_model_path<P: AsRef<Path>>(&mut self, path: P) {
        self.health_model_path = Some(path.as_ref().to_path_buf());
        info!("Watching health model: {}", path.as_ref().display());
    }

    /// Set the ordering model path to watch
    pub fn set_ordering_model_path<P: AsRef<Path>>(&mut self, path: P) {
        self.ordering_model_path = Some(path.as_ref().to_path_buf());
        info!("Watching ordering model: {}", path.as_ref().display());
    }

    /// Start the model reload watcher
    ///
    /// Checks for file changes every `check_interval` and reloads models if they've changed
    pub fn start_watcher(self: Arc<Self>, check_interval: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = interval(check_interval);
            loop {
                ticker.tick().await;
                self.check_and_reload().await;
            }
        })
    }

    /// Check all model files and reload if changed
    async fn check_and_reload(&self) {
        // Check validator model
        if let Some(path) = &self.validator_model_path {
            if let Err(e) = self
                .check_and_reload_model(path, &self.last_validator_mtime, |model| {
                    (self.reload_callback)(ModelUpdate::Validator(model))
                })
                .await
            {
                error!("Failed to reload validator model: {}", e);
            }
        }

        // Check fee model
        if let Some(path) = &self.fee_model_path {
            if let Err(e) = self
                .check_and_reload_model(path, &self.last_fee_mtime, |model| {
                    (self.reload_callback)(ModelUpdate::Fee(model))
                })
                .await
            {
                error!("Failed to reload fee model: {}", e);
            }
        }

        // Check health model
        if let Some(path) = &self.health_model_path {
            if let Err(e) = self
                .check_and_reload_model(path, &self.last_health_mtime, |model| {
                    (self.reload_callback)(ModelUpdate::Health(model))
                })
                .await
            {
                error!("Failed to reload health model: {}", e);
            }
        }

        // Check ordering model
        if let Some(path) = &self.ordering_model_path {
            if let Err(e) = self
                .check_and_reload_model(path, &self.last_ordering_mtime, |model| {
                    (self.reload_callback)(ModelUpdate::Ordering(model))
                })
                .await
            {
                error!("Failed to reload ordering model: {}", e);
            }
        }
    }

    /// Check a single model file and reload if changed
    async fn check_and_reload_model<F>(
        &self,
        path: &Path,
        last_mtime: &Arc<RwLock<Option<SystemTime>>>,
        callback: F,
    ) -> Result<()>
    where
        F: FnOnce(GBDTModel) -> Result<()>,
    {
        // Get file metadata
        let metadata = match tokio::fs::metadata(path).await {
            Ok(m) => m,
            Err(e) => {
                warn!("Cannot access model file {}: {}", path.display(), e);
                return Ok(());
            }
        };

        let mtime = metadata.modified()?;

        // Check if file has changed
        let should_reload = {
            let last = last_mtime.read();
            match *last {
                None => true, // First time, always load
                Some(last_time) => mtime > last_time,
            }
        };

        if should_reload {
            info!("Reloading model from {}", path.display());

            // Load the model
            let model = if path.extension().and_then(|s| s.to_str()) == Some("json") {
                GBDTModel::from_json_file(path)?
            } else {
                GBDTModel::from_binary_file(path)?
            };

            // Validate the model
            model.validate()?;

            // Call the callback to update the model
            callback(model)?;

            // Update last modification time
            *last_mtime.write() = Some(mtime);

            info!("Successfully reloaded model from {}", path.display());
        }

        Ok(())
    }

    /// Force reload all models immediately
    pub async fn force_reload_all(&self) -> Result<()> {
        // Clear all mtimes to force reload
        *self.last_validator_mtime.write() = None;
        *self.last_fee_mtime.write() = None;
        *self.last_health_mtime.write() = None;
        *self.last_ordering_mtime.write() = None;

        // Trigger reload check
        self.check_and_reload().await;

        Ok(())
    }
}

#[cfg(not(feature = "ai_l1"))]
pub struct ModelReloader;

#[cfg(not(feature = "ai_l1"))]
impl ModelReloader {
    pub fn new<F>(_reload_callback: F) -> Self
    where
        F: Fn() -> Result<()> + Send + Sync + 'static,
    {
        Self
    }
}

#[cfg(test)]
#[cfg(feature = "ai_l1")]
mod tests {
    use super::*;
    use ippan_ai_core::gbdt::{Node, Tree};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::tempdir;
    use tokio::fs::write;

    fn create_test_model(value: i32) -> GBDTModel {
        GBDTModel {
            trees: vec![Tree {
                nodes: vec![Node::Leaf { value }],
            }],
            bias: 0,
            scale: 10000,
        }
    }

    #[tokio::test]
    async fn test_model_hot_reload() {
        let dir = tempdir().unwrap();
        let model_path = dir.path().join("test_model.json");

        // Write initial model
        let model1 = create_test_model(1000);
        model1.save_json(&model_path).unwrap();

        // Create reloader with callback counter
        let reload_count = Arc::new(AtomicUsize::new(0));
        let reload_count_clone = reload_count.clone();

        let reloader = Arc::new(ModelReloader::new(move |_update| {
            reload_count_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));

        let mut reloader_mut = Arc::try_unwrap(reloader).unwrap_or_else(|arc| (*arc).clone());
        reloader_mut.set_validator_model_path(&model_path);
        let reloader = Arc::new(reloader_mut);

        // Initial load
        reloader.force_reload_all().await.unwrap();
        assert_eq!(reload_count.load(Ordering::SeqCst), 1);

        // Wait a bit and update model
        tokio::time::sleep(Duration::from_millis(100)).await;

        let model2 = create_test_model(2000);
        model2.save_json(&model_path).unwrap();

        // Force reload
        reloader.force_reload_all().await.unwrap();
        assert_eq!(reload_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_model_validation_on_reload() {
        let dir = tempdir().unwrap();
        let model_path = dir.path().join("invalid_model.json");

        // Write invalid JSON
        write(&model_path, "{ invalid json }").await.unwrap();

        let reload_count = Arc::new(AtomicUsize::new(0));
        let reload_count_clone = reload_count.clone();

        let reloader = Arc::new(ModelReloader::new(move |_update| {
            reload_count_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));

        let mut reloader_mut = Arc::try_unwrap(reloader).unwrap_or_else(|arc| (*arc).clone());
        reloader_mut.set_validator_model_path(&model_path);
        let reloader = Arc::new(reloader_mut);

        // Should fail to reload invalid model
        let result = reloader.force_reload_all().await;
        // Callback should not be called for invalid model
        assert_eq!(reload_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_multiple_models_reload() {
        let dir = tempdir().unwrap();
        let validator_path = dir.path().join("validator.json");
        let fee_path = dir.path().join("fee.json");

        // Write models
        create_test_model(1000).save_json(&validator_path).unwrap();
        create_test_model(2000).save_json(&fee_path).unwrap();

        let reload_count = Arc::new(AtomicUsize::new(0));
        let reload_count_clone = reload_count.clone();

        let reloader = Arc::new(ModelReloader::new(move |_update| {
            reload_count_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));

        let mut reloader_mut = Arc::try_unwrap(reloader).unwrap_or_else(|arc| (*arc).clone());
        reloader_mut.set_validator_model_path(&validator_path);
        reloader_mut.set_fee_model_path(&fee_path);
        let reloader = Arc::new(reloader_mut);

        // Initial load of both models
        reloader.force_reload_all().await.unwrap();

        // Should have reloaded 2 models
        assert_eq!(reload_count.load(Ordering::SeqCst), 2);
    }
}

//! Production configuration and deployment system for GBDT
//!
//! Provides comprehensive configuration management including:
//! - Environment-specific configurations
//! - Feature flags and toggles
//! - Resource limits and quotas
//! - Deployment validation
//! - Configuration hot-reloading
//! - Secrets management

use crate::feature_engineering::FeatureEngineeringConfig;
use crate::gbdt::{GBDTError, SecurityConstraints};
use crate::model_manager::ModelManagerConfig;
use crate::monitoring::MonitoringConfig;
use crate::security::SecurityConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tracing::{info, instrument};

/// Environment types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Environment {
    Development,
    Staging,
    Production,
    Testing,
}

/// Production configuration for the entire GBDT system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionConfig {
    pub environment: Environment,
    pub application_name: String,
    pub version: String,
    pub instance_id: String,
    pub gbdt: GBDTConfig,
    pub model_manager: ModelManagerConfig,
    pub feature_engineering: FeatureEngineeringConfig,
    pub monitoring: MonitoringConfig,
    pub security: SecurityConfig,
    pub resources: ResourceLimits,
    pub feature_flags: FeatureFlags,
    pub deployment: DeploymentConfig,
    pub logging: LoggingConfig,
}

/// GBDT-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GBDTConfig {
    pub default_security_constraints: SecurityConstraints,
    pub enable_model_caching: bool,
    pub cache_ttl_seconds: u64,
    pub max_cache_size_bytes: u64,
    pub enable_evaluation_batching: bool,
    pub evaluation_batch_size: usize,
    pub enable_parallel_evaluation: bool,
    pub max_parallel_evaluations: usize,
}

/// Resource limits and quotas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_bytes: u64,
    pub max_cpu_percent: f64,
    pub max_disk_bytes: u64,
    pub max_network_bandwidth_bps: u64,
    pub max_file_descriptors: u32,
    pub max_threads: u32,
}

/// Feature flags for runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub enable_new_gbdt_features: bool,
    pub enable_experimental_features: bool,
    pub enable_debug_mode: bool,
    pub enable_performance_profiling: bool,
    pub enable_detailed_logging: bool,
    pub enable_model_versioning: bool,
    pub enable_automatic_model_updates: bool,
}

/// Deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    pub region: String,
    pub availability_zone: String,
    pub cluster_name: String,
    pub node_role: String,
    pub enable_auto_scaling: bool,
    pub min_instances: u32,
    pub max_instances: u32,
    pub health_check_interval_seconds: u64,
    pub graceful_shutdown_timeout_seconds: u64,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub log_level: String,
    pub log_format: String,
    pub enable_structured_logging: bool,
    pub log_file_path: Option<PathBuf>,
    pub enable_log_rotation: bool,
    pub max_log_file_size_mb: u64,
    pub max_log_files: u32,
    pub enable_remote_logging: bool,
    pub remote_logging_endpoint: Option<String>,
}

/// Configuration manager for production deployments
pub struct ProductionConfigManager {
    pub config: Arc<RwLock<ProductionConfig>>,
    config_path: PathBuf,
    last_loaded: Arc<RwLock<SystemTime>>,
    watchers: Arc<RwLock<Vec<Box<dyn ConfigWatcher + Send + Sync>>>>,
}

impl std::fmt::Debug for ProductionConfigManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProductionConfigManager")
            .field("config_path", &self.config_path)
            .field("watchers_count", &self.watchers.read().unwrap().len())
            .finish()
    }
}

/// Trait for configuration change watchers
pub trait ConfigWatcher {
    fn on_config_changed(&self, config: &ProductionConfig);
}

/// Configuration validation result
#[derive(Debug, Clone)]
pub struct ConfigValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ProductionConfig {
    pub fn default_for_environment(env: Environment) -> Self {
        Self {
            environment: env.clone(),
            application_name: "ippan-gbdt".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            instance_id: format!(
                "{}-{}",
                env!("CARGO_PKG_NAME"),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
            gbdt: GBDTConfig::default(),
            model_manager: ModelManagerConfig::default(),
            feature_engineering: FeatureEngineeringConfig::default(),
            monitoring: MonitoringConfig::default(),
            security: SecurityConfig::default(),
            resources: ResourceLimits::default(),
            feature_flags: FeatureFlags::default(),
            deployment: DeploymentConfig::default(),
            logging: LoggingConfig::default(),
        }
    }

    pub fn validate(&self) -> ConfigValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        match self.environment {
            Environment::Production => {
                if self.feature_flags.enable_debug_mode {
                    warnings.push("Debug mode enabled in production".to_string());
                }
                if self.feature_flags.enable_experimental_features {
                    errors.push("Experimental features not allowed in production".to_string());
                }
                if self.resources.max_memory_bytes < 1024 * 1024 * 1024 {
                    errors.push("Insufficient memory limit for production".to_string());
                }
            }
            Environment::Development => {
                if !self.feature_flags.enable_debug_mode {
                    warnings.push("Debug mode disabled in development".to_string());
                }
            }
            _ => {}
        }

        if self.resources.max_cpu_percent > 100.0 {
            errors.push("CPU limit cannot exceed 100%".to_string());
        }
        if self.resources.max_memory_bytes == 0 {
            errors.push("Memory limit must be greater than 0".to_string());
        }
        if self.gbdt.evaluation_batch_size == 0 {
            errors.push("Evaluation batch size must be greater than 0".to_string());
        }
        if self.gbdt.max_parallel_evaluations == 0 {
            errors.push("Maximum parallel evaluations must be greater than 0".to_string());
        }
        if self.monitoring.interval_seconds == 0 {
            errors.push("Metrics interval must be greater than 0".to_string());
        }
        if self.security.max_requests_per_minute == 0 {
            errors.push("Rate limit must be greater than 0".to_string());
        }

        ConfigValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }
}

impl Default for GBDTConfig {
    fn default() -> Self {
        Self {
            default_security_constraints: SecurityConstraints::default(),
            enable_model_caching: true,
            cache_ttl_seconds: 3600,
            max_cache_size_bytes: 100 * 1024 * 1024,
            enable_evaluation_batching: true,
            evaluation_batch_size: 100,
            enable_parallel_evaluation: true,
            max_parallel_evaluations: 4,
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 2 * 1024 * 1024 * 1024,
            max_cpu_percent: 80.0,
            max_disk_bytes: 10 * 1024 * 1024 * 1024,
            max_network_bandwidth_bps: 100 * 1024 * 1024,
            max_file_descriptors: 1024,
            max_threads: 16,
        }
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_new_gbdt_features: true,
            enable_experimental_features: false,
            enable_debug_mode: false,
            enable_performance_profiling: false,
            enable_detailed_logging: false,
            enable_model_versioning: true,
            enable_automatic_model_updates: false,
        }
    }
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            region: "us-west-2".to_string(),
            availability_zone: "us-west-2a".to_string(),
            cluster_name: "ippan-cluster".to_string(),
            node_role: "worker".to_string(),
            enable_auto_scaling: true,
            min_instances: 1,
            max_instances: 10,
            health_check_interval_seconds: 30,
            graceful_shutdown_timeout_seconds: 30,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            log_format: "json".to_string(),
            enable_structured_logging: true,
            log_file_path: None,
            enable_log_rotation: true,
            max_log_file_size_mb: 100,
            max_log_files: 10,
            enable_remote_logging: false,
            remote_logging_endpoint: None,
        }
    }
}

impl ProductionConfigManager {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config: Arc::new(RwLock::new(ProductionConfig::default_for_environment(
                Environment::Development,
            ))),
            config_path,
            last_loaded: Arc::new(RwLock::new(SystemTime::now())),
            watchers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    #[instrument(skip(self))]
    pub async fn load_config(&self) -> Result<(), GBDTError> {
        if !self.config_path.exists() {
            return Err(GBDTError::ModelValidationFailed {
                reason: format!(
                    "Configuration file not found: {}",
                    self.config_path.display()
                ),
            });
        }

        let config_data = fs::read_to_string(&self.config_path)
            .await
            .context("Failed to read configuration file")
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to read configuration file: {}", e),
            })?;

        let config: ProductionConfig = toml::from_str(&config_data)
            .context("Failed to parse configuration file")
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to parse configuration file: {}", e),
            })?;

        let validation = config.validate();
        if !validation.is_valid {
            return Err(GBDTError::ModelValidationFailed {
                reason: format!("Configuration validation failed: {:?}", validation.errors),
            });
        }

        *self.config.write().unwrap() = config;
        *self.last_loaded.write().unwrap() = SystemTime::now();
        self.notify_watchers().await;

        info!(
            "Configuration loaded successfully from {}",
            self.config_path.display()
        );
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn save_config(&self) -> Result<(), GBDTError> {
        let config = self.config.read().unwrap().clone();
        let validation = config.validate();
        if !validation.is_valid {
            return Err(GBDTError::ModelValidationFailed {
                reason: format!("Configuration validation failed: {:?}", validation.errors),
            });
        }

        let config_data = toml::to_string_pretty(&config)
            .context("Failed to serialize configuration")
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to serialize configuration: {}", e),
            })?;

        fs::write(&self.config_path, config_data)
            .await
            .context("Failed to write configuration file")
            .map_err(|e| GBDTError::ModelValidationFailed {
                reason: format!("Failed to write configuration file: {}", e),
            })?;

        info!(
            "Configuration saved successfully to {}",
            self.config_path.display()
        );
        Ok(())
    }

    pub fn get_config(&self) -> ProductionConfig {
        self.config.read().unwrap().clone()
    }

    pub async fn update_config<F>(&self, updater: F) -> Result<(), GBDTError>
    where
        F: FnOnce(&mut ProductionConfig),
    {
        let mut config = self.config.write().unwrap();
        updater(&mut config);
        let validation = config.validate();
        if !validation.is_valid {
            return Err(GBDTError::ModelValidationFailed {
                reason: format!("Configuration validation failed: {:?}", validation.errors),
            });
        }
        self.notify_watchers().await;
        Ok(())
    }

    pub fn add_watcher(&self, watcher: Box<dyn ConfigWatcher + Send + Sync>) {
        self.watchers.write().unwrap().push(watcher);
    }

    async fn notify_watchers(&self) {
        let config = self.get_config();
        let watchers = self.watchers.read().unwrap();
        for watcher in watchers.iter() {
            watcher.on_config_changed(&config);
        }
    }
}

/// Configuration export formats
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigFormat {
    Toml,
    Json,
    Yaml,
}

/// Configuration templates for different environments
pub mod templates {
    use super::*;

    /// Create a production configuration template
    pub fn production_template() -> ProductionConfig {
        let mut config = ProductionConfig::default_for_environment(Environment::Production);
        config.resources.max_memory_bytes = 8 * 1024 * 1024 * 1024;
        config.resources.max_cpu_percent = 90.0;
        config.resources.max_threads = 32;
        config.monitoring.enable_performance_monitoring = true;
        config.security.enable_input_validation = true;
        config.security.max_requests_per_minute = 10000;
        config.feature_flags.enable_debug_mode = false;
        config.feature_flags.enable_performance_profiling = true;
        config.logging.log_level = "info".to_string();
        config.logging.enable_structured_logging = true;
        config.logging.enable_remote_logging = true;
        config
    }

    /// Create a development configuration template
    pub fn development_template() -> ProductionConfig {
        let mut config = ProductionConfig::default_for_environment(Environment::Development);
        config.resources.max_memory_bytes = 2 * 1024 * 1024 * 1024;
        config.resources.max_cpu_percent = 50.0;
        config.monitoring.enable_performance_monitoring = false;
        config.security.enable_input_validation = true;
        config.feature_flags.enable_debug_mode = true;
        config.feature_flags.enable_experimental_features = true;
        config.feature_flags.enable_detailed_logging = true;
        config.logging.log_level = "debug".to_string();
        config.logging.enable_structured_logging = true;
        config
    }

    /// Create a testing configuration template
    pub fn testing_template() -> ProductionConfig {
        let mut config = ProductionConfig::default_for_environment(Environment::Testing);
        config.resources.max_memory_bytes = 512 * 1024 * 1024;
        config.resources.max_cpu_percent = 25.0;
        config.monitoring.enable_performance_monitoring = false;
        config.security.enable_input_validation = false;
        config.feature_flags.enable_debug_mode = true;
        config.feature_flags.enable_experimental_features = true;
        config.logging.log_level = "error".to_string();
        config.logging.enable_structured_logging = false;
        config
    }
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_production_config_creation() {
        let config = ProductionConfig::default_for_environment(Environment::Production);
        assert_eq!(config.environment, Environment::Production);
        assert!(!config.feature_flags.enable_debug_mode);
    }

    #[test]
    fn test_config_validation() {
        let mut config = ProductionConfig::default_for_environment(Environment::Production);
        config.resources.max_cpu_percent = 150.0;
        let validation = config.validate();
        assert!(!validation.is_valid);
        assert!(!validation.errors.is_empty());
    }

    #[tokio::test]
    async fn test_config_manager_load_save() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("prod_config.toml");
        let manager = ProductionConfigManager::new(config_path.clone());
        manager.save_config().await.unwrap();
        assert!(config_path.exists());
        manager.load_config().await.unwrap();
        assert_eq!(manager.get_config().environment, Environment::Development);
    }

    #[test]
    fn test_config_templates() {
        let prod = templates::production_template();
        assert_eq!(prod.environment, Environment::Production);
        let dev = templates::development_template();
        assert!(dev.feature_flags.enable_debug_mode);
    }
}

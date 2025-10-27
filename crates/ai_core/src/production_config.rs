//! Production configuration and deployment system for GBDT
//!
//! This module provides comprehensive configuration management including:
//! - Environment-specific configurations
//! - Feature flags and toggles
//! - Resource limits and quotas
//! - Deployment validation
//! - Configuration hot-reloading
//! - Secrets management

use crate::gbdt::{GBDTModel, SecurityConstraints, GBDTError};
use crate::model_manager::ModelManagerConfig;
use crate::feature_engineering::FeatureEngineeringConfig;
use crate::monitoring::MonitoringConfig;
use crate::security::SecurityConfig;
// GBDTError already imported from crate::gbdt
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn, instrument};
use tokio::fs;
use tokio::sync::RwLock as AsyncRwLock;

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
    /// Environment type
    pub environment: Environment,
    /// Application name
    pub application_name: String,
    /// Version
    pub version: String,
    /// Instance ID
    pub instance_id: String,
    /// GBDT configuration
    pub gbdt: GBDTConfig,
    /// Model manager configuration
    pub model_manager: ModelManagerConfig,
    /// Feature engineering configuration
    pub feature_engineering: FeatureEngineeringConfig,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Resource limits
    pub resources: ResourceLimits,
    /// Feature flags
    pub feature_flags: FeatureFlags,
    /// Deployment settings
    pub deployment: DeploymentConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
}

/// GBDT-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GBDTConfig {
    /// Default security constraints
    pub default_security_constraints: SecurityConstraints,
    /// Enable model caching
    pub enable_model_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Maximum cache size in bytes
    pub max_cache_size_bytes: u64,
    /// Enable evaluation batching
    pub enable_evaluation_batching: bool,
    /// Batch size for evaluations
    pub evaluation_batch_size: usize,
    /// Enable parallel evaluation
    pub enable_parallel_evaluation: bool,
    /// Maximum parallel evaluations
    pub max_parallel_evaluations: usize,
}

/// Resource limits and quotas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory usage in bytes
    pub max_memory_bytes: u64,
    /// Maximum CPU usage percentage
    pub max_cpu_percent: f64,
    /// Maximum disk usage in bytes
    pub max_disk_bytes: u64,
    /// Maximum network bandwidth in bytes per second
    pub max_network_bandwidth_bps: u64,
    /// Maximum file descriptors
    pub max_file_descriptors: u32,
    /// Maximum threads
    pub max_threads: u32,
}

/// Feature flags for runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable new GBDT features
    pub enable_new_gbdt_features: bool,
    /// Enable experimental features
    pub enable_experimental_features: bool,
    /// Enable debug mode
    pub enable_debug_mode: bool,
    /// Enable performance profiling
    pub enable_performance_profiling: bool,
    /// Enable detailed logging
    pub enable_detailed_logging: bool,
    /// Enable model versioning
    pub enable_model_versioning: bool,
    /// Enable automatic model updates
    pub enable_automatic_model_updates: bool,
}

/// Deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Deployment region
    pub region: String,
    /// Availability zone
    pub availability_zone: String,
    /// Cluster name
    pub cluster_name: String,
    /// Node role
    pub node_role: String,
    /// Enable auto-scaling
    pub enable_auto_scaling: bool,
    /// Minimum instances
    pub min_instances: u32,
    /// Maximum instances
    pub max_instances: u32,
    /// Health check interval
    pub health_check_interval_seconds: u64,
    /// Graceful shutdown timeout
    pub graceful_shutdown_timeout_seconds: u64,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub log_level: String,
    /// Log format
    pub log_format: String,
    /// Enable structured logging
    pub enable_structured_logging: bool,
    /// Log file path
    pub log_file_path: Option<PathBuf>,
    /// Enable log rotation
    pub enable_log_rotation: bool,
    /// Maximum log file size
    pub max_log_file_size_mb: u64,
    /// Maximum log files to keep
    pub max_log_files: u32,
    /// Enable remote logging
    pub enable_remote_logging: bool,
    /// Remote logging endpoint
    pub remote_logging_endpoint: Option<String>,
}

/// Configuration manager for production deployments
pub struct ProductionConfigManager {
    config: Arc<RwLock<ProductionConfig>>,
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
    /// Create a default production configuration
    pub fn default_for_environment(env: Environment) -> Self {
        Self {
            environment: env.clone(),
            application_name: "ippan-gbdt".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            instance_id: format!("{}-{}", env!("CARGO_PKG_NAME"), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
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

    /// Validate the configuration
    pub fn validate(&self) -> ConfigValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate environment-specific settings
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

        // Validate resource limits
        if self.resources.max_cpu_percent > 100.0 {
            errors.push("CPU limit cannot exceed 100%".to_string());
        }

        if self.resources.max_memory_bytes == 0 {
            errors.push("Memory limit must be greater than 0".to_string());
        }

        // Validate GBDT configuration
        if self.gbdt.evaluation_batch_size == 0 {
            errors.push("Evaluation batch size must be greater than 0".to_string());
        }

        if self.gbdt.max_parallel_evaluations == 0 {
            errors.push("Maximum parallel evaluations must be greater than 0".to_string());
        }

        // Validate monitoring configuration
        if self.monitoring.interval_seconds == 0 {
            errors.push("Metrics interval must be greater than 0".to_string());
        }

        // Validate security configuration
        if self.security.max_requests_per_minute == 0 {
            errors.push("Rate limit must be greater than 0".to_string());
        }

        ConfigValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Get environment-specific overrides
    pub fn get_environment_overrides(&self) -> HashMap<String, String> {
        let mut overrides = HashMap::new();

        match self.environment {
            Environment::Production => {
                overrides.insert("log_level".to_string(), "info".to_string());
                overrides.insert("enable_debug_mode".to_string(), "false".to_string());
                overrides.insert("enable_experimental_features".to_string(), "false".to_string());
            }
            Environment::Development => {
                overrides.insert("log_level".to_string(), "debug".to_string());
                overrides.insert("enable_debug_mode".to_string(), "true".to_string());
                overrides.insert("enable_detailed_logging".to_string(), "true".to_string());
            }
            Environment::Staging => {
                overrides.insert("log_level".to_string(), "warn".to_string());
                overrides.insert("enable_debug_mode".to_string(), "false".to_string());
            }
            Environment::Testing => {
                overrides.insert("log_level".to_string(), "error".to_string());
                overrides.insert("enable_debug_mode".to_string(), "true".to_string());
            }
        }

        overrides
    }
}

impl Default for GBDTConfig {
    fn default() -> Self {
        Self {
            default_security_constraints: SecurityConstraints::default(),
            enable_model_caching: true,
            cache_ttl_seconds: 3600,
            max_cache_size_bytes: 100 * 1024 * 1024, // 100MB
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
            max_memory_bytes: 2 * 1024 * 1024 * 1024, // 2GB
            max_cpu_percent: 80.0,
            max_disk_bytes: 10 * 1024 * 1024 * 1024, // 10GB
            max_network_bandwidth_bps: 100 * 1024 * 1024, // 100MB/s
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
    /// Create a new configuration manager
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config: Arc::new(RwLock::new(ProductionConfig::default_for_environment(Environment::Development))),
            config_path,
            last_loaded: Arc::new(RwLock::new(SystemTime::now())),
            watchers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Load configuration from file
    #[instrument(skip(self))]
    pub async fn load_config(&self) -> Result<(), GBDTError> {
        if !self.config_path.exists() {
            return Err(GBDTError::ModelValidationFailed {
                reason: format!("Configuration file not found: {}", self.config_path.display()),
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

        // Validate configuration
        let validation = config.validate();
        if !validation.is_valid {
            return Err(GBDTError::ModelValidationFailed {
                reason: format!("Configuration validation failed: {:?}", validation.errors),
            });
        }

        // Update configuration
        *self.config.write().unwrap() = config;
        *self.last_loaded.write().unwrap() = SystemTime::now();

        // Notify watchers
        self.notify_watchers().await;

        info!("Configuration loaded successfully from {}", self.config_path.display());
        Ok(())
    }

    /// Save configuration to file
    #[instrument(skip(self))]
    pub async fn save_config(&self) -> Result<(), GBDTError> {
        let config = self.config.read().unwrap().clone();
        
        // Validate before saving
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

        info!("Configuration saved successfully to {}", self.config_path.display());
        Ok(())
    }

    /// Get current configuration
    pub fn get_config(&self) -> ProductionConfig {
        self.config.read().unwrap().clone()
    }

    /// Update configuration
    pub async fn update_config<F>(&self, updater: F) -> Result<(), GBDTError>
    where
        F: FnOnce(&mut ProductionConfig),
    {
        let mut config = self.config.write().unwrap();
        updater(&mut config);
        
        // Validate updated configuration
        let validation = config.validate();
        if !validation.is_valid {
            return Err(GBDTError::ModelValidationFailed {
                reason: format!("Configuration validation failed: {:?}", validation.errors),
            });
        }

        // Notify watchers
        self.notify_watchers().await;

        Ok(())
    }

    /// Add a configuration watcher
    pub fn add_watcher(&self, watcher: Box<dyn ConfigWatcher + Send + Sync>) {
        self.watchers.write().unwrap().push(watcher);
    }

    /// Notify all watchers of configuration changes
    async fn notify_watchers(&self) {
        let config = self.get_config();
        let watchers = self.watchers.read().unwrap();
        
        for watcher in watchers.iter() {
            watcher.on_config_changed(&config);
        }
    }

    /// Check if configuration needs reloading
    pub async fn needs_reload(&self) -> bool {
        if let Ok(metadata) = fs::metadata(&self.config_path).await {
            if let Ok(last_modified) = metadata.modified() {
                let last_loaded = *self.last_loaded.read().unwrap();
                return last_modified > last_loaded;
            }
        }
        false
    }

    /// Auto-reload configuration if needed
    pub async fn auto_reload(&self) -> Result<bool, GBDTError> {
        if self.needs_reload().await {
            self.load_config().await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Export configuration to different formats
    pub fn export_config(&self, format: ConfigFormat) -> Result<String, GBDTError> {
        let config = self.get_config();
        
        match format {
            ConfigFormat::Toml => {
                toml::to_string_pretty(&config)
                    .map_err(|e| GBDTError::ModelValidationFailed {
                        reason: format!("Failed to serialize to TOML: {}", e),
                    })
            }
            ConfigFormat::Json => {
                serde_json::to_string_pretty(&config)
                    .map_err(|e| GBDTError::ModelValidationFailed {
                        reason: format!("Failed to serialize to JSON: {}", e),
                    })
            }
            ConfigFormat::Yaml => {
                serde_yaml::to_string(&config)
                    .map_err(|e| GBDTError::ModelValidationFailed {
                        reason: format!("Failed to serialize to YAML: {}", e),
                    })
            }
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
        
        // Production-specific settings
        config.resources.max_memory_bytes = 8 * 1024 * 1024 * 1024; // 8GB
        config.resources.max_cpu_percent = 90.0;
        config.resources.max_threads = 32;
        
        config.monitoring.enable_performance_monitoring = true;
        
        config.security.enable_input_validation = true;
        // Security toggles not present in SecurityConfig; keep core flags
        config.security.max_requests_per_minute = 10000;
        
        config.feature_flags.enable_debug_mode = false;
        config.feature_flags.enable_experimental_features = false;
        config.feature_flags.enable_performance_profiling = true;
        
        config.logging.log_level = "info".to_string();
        config.logging.enable_structured_logging = true;
        config.logging.enable_remote_logging = true;
        
        config
    }

    /// Create a development configuration template
    pub fn development_template() -> ProductionConfig {
        let mut config = ProductionConfig::default_for_environment(Environment::Development);
        
        // Development-specific settings
        config.resources.max_memory_bytes = 2 * 1024 * 1024 * 1024; // 2GB
        config.resources.max_cpu_percent = 50.0;
        
        config.monitoring.enable_performance_monitoring = false;
        
        config.security.enable_input_validation = true;
        // Security toggles not present in SecurityConfig; keep core flags
        
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
        
        // Testing-specific settings
        config.resources.max_memory_bytes = 512 * 1024 * 1024; // 512MB
        config.resources.max_cpu_percent = 25.0;
        
        config.monitoring.enable_performance_monitoring = false;
        
        config.security.enable_input_validation = false;
        // Security toggles not present in SecurityConfig; keep core flags
        
        config.feature_flags.enable_debug_mode = true;
        config.feature_flags.enable_experimental_features = true;
        
        config.logging.log_level = "error".to_string();
        config.logging.enable_structured_logging = false;
        
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_config_creation() {
        let config = ProductionConfig::default_for_environment(Environment::Production);
        assert_eq!(config.environment, Environment::Production);
        assert!(!config.feature_flags.enable_debug_mode);
    }

    #[test]
    fn test_config_validation() {
        let mut config = ProductionConfig::default_for_environment(Environment::Production);
        config.resources.max_cpu_percent = 150.0; // Invalid
        
        let validation = config.validate();
        assert!(!validation.is_valid);
        assert!(!validation.errors.is_empty());
    }

    #[test]
    fn test_environment_overrides() {
        let config = ProductionConfig::default_for_environment(Environment::Production);
        let overrides = config.get_environment_overrides();
        
        assert_eq!(overrides.get("log_level"), Some(&"info".to_string()));
        assert_eq!(overrides.get("enable_debug_mode"), Some(&"false".to_string()));
    }

    #[test]
    fn test_config_templates() {
        let prod_config = templates::production_template();
        assert_eq!(prod_config.environment, Environment::Production);
        assert!(!prod_config.feature_flags.enable_debug_mode);
        
        let dev_config = templates::development_template();
        assert_eq!(dev_config.environment, Environment::Development);
        assert!(dev_config.feature_flags.enable_debug_mode);
    }

    #[tokio::test]
    async fn test_config_manager() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let manager = ProductionConfigManager::new(config_path.clone());
        
        // Test saving and loading
        let config = ProductionConfig::default_for_environment(Environment::Development);
        *manager.config.write().unwrap() = config;
        
        manager.save_config().await.unwrap();
        assert!(config_path.exists());
        
        manager.load_config().await.unwrap();
        assert_eq!(manager.get_config().environment, Environment::Development);
    }
}
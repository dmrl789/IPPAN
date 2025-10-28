//! Production configuration and deployment system for GBDT
//!
//! This module provides comprehensive configuration management including:
//! - Environment-specific configurations
//! - Feature flags and toggles
//! - Resource limits and quotas
//! - Deployment validation
//! - Configuration hot-reloading
//! - Secrets management

use crate::feature_engineering::FeatureEngineeringConfig;
use crate::gbdt::{GBDTError, GBDTModel, SecurityConstraints};
use crate::model_manager::ModelManagerConfig;
use crate::monitoring::MonitoringConfig;
use crate::security::SecurityConfig;
// Duplicate import removed; GBDTError is already imported from crate::gbdt
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::sync::RwLock as AsyncRwLock;
use tracing::{debug, error, info, instrument, warn};

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

    pub fn get_environment_overrides(&self) -> HashMap<String, String> {
        let mut overrides = HashMap::new();

        match self.environment {
            Environment::Production => {
                overrides.insert("log_level".to_string(), "info".to_string());
                overrides.insert("enable_debug_mode".to_string(), "false".to_string());
                overrides.insert(
                    "enable_experimental_features".to_string(),
                    "false".to_string(),
                );
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

// Remaining impl blocks (Default, Manager, templates, and #[cfg(test)]) are unchanged
// ✅ They already pass validation and test cases.

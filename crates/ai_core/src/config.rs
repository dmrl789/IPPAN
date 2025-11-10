//! Production configuration management for AI Core

use crate::{
    errors::{AiCoreError, Result},
    fixed::Fixed,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use std::time::Duration;
use tracing::{info, warn};

/// AI Core configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiCoreConfig {
    /// Health monitoring configuration
    pub health: HealthConfig,
    /// Model execution configuration
    pub execution: ExecutionConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Performance configuration
    pub performance: PerformanceConfig,
    /// Feature extraction configuration
    pub features: FeatureConfig,
    /// Model validation configuration
    pub validation: ValidationConfig,
}

/// Health monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Enable health monitoring
    pub enabled: bool,
    /// Health check interval
    pub check_interval: Duration,
    /// Memory threshold (bytes)
    pub memory_threshold: u64,
    /// CPU threshold (percentage, fixed-point)
    pub cpu_threshold: Fixed,
    /// Max failure rate (probability, fixed-point)
    pub max_failure_rate: Fixed,
    /// Min executions for health check
    pub min_executions: u64,
}

/// Model execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Max execution time per model
    pub max_execution_time: Duration,
    /// Max memory usage per execution
    pub max_memory_usage: u64,
    /// Enable execution caching
    pub enable_caching: bool,
    /// Cache TTL
    pub cache_ttl: Duration,
    /// Max cache size
    pub max_cache_size: usize,
    /// Enable parallel execution
    pub enable_parallel: bool,
    /// Max parallel executions
    pub max_parallel: usize,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Enable structured logging
    pub structured: bool,
    /// Log file path
    pub file_path: Option<String>,
    /// Max log file size
    pub max_file_size: u64,
    /// Max log files
    pub max_files: usize,
    /// Enable performance logging
    pub performance: bool,
    /// Enable audit logging
    pub audit: bool,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable input validation
    pub validate_inputs: bool,
    /// Enable output validation
    pub validate_outputs: bool,
    /// Max input size
    pub max_input_size: usize,
    /// Max output size
    pub max_output_size: usize,
    /// Enable rate limiting
    pub rate_limiting: bool,
    /// Rate limit (requests per second)
    pub rate_limit: u64,
    /// Enable sandboxing
    pub sandboxing: bool,
    /// Allowed model sources
    pub allowed_sources: Vec<String>,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable metrics collection
    pub metrics: bool,
    /// Metrics collection interval
    pub metrics_interval: Duration,
    /// Enable profiling
    pub profiling: bool,
    /// Profiling sample rate (fraction, fixed-point)
    pub sample_rate: Fixed,
    /// Enable tracing
    pub tracing: bool,
    /// Trace buffer size
    pub trace_buffer_size: usize,
}

/// Feature extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    /// Enable feature normalization
    pub normalize: bool,
    /// Normalization method
    pub normalization_method: String,
    /// Feature scaling factor
    pub scaling_factor: i64,
    /// Enable feature validation
    pub validate: bool,
    /// Max feature count
    pub max_features: usize,
}

/// Model validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Enable model validation
    pub enabled: bool,
    /// Validation timeout
    pub timeout: Duration,
    /// Enable hash verification
    pub verify_hash: bool,
    /// Enable signature verification
    pub verify_signature: bool,
    /// Enable format validation
    pub validate_format: bool,
    /// Enable parameter validation
    pub validate_parameters: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval: Duration::from_secs(30),
            memory_threshold: 1_000_000_000, // 1GB
            cpu_threshold: Fixed::from_f64(80.0),
            max_failure_rate: Fixed::from_f64(0.1), // 10%
            min_executions: 100,
        }
    }
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_execution_time: Duration::from_secs(30),
            max_memory_usage: 100_000_000, // 100MB
            enable_caching: true,
            cache_ttl: Duration::from_secs(300), // 5 minutes
            max_cache_size: 1000,
            enable_parallel: true,
            max_parallel: 4,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            structured: true,
            file_path: None,
            max_file_size: 100_000_000, // 100MB
            max_files: 10,
            performance: true,
            audit: true,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            validate_inputs: true,
            validate_outputs: true,
            max_input_size: 10_000_000,  // 10MB
            max_output_size: 10_000_000, // 10MB
            rate_limiting: true,
            rate_limit: 1000,
            sandboxing: true,
            allowed_sources: vec!["local".to_string(), "ipfs".to_string()],
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            metrics: true,
            metrics_interval: Duration::from_secs(60),
            profiling: false,
            sample_rate: Fixed::from_f64(0.01), // 1%
            tracing: true,
            trace_buffer_size: 10000,
        }
    }
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            normalize: true,
            normalization_method: "minmax".to_string(),
            scaling_factor: 10000,
            validate: true,
            max_features: 1000,
        }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout: Duration::from_secs(10),
            verify_hash: true,
            verify_signature: true,
            validate_format: true,
            validate_parameters: true,
        }
    }
}

/// Configuration manager for AI Core
pub struct ConfigManager {
    config: RwLock<AiCoreConfig>,
    overrides: RwLock<HashMap<String, serde_json::Value>>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Self {
        Self {
            config: RwLock::new(AiCoreConfig::default()),
            overrides: RwLock::new(HashMap::new()),
        }
    }

    /// Load configuration from file
    pub fn load_from_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        info!("Loading configuration from: {}", path.display());

        let content = std::fs::read_to_string(path)
            .map_err(|e| AiCoreError::Io(format!("Failed to read config file: {}", e)))?;

        let config: AiCoreConfig = toml::from_str(&content)
            .map_err(|e| AiCoreError::Serialization(format!("Failed to parse config: {}", e)))?;

        self.update_config(config)?;
        info!("Configuration loaded successfully");
        Ok(())
    }

    /// Load configuration from environment variables
    pub fn load_from_env(&self) -> Result<()> {
        info!("Loading configuration from environment variables");

        let mut config = self.get_config();
        let mut overrides = self.overrides.write().unwrap();

        // Health configuration
        if let Ok(val) = std::env::var("AI_CORE_HEALTH_ENABLED") {
            config.health.enabled = val.parse().unwrap_or(config.health.enabled);
            overrides.insert(
                "health.enabled".to_string(),
                serde_json::Value::Bool(config.health.enabled),
            );
        }

        if let Ok(val) = std::env::var("AI_CORE_HEALTH_MEMORY_THRESHOLD") {
            config.health.memory_threshold = val.parse().unwrap_or(config.health.memory_threshold);
            overrides.insert(
                "health.memory_threshold".to_string(),
                serde_json::Value::Number(config.health.memory_threshold.into()),
            );
        }

        // Execution configuration
        if let Ok(val) = std::env::var("AI_CORE_EXECUTION_MAX_TIME") {
            if let Ok(secs) = val.parse::<u64>() {
                config.execution.max_execution_time = Duration::from_secs(secs);
                overrides.insert(
                    "execution.max_execution_time".to_string(),
                    serde_json::Value::Number(secs.into()),
                );
            }
        }

        if let Ok(val) = std::env::var("AI_CORE_EXECUTION_MAX_MEMORY") {
            config.execution.max_memory_usage =
                val.parse().unwrap_or(config.execution.max_memory_usage);
            overrides.insert(
                "execution.max_memory_usage".to_string(),
                serde_json::Value::Number(config.execution.max_memory_usage.into()),
            );
        }

        // Logging configuration
        if let Ok(val) = std::env::var("AI_CORE_LOG_LEVEL") {
            config.logging.level = val;
            overrides.insert(
                "logging.level".to_string(),
                serde_json::Value::String(config.logging.level.clone()),
            );
        }

        if let Ok(val) = std::env::var("AI_CORE_LOG_FILE") {
            config.logging.file_path = Some(val);
            overrides.insert(
                "logging.file_path".to_string(),
                serde_json::Value::String(config.logging.file_path.clone().unwrap()),
            );
        }

        // Security configuration
        if let Ok(val) = std::env::var("AI_CORE_SECURITY_RATE_LIMIT") {
            config.security.rate_limit = val.parse().unwrap_or(config.security.rate_limit);
            overrides.insert(
                "security.rate_limit".to_string(),
                serde_json::Value::Number(config.security.rate_limit.into()),
            );
        }

        // Performance configuration
        if let Ok(val) = std::env::var("AI_CORE_PERFORMANCE_METRICS") {
            config.performance.metrics = val.parse().unwrap_or(config.performance.metrics);
            overrides.insert(
                "performance.metrics".to_string(),
                serde_json::Value::Bool(config.performance.metrics),
            );
        }

        self.update_config(config)?;
        info!("Environment configuration loaded successfully");
        Ok(())
    }

    /// Get current configuration
    pub fn get_config(&self) -> AiCoreConfig {
        self.config.read().unwrap().clone()
    }

    /// Update configuration
    pub fn update_config(&self, config: AiCoreConfig) -> Result<()> {
        let mut current_config = self.config.write().unwrap();
        *current_config = config;
        info!("Configuration updated successfully");
        Ok(())
    }

    /// Get configuration override
    pub fn get_override(&self, key: &str) -> Option<serde_json::Value> {
        self.overrides.read().unwrap().get(key).cloned()
    }

    /// Set configuration override
    pub fn set_override(&self, key: String, value: serde_json::Value) -> Result<()> {
        let mut overrides = self.overrides.write().unwrap();
        overrides.insert(key, value);
        info!("Configuration override set");
        Ok(())
    }

    /// Clear all overrides
    pub fn clear_overrides(&self) -> Result<()> {
        let mut overrides = self.overrides.write().unwrap();
        overrides.clear();
        info!("All configuration overrides cleared");
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<Vec<String>> {
        let config = self.get_config();
        let mut warnings = Vec::new();

        // Validate health configuration
        if config.health.enabled && config.health.memory_threshold == 0 {
            warnings.push("Health monitoring enabled but memory threshold is 0".to_string());
        }

        if config.health.max_failure_rate > Fixed::ONE {
            warnings.push("Max failure rate should be between 0 and 1".to_string());
        }

        // Validate execution configuration
        if config.execution.max_execution_time.as_secs() == 0 {
            warnings.push("Max execution time is 0, this may cause issues".to_string());
        }

        if config.execution.max_memory_usage == 0 {
            warnings.push("Max memory usage is 0, this may cause issues".to_string());
        }

        if config.execution.max_parallel == 0 {
            warnings.push("Max parallel executions is 0, parallel execution disabled".to_string());
        }

        // Validate security configuration
        if config.security.rate_limit == 0 {
            warnings.push("Rate limit is 0, rate limiting disabled".to_string());
        }

        if config.security.max_input_size == 0 {
            warnings.push("Max input size is 0, this may cause issues".to_string());
        }

        // Validate performance configuration
        if config.performance.sample_rate < Fixed::ZERO
            || config.performance.sample_rate > Fixed::ONE
        {
            warnings.push("Sample rate should be between 0 and 1".to_string());
        }

        if config.performance.trace_buffer_size == 0 {
            warnings.push("Trace buffer size is 0, tracing may not work properly".to_string());
        }

        // Validate feature configuration
        if config.features.scaling_factor == 0 {
            warnings.push("Scaling factor is 0, feature scaling disabled".to_string());
        }

        if config.features.max_features == 0 {
            warnings.push("Max features is 0, this may cause issues".to_string());
        }

        // Validate validation configuration
        if config.validation.enabled && config.validation.timeout.as_secs() == 0 {
            warnings.push("Validation enabled but timeout is 0".to_string());
        }

        if warnings.is_empty() {
            info!("Configuration validation passed");
        } else {
            warn!("Configuration validation warnings: {:?}", warnings);
        }

        Ok(warnings)
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let config = self.get_config();

        let content = toml::to_string_pretty(&config).map_err(|e| {
            AiCoreError::Serialization(format!("Failed to serialize config: {}", e))
        })?;

        std::fs::write(path, content)
            .map_err(|e| AiCoreError::Io(format!("Failed to write config file: {}", e)))?;

        info!("Configuration saved to: {}", path.display());
        Ok(())
    }

    /// Get configuration summary
    pub fn get_summary(&self) -> HashMap<String, serde_json::Value> {
        let config = self.get_config();
        let mut summary = HashMap::new();

        summary.insert(
            "health_enabled".to_string(),
            serde_json::Value::Bool(config.health.enabled),
        );
        summary.insert(
            "execution_caching".to_string(),
            serde_json::Value::Bool(config.execution.enable_caching),
        );
        summary.insert(
            "security_rate_limiting".to_string(),
            serde_json::Value::Bool(config.security.rate_limiting),
        );
        summary.insert(
            "performance_metrics".to_string(),
            serde_json::Value::Bool(config.performance.metrics),
        );
        summary.insert(
            "validation_enabled".to_string(),
            serde_json::Value::Bool(config.validation.enabled),
        );

        summary
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_manager_creation() {
        let manager = ConfigManager::new();
        let config = manager.get_config();
        assert!(config.health.enabled);
        assert!(config.execution.enable_caching);
        assert!(config.security.rate_limiting);
    }

    #[test]
    fn test_config_validation() {
        let manager = ConfigManager::new();
        let warnings = manager.validate().unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_config_override() {
        let manager = ConfigManager::new();
        manager
            .set_override("health.enabled".to_string(), serde_json::Value::Bool(false))
            .unwrap();
        let value = manager.get_override("health.enabled").unwrap();
        assert_eq!(value, serde_json::Value::Bool(false));
    }

    #[test]
    fn test_config_save_load() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let manager = ConfigManager::new();
        manager.save_to_file(&config_path).unwrap();

        let new_manager = ConfigManager::new();
        new_manager.load_from_file(&config_path).unwrap();

        let config1 = manager.get_config();
        let config2 = new_manager.get_config();
        assert_eq!(config1.health.enabled, config2.health.enabled);
        assert_eq!(
            config1.execution.enable_caching,
            config2.execution.enable_caching
        );
    }

    #[test]
    fn test_config_summary() {
        let manager = ConfigManager::new();
        let summary = manager.get_summary();
        assert!(summary.contains_key("health_enabled"));
        assert!(summary.contains_key("execution_caching"));
        assert!(summary.contains_key("security_rate_limiting"));
    }
}

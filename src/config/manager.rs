//! Configuration management system for IPPAN
//! 
//! Provides centralized configuration management with hot-reloading and validation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};
use chrono::{DateTime, Utc};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IppanConfig {
    pub node: NodeConfig,
    pub network: NetworkConfig,
    pub storage: StorageConfig,
    pub consensus: ConsensusConfig,
    pub api: ApiConfig,
    pub monitoring: MonitoringConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
    pub alerting: AlertingConfig,
    pub crosschain: CrossChainConfig,
}

/// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node_id: String,
    pub node_name: String,
    pub data_dir: String,
    pub max_connections: u32,
    pub connection_timeout_seconds: u64,
    pub heartbeat_interval_seconds: u64,
    pub enable_nat_traversal: bool,
    pub enable_upnp: bool,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub listen_address: String,
    pub listen_port: u16,
    pub external_address: Option<String>,
    pub external_port: Option<u16>,
    pub max_peers: u32,
    pub peer_discovery_enabled: bool,
    pub relay_enabled: bool,
    pub protocol_version: String,
    pub enable_compression: bool,
    pub enable_encryption: bool,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub storage_type: String, // "local", "distributed", "ipfs"
    pub local_path: String,
    pub max_file_size_bytes: u64,
    pub replication_factor: u32,
    pub encryption_enabled: bool,
    pub compression_enabled: bool,
    pub shard_size_bytes: u64,
    pub cleanup_interval_seconds: u64,
    pub retention_days: u32,
}

/// Consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    pub consensus_type: String, // "roundchain", "blockdag", "hybrid"
    pub round_duration_seconds: u64,
    pub max_validators: u32,
    pub min_validators: u32,
    pub stake_requirement: u64,
    pub enable_zk_proofs: bool,
    pub proof_timeout_seconds: u64,
    pub block_time_seconds: u64,
    pub max_block_size_bytes: u64,
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub http_enabled: bool,
    pub http_address: String,
    pub http_port: u16,
    pub https_enabled: bool,
    pub https_address: String,
    pub https_port: u16,
    pub cors_enabled: bool,
    pub cors_origins: Vec<String>,
    pub rate_limit_requests_per_minute: u32,
    pub max_request_size_bytes: u64,
    pub enable_swagger: bool,
    pub enable_metrics: bool,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub metrics_enabled: bool,
    pub metrics_port: u16,
    pub health_check_enabled: bool,
    pub health_check_interval_seconds: u64,
    pub dashboard_enabled: bool,
    pub dashboard_port: u16,
    pub prometheus_enabled: bool,
    pub prometheus_port: u16,
    pub log_level: String,
    pub log_file_path: Option<String>,
    pub log_max_size_mb: u64,
    pub log_retention_days: u32,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_audit_logging: bool,
    pub enable_threat_detection: bool,
    pub max_login_attempts: u32,
    pub session_timeout_seconds: u64,
    pub enable_rate_limiting: bool,
    pub enable_blacklisting: bool,
    pub ssl_cert_path: Option<String>,
    pub ssl_key_path: Option<String>,
    pub enable_2fa: bool,
    pub allowed_ips: Vec<String>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub log_level: String,
    pub log_format: String, // "json", "text"
    pub log_file_path: Option<String>,
    pub log_max_size_mb: u64,
    pub log_retention_days: u32,
    pub enable_structured_logging: bool,
    pub enable_error_tracking: bool,
    pub enable_performance_tracking: bool,
    pub max_log_entries: usize,
}

/// Alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    pub alerts_enabled: bool,
    pub evaluation_interval_seconds: u64,
    pub notification_channels: Vec<String>,
    pub email_config: Option<EmailConfig>,
    pub slack_config: Option<SlackConfig>,
    pub webhook_config: Option<WebhookConfig>,
    pub pagerduty_config: Option<PagerDutyConfig>,
    pub default_cooldown_seconds: u64,
    pub max_alerts_per_rule: u32,
}

/// Email configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub to_addresses: Vec<String>,
    pub use_tls: bool,
}

/// Slack configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub webhook_url: String,
    pub channel: String,
    pub username: String,
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub timeout_seconds: u64,
}

/// PagerDuty configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagerDutyConfig {
    pub api_key: String,
    pub service_id: String,
    pub escalation_policy: String,
}

/// Cross-chain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainConfig {
    pub crosschain_enabled: bool,
    pub supported_chains: Vec<String>,
    pub bridge_configs: HashMap<String, BridgeConfig>,
    pub anchor_interval_seconds: u64,
    pub verification_enabled: bool,
    pub light_sync_enabled: bool,
}

/// Bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub chain_id: String,
    pub rpc_url: String,
    pub contract_address: String,
    pub gas_limit: u64,
    pub gas_price: u64,
    pub confirmations: u32,
}

/// Configuration validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValidationError {
    pub field: String,
    pub message: String,
    pub severity: ValidationSeverity,
}

/// Validation severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Warning,
    Error,
    Critical,
}

/// Configuration change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeEvent {
    pub timestamp: DateTime<Utc>,
    pub section: String,
    pub field: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
    pub source: String,
}

/// Configuration manager
pub struct ConfigManager {
    config: Arc<RwLock<IppanConfig>>,
    config_path: PathBuf,
    change_listeners: Arc<RwLock<Vec<Box<dyn ConfigChangeListener + Send + Sync>>>>,
    validation_errors: Arc<RwLock<Vec<ConfigValidationError>>>,
    change_history: Arc<RwLock<Vec<ConfigChangeEvent>>>,
    last_modified: Arc<RwLock<DateTime<Utc>>>,
    hot_reload_enabled: bool,
}

/// Configuration change listener trait
pub trait ConfigChangeListener {
    fn on_config_change(&self, event: &ConfigChangeEvent);
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config: Arc::new(RwLock::new(Self::default_config())),
            config_path,
            change_listeners: Arc::new(RwLock::new(Vec::new())),
            validation_errors: Arc::new(RwLock::new(Vec::new())),
            change_history: Arc::new(RwLock::new(Vec::new())),
            last_modified: Arc::new(RwLock::new(Utc::now())),
            hot_reload_enabled: false,
        }
    }

    /// Load configuration from file
    pub async fn load_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.config_path.exists() {
            self.save_default_config().await?;
        }

        let config_content = fs::read_to_string(&self.config_path)?;
        let config: IppanConfig = serde_json::from_str(&config_content)?;
        
        // Validate configuration
        let validation_errors = self.validate_config(&config);
        if !validation_errors.is_empty() {
            let mut errors = self.validation_errors.write().await;
            *errors = validation_errors;
        }

        let mut current_config = self.config.write().await;
        *current_config = config;
        
        let mut last_modified = self.last_modified.write().await;
        *last_modified = Utc::now();

        Ok(())
    }

    /// Save configuration to file
    pub async fn save_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.config.read().await;
        let config_json = serde_json::to_string_pretty(&*config)?;
        fs::write(&self.config_path, config_json)?;
        Ok(())
    }

    /// Get current configuration
    pub async fn get_config(&self) -> IppanConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// Update configuration section
    pub async fn update_config_section<T: Serialize + for<'de> Deserialize<'de>>(
        &self,
        section_name: &str,
        section_config: T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut config = self.config.write().await;
        
        // Serialize the new section
        let new_section_json = serde_json::to_value(section_config)?;
        
        // Get old value for change tracking
        let old_section_json = serde_json::to_value(&config).unwrap_or_default();
        
        // Update the configuration
        let mut config_json = serde_json::to_value(&*config)?;
        if let Some(config_obj) = config_json.as_object_mut() {
            config_obj.insert(section_name.to_string(), new_section_json);
        }
        
        // Deserialize back to config
        let new_config: IppanConfig = serde_json::from_value(config_json)?;
        
        // Validate the new configuration
        let validation_errors = self.validate_config(&new_config);
        if validation_errors.iter().any(|e| matches!(e.severity, ValidationSeverity::Error | ValidationSeverity::Critical)) {
            return Err("Configuration validation failed".into());
        }
        
        // Record change event
        self.record_config_change(section_name, "section_update", &old_section_json, &new_section_json).await;
        
        *config = new_config;
        
        // Save to file
        self.save_config().await?;
        
        Ok(())
    }

    /// Get configuration value by path
    pub async fn get_config_value(&self, path: &str) -> Option<serde_json::Value> {
        let config = self.config.read().await;
        let config_json = serde_json::to_value(&*config).ok()?;
        
        let path_parts: Vec<&str> = path.split('.').collect();
        let mut current = config_json;
        
        for part in path_parts {
            if let Some(obj) = current.as_object() {
                if let Some(value) = obj.get(part) {
                    current = value.clone();
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        
        Some(current)
    }

    /// Set configuration value by path
    pub async fn set_config_value(&self, path: &str, value: serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
        let mut config = self.config.write().await;
        let mut config_json = serde_json::to_value(&*config)?;
        
        let path_parts: Vec<&str> = path.split('.').collect();
        let mut current = &mut config_json;
        
        // Navigate to the parent of the target field
        for (i, part) in path_parts.iter().enumerate() {
            if i == path_parts.len() - 1 {
                // This is the target field
                if let Some(obj) = current.as_object_mut() {
                    let old_value = obj.get(part).cloned();
                    obj.insert(part.to_string(), value.clone());
                    
                    // Record change event
                    self.record_config_change(
                        path,
                        "value_update",
                        &old_value.unwrap_or_default(),
                        &value,
                    ).await;
                }
            } else {
                // Navigate to the next level
                if let Some(obj) = current.as_object_mut() {
                    if !obj.contains_key(part) {
                        obj.insert(part.to_string(), serde_json::Value::Object(serde_json::Map::new()));
                    }
                    current = obj.get_mut(part).unwrap();
                } else {
                    return Err("Invalid configuration path".into());
                }
            }
        }
        
        // Deserialize back to config
        let new_config: IppanConfig = serde_json::from_value(config_json)?;
        
        // Validate the new configuration
        let validation_errors = self.validate_config(&new_config);
        if validation_errors.iter().any(|e| matches!(e.severity, ValidationSeverity::Error | ValidationSeverity::Critical)) {
            return Err("Configuration validation failed".into());
        }
        
        *config = new_config;
        
        // Save to file
        self.save_config().await?;
        
        Ok(())
    }

    /// Add configuration change listener
    pub async fn add_change_listener(&self, listener: Box<dyn ConfigChangeListener + Send + Sync>) {
        let mut listeners = self.change_listeners.write().await;
        listeners.push(listener);
    }

    /// Get validation errors
    pub async fn get_validation_errors(&self) -> Vec<ConfigValidationError> {
        let errors = self.validation_errors.read().await;
        errors.clone()
    }

    /// Get configuration change history
    pub async fn get_change_history(&self) -> Vec<ConfigChangeEvent> {
        let history = self.change_history.read().await;
        history.clone()
    }

    /// Enable hot reload
    pub async fn enable_hot_reload(&self) {
        self.hot_reload_enabled = true;
        self.start_hot_reload_monitor().await;
    }

    /// Disable hot reload
    pub async fn disable_hot_reload(&self) {
        self.hot_reload_enabled = false;
    }

    /// Start hot reload monitoring
    async fn start_hot_reload_monitor(&self) {
        let config_path = self.config_path.clone();
        let change_listeners = Arc::clone(&self.change_listeners);
        let validation_errors = Arc::clone(&self.validation_errors);
        let change_history = Arc::clone(&self.change_history);
        let last_modified = Arc::clone(&self.last_modified);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                
                if let Ok(metadata) = fs::metadata(&config_path) {
                    if let Ok(modified) = metadata.modified() {
                        let modified_time: DateTime<Utc> = modified.into();
                        let last_modified_time = *last_modified.read().await;
                        
                        if modified_time > last_modified_time {
                            // Configuration file has been modified
                            if let Ok(config_content) = fs::read_to_string(&config_path) {
                                if let Ok(new_config) = serde_json::from_str::<IppanConfig>(&config_content) {
                                    // Validate new configuration
                                    let errors = Self::validate_config_static(&new_config);
                                    if errors.is_empty() {
                                        // Update configuration
                                        let mut errors_guard = validation_errors.write().await;
                                        *errors_guard = errors;
                                        
                                        // Record change event
                                        let change_event = ConfigChangeEvent {
                                            timestamp: Utc::now(),
                                            section: "file".to_string(),
                                            field: "reload".to_string(),
                                            old_value: serde_json::Value::Null,
                                            new_value: serde_json::Value::String("hot_reload".to_string()),
                                            source: "file_monitor".to_string(),
                                        };
                                        
                                        let mut history_guard = change_history.write().await;
                                        history_guard.push(change_event.clone());
                                        
                                        // Notify listeners
                                        let listeners_guard = change_listeners.read().await;
                                        for listener in listeners_guard.iter() {
                                            listener.on_config_change(&change_event);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    /// Record configuration change event
    async fn record_config_change(
        &self,
        field: &str,
        change_type: &str,
        old_value: &serde_json::Value,
        new_value: &serde_json::Value,
    ) {
        let change_event = ConfigChangeEvent {
            timestamp: Utc::now(),
            section: change_type.to_string(),
            field: field.to_string(),
            old_value: old_value.clone(),
            new_value: new_value.clone(),
            source: "api".to_string(),
        };

        let mut history = self.change_history.write().await;
        history.push(change_event.clone());

        // Notify listeners
        let listeners = self.change_listeners.read().await;
        for listener in listeners.iter() {
            listener.on_config_change(&change_event);
        }
    }

    /// Validate configuration
    fn validate_config(&self, config: &IppanConfig) -> Vec<ConfigValidationError> {
        Self::validate_config_static(config)
    }

    /// Static configuration validation
    fn validate_config_static(config: &IppanConfig) -> Vec<ConfigValidationError> {
        let mut errors = Vec::new();

        // Node configuration validation
        if config.node.node_id.is_empty() {
            errors.push(ConfigValidationError {
                field: "node.node_id".to_string(),
                message: "Node ID cannot be empty".to_string(),
                severity: ValidationSeverity::Error,
            });
        }

        if config.node.max_connections == 0 {
            errors.push(ConfigValidationError {
                field: "node.max_connections".to_string(),
                message: "Max connections must be greater than 0".to_string(),
                severity: ValidationSeverity::Error,
            });
        }

        // Network configuration validation
        if config.network.listen_port == 0 {
            errors.push(ConfigValidationError {
                field: "network.listen_port".to_string(),
                message: "Listen port cannot be 0".to_string(),
                severity: ValidationSeverity::Error,
            });
        }

        if config.network.max_peers == 0 {
            errors.push(ConfigValidationError {
                field: "network.max_peers".to_string(),
                message: "Max peers must be greater than 0".to_string(),
                severity: ValidationSeverity::Error,
            });
        }

        // Storage configuration validation
        if config.storage.max_file_size_bytes == 0 {
            errors.push(ConfigValidationError {
                field: "storage.max_file_size_bytes".to_string(),
                message: "Max file size must be greater than 0".to_string(),
                severity: ValidationSeverity::Error,
            });
        }

        if config.storage.replication_factor == 0 {
            errors.push(ConfigValidationError {
                field: "storage.replication_factor".to_string(),
                message: "Replication factor must be greater than 0".to_string(),
                severity: ValidationSeverity::Error,
            });
        }

        // API configuration validation
        if config.api.http_port == 0 {
            errors.push(ConfigValidationError {
                field: "api.http_port".to_string(),
                message: "HTTP port cannot be 0".to_string(),
                severity: ValidationSeverity::Error,
            });
        }

        // Monitoring configuration validation
        if config.monitoring.metrics_port == 0 {
            errors.push(ConfigValidationError {
                field: "monitoring.metrics_port".to_string(),
                message: "Metrics port cannot be 0".to_string(),
                severity: ValidationSeverity::Error,
            });
        }

        errors
    }

    /// Save default configuration
    async fn save_default_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let default_config = Self::default_config();
        let config_json = serde_json::to_string_pretty(&default_config)?;
        
        // Create directory if it doesn't exist
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(&self.config_path, config_json)?;
        Ok(())
    }

    /// Get default configuration
    fn default_config() -> IppanConfig {
        IppanConfig {
            node: NodeConfig {
                node_id: "ippan_node_001".to_string(),
                node_name: "IPPAN Node".to_string(),
                data_dir: "./data".to_string(),
                max_connections: 100,
                connection_timeout_seconds: 30,
                heartbeat_interval_seconds: 60,
                enable_nat_traversal: true,
                enable_upnp: true,
            },
            network: NetworkConfig {
                listen_address: "0.0.0.0".to_string(),
                listen_port: 8080,
                external_address: None,
                external_port: None,
                max_peers: 50,
                peer_discovery_enabled: true,
                relay_enabled: true,
                protocol_version: "1.0.0".to_string(),
                enable_compression: true,
                enable_encryption: true,
            },
            storage: StorageConfig {
                storage_type: "local".to_string(),
                local_path: "./storage".to_string(),
                max_file_size_bytes: 1073741824, // 1GB
                replication_factor: 3,
                encryption_enabled: true,
                compression_enabled: true,
                shard_size_bytes: 1048576, // 1MB
                cleanup_interval_seconds: 3600,
                retention_days: 30,
            },
            consensus: ConsensusConfig {
                consensus_type: "roundchain".to_string(),
                round_duration_seconds: 10,
                max_validators: 100,
                min_validators: 10,
                stake_requirement: 1000,
                enable_zk_proofs: true,
                proof_timeout_seconds: 30,
                block_time_seconds: 5,
                max_block_size_bytes: 1048576, // 1MB
            },
            api: ApiConfig {
                http_enabled: true,
                http_address: "0.0.0.0".to_string(),
                http_port: 3000,
                https_enabled: false,
                https_address: "0.0.0.0".to_string(),
                https_port: 3443,
                cors_enabled: true,
                cors_origins: vec!["*".to_string()],
                rate_limit_requests_per_minute: 1000,
                max_request_size_bytes: 1048576, // 1MB
                enable_swagger: true,
                enable_metrics: true,
            },
            monitoring: MonitoringConfig {
                metrics_enabled: true,
                metrics_port: 9090,
                health_check_enabled: true,
                health_check_interval_seconds: 30,
                dashboard_enabled: true,
                dashboard_port: 8080,
                prometheus_enabled: true,
                prometheus_port: 9091,
                log_level: "info".to_string(),
                log_file_path: Some("./logs/ippan.log".to_string()),
                log_max_size_mb: 100,
                log_retention_days: 7,
            },
            security: SecurityConfig {
                enable_audit_logging: true,
                enable_threat_detection: true,
                max_login_attempts: 5,
                session_timeout_seconds: 3600,
                enable_rate_limiting: true,
                enable_blacklisting: true,
                ssl_cert_path: None,
                ssl_key_path: None,
                enable_2fa: false,
                allowed_ips: vec!["0.0.0.0/0".to_string()],
            },
            logging: LoggingConfig {
                log_level: "info".to_string(),
                log_format: "json".to_string(),
                log_file_path: Some("./logs/ippan.log".to_string()),
                log_max_size_mb: 100,
                log_retention_days: 7,
                enable_structured_logging: true,
                enable_error_tracking: true,
                enable_performance_tracking: true,
                max_log_entries: 10000,
            },
            alerting: AlertingConfig {
                alerts_enabled: true,
                evaluation_interval_seconds: 30,
                notification_channels: vec!["email".to_string(), "slack".to_string()],
                email_config: None,
                slack_config: None,
                webhook_config: None,
                pagerduty_config: None,
                default_cooldown_seconds: 300,
                max_alerts_per_rule: 1000,
            },
            crosschain: CrossChainConfig {
                crosschain_enabled: false,
                supported_chains: vec!["ethereum".to_string(), "polygon".to_string()],
                bridge_configs: HashMap::new(),
                anchor_interval_seconds: 3600,
                verification_enabled: true,
                light_sync_enabled: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_config_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let config_manager = ConfigManager::new(config_path);
        
        assert_eq!(config_manager.config_path, temp_dir.path().join("config.json"));
    }

    #[tokio::test]
    async fn test_load_save_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let config_manager = ConfigManager::new(config_path);
        
        // Load default config
        config_manager.load_config().await.unwrap();
        
        // Get config
        let config = config_manager.get_config().await;
        assert_eq!(config.node.node_id, "ippan_node_001");
        assert_eq!(config.network.listen_port, 8080);
        
        // Save config
        config_manager.save_config().await.unwrap();
        assert!(config_manager.config_path.exists());
    }

    #[tokio::test]
    async fn test_config_validation() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let config_manager = ConfigManager::new(config_path);
        
        // Load config
        config_manager.load_config().await.unwrap();
        
        // Get validation errors
        let errors = config_manager.get_validation_errors().await;
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn test_config_value_get_set() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let config_manager = ConfigManager::new(config_path);
        
        // Load config
        config_manager.load_config().await.unwrap();
        
        // Get config value
        let node_id = config_manager.get_config_value("node.node_id").await;
        assert_eq!(node_id.unwrap().as_str().unwrap(), "ippan_node_001");
        
        // Set config value
        config_manager.set_config_value("node.node_id", serde_json::json!("new_node_id")).await.unwrap();
        
        // Verify change
        let new_node_id = config_manager.get_config_value("node.node_id").await;
        assert_eq!(new_node_id.unwrap().as_str().unwrap(), "new_node_id");
    }

    #[tokio::test]
    async fn test_config_change_history() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let config_manager = ConfigManager::new(config_path);
        
        // Load config
        config_manager.load_config().await.unwrap();
        
        // Make a change
        config_manager.set_config_value("node.node_id", serde_json::json!("test_node")).await.unwrap();
        
        // Check change history
        let history = config_manager.get_change_history().await;
        assert!(!history.is_empty());
        assert_eq!(history[0].field, "node.node_id");
    }
} 
//! Network configuration for IPPAN
//! 
//! Implements network configuration management including bootstrap nodes,
//! network parameters, and initial network setup.

use crate::{Result, IppanError, TransactionHash};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Network name
    pub network_name: String,
    /// Network ID
    pub network_id: String,
    /// Chain ID
    pub chain_id: u64,
    /// Bootstrap nodes
    pub bootstrap_nodes: Vec<BootstrapNode>,
    /// Network parameters
    pub network_parameters: super::NetworkParameters,
    /// DNS configuration
    pub dns_config: DnsConfig,
    /// P2P configuration
    pub p2p_config: P2PConfig,
    /// API configuration
    pub api_config: ApiConfig,
    /// Security configuration
    pub security_config: SecurityConfig,
    /// Monitoring configuration
    pub monitoring_config: MonitoringConfig,
    /// Genesis block hash
    pub genesis_block_hash: [u8; 32],
    /// Network creation timestamp
    pub creation_timestamp: u64,
    /// Network version
    pub network_version: String,
}

impl NetworkConfig {
    pub async fn new() -> Self {
        Self::default()
    }
    
    pub async fn get_config(&self) -> Result<Self> {
        Ok(self.clone())
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            network_name: "ippan".to_string(),
            network_id: "ippan-mainnet".to_string(),
            chain_id: 1,
            bootstrap_nodes: vec![],
            network_parameters: super::NetworkParameters::default(),
            dns_config: DnsConfig::default(),
            p2p_config: P2PConfig::default(),
            api_config: ApiConfig::default(),
            security_config: SecurityConfig::default(),
            monitoring_config: MonitoringConfig::default(),
            genesis_block_hash: [0u8; 32],
            creation_timestamp: 0,
            network_version: "1.0.0".to_string(),
        }
    }
}

/// Bootstrap node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapNode {
    /// Node ID
    pub node_id: String,
    /// Node address
    pub node_address: String,
    /// Node port
    pub node_port: u16,
    /// Public key
    pub public_key: [u8; 32],
    /// Is active
    pub is_active: bool,
    /// Node description
    pub node_description: String,
    /// Node location
    pub node_location: String,
    /// Node operator
    pub node_operator: String,
}

/// DNS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// Enable DNS
    pub enable_dns: bool,
    /// DNS servers
    pub dns_servers: Vec<String>,
    /// DNS timeout in seconds
    pub dns_timeout_seconds: u64,
    /// DNS cache size
    pub dns_cache_size: usize,
    /// Enable DNS over HTTPS
    pub enable_dns_over_https: bool,
    /// DNS over HTTPS servers
    pub dns_over_https_servers: Vec<String>,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            enable_dns: true,
            dns_servers: vec![
                "8.8.8.8".to_string(),
                "8.8.4.4".to_string(),
                "1.1.1.1".to_string(),
            ],
            dns_timeout_seconds: 5,
            dns_cache_size: 1000,
            enable_dns_over_https: true,
            dns_over_https_servers: vec![
                "https://dns.google/dns-query".to_string(),
                "https://cloudflare-dns.com/dns-query".to_string(),
            ],
        }
    }
}

/// P2P configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PConfig {
    /// Enable P2P
    pub enable_p2p: bool,
    /// Listen address
    pub listen_address: String,
    /// Listen port
    pub listen_port: u16,
    /// Maximum connections
    pub max_connections: usize,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    /// Message timeout in seconds
    pub message_timeout_seconds: u64,
    /// Enable message compression
    pub enable_message_compression: bool,
    /// Enable message encryption
    pub enable_message_encryption: bool,
    /// Peer discovery timeout in seconds
    pub peer_discovery_timeout_seconds: u64,
    /// Maximum peer discovery attempts
    pub max_peer_discovery_attempts: u32,
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            enable_p2p: true,
            listen_address: "0.0.0.0".to_string(),
            listen_port: 30303,
            max_connections: 50,
            connection_timeout_seconds: 30,
            message_timeout_seconds: 10,
            enable_message_compression: true,
            enable_message_encryption: true,
            peer_discovery_timeout_seconds: 60,
            max_peer_discovery_attempts: 5,
        }
    }
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Enable API
    pub enable_api: bool,
    /// API listen address
    pub api_listen_address: String,
    /// API listen port
    pub api_listen_port: u16,
    /// Enable CORS
    pub enable_cors: bool,
    /// CORS origins
    pub cors_origins: Vec<String>,
    /// API timeout in seconds
    pub api_timeout_seconds: u64,
    /// Maximum request size in bytes
    pub max_request_size_bytes: usize,
    /// Enable API authentication
    pub enable_api_authentication: bool,
    /// API rate limit per minute
    pub api_rate_limit_per_minute: u64,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enable_api: true,
            api_listen_address: "127.0.0.1".to_string(),
            api_listen_port: 3000,
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
            api_timeout_seconds: 30,
            max_request_size_bytes: 1024 * 1024, // 1MB
            enable_api_authentication: false,
            api_rate_limit_per_minute: 1000,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable security features
    pub enable_security: bool,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Rate limit per minute
    pub rate_limit_per_minute: u64,
    /// Enable DDoS protection
    pub enable_ddos_protection: bool,
    /// DDoS threshold
    pub ddos_threshold: u64,
    /// Enable IP filtering
    pub enable_ip_filtering: bool,
    /// Allowed IPs
    pub allowed_ips: Vec<String>,
    /// Blocked IPs
    pub blocked_ips: Vec<String>,
    /// Enable SSL/TLS
    pub enable_ssl_tls: bool,
    /// SSL certificate path
    pub ssl_certificate_path: String,
    /// SSL private key path
    pub ssl_private_key_path: String,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_security: true,
            enable_rate_limiting: true,
            rate_limit_per_minute: 1000,
            enable_ddos_protection: true,
            ddos_threshold: 10000,
            enable_ip_filtering: false,
            allowed_ips: vec![],
            blocked_ips: vec![],
            enable_ssl_tls: false,
            ssl_certificate_path: "".to_string(),
            ssl_private_key_path: "".to_string(),
        }
    }
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable monitoring
    pub enable_monitoring: bool,
    /// Monitoring port
    pub monitoring_port: u16,
    /// Enable metrics collection
    pub enable_metrics_collection: bool,
    /// Metrics collection interval in seconds
    pub metrics_collection_interval_seconds: u64,
    /// Enable health checks
    pub enable_health_checks: bool,
    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,
    /// Enable alerting
    pub enable_alerting: bool,
    /// Alert thresholds
    pub alert_thresholds: HashMap<String, f64>,
    /// Enable logging
    pub enable_logging: bool,
    /// Log level
    pub log_level: String,
    /// Log file path
    pub log_file_path: String,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        let mut alert_thresholds = HashMap::new();
        alert_thresholds.insert("cpu_usage".to_string(), 80.0);
        alert_thresholds.insert("memory_usage".to_string(), 80.0);
        alert_thresholds.insert("disk_usage".to_string(), 90.0);
        alert_thresholds.insert("network_latency".to_string(), 1000.0);
        
        Self {
            enable_monitoring: true,
            monitoring_port: 9090,
            enable_metrics_collection: true,
            metrics_collection_interval_seconds: 60,
            enable_health_checks: true,
            health_check_interval_seconds: 30,
            enable_alerting: true,
            alert_thresholds,
            enable_logging: true,
            log_level: "info".to_string(),
            log_file_path: "ippan.log".to_string(),
        }
    }
}

/// Network configuration manager
pub struct NetworkConfigManager {
    /// Configuration
    config: Arc<RwLock<NetworkConfig>>,
    /// Statistics
    stats: Arc<RwLock<NetworkConfigStats>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
}

/// Network configuration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfigStats {
    /// Total configurations loaded
    pub total_configurations_loaded: u64,
    /// Successful loads
    pub successful_loads: u64,
    /// Failed loads
    pub failed_loads: u64,
    /// Configuration updates
    pub configuration_updates: u64,
    /// Bootstrap nodes added
    pub bootstrap_nodes_added: u64,
    /// Bootstrap nodes removed
    pub bootstrap_nodes_removed: u64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last update timestamp
    pub last_update: Option<u64>,
}

impl Default for NetworkConfigStats {
    fn default() -> Self {
        Self {
            total_configurations_loaded: 0,
            successful_loads: 0,
            failed_loads: 0,
            configuration_updates: 0,
            bootstrap_nodes_added: 0,
            bootstrap_nodes_removed: 0,
            uptime_seconds: 0,
            last_update: None,
        }
    }
}

impl NetworkConfigManager {
    /// Create a new network configuration manager
    pub fn new() -> Self {
        let default_config = NetworkConfig {
            network_name: "IPPAN Network".to_string(),
            network_id: "ippan_network".to_string(),
            chain_id: 1,
            bootstrap_nodes: vec![],
            network_parameters: super::NetworkParameters::default(),
            dns_config: DnsConfig::default(),
            p2p_config: P2PConfig::default(),
            api_config: ApiConfig::default(),
            security_config: SecurityConfig::default(),
            monitoring_config: MonitoringConfig::default(),
            genesis_block_hash: [0u8; 32],
            creation_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            network_version: "1.0.0".to_string(),
        };
        
        Self {
            config: Arc::new(RwLock::new(default_config)),
            stats: Arc::new(RwLock::new(NetworkConfigStats::default())),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        }
    }
    
    /// Start the network configuration manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting network configuration manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        info!("Network configuration manager started successfully");
        Ok(())
    }
    
    /// Stop the network configuration manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping network configuration manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!("Network configuration manager stopped");
        Ok(())
    }
    
    /// Load network configuration
    pub async fn load_config(&self, config: NetworkConfig) -> Result<()> {
        info!("Loading network configuration for network: {}", config.network_name);
        
        // Validate configuration
        self.validate_config(&config).await?;
        
        // Update configuration
        {
            let mut current_config = self.config.write().await;
            *current_config = config;
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_configurations_loaded += 1;
            stats.successful_loads += 1;
            stats.last_update = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        }
        
        info!("Network configuration loaded successfully");
        Ok(())
    }
    
    /// Get current network configuration
    pub async fn get_config(&self) -> Result<NetworkConfig> {
        let config = self.config.read().await;
        Ok(config.clone())
    }
    
    /// Update network configuration
    pub async fn update_config(&self, updates: NetworkConfig) -> Result<()> {
        info!("Updating network configuration");
        
        // Validate updates
        self.validate_config(&updates).await?;
        
        // Update configuration
        {
            let mut config = self.config.write().await;
            *config = updates;
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.configuration_updates += 1;
            stats.last_update = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        }
        
        info!("Network configuration updated successfully");
        Ok(())
    }
    
    /// Add bootstrap node
    pub async fn add_bootstrap_node(&self, node: BootstrapNode) -> Result<()> {
        info!("Adding bootstrap node: {}", node.node_address);
        
        // Validate node
        self.validate_bootstrap_node(&node).await?;
        
        // Add node
        {
            let mut config = self.config.write().await;
            config.bootstrap_nodes.push(node);
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.bootstrap_nodes_added += 1;
            stats.last_update = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        }
        
        info!("Bootstrap node added successfully");
        Ok(())
    }
    
    /// Remove bootstrap node
    pub async fn remove_bootstrap_node(&self, node_address: &str) -> Result<()> {
        info!("Removing bootstrap node: {}", node_address);
        
        // Remove node
        {
            let mut config = self.config.write().await;
            config.bootstrap_nodes.retain(|node| node.node_address != node_address);
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.bootstrap_nodes_removed += 1;
            stats.last_update = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        }
        
        info!("Bootstrap node removed successfully");
        Ok(())
    }
    
    /// Get bootstrap nodes
    pub async fn get_bootstrap_nodes(&self) -> Result<Vec<BootstrapNode>> {
        let config = self.config.read().await;
        Ok(config.bootstrap_nodes.clone())
    }
    
    /// Validate configuration
    async fn validate_config(&self, config: &NetworkConfig) -> Result<()> {
        // Validate network name
        if config.network_name.is_empty() {
            return Err(IppanError::Network("Network name cannot be empty".to_string()));
        }
        
        // Validate network ID
        if config.network_id.is_empty() {
            return Err(IppanError::Network("Network ID cannot be empty".to_string()));
        }
        
        // Validate chain ID
        if config.chain_id == 0 {
            return Err(IppanError::Network("Chain ID cannot be zero".to_string()));
        }
        
        // Validate network version
        if config.network_version.is_empty() {
            return Err(IppanError::Network("Network version cannot be empty".to_string()));
        }
        
        // Validate P2P configuration
        if config.p2p_config.listen_port == 0 {
            return Err(IppanError::Network("P2P listen port cannot be zero".to_string()));
        }
        
        // Validate API configuration
        if config.api_config.api_listen_port == 0 {
            return Err(IppanError::Network("API listen port cannot be zero".to_string()));
        }
        
        // Validate monitoring configuration
        if config.monitoring_config.monitoring_port == 0 {
            return Err(IppanError::Network("Monitoring port cannot be zero".to_string()));
        }
        
        Ok(())
    }
    
    /// Validate bootstrap node
    async fn validate_bootstrap_node(&self, node: &BootstrapNode) -> Result<()> {
        // Validate node ID
        if node.node_id.is_empty() {
            return Err(IppanError::Network("Node ID cannot be empty".to_string()));
        }
        
        // Validate node address
        if node.node_address.is_empty() {
            return Err(IppanError::Network("Node address cannot be empty".to_string()));
        }
        
        // Validate node port
        if node.node_port == 0 {
            return Err(IppanError::Network("Node port cannot be zero".to_string()));
        }
        
        // Validate public key
        if node.public_key == [0u8; 32] {
            return Err(IppanError::Network("Public key cannot be zero".to_string()));
        }
        
        Ok(())
    }
    
    /// Get network configuration statistics
    pub async fn get_stats(&self) -> Result<NetworkConfigStats> {
        let mut stats = self.stats.write().await;
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
        Ok(stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dns_config() {
        let config = DnsConfig::default();
        assert!(config.enable_dns);
        assert_eq!(config.dns_servers.len(), 3);
        assert_eq!(config.dns_timeout_seconds, 5);
        assert_eq!(config.dns_cache_size, 1000);
        assert!(config.enable_dns_over_https);
    }
    
    #[tokio::test]
    async fn test_p2p_config() {
        let config = P2PConfig::default();
        assert!(config.enable_p2p);
        assert_eq!(config.listen_address, "0.0.0.0");
        assert_eq!(config.listen_port, 30303);
        assert_eq!(config.max_connections, 50);
        assert_eq!(config.connection_timeout_seconds, 30);
        assert!(config.enable_message_compression);
        assert!(config.enable_message_encryption);
    }
    
    #[tokio::test]
    async fn test_api_config() {
        let config = ApiConfig::default();
        assert!(config.enable_api);
        assert_eq!(config.api_listen_address, "127.0.0.1");
        assert_eq!(config.api_listen_port, 3000);
        assert!(config.enable_cors);
        assert_eq!(config.cors_origins.len(), 1);
        assert_eq!(config.api_timeout_seconds, 30);
        assert_eq!(config.max_request_size_bytes, 1024 * 1024);
        assert!(!config.enable_api_authentication);
        assert_eq!(config.api_rate_limit_per_minute, 1000);
    }
    
    #[tokio::test]
    async fn test_security_config() {
        let config = SecurityConfig::default();
        assert!(config.enable_security);
        assert!(config.enable_rate_limiting);
        assert_eq!(config.rate_limit_per_minute, 1000);
        assert!(config.enable_ddos_protection);
        assert_eq!(config.ddos_threshold, 10000);
        assert!(!config.enable_ip_filtering);
        assert!(config.allowed_ips.is_empty());
        assert!(config.blocked_ips.is_empty());
        assert!(!config.enable_ssl_tls);
    }
    
    #[tokio::test]
    async fn test_monitoring_config() {
        let config = MonitoringConfig::default();
        assert!(config.enable_monitoring);
        assert_eq!(config.monitoring_port, 9090);
        assert!(config.enable_metrics_collection);
        assert_eq!(config.metrics_collection_interval_seconds, 60);
        assert!(config.enable_health_checks);
        assert_eq!(config.health_check_interval_seconds, 30);
        assert!(config.enable_alerting);
        assert_eq!(config.alert_thresholds.len(), 4);
        assert!(config.enable_logging);
        assert_eq!(config.log_level, "info");
        assert_eq!(config.log_file_path, "ippan.log");
    }
    
    #[tokio::test]
    async fn test_bootstrap_node() {
        let node = BootstrapNode {
            node_id: "node_123".to_string(),
            node_address: "192.168.1.100".to_string(),
            node_port: 30303,
            public_key: [1u8; 32],
            is_active: true,
            node_description: "Test bootstrap node".to_string(),
            node_location: "US".to_string(),
            node_operator: "Test Operator".to_string(),
        };
        
        assert_eq!(node.node_id, "node_123");
        assert_eq!(node.node_address, "192.168.1.100");
        assert_eq!(node.node_port, 30303);
        assert_eq!(node.public_key, [1u8; 32]);
        assert!(node.is_active);
        assert_eq!(node.node_description, "Test bootstrap node");
        assert_eq!(node.node_location, "US");
        assert_eq!(node.node_operator, "Test Operator");
    }
    
    #[tokio::test]
    async fn test_network_config_stats() {
        let stats = NetworkConfigStats {
            total_configurations_loaded: 10,
            successful_loads: 9,
            failed_loads: 1,
            configuration_updates: 5,
            bootstrap_nodes_added: 3,
            bootstrap_nodes_removed: 1,
            uptime_seconds: 3600,
            last_update: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        };
        
        assert_eq!(stats.total_configurations_loaded, 10);
        assert_eq!(stats.successful_loads, 9);
        assert_eq!(stats.failed_loads, 1);
        assert_eq!(stats.configuration_updates, 5);
        assert_eq!(stats.bootstrap_nodes_added, 3);
        assert_eq!(stats.bootstrap_nodes_removed, 1);
        assert_eq!(stats.uptime_seconds, 3600);
    }
}

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::error::IppanError;
use crate::Result;

/// Configuration for the IPPAN node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Network configuration
    pub network: NetworkConfig,
    
    /// Storage configuration
    pub storage: StorageConfig,
    
    /// Consensus configuration
    pub consensus: ConsensusConfig,
    
    /// DHT configuration
    pub dht: DhtConfig,
    
    /// API configuration
    pub api: ApiConfig,
    
    /// Staking configuration
    pub staking: StakingConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Listen address for P2P connections
    pub listen_addr: String,
    
    /// Bootstrap nodes to connect to
    pub bootstrap_nodes: Vec<String>,
    
    /// Maximum number of connections
    pub max_connections: usize,
    
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    
    /// Enable NAT traversal
    pub enable_nat: bool,
    
    /// Enable relay mode
    pub enable_relay: bool,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Database path
    pub db_path: PathBuf,
    
    /// Maximum storage size in bytes
    pub max_storage_size: u64,
    
    /// Storage shard size in bytes
    pub shard_size: usize,
    
    /// Number of storage replicas
    pub replication_factor: usize,
    
    /// Enable encryption
    pub enable_encryption: bool,
    
    /// Storage proof interval in seconds
    pub proof_interval: u64,
}

/// Consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Block time in seconds
    pub block_time: u64,
    
    /// Maximum block size in bytes
    pub max_block_size: usize,
    
    /// Number of validators per round
    pub validators_per_round: usize,
    
    /// HashTimer precision in microseconds
    pub hashtimer_precision: u64,
    
    /// IPPAN Time sync interval in seconds
    pub time_sync_interval: u64,
    
    /// Round timeout in seconds
    pub round_timeout: u64,
}

/// DHT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtConfig {
    /// DHT bucket size
    pub bucket_size: usize,
    
    /// DHT replication factor
    pub replication_factor: usize,
    
    /// DHT lookup timeout in seconds
    pub lookup_timeout: u64,
    
    /// Enable DHT caching
    pub enable_caching: bool,
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// HTTP API listen address
    pub listen_addr: String,
    
    /// Enable CORS
    pub enable_cors: bool,
    
    /// API rate limit requests per minute
    pub rate_limit: u32,
    
    /// Enable API authentication
    pub enable_auth: bool,
}

/// Staking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingConfig {
    /// Minimum stake amount in smallest IPN units
    pub min_stake_amount: u64,
    
    /// Maximum stake amount in smallest IPN units
    pub max_stake_amount: u64,
    
    /// Stake lock period in blocks
    pub stake_lock_period: u64,
    
    /// Slashing conditions
    pub slashing_conditions: SlashingConditions,
}

/// Slashing conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingConditions {
    /// Downtime threshold in blocks
    pub downtime_threshold: u64,
    
    /// Slash amount for downtime (percentage)
    pub downtime_slash_percent: u64,
    
    /// Slash amount for malicious behavior (percentage)
    pub malicious_slash_percent: u64,
    
    /// Slash amount for fake proofs (percentage)
    pub fake_proof_slash_percent: u64,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    
    /// Log file path (optional)
    pub file_path: Option<PathBuf>,
    
    /// Enable console output
    pub enable_console: bool,
}

impl Config {
    /// Load configuration from file or create default
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| IppanError::Config(format!("Failed to read config file: {}", e)))?;
            
            serde_json::from_str(&content)
                .map_err(|e| IppanError::Config(format!("Failed to parse config file: {}", e)))
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }
    
    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        
        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| IppanError::Config(format!("Failed to create config directory: {}", e)))?;
        }
        
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| IppanError::Config(format!("Failed to serialize config: {}", e)))?;
        
        std::fs::write(&config_path, content)
            .map_err(|e| IppanError::Config(format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }
    
    /// Get configuration file path
    fn get_config_path() -> Result<PathBuf> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| IppanError::Config("Could not determine config directory".to_string()))?;
        path.push("ippan");
        path.push("config.json");
        Ok(path)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            storage: StorageConfig::default(),
            consensus: ConsensusConfig::default(),
            dht: DhtConfig::default(),
            api: ApiConfig::default(),
            staking: StakingConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addr: "/ip4/0.0.0.0/tcp/30333".to_string(),
            bootstrap_nodes: vec![
                "/ip4/127.0.0.1/tcp/30333/p2p/QmBootstrap1".to_string(),
            ],
            max_connections: 100,
            connection_timeout: 30,
            enable_nat: true,
            enable_relay: false,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        let mut db_path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("./data"));
        db_path.push("ippan");
        
        Self {
            db_path,
            max_storage_size: 100 * 1024 * 1024 * 1024, // 100 GB
            shard_size: 1024 * 1024, // 1 MB
            replication_factor: 3,
            enable_encryption: true,
            proof_interval: 3600, // 1 hour
        }
    }
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            block_time: 10, // 10 seconds
            max_block_size: 1024 * 1024, // 1 MB
            validators_per_round: 21,
            hashtimer_precision: 100, // 0.1 microseconds
            time_sync_interval: 60, // 1 minute
            round_timeout: 30, // 30 seconds
        }
    }
}

impl Default for DhtConfig {
    fn default() -> Self {
        Self {
            bucket_size: 20,
            replication_factor: 3,
            lookup_timeout: 30,
            enable_caching: true,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8080".to_string(),
            enable_cors: true,
            rate_limit: 1000,
            enable_auth: false,
        }
    }
}

impl Default for StakingConfig {
    fn default() -> Self {
        Self {
            min_stake_amount: crate::MIN_STAKE_AMOUNT,
            max_stake_amount: crate::MAX_STAKE_AMOUNT,
            stake_lock_period: 1000, // 1000 blocks
            slashing_conditions: SlashingConditions::default(),
        }
    }
}

impl Default for SlashingConditions {
    fn default() -> Self {
        Self {
            downtime_threshold: 100, // 100 blocks
            downtime_slash_percent: 5, // 5%
            malicious_slash_percent: 50, // 50%
            fake_proof_slash_percent: 25, // 25%
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_path: None,
            enable_console: true,
        }
    }
}

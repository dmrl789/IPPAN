use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::error::IppanError;
use crate::Result;
use std::path::Path;

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
            
            let mut config: Config = serde_json::from_str(&content)
                .map_err(|e| IppanError::Config(format!("Failed to parse config file: {}", e)))?;
            
            // Apply environment variable overrides
            config.apply_environment_overrides()?;
            
            // Validate configuration
            config.validate()?;
            
            Ok(config)
        } else {
            let mut config = Self::default();
            config.apply_environment_overrides()?;
            config.validate()?;
            config.save()?;
            Ok(config)
        }
    }
    
    /// Load configuration from a specific path
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| IppanError::Config(format!("Failed to read config file: {}", e)))?;
        
        let mut config: Config = serde_json::from_str(&content)
            .map_err(|e| IppanError::Config(format!("Failed to parse config file: {}", e)))?;
        
        config.apply_environment_overrides()?;
        config.validate()?;
        Ok(config)
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
    
    /// Save configuration to a specific path
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| IppanError::Config(format!("Failed to serialize config: {}", e)))?;
        
        std::fs::write(path, content)
            .map_err(|e| IppanError::Config(format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }
    
    /// Apply environment variable overrides
    fn apply_environment_overrides(&mut self) -> Result<()> {
        // Network overrides
        if let Ok(addr) = std::env::var("IPPAN_NETWORK_LISTEN_ADDR") {
            self.network.listen_addr = addr;
        }
        if let Ok(max_conn) = std::env::var("IPPAN_NETWORK_MAX_CONNECTIONS") {
            self.network.max_connections = max_conn.parse()
                .map_err(|e| IppanError::Config(format!("Invalid max_connections: {}", e)))?;
        }
        
        // Storage overrides
        if let Ok(db_path) = std::env::var("IPPAN_STORAGE_DB_PATH") {
            self.storage.db_path = PathBuf::from(db_path);
        }
        if let Ok(max_size) = std::env::var("IPPAN_STORAGE_MAX_SIZE") {
            self.storage.max_storage_size = max_size.parse()
                .map_err(|e| IppanError::Config(format!("Invalid max_storage_size: {}", e)))?;
        }
        
        // Consensus overrides
        if let Ok(block_time) = std::env::var("IPPAN_CONSENSUS_BLOCK_TIME") {
            self.consensus.block_time = block_time.parse()
                .map_err(|e| IppanError::Config(format!("Invalid block_time: {}", e)))?;
        }
        if let Ok(validators) = std::env::var("IPPAN_CONSENSUS_VALIDATORS_PER_ROUND") {
            self.consensus.validators_per_round = validators.parse()
                .map_err(|e| IppanError::Config(format!("Invalid validators_per_round: {}", e)))?;
        }
        
        // API overrides
        if let Ok(api_addr) = std::env::var("IPPAN_API_LISTEN_ADDR") {
            self.api.listen_addr = api_addr;
        }
        
        // Logging overrides
        if let Ok(log_level) = std::env::var("IPPAN_LOG_LEVEL") {
            self.logging.level = log_level;
        }
        if let Ok(log_file) = std::env::var("IPPAN_LOG_FILE") {
            self.logging.file_path = Some(PathBuf::from(log_file));
        }
        
        Ok(())
    }
    
    /// Validate configuration
    fn validate(&self) -> Result<()> {
        // Validate network configuration
        if self.network.max_connections == 0 {
            return Err(IppanError::Config("max_connections must be greater than 0".to_string()));
        }
        if self.network.connection_timeout == 0 {
            return Err(IppanError::Config("connection_timeout must be greater than 0".to_string()));
        }
        
        // Validate storage configuration
        if self.storage.max_storage_size == 0 {
            return Err(IppanError::Config("max_storage_size must be greater than 0".to_string()));
        }
        if self.storage.shard_size == 0 {
            return Err(IppanError::Config("shard_size must be greater than 0".to_string()));
        }
        if self.storage.replication_factor == 0 {
            return Err(IppanError::Config("replication_factor must be greater than 0".to_string()));
        }
        
        // Validate consensus configuration
        if self.consensus.block_time == 0 {
            return Err(IppanError::Config("block_time must be greater than 0".to_string()));
        }
        if self.consensus.max_block_size == 0 {
            return Err(IppanError::Config("max_block_size must be greater than 0".to_string()));
        }
        if self.consensus.validators_per_round == 0 {
            return Err(IppanError::Config("validators_per_round must be greater than 0".to_string()));
        }
        if self.consensus.round_timeout == 0 {
            return Err(IppanError::Config("round_timeout must be greater than 0".to_string()));
        }
        
        // Validate staking configuration
        if self.staking.min_stake_amount == 0 {
            return Err(IppanError::Config("min_stake_amount must be greater than 0".to_string()));
        }
        if self.staking.max_stake_amount <= self.staking.min_stake_amount {
            return Err(IppanError::Config("max_stake_amount must be greater than min_stake_amount".to_string()));
        }
        
        // Validate API configuration
        if self.api.rate_limit == 0 {
            return Err(IppanError::Config("rate_limit must be greater than 0".to_string()));
        }
        
        Ok(())
    }
    
    /// Merge with another configuration (override values)
    pub fn merge(&mut self, other: &Config) {
        // Merge network config
        if !other.network.listen_addr.is_empty() {
            self.network.listen_addr = other.network.listen_addr.clone();
        }
        if other.network.max_connections > 0 {
            self.network.max_connections = other.network.max_connections;
        }
        if other.network.connection_timeout > 0 {
            self.network.connection_timeout = other.network.connection_timeout;
        }
        self.network.enable_nat = other.network.enable_nat;
        self.network.enable_relay = other.network.enable_relay;
        
        // Merge storage config
        if other.storage.max_storage_size > 0 {
            self.storage.max_storage_size = other.storage.max_storage_size;
        }
        if other.storage.shard_size > 0 {
            self.storage.shard_size = other.storage.shard_size;
        }
        if other.storage.replication_factor > 0 {
            self.storage.replication_factor = other.storage.replication_factor;
        }
        self.storage.enable_encryption = other.storage.enable_encryption;
        
        // Merge consensus config
        if other.consensus.block_time > 0 {
            self.consensus.block_time = other.consensus.block_time;
        }
        if other.consensus.max_block_size > 0 {
            self.consensus.max_block_size = other.consensus.max_block_size;
        }
        if other.consensus.validators_per_round > 0 {
            self.consensus.validators_per_round = other.consensus.validators_per_round;
        }
        if other.consensus.round_timeout > 0 {
            self.consensus.round_timeout = other.consensus.round_timeout;
        }
        
        // Merge API config
        if !other.api.listen_addr.is_empty() {
            self.api.listen_addr = other.api.listen_addr.clone();
        }
        if other.api.rate_limit > 0 {
            self.api.rate_limit = other.api.rate_limit;
        }
        self.api.enable_cors = other.api.enable_cors;
        self.api.enable_auth = other.api.enable_auth;
        
        // Merge logging config
        if !other.logging.level.is_empty() {
            self.logging.level = other.logging.level.clone();
        }
        if other.logging.file_path.is_some() {
            self.logging.file_path = other.logging.file_path.clone();
        }
        self.logging.enable_console = other.logging.enable_console;
    }
    
    /// Get configuration file path
    fn get_config_path() -> Result<PathBuf> {
        // Check if IPPAN_CONFIG_PATH environment variable is set
        if let Ok(env_path) = std::env::var("IPPAN_CONFIG_PATH") {
            return Ok(PathBuf::from(env_path));
        }
        
        let mut path = dirs::config_dir()
            .ok_or_else(|| IppanError::Config("Could not determine config directory".to_string()))?;
        path.push("ippan");
        path.push("config.json");
        Ok(path)
    }
    
    /// Get configuration as JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| IppanError::Config(format!("Failed to serialize config: {}", e)))
    }
    
    /// Create configuration from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        let mut config: Config = serde_json::from_str(json)
            .map_err(|e| IppanError::Config(format!("Failed to parse config JSON: {}", e)))?;
        
        config.apply_environment_overrides()?;
        config.validate()?;
        Ok(config)
    }
    
    /// Check if configuration has changed (for hot-reloading)
    pub fn has_changed(&self, other: &Config) -> bool {
        self.network.listen_addr != other.network.listen_addr ||
        self.network.max_connections != other.network.max_connections ||
        self.consensus.block_time != other.consensus.block_time ||
        self.api.listen_addr != other.api.listen_addr ||
        self.logging.level != other.logging.level
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
            max_block_size: 24 * 1024, // 24 KB soft target (clamped to 32 KB hard limit)
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

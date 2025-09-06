//! Configuration utilities for IPPAN
//! 
//! This module provides configuration loading and management functionality.

use crate::{Config, Result};
use std::path::Path;
use std::collections::HashMap;

/// Load configuration from file
pub fn load_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Err(crate::error::IppanError::Config(
            format!("Config file does not exist: {}", path.display())
        ));
    }
    
    let content = std::fs::read_to_string(path)
        .map_err(|e| crate::error::IppanError::Config(
            format!("Failed to read config file: {}", e)
        ))?;
    
    let mut config: Config = serde_json::from_str(&content)
        .map_err(|e| crate::error::IppanError::Config(
            format!("Failed to parse config file: {}", e)
        ))?;
    
    // Apply environment variable overrides
    apply_environment_overrides(&mut config)?;
    
    // Validate configuration
    validate_config(&config)?;
    
    Ok(config)
}

/// Save configuration to file
pub fn save_config(config: &Config, path: &Path) -> Result<()> {
    // Create directory if it doesn't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| crate::error::IppanError::Config(
                format!("Failed to create config directory: {}", e)
            ))?;
    }
    
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| crate::error::IppanError::Config(
            format!("Failed to serialize config: {}", e)
        ))?;
    
    std::fs::write(path, content)
        .map_err(|e| crate::error::IppanError::Config(
            format!("Failed to write config file: {}", e)
        ))?;
    
    Ok(())
}

/// Merge two configurations
pub fn merge_configs(base: &Config, override_config: &Config) -> Config {
    let mut merged = base.clone();
    merged.merge(override_config);
    merged
}

/// Apply environment variable overrides to configuration
pub fn apply_environment_overrides(config: &mut Config) -> Result<()> {
    // Network overrides
    if let Ok(addr) = std::env::var("IPPAN_NETWORK_LISTEN_ADDR") {
        config.network.listen_addr = addr;
    }
    if let Ok(bootstrap) = std::env::var("IPPAN_NETWORK_BOOTSTRAP_NODES") {
        config.network.bootstrap_nodes = bootstrap.split(',').map(|s| s.trim().to_string()).collect();
    }
    if let Ok(max_conn) = std::env::var("IPPAN_NETWORK_MAX_CONNECTIONS") {
        config.network.max_connections = max_conn.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid max_connections: {}", e)))?;
    }
    if let Ok(timeout) = std::env::var("IPPAN_NETWORK_CONNECTION_TIMEOUT") {
        config.network.connection_timeout = timeout.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid connection_timeout: {}", e)))?;
    }
    if let Ok(enable_nat) = std::env::var("IPPAN_NETWORK_ENABLE_NAT") {
        config.network.enable_nat = enable_nat.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid enable_nat: {}", e)))?;
    }
    if let Ok(enable_relay) = std::env::var("IPPAN_NETWORK_ENABLE_RELAY") {
        config.network.enable_relay = enable_relay.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid enable_relay: {}", e)))?;
    }
    
    // Storage overrides
    if let Ok(db_path) = std::env::var("IPPAN_STORAGE_DB_PATH") {
        config.storage.db_path = std::path::PathBuf::from(db_path);
    }
    if let Ok(max_size) = std::env::var("IPPAN_STORAGE_MAX_SIZE") {
        config.storage.max_storage_size = max_size.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid max_storage_size: {}", e)))?;
    }
    if let Ok(shard_size) = std::env::var("IPPAN_STORAGE_SHARD_SIZE") {
        config.storage.shard_size = shard_size.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid shard_size: {}", e)))?;
    }
    if let Ok(replication) = std::env::var("IPPAN_STORAGE_REPLICATION_FACTOR") {
        config.storage.replication_factor = replication.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid replication_factor: {}", e)))?;
    }
    if let Ok(enable_encryption) = std::env::var("IPPAN_STORAGE_ENABLE_ENCRYPTION") {
        config.storage.enable_encryption = enable_encryption.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid enable_encryption: {}", e)))?;
    }
    
    // Consensus overrides
    if let Ok(block_time) = std::env::var("IPPAN_CONSENSUS_BLOCK_TIME") {
        config.consensus.block_time = block_time.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid block_time: {}", e)))?;
    }
    if let Ok(block_soft_target_kb) = std::env::var("IPPAN_BLOCK_SOFT_TARGET_KB") {
        if let Ok(kb) = block_soft_target_kb.parse::<usize>() {
            let bytes = kb * 1024;
            // Clamp to [4 KB, 32 KB] range
            config.consensus.max_block_size = bytes.clamp(4 * 1024, 32 * 1024);
        }
    }
    if let Ok(validators) = std::env::var("IPPAN_CONSENSUS_VALIDATORS_PER_ROUND") {
        config.consensus.validators_per_round = validators.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid validators_per_round: {}", e)))?;
    }
    if let Ok(precision) = std::env::var("IPPAN_CONSENSUS_HASHTIMER_PRECISION") {
        config.consensus.hashtimer_precision = precision.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid hashtimer_precision: {}", e)))?;
    }
    if let Ok(timeout) = std::env::var("IPPAN_CONSENSUS_ROUND_TIMEOUT") {
        config.consensus.round_timeout = timeout.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid round_timeout: {}", e)))?;
    }
    
    // DHT overrides
    if let Ok(bucket_size) = std::env::var("IPPAN_DHT_BUCKET_SIZE") {
        config.dht.bucket_size = bucket_size.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid bucket_size: {}", e)))?;
    }
    if let Ok(replication) = std::env::var("IPPAN_DHT_REPLICATION_FACTOR") {
        config.dht.replication_factor = replication.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid dht_replication_factor: {}", e)))?;
    }
    if let Ok(timeout) = std::env::var("IPPAN_DHT_LOOKUP_TIMEOUT") {
        config.dht.lookup_timeout = timeout.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid lookup_timeout: {}", e)))?;
    }
    if let Ok(enable_caching) = std::env::var("IPPAN_DHT_ENABLE_CACHING") {
        config.dht.enable_caching = enable_caching.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid enable_caching: {}", e)))?;
    }
    
    // API overrides
    if let Ok(api_addr) = std::env::var("IPPAN_API_LISTEN_ADDR") {
        config.api.listen_addr = api_addr;
    }
    if let Ok(enable_cors) = std::env::var("IPPAN_API_ENABLE_CORS") {
        config.api.enable_cors = enable_cors.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid enable_cors: {}", e)))?;
    }
    if let Ok(rate_limit) = std::env::var("IPPAN_API_RATE_LIMIT") {
        config.api.rate_limit = rate_limit.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid rate_limit: {}", e)))?;
    }
    if let Ok(enable_auth) = std::env::var("IPPAN_API_ENABLE_AUTH") {
        config.api.enable_auth = enable_auth.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid enable_auth: {}", e)))?;
    }
    
    // Staking overrides
    if let Ok(min_stake) = std::env::var("IPPAN_STAKING_MIN_STAKE_AMOUNT") {
        config.staking.min_stake_amount = min_stake.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid min_stake_amount: {}", e)))?;
    }
    if let Ok(max_stake) = std::env::var("IPPAN_STAKING_MAX_STAKE_AMOUNT") {
        config.staking.max_stake_amount = max_stake.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid max_stake_amount: {}", e)))?;
    }
    if let Ok(lock_period) = std::env::var("IPPAN_STAKING_LOCK_PERIOD") {
        config.staking.stake_lock_period = lock_period.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid stake_lock_period: {}", e)))?;
    }
    
    // Logging overrides
    if let Ok(log_level) = std::env::var("IPPAN_LOG_LEVEL") {
        config.logging.level = log_level;
    }
    if let Ok(log_file) = std::env::var("IPPAN_LOG_FILE") {
        config.logging.file_path = Some(std::path::PathBuf::from(log_file));
    }
    if let Ok(enable_console) = std::env::var("IPPAN_LOG_ENABLE_CONSOLE") {
        config.logging.enable_console = enable_console.parse()
            .map_err(|e| crate::error::IppanError::Config(format!("Invalid enable_console: {}", e)))?;
    }
    
    Ok(())
}

/// Validate configuration values
pub fn validate_config(config: &Config) -> Result<()> {
    // Validate network configuration
    if config.network.max_connections == 0 {
        return Err(crate::error::IppanError::Config("max_connections must be greater than 0".to_string()));
    }
    if config.network.connection_timeout == 0 {
        return Err(crate::error::IppanError::Config("connection_timeout must be greater than 0".to_string()));
    }
    
    // Validate storage configuration
    if config.storage.max_storage_size == 0 {
        return Err(crate::error::IppanError::Config("max_storage_size must be greater than 0".to_string()));
    }
    if config.storage.shard_size == 0 {
        return Err(crate::error::IppanError::Config("shard_size must be greater than 0".to_string()));
    }
    if config.storage.replication_factor == 0 {
        return Err(crate::error::IppanError::Config("replication_factor must be greater than 0".to_string()));
    }
    
    // Validate consensus configuration
    if config.consensus.block_time == 0 {
        return Err(crate::error::IppanError::Config("block_time must be greater than 0".to_string()));
    }
    if config.consensus.max_block_size == 0 {
        return Err(crate::error::IppanError::Config("max_block_size must be greater than 0".to_string()));
    }
    if config.consensus.validators_per_round == 0 {
        return Err(crate::error::IppanError::Config("validators_per_round must be greater than 0".to_string()));
    }
    if config.consensus.round_timeout == 0 {
        return Err(crate::error::IppanError::Config("round_timeout must be greater than 0".to_string()));
    }
    
    // Validate DHT configuration
    if config.dht.bucket_size == 0 {
        return Err(crate::error::IppanError::Config("bucket_size must be greater than 0".to_string()));
    }
    if config.dht.replication_factor == 0 {
        return Err(crate::error::IppanError::Config("dht_replication_factor must be greater than 0".to_string()));
    }
    if config.dht.lookup_timeout == 0 {
        return Err(crate::error::IppanError::Config("lookup_timeout must be greater than 0".to_string()));
    }
    
    // Validate staking configuration
    if config.staking.min_stake_amount == 0 {
        return Err(crate::error::IppanError::Config("min_stake_amount must be greater than 0".to_string()));
    }
    if config.staking.max_stake_amount <= config.staking.min_stake_amount {
        return Err(crate::error::IppanError::Config("max_stake_amount must be greater than min_stake_amount".to_string()));
    }
    
    // Validate API configuration
    if config.api.rate_limit == 0 {
        return Err(crate::error::IppanError::Config("rate_limit must be greater than 0".to_string()));
    }
    
    Ok(())
}

/// Get all environment variables that can override configuration
pub fn get_config_environment_vars() -> HashMap<String, String> {
    let mut vars = HashMap::new();
    
    // Network variables
    if let Ok(val) = std::env::var("IPPAN_NETWORK_LISTEN_ADDR") { vars.insert("IPPAN_NETWORK_LISTEN_ADDR".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_NETWORK_BOOTSTRAP_NODES") { vars.insert("IPPAN_NETWORK_BOOTSTRAP_NODES".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_NETWORK_MAX_CONNECTIONS") { vars.insert("IPPAN_NETWORK_MAX_CONNECTIONS".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_NETWORK_CONNECTION_TIMEOUT") { vars.insert("IPPAN_NETWORK_CONNECTION_TIMEOUT".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_NETWORK_ENABLE_NAT") { vars.insert("IPPAN_NETWORK_ENABLE_NAT".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_NETWORK_ENABLE_RELAY") { vars.insert("IPPAN_NETWORK_ENABLE_RELAY".to_string(), val); }
    
    // Storage variables
    if let Ok(val) = std::env::var("IPPAN_STORAGE_DB_PATH") { vars.insert("IPPAN_STORAGE_DB_PATH".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_STORAGE_MAX_SIZE") { vars.insert("IPPAN_STORAGE_MAX_SIZE".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_STORAGE_SHARD_SIZE") { vars.insert("IPPAN_STORAGE_SHARD_SIZE".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_STORAGE_REPLICATION_FACTOR") { vars.insert("IPPAN_STORAGE_REPLICATION_FACTOR".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_STORAGE_ENABLE_ENCRYPTION") { vars.insert("IPPAN_STORAGE_ENABLE_ENCRYPTION".to_string(), val); }
    
    // Consensus variables
    if let Ok(val) = std::env::var("IPPAN_CONSENSUS_BLOCK_TIME") { vars.insert("IPPAN_CONSENSUS_BLOCK_TIME".to_string(), val); }
    // Deprecated: IPPAN_CONSENSUS_MAX_BLOCK_SIZE replaced with IPPAN_BLOCK_SOFT_TARGET_KB
    if let Ok(val) = std::env::var("IPPAN_CONSENSUS_VALIDATORS_PER_ROUND") { vars.insert("IPPAN_CONSENSUS_VALIDATORS_PER_ROUND".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_CONSENSUS_HASHTIMER_PRECISION") { vars.insert("IPPAN_CONSENSUS_HASHTIMER_PRECISION".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_CONSENSUS_ROUND_TIMEOUT") { vars.insert("IPPAN_CONSENSUS_ROUND_TIMEOUT".to_string(), val); }
    
    // DHT variables
    if let Ok(val) = std::env::var("IPPAN_DHT_BUCKET_SIZE") { vars.insert("IPPAN_DHT_BUCKET_SIZE".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_DHT_REPLICATION_FACTOR") { vars.insert("IPPAN_DHT_REPLICATION_FACTOR".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_DHT_LOOKUP_TIMEOUT") { vars.insert("IPPAN_DHT_LOOKUP_TIMEOUT".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_DHT_ENABLE_CACHING") { vars.insert("IPPAN_DHT_ENABLE_CACHING".to_string(), val); }
    
    // API variables
    if let Ok(val) = std::env::var("IPPAN_API_LISTEN_ADDR") { vars.insert("IPPAN_API_LISTEN_ADDR".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_API_ENABLE_CORS") { vars.insert("IPPAN_API_ENABLE_CORS".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_API_RATE_LIMIT") { vars.insert("IPPAN_API_RATE_LIMIT".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_API_ENABLE_AUTH") { vars.insert("IPPAN_API_ENABLE_AUTH".to_string(), val); }
    
    // Staking variables
    if let Ok(val) = std::env::var("IPPAN_STAKING_MIN_STAKE_AMOUNT") { vars.insert("IPPAN_STAKING_MIN_STAKE_AMOUNT".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_STAKING_MAX_STAKE_AMOUNT") { vars.insert("IPPAN_STAKING_MAX_STAKE_AMOUNT".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_STAKING_LOCK_PERIOD") { vars.insert("IPPAN_STAKING_LOCK_PERIOD".to_string(), val); }
    
    // Logging variables
    if let Ok(val) = std::env::var("IPPAN_LOG_LEVEL") { vars.insert("IPPAN_LOG_LEVEL".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_LOG_FILE") { vars.insert("IPPAN_LOG_FILE".to_string(), val); }
    if let Ok(val) = std::env::var("IPPAN_LOG_ENABLE_CONSOLE") { vars.insert("IPPAN_LOG_ENABLE_CONSOLE".to_string(), val); }
    
    vars
}

/// Create a default configuration file if it doesn't exist
pub fn create_default_config_if_missing(path: &Path) -> Result<()> {
    if !path.exists() {
        let config = Config::default();
        save_config(&config, path)?;
        log::info!("Created default configuration file at: {}", path.display());
    }
    Ok(())
}

/// Hot-reload configuration from file
pub fn hot_reload_config(path: &Path, current_config: &Config) -> Result<Option<Config>> {
    if !path.exists() {
        return Ok(None);
    }
    
    let new_config = load_config(path)?;
    
    if current_config.has_changed(&new_config) {
        log::info!("Configuration changed, reloading...");
        Ok(Some(new_config))
    } else {
        Ok(None)
    }
}

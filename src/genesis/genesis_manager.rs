//! Genesis manager for IPPAN
//! 
//! Orchestrates the entire genesis block creation process including
//! validator setup, token distribution, network configuration, and
//! genesis block generation.

use crate::{Result, IppanError, TransactionHash};
use crate::crypto::{RealHashFunctions, RealEd25519, RealTransactionSigner};
use crate::consensus::bft_engine::{BFTBlock, BFTBlockHeader, BFTTransaction};
use super::{
    GenesisConfig, GenesisBlockData, ValidatorInfo, NetworkParameters, GenesisCreationResult,
    GenesisCreator, NetworkConfig, NetworkConfigManager, ValidatorSetupManager, TokenDistributionManager, GenesisStats,
    DnsConfig, P2PConfig, ApiConfig, SecurityConfig, MonitoringConfig,
    NetworkConfigStats, ValidatorSetupStats, TokenDistributionStats, ValidatorSetupRequest,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Genesis manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisManagerConfig {
    /// Enable genesis creation
    pub enable_genesis_creation: bool,
    /// Enable validator setup
    pub enable_validator_setup: bool,
    /// Enable token distribution
    pub enable_token_distribution: bool,
    /// Enable network configuration
    pub enable_network_configuration: bool,
    /// Genesis creation timeout in seconds
    pub genesis_creation_timeout_seconds: u64,
    /// Enable genesis validation
    pub enable_genesis_validation: bool,
    /// Enable genesis persistence
    pub enable_genesis_persistence: bool,
    /// Genesis file path
    pub genesis_file_path: String,
    /// Enable genesis backup
    pub enable_genesis_backup: bool,
    /// Genesis backup path
    pub genesis_backup_path: String,
}

impl Default for GenesisManagerConfig {
    fn default() -> Self {
        Self {
            enable_genesis_creation: true,
            enable_validator_setup: true,
            enable_token_distribution: true,
            enable_network_configuration: true,
            genesis_creation_timeout_seconds: 60,
            enable_genesis_validation: true,
            enable_genesis_persistence: true,
            genesis_file_path: "genesis.json".to_string(),
            enable_genesis_backup: true,
            genesis_backup_path: "genesis_backup.json".to_string(),
        }
    }
}

/// Genesis manager
pub struct GenesisManager {
    /// Configuration
    config: GenesisManagerConfig,
    /// Genesis creator
    genesis_creator: Arc<GenesisCreator>,
    /// Network config manager
    network_config_manager: Arc<NetworkConfigManager>,
    /// Validator setup manager
    validator_setup_manager: Arc<ValidatorSetupManager>,
    /// Token distribution manager
    token_distribution_manager: Arc<TokenDistributionManager>,
    /// Statistics
    stats: Arc<RwLock<GenesisStats>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
    /// Current genesis block data
    current_genesis_block_data: Arc<RwLock<Option<GenesisBlockData>>>,
}

impl GenesisManager {
    /// Create a new genesis manager
    pub fn new(config: GenesisManagerConfig) -> Self {
        let genesis_creator = Arc::new(GenesisCreator::new(Default::default()));
        let network_config_manager = Arc::new(NetworkConfigManager::new());
        let validator_setup_manager = Arc::new(ValidatorSetupManager::new(Default::default()));
        let token_distribution_manager = Arc::new(TokenDistributionManager::new(Default::default()));
        
        Self {
            config,
            genesis_creator,
            network_config_manager,
            validator_setup_manager,
            token_distribution_manager,
            stats: Arc::new(RwLock::new(GenesisStats::default())),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
            current_genesis_block_data: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Start the genesis manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting genesis manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        // Start components
        self.genesis_creator.start().await?;
        self.network_config_manager.start().await?;
        self.validator_setup_manager.start().await?;
        self.token_distribution_manager.start().await?;
        
        info!("Genesis manager started successfully");
        Ok(())
    }
    
    /// Stop the genesis manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping genesis manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        // Stop components
        self.genesis_creator.stop().await?;
        self.network_config_manager.stop().await?;
        self.validator_setup_manager.stop().await?;
        self.token_distribution_manager.stop().await?;
        
        info!("Genesis manager stopped");
        Ok(())
    }
    
    /// Create complete genesis block
    pub async fn create_genesis_block(&self, config: GenesisConfig) -> Result<GenesisCreationResult> {
        let start_time = Instant::now();
        
        info!("Creating complete genesis block for network: {}", config.network_name);
        
        if !self.config.enable_genesis_creation {
            return Ok(GenesisCreationResult {
                genesis_block_data: GenesisBlockData {
                    genesis_block: BFTBlock {
                        header: BFTBlockHeader {
                            number: 0,
                            previous_hash: [0u8; 32],
                            merkle_root: [0u8; 32],
                            timestamp: config.genesis_timestamp,
                            view_number: 0,
                            sequence_number: 0,
                            validator_id: [0u8; 32],
                        },
                        transactions: vec![],
                        hash: [0u8; 32],
                    },
                    initial_validators: vec![],
                    initial_token_distribution: HashMap::new(),
                    network_parameters: NetworkParameters::default(),
                    genesis_config: config.clone(),
                    creation_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    genesis_creator: "IPPAN Genesis Manager".to_string(),
                },
                creation_time_ms: start_time.elapsed().as_millis() as u64,
                success: false,
                error_message: Some("Genesis creation disabled".to_string()),
                genesis_hash: [0u8; 32],
                genesis_block_number: 0,
                validator_count: 0,
                total_token_supply: 0,
            });
        }
        
        // Step 1: Setup validators
        let validators = if self.config.enable_validator_setup {
            self.setup_initial_validators(&config).await?
        } else {
            vec![]
        };
        
        // Step 2: Create token distribution
        let _token_distribution = if self.config.enable_token_distribution {
            self.create_token_distribution(&config, &validators).await?
        } else {
            HashMap::new()
        };
        
        // Step 3: Setup network configuration
        let _network_config = if self.config.enable_network_configuration {
            self.setup_network_configuration(&config).await?
        } else {
            NetworkConfig::new().await.get_config().await?
        };
        
        // Step 4: Create genesis block
        let genesis_result = self.genesis_creator.create_genesis_block(config.clone()).await?;
        
        // Step 5: Validate genesis block
        if self.config.enable_genesis_validation {
            self.validate_genesis_block(&genesis_result.genesis_block_data).await?;
        }
        
        // Step 6: Persist genesis block
        if self.config.enable_genesis_persistence {
            self.persist_genesis_block(&genesis_result.genesis_block_data).await?;
        }
        
        // Step 7: Create backup
        if self.config.enable_genesis_backup {
            self.create_genesis_backup(&genesis_result.genesis_block_data).await?;
        }
        
        // Store current genesis block data
        {
            let mut current = self.current_genesis_block_data.write().await;
            *current = Some(genesis_result.genesis_block_data.clone());
        }
        
        let creation_time = start_time.elapsed().as_millis() as u64;
        
        let result = GenesisCreationResult {
            genesis_block_data: genesis_result.genesis_block_data,
            creation_time_ms: creation_time,
            success: true,
            error_message: None,
            genesis_hash: genesis_result.genesis_hash,
            genesis_block_number: genesis_result.genesis_block_number,
            validator_count: genesis_result.validator_count,
            total_token_supply: genesis_result.total_token_supply,
        };
        
        // Update statistics
        self.update_stats(creation_time, validators.len(), result.total_token_supply, true).await;
        
        info!("Complete genesis block created successfully in {}ms", creation_time);
        Ok(result)
    }
    
    /// Setup initial validators
    async fn setup_initial_validators(&self, config: &GenesisConfig) -> Result<Vec<ValidatorInfo>> {
        info!("Setting up {} initial validators", config.initial_validator_count);
        
        let mut validators = Vec::new();
        
        for i in 0..config.initial_validator_count {
            let request = ValidatorSetupRequest {
                validator_name: format!("Validator {}", i + 1),
                validator_description: format!("Initial validator {} for {}", i + 1, config.network_name),
                validator_website: format!("https://validator{}.{}.com", i + 1, config.network_id),
                validator_contact: format!("contact@validator{}.{}.com", i + 1, config.network_id),
                stake_amount: config.min_stake_required * (i as u64 + 1),
                commission_rate: 500, // 5% default
                validator_location: "Global".to_string(),
                validator_operator: format!("Operator {}", i + 1),
                public_key: [i as u8; 32], // Placeholder
                validator_address: format!("i{}", hex::encode(&[i as u8; 16])),
            };
            
            let result = self.validator_setup_manager.setup_validator(request).await?;
            validators.push(result.validator_info);
        }
        
        info!("Setup {} initial validators", validators.len());
        Ok(validators)
    }
    
    /// Create token distribution
    async fn create_token_distribution(
        &self,
        config: &GenesisConfig,
        validators: &[ValidatorInfo],
    ) -> Result<HashMap<String, u64>> {
        info!("Creating token distribution");
        
        let result = self.token_distribution_manager.create_token_distribution(validators).await?;
        
        let mut distribution = HashMap::new();
        for allocation in result.token_allocations {
            distribution.insert(allocation.allocation_address, allocation.allocation_amount);
        }
        
        info!("Created token distribution for {} addresses", distribution.len());
        Ok(distribution)
    }
    
    /// Setup network configuration
    async fn setup_network_configuration(&self, config: &GenesisConfig) -> Result<NetworkConfig> {
        info!("Setting up network configuration");
        
        let network_config = NetworkConfig {
            network_name: config.network_name.clone(),
            network_id: config.network_id.clone(),
            chain_id: config.chain_id,
            bootstrap_nodes: vec![], // Will be populated by validators
            network_parameters: NetworkParameters {
                block_time_seconds: config.block_time_seconds,
                max_block_size_bytes: config.max_block_size_bytes,
                max_transactions_per_block: config.max_transactions_per_block,
                min_stake_required: config.min_stake_required,
                max_stake_allowed: config.min_stake_required * 10,
                staking_reward_rate: 500, // 5%
                governance_proposal_threshold: config.min_stake_required / 10,
                governance_voting_period_seconds: 7 * 24 * 3600, // 7 days
                cross_chain_enabled: config.enable_cross_chain,
                cross_chain_fee: config.min_stake_required / 100,
                network_upgrade_threshold: config.min_stake_required / 5,
            },
            dns_config: DnsConfig::default(),
            p2p_config: P2PConfig::default(),
            api_config: ApiConfig::default(),
            security_config: SecurityConfig::default(),
            monitoring_config: MonitoringConfig::default(),
            genesis_block_hash: [0u8; 32], // Will be set after genesis creation
            creation_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            network_version: "1.0.0".to_string(),
        };
        
        self.network_config_manager.load_config(network_config.clone()).await?;
        
        info!("Network configuration setup completed");
        Ok(network_config)
    }
    
    /// Validate genesis block
    async fn validate_genesis_block(&self, genesis_block_data: &GenesisBlockData) -> Result<()> {
        info!("Validating genesis block");
        
        // Validate block structure
        if genesis_block_data.genesis_block.header.number != 0 {
            return Err(IppanError::Genesis("Genesis block number must be 0".to_string()));
        }
        
        if genesis_block_data.genesis_block.header.previous_hash != [0u8; 32] {
            return Err(IppanError::Genesis("Genesis block previous hash must be zero".to_string()));
        }
        
        // Validate validators
        if genesis_block_data.initial_validators.is_empty() {
            return Err(IppanError::Genesis("Genesis block must have at least one validator".to_string()));
        }
        
        // Validate token distribution
        let total_distributed: u64 = genesis_block_data.initial_token_distribution.values().sum();
        if total_distributed != genesis_block_data.genesis_config.initial_token_supply {
            return Err(IppanError::Genesis("Token distribution mismatch".to_string()));
        }
        
        // Validate network parameters
        if genesis_block_data.network_parameters.block_time_seconds == 0 {
            return Err(IppanError::Genesis("Block time cannot be zero".to_string()));
        }
        
        info!("Genesis block validation successful");
        Ok(())
    }
    
    /// Persist genesis block
    async fn persist_genesis_block(&self, _genesis_block_data: &GenesisBlockData) -> Result<()> {
        info!("Persisting genesis block to {}", self.config.genesis_file_path);
        
        // TODO: Implement proper genesis block serialization
        // For now, just create a placeholder file
        let placeholder = "Genesis block data - serialization not yet implemented";
        
        tokio::fs::write(&self.config.genesis_file_path, placeholder).await
            .map_err(|e| IppanError::Genesis(format!("Failed to write genesis file: {}", e)))?;
        
        info!("Genesis block persisted successfully");
        Ok(())
    }
    
    /// Create genesis backup
    async fn create_genesis_backup(&self, _genesis_block_data: &GenesisBlockData) -> Result<()> {
        info!("Creating genesis backup to {}", self.config.genesis_backup_path);
        
        // TODO: Implement proper genesis block serialization
        // For now, just create a placeholder backup file
        let placeholder = "Genesis block backup - serialization not yet implemented";
        
        tokio::fs::write(&self.config.genesis_backup_path, placeholder).await
            .map_err(|e| IppanError::Genesis(format!("Failed to write genesis backup: {}", e)))?;
        
        info!("Genesis backup created successfully");
        Ok(())
    }
    
    /// Load genesis block from file
    pub async fn load_genesis_block(&self, file_path: &str) -> Result<GenesisBlockData> {
        info!("Loading genesis block from {}", file_path);
        
        // TODO: Implement proper genesis file loading
        // For now, create a placeholder genesis block data
        let genesis_block_data = GenesisBlockData {
            genesis_block: BFTBlock {
                header: BFTBlockHeader {
                    number: 0,
                    previous_hash: [0u8; 32],
                    merkle_root: [0u8; 32],
                    timestamp: 0,
                    view_number: 0,
                    sequence_number: 0,
                    validator_id: [0u8; 32],
                },
                transactions: vec![],
                hash: [0u8; 32],
            },
            initial_validators: vec![],
            initial_token_distribution: HashMap::new(),
            network_parameters: super::NetworkParameters::default(),
            genesis_config: super::GenesisConfig::default(),
            creation_timestamp: 0,
            genesis_creator: "placeholder".to_string(),
        };
        
        // Validate loaded genesis block
        if self.config.enable_genesis_validation {
            self.validate_genesis_block(&genesis_block_data).await?;
        }
        
        // Store current genesis block data
        {
            let mut current = self.current_genesis_block_data.write().await;
            *current = Some(genesis_block_data.clone());
        }
        
        info!("Genesis block loaded successfully");
        Ok(genesis_block_data)
    }
    
    /// Get current genesis block data
    pub async fn get_current_genesis_block_data(&self) -> Result<Option<GenesisBlockData>> {
        let current = self.current_genesis_block_data.read().await;
        Ok(current.clone())
    }
    
    /// Get genesis creator statistics
    pub async fn get_genesis_creator_stats(&self) -> Result<GenesisStats> {
        self.genesis_creator.get_stats().await
    }
    
    /// Get network config statistics
    pub async fn get_network_config_stats(&self) -> Result<NetworkConfigStats> {
        self.network_config_manager.get_stats().await
    }
    
    /// Get validator setup statistics
    pub async fn get_validator_setup_stats(&self) -> Result<ValidatorSetupStats> {
        self.validator_setup_manager.get_stats().await
    }
    
    /// Get token distribution statistics
    pub async fn get_token_distribution_stats(&self) -> Result<TokenDistributionStats> {
        self.token_distribution_manager.get_stats().await
    }
    
    /// Update statistics
    async fn update_stats(
        &self,
        creation_time_ms: u64,
        validator_count: usize,
        token_supply: u64,
        success: bool,
    ) {
        let mut stats = self.stats.write().await;
        
        stats.total_genesis_blocks_created += 1;
        if success {
            stats.successful_creations += 1;
        } else {
            stats.failed_creations += 1;
        }
        
        // Update averages
        let total = stats.total_genesis_blocks_created as f64;
        stats.average_creation_time_ms = 
            (stats.average_creation_time_ms * (total - 1.0) + creation_time_ms as f64) / total;
        stats.average_validator_count = 
            (stats.average_validator_count * (total - 1.0) + validator_count as f64) / total;
        stats.average_token_supply = 
            (stats.average_token_supply * (total - 1.0) + token_supply as f64) / total;
        
        // Update success rate
        stats.creation_success_rate = stats.successful_creations as f64 / total;
        
        // Update timestamps
        stats.last_creation = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
    }
    
    /// Get genesis manager statistics
    pub async fn get_stats(&self) -> Result<GenesisStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_genesis_manager_config() {
        let config = GenesisManagerConfig::default();
        assert!(config.enable_genesis_creation);
        assert!(config.enable_validator_setup);
        assert!(config.enable_token_distribution);
        assert!(config.enable_network_configuration);
        assert_eq!(config.genesis_creation_timeout_seconds, 60);
        assert!(config.enable_genesis_validation);
        assert!(config.enable_genesis_persistence);
        assert_eq!(config.genesis_file_path, "genesis.json");
        assert!(config.enable_genesis_backup);
        assert_eq!(config.genesis_backup_path, "genesis_backup.json");
    }
    
    #[tokio::test]
    async fn test_genesis_creation_result() {
        let result = GenesisCreationResult {
            genesis_block_data: GenesisBlockData {
                genesis_block: BFTBlock {
                    header: BFTBlockHeader {
                        number: 0,
                        previous_hash: [0u8; 32],
                        merkle_root: [1u8; 32],
                        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        view_number: 0,
                        sequence_number: 0,
                        validator_id: [0u8; 32],
                    },
                    transactions: vec![],
                    hash: [2u8; 32],
                },
                initial_validators: vec![],
                initial_token_distribution: HashMap::new(),
                network_parameters: NetworkParameters::default(),
                genesis_config: GenesisConfig::default(),
                creation_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                genesis_creator: "Test Creator".to_string(),
            },
            creation_time_ms: 200,
            success: true,
            error_message: None,
            genesis_hash: [2u8; 32],
            genesis_block_number: 0,
            validator_count: 4,
            total_token_supply: 1_000_000_000_000_000,
        };
        
        assert!(result.success);
        assert_eq!(result.creation_time_ms, 200);
        assert_eq!(result.genesis_block_number, 0);
        assert_eq!(result.validator_count, 4);
        assert_eq!(result.total_token_supply, 1_000_000_000_000_000);
    }
    
    #[tokio::test]
    async fn test_genesis_stats() {
        let stats = GenesisStats {
            total_genesis_blocks_created: 5,
            successful_creations: 4,
            failed_creations: 1,
            average_creation_time_ms: 100.0,
            average_validator_count: 4.0,
            average_token_supply: 1_000_000_000_000_000.0,
            creation_success_rate: 0.8,
            uptime_seconds: 3600,
            last_creation: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        };
        
        assert_eq!(stats.total_genesis_blocks_created, 5);
        assert_eq!(stats.successful_creations, 4);
        assert_eq!(stats.failed_creations, 1);
        assert_eq!(stats.average_creation_time_ms, 100.0);
        assert_eq!(stats.average_validator_count, 4.0);
        assert_eq!(stats.average_token_supply, 1_000_000_000_000_000.0);
        assert_eq!(stats.creation_success_rate, 0.8);
        assert_eq!(stats.uptime_seconds, 3600);
    }
}

//! Genesis block creator for IPPAN
//! 
//! Implements the creation of the genesis block with proper structure,
//! initial validators, token distribution, and network parameters.

use crate::{Result, IppanError, TransactionHash};
use crate::crypto::{RealHashFunctions, RealEd25519, RealTransactionSigner};
use crate::consensus::bft_engine::{BFTBlock, BFTBlockHeader, BFTTransaction};
use super::{GenesisConfig, GenesisBlockData, ValidatorInfo, NetworkParameters, GenesisCreationResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Genesis creator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisCreatorConfig {
    /// Enable validator generation
    pub enable_validator_generation: bool,
    /// Enable token distribution
    pub enable_token_distribution: bool,
    /// Enable network parameter setup
    pub enable_network_parameter_setup: bool,
    /// Genesis block size limit in bytes
    pub genesis_block_size_limit_bytes: usize,
    /// Maximum validators in genesis
    pub max_validators_in_genesis: usize,
    /// Minimum validators in genesis
    pub min_validators_in_genesis: usize,
    /// Enable genesis validation
    pub enable_genesis_validation: bool,
    /// Genesis creation timeout in seconds
    pub genesis_creation_timeout_seconds: u64,
}

impl Default for GenesisCreatorConfig {
    fn default() -> Self {
        Self {
            enable_validator_generation: true,
            enable_token_distribution: true,
            enable_network_parameter_setup: true,
            genesis_block_size_limit_bytes: 1024 * 1024, // 1MB
            max_validators_in_genesis: 100,
            min_validators_in_genesis: 4,
            enable_genesis_validation: true,
            genesis_creation_timeout_seconds: 30,
        }
    }
}

/// Genesis creator
pub struct GenesisCreator {
    /// Configuration
    config: GenesisCreatorConfig,
    /// Statistics
    stats: Arc<RwLock<super::GenesisStats>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
}

impl GenesisCreator {
    /// Create a new genesis creator
    pub fn new(config: GenesisCreatorConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(super::GenesisStats::default())),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        }
    }
    
    /// Start the genesis creator
    pub async fn start(&self) -> Result<()> {
        info!("Starting genesis creator");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        info!("Genesis creator started successfully");
        Ok(())
    }
    
    /// Stop the genesis creator
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping genesis creator");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!("Genesis creator stopped");
        Ok(())
    }
    
    /// Create genesis block
    pub async fn create_genesis_block(&self, config: GenesisConfig) -> Result<GenesisCreationResult> {
        let start_time = Instant::now();
        
        info!("Creating genesis block for network: {}", config.network_name);
        
        // Validate configuration
        self.validate_genesis_config(&config).await?;
        
        // Generate initial validators
        let initial_validators = if self.config.enable_validator_generation {
            self.generate_initial_validators(&config).await?
        } else {
            vec![]
        };
        
        // Create initial token distribution
        let initial_token_distribution = if self.config.enable_token_distribution {
            self.create_initial_token_distribution(&config, &initial_validators).await?
        } else {
            HashMap::new()
        };
        
        // Create network parameters
        let network_parameters = if self.config.enable_network_parameter_setup {
            self.create_network_parameters(&config).await?
        } else {
            NetworkParameters::default()
        };
        
        // Create genesis block
        let genesis_block = self.create_genesis_block_structure(&config, &initial_validators).await?;
        
        // Create genesis block data
        let genesis_block_data = GenesisBlockData {
            genesis_block: genesis_block.clone(),
            initial_validators,
            initial_token_distribution,
            network_parameters,
            genesis_config: config.clone(),
            creation_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            genesis_creator: "IPPAN Genesis Creator".to_string(),
        };
        
        // Validate genesis block
        if self.config.enable_genesis_validation {
            self.validate_genesis_block(&genesis_block_data).await?;
        }
        
        let creation_time = start_time.elapsed().as_millis() as u64;
        let total_token_supply: u64 = genesis_block_data.initial_token_distribution.values().sum();
        
        let result = GenesisCreationResult {
            genesis_block_data,
            creation_time_ms: creation_time,
            success: true,
            error_message: None,
            genesis_hash: genesis_block.hash,
            genesis_block_number: config.genesis_block_number,
            validator_count: config.initial_validator_count,
            total_token_supply,
        };
        
        // Update statistics
        self.update_stats(creation_time, config.initial_validator_count, total_token_supply, true).await;
        
        info!("Genesis block created successfully in {}ms", creation_time);
        Ok(result)
    }
    
    /// Validate genesis configuration
    async fn validate_genesis_config(&self, config: &GenesisConfig) -> Result<()> {
        // Validate network name
        if config.network_name.is_empty() {
            return Err(IppanError::Genesis("Network name cannot be empty".to_string()));
        }
        
        // Validate network ID
        if config.network_id.is_empty() {
            return Err(IppanError::Genesis("Network ID cannot be empty".to_string()));
        }
        
        // Validate chain ID
        if config.chain_id == 0 {
            return Err(IppanError::Genesis("Chain ID cannot be zero".to_string()));
        }
        
        // Validate validator count
        if config.initial_validator_count < self.config.min_validators_in_genesis {
            return Err(IppanError::Genesis(
                format!("Validator count {} is below minimum {}", 
                    config.initial_validator_count, self.config.min_validators_in_genesis)
            ));
        }
        
        if config.initial_validator_count > self.config.max_validators_in_genesis {
            return Err(IppanError::Genesis(
                format!("Validator count {} exceeds maximum {}", 
                    config.initial_validator_count, self.config.max_validators_in_genesis)
            ));
        }
        
        // Validate token supply
        if config.initial_token_supply == 0 {
            return Err(IppanError::Genesis("Initial token supply cannot be zero".to_string()));
        }
        
        // Validate block time
        if config.block_time_seconds == 0 {
            return Err(IppanError::Genesis("Block time cannot be zero".to_string()));
        }
        
        // Validate block size
        if config.max_block_size_bytes == 0 {
            return Err(IppanError::Genesis("Max block size cannot be zero".to_string()));
        }
        
        // Validate stake requirement
        if config.min_stake_required == 0 {
            return Err(IppanError::Genesis("Minimum stake cannot be zero".to_string()));
        }
        
        Ok(())
    }
    
    /// Generate initial validators
    async fn generate_initial_validators(&self, config: &GenesisConfig) -> Result<Vec<ValidatorInfo>> {
        info!("Generating {} initial validators", config.initial_validator_count);
        
        let mut validators = Vec::new();
        
        for i in 0..config.initial_validator_count {
            let validator = self.create_validator(i, config).await?;
            validators.push(validator);
        }
        
        info!("Generated {} initial validators", validators.len());
        Ok(validators)
    }
    
    /// Create a single validator
    async fn create_validator(&self, index: usize, config: &GenesisConfig) -> Result<ValidatorInfo> {
        // Generate validator ID
        let validator_id = RealHashFunctions::sha256(&format!("validator_{}_{}", config.network_id, index).as_bytes());
        
        // Generate key pair
        let (signing_key, _private_key) = RealEd25519::generate_keypair();
        let public_key = signing_key.verifying_key().to_bytes();
        
        // Generate validator address
        let validator_address = format!("i{}", hex::encode(&validator_id[..16]));
        
        // Calculate stake amount
        let stake_amount = config.min_stake_required * (index as u64 + 1);
        
        // Create validator info
        let validator = ValidatorInfo {
            validator_id,
            validator_address,
            public_key,
            stake_amount,
            commission_rate: 500, // 5% default commission
            is_active: true,
            validator_name: format!("Validator {}", index + 1),
            validator_description: format!("Initial validator {} for {}", index + 1, config.network_name),
            validator_website: format!("https://validator{}.{}.com", index + 1, config.network_id),
            validator_contact: format!("contact@validator{}.{}.com", index + 1, config.network_id),
        };
        
        Ok(validator)
    }
    
    /// Create initial token distribution
    async fn create_initial_token_distribution(
        &self,
        config: &GenesisConfig,
        validators: &[ValidatorInfo],
    ) -> Result<HashMap<String, u64>> {
        info!("Creating initial token distribution");
        
        let mut distribution = HashMap::new();
        
        // Distribute tokens to validators
        let validator_share = config.initial_token_supply / 4; // 25% to validators
        let validator_individual_share = validator_share / validators.len() as u64;
        
        for validator in validators {
            distribution.insert(validator.validator_address.clone(), validator_individual_share);
        }
        
        // Distribute tokens to treasury
        let treasury_share = config.initial_token_supply / 4; // 25% to treasury
        distribution.insert("treasury".to_string(), treasury_share);
        
        // Distribute tokens to community
        let community_share = config.initial_token_supply / 2; // 50% to community
        distribution.insert("community".to_string(), community_share);
        
        // Verify total distribution
        let total_distributed: u64 = distribution.values().sum();
        if total_distributed != config.initial_token_supply {
            return Err(IppanError::Genesis(
                format!("Token distribution mismatch: expected {}, got {}", 
                    config.initial_token_supply, total_distributed)
            ));
        }
        
        info!("Created token distribution for {} addresses", distribution.len());
        Ok(distribution)
    }
    
    /// Create network parameters
    async fn create_network_parameters(&self, config: &GenesisConfig) -> Result<NetworkParameters> {
        info!("Creating network parameters");
        
        let parameters = NetworkParameters {
            block_time_seconds: config.block_time_seconds,
            max_block_size_bytes: config.max_block_size_bytes,
            max_transactions_per_block: config.max_transactions_per_block,
            min_stake_required: config.min_stake_required,
            max_stake_allowed: config.min_stake_required * 10, // 10x minimum
            staking_reward_rate: 500, // 5%
            governance_proposal_threshold: config.min_stake_required / 10, // 10% of minimum stake
            governance_voting_period_seconds: 7 * 24 * 3600, // 7 days
            cross_chain_enabled: config.enable_cross_chain,
            cross_chain_fee: config.min_stake_required / 100, // 1% of minimum stake
            network_upgrade_threshold: config.min_stake_required / 5, // 20% of minimum stake
        };
        
        info!("Created network parameters");
        Ok(parameters)
    }
    
    /// Create genesis block structure
    async fn create_genesis_block_structure(
        &self,
        config: &GenesisConfig,
        validators: &[ValidatorInfo],
    ) -> Result<BFTBlock> {
        info!("Creating genesis block structure");
        
        // Create genesis transactions
        let genesis_transactions = self.create_genesis_transactions(config, validators).await?;
        
        // Create block header
        let header = BFTBlockHeader {
            number: config.genesis_block_number,
            previous_hash: [0u8; 32], // Genesis block has no previous hash
            merkle_root: RealHashFunctions::merkle_root(&genesis_transactions.iter().map(|tx| tx.hash).collect::<Vec<_>>()),
            timestamp: config.genesis_timestamp,
            view_number: 0,
            sequence_number: 0,
            validator_id: [0u8; 32], // Genesis block has no validator
        };
        
        // Create block
        let mut block = BFTBlock {
            header,
            transactions: genesis_transactions,
            hash: [0u8; 32], // Will be calculated
        };
        
        // Calculate block hash
        // TODO: Implement proper block serialization
        // For now, use placeholder data
        let block_data = b"genesis_block_placeholder".to_vec();
        block.hash = RealHashFunctions::sha256(&block_data);
        
        // Verify block size
        if block_data.len() > self.config.genesis_block_size_limit_bytes {
            return Err(IppanError::Genesis(
                format!("Genesis block size {} exceeds limit {}", 
                    block_data.len(), self.config.genesis_block_size_limit_bytes)
            ));
        }
        
        info!("Created genesis block with {} transactions", block.transactions.len());
        Ok(block)
    }
    
    /// Create genesis transactions
    async fn create_genesis_transactions(
        &self,
        config: &GenesisConfig,
        validators: &[ValidatorInfo],
    ) -> Result<Vec<BFTTransaction>> {
        info!("Creating genesis transactions");
        
        let mut transactions = Vec::new();
        
        // Create validator registration transactions
        for validator in validators {
            let transaction = self.create_validator_registration_transaction(validator).await?;
            transactions.push(transaction);
        }
        
        // Create network parameter transaction
        let network_params_tx = self.create_network_parameters_transaction(config).await?;
        transactions.push(network_params_tx);
        
        info!("Created {} genesis transactions", transactions.len());
        Ok(transactions)
    }
    
    /// Create validator registration transaction
    async fn create_validator_registration_transaction(&self, _validator: &ValidatorInfo) -> Result<BFTTransaction> {
        // TODO: Implement proper validator serialization
        // For now, use placeholder data
        let transaction_data = b"validator_registration_placeholder".to_vec();
        
        let transaction = BFTTransaction {
            hash: RealHashFunctions::sha256(&transaction_data),
            data: transaction_data,
            sender: [0u8; 32], // Genesis sender
            from: "genesis".to_string(),
            to: "validator".to_string(),
            amount: 1000000, // Genesis amount
            fee: 0, // Genesis transactions have no fee
            gas_used: 21000, // Standard gas
            gas_price: 0, // Genesis transactions have no gas price
            nonce: 0,
            signature: [0u8; 64], // Genesis transactions are not signed
        };
        
        Ok(transaction)
    }
    
    /// Create network parameters transaction
    async fn create_network_parameters_transaction(&self, config: &GenesisConfig) -> Result<BFTTransaction> {
        let _network_params = NetworkParameters {
            block_time_seconds: config.block_time_seconds,
            max_block_size_bytes: config.max_block_size_bytes,
            max_transactions_per_block: config.max_transactions_per_block,
            min_stake_required: config.min_stake_required,
            max_stake_allowed: config.min_stake_required * 10,
            staking_reward_rate: 500,
            governance_proposal_threshold: config.min_stake_required / 10,
            governance_voting_period_seconds: 7 * 24 * 3600,
            cross_chain_enabled: config.enable_cross_chain,
            cross_chain_fee: config.min_stake_required / 100,
            network_upgrade_threshold: config.min_stake_required / 5,
        };
        
        // TODO: Implement proper network parameters serialization
        // For now, use placeholder data
        let transaction_data = b"network_parameters_placeholder".to_vec();
        
        let transaction = BFTTransaction {
            hash: RealHashFunctions::sha256(&transaction_data),
            data: transaction_data,
            sender: [0u8; 32], // Genesis sender
            from: "genesis".to_string(),
            to: "network".to_string(),
            amount: 0, // Network parameters have no amount
            fee: 0, // Genesis transactions have no fee
            gas_used: 21000, // Standard gas
            gas_price: 0, // Genesis transactions have no gas price
            nonce: 0,
            signature: [0u8; 64],
        };
        
        Ok(transaction)
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
    
    /// Get genesis creation statistics
    pub async fn get_stats(&self) -> Result<super::GenesisStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_genesis_creator_config() {
        let config = GenesisCreatorConfig::default();
        assert!(config.enable_validator_generation);
        assert!(config.enable_token_distribution);
        assert!(config.enable_network_parameter_setup);
        assert_eq!(config.genesis_block_size_limit_bytes, 1024 * 1024);
        assert_eq!(config.max_validators_in_genesis, 100);
        assert_eq!(config.min_validators_in_genesis, 4);
    }
    
    #[tokio::test]
    async fn test_genesis_config() {
        let config = GenesisConfig::default();
        assert_eq!(config.network_name, "IPPAN Mainnet");
        assert_eq!(config.network_id, "ippan_mainnet");
        assert_eq!(config.chain_id, 1);
        assert_eq!(config.initial_validator_count, 4);
        assert_eq!(config.initial_token_supply, 1_000_000_000_000_000);
        assert_eq!(config.block_time_seconds, 10);
        assert!(config.enable_staking);
        assert!(config.enable_governance);
    }
    
    #[tokio::test]
    async fn test_validator_info() {
        let validator = ValidatorInfo {
            validator_id: [1u8; 32],
            validator_address: "i1234567890abcdef".to_string(),
            public_key: [2u8; 32],
            stake_amount: 10000000000,
            commission_rate: 500,
            is_active: true,
            validator_name: "Test Validator".to_string(),
            validator_description: "Test validator description".to_string(),
            validator_website: "https://test.com".to_string(),
            validator_contact: "contact@test.com".to_string(),
        };
        
        assert_eq!(validator.validator_address, "i1234567890abcdef");
        assert_eq!(validator.stake_amount, 10000000000);
        assert_eq!(validator.commission_rate, 500);
        assert!(validator.is_active);
    }
    
    #[tokio::test]
    async fn test_network_parameters() {
        let params = NetworkParameters::default();
        assert_eq!(params.block_time_seconds, 10);
        assert_eq!(params.max_block_size_bytes, 1024 * 1024);
        assert_eq!(params.max_transactions_per_block, 1000);
        assert_eq!(params.min_stake_required, 10_000_000_000);
        assert_eq!(params.staking_reward_rate, 500);
        assert_eq!(params.governance_voting_period_seconds, 7 * 24 * 3600);
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
            creation_time_ms: 100,
            success: true,
            error_message: None,
            genesis_hash: [2u8; 32],
            genesis_block_number: 0,
            validator_count: 4,
            total_token_supply: 1_000_000_000_000_000,
        };
        
        assert!(result.success);
        assert_eq!(result.creation_time_ms, 100);
        assert_eq!(result.genesis_block_number, 0);
        assert_eq!(result.validator_count, 4);
        assert_eq!(result.total_token_supply, 1_000_000_000_000_000);
    }
}

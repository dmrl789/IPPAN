//! Genesis block and network configuration for IPPAN
//!
//! Handles the creation of the genesis block and initial network configuration
//! including validator setup, initial token distribution, and network parameters.

use crate::{Result, IppanError, TransactionHash};
use crate::crypto::{RealHashFunctions, RealEd25519, RealTransactionSigner};
use crate::consensus::bft_engine::{BFTBlock, BFTBlockHeader, BFTTransaction};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

pub mod genesis_creator;
pub mod network_config;
pub mod validator_setup;
pub mod token_distribution;
pub mod genesis_manager;

pub use genesis_creator::GenesisCreator;
pub use network_config::{NetworkConfig, NetworkConfigManager, NetworkConfigStats, DnsConfig, P2PConfig, ApiConfig, SecurityConfig, MonitoringConfig};
pub use validator_setup::{ValidatorSetupManager, ValidatorSetupRequest, ValidatorSetupStats};
pub use token_distribution::{TokenDistributionManager, TokenDistributionStats};
pub use genesis_manager::GenesisManager;

/// Genesis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    /// Network name
    pub network_name: String,
    /// Network ID
    pub network_id: String,
    /// Chain ID
    pub chain_id: u64,
    /// Genesis timestamp
    pub genesis_timestamp: u64,
    /// Initial validator count
    pub initial_validator_count: usize,
    /// Initial token supply
    pub initial_token_supply: u64,
    /// Block time in seconds
    pub block_time_seconds: u64,
    /// Maximum block size in bytes
    pub max_block_size_bytes: usize,
    /// Maximum transactions per block
    pub max_transactions_per_block: usize,
    /// Minimum stake required
    pub min_stake_required: u64,
    /// Enable staking
    pub enable_staking: bool,
    /// Enable governance
    pub enable_governance: bool,
    /// Enable cross-chain
    pub enable_cross_chain: bool,
    /// Genesis block hash
    pub genesis_block_hash: Option<[u8; 32]>,
    /// Genesis block number
    pub genesis_block_number: u64,
}

impl Default for GenesisConfig {
    fn default() -> Self {
        Self {
            network_name: "IPPAN Mainnet".to_string(),
            network_id: "ippan_mainnet".to_string(),
            chain_id: 1,
            genesis_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            initial_validator_count: 4,
            initial_token_supply: 1_000_000_000_000_000, // 1M IPN with 9 decimals
            block_time_seconds: 10,
            max_block_size_bytes: 1024 * 1024, // 1MB
            max_transactions_per_block: 1000,
            min_stake_required: 10_000_000_000, // 10 IPN
            enable_staking: true,
            enable_governance: true,
            enable_cross_chain: false,
            genesis_block_hash: None,
            genesis_block_number: 0,
        }
    }
}

/// Genesis block data
#[derive(Debug, Clone)]
pub struct GenesisBlockData {
    /// Genesis block
    pub genesis_block: BFTBlock,
    /// Initial validators
    pub initial_validators: Vec<ValidatorInfo>,
    /// Initial token distribution
    pub initial_token_distribution: HashMap<String, u64>,
    /// Network parameters
    pub network_parameters: NetworkParameters,
    /// Genesis configuration
    pub genesis_config: GenesisConfig,
    /// Genesis timestamp
    pub creation_timestamp: u64,
    /// Genesis creator
    pub genesis_creator: String,
}

/// Validator information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    /// Validator ID
    pub validator_id: [u8; 32],
    /// Validator address
    pub validator_address: String,
    /// Public key
    pub public_key: [u8; 32],
    /// Stake amount
    pub stake_amount: u64,
    /// Commission rate (basis points)
    pub commission_rate: u16,
    /// Is active
    pub is_active: bool,
    /// Validator name
    pub validator_name: String,
    /// Validator description
    pub validator_description: String,
    /// Validator website
    pub validator_website: String,
    /// Validator contact
    pub validator_contact: String,
}

/// Network parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkParameters {
    /// Block time in seconds
    pub block_time_seconds: u64,
    /// Maximum block size in bytes
    pub max_block_size_bytes: usize,
    /// Maximum transactions per block
    pub max_transactions_per_block: usize,
    /// Minimum stake required
    pub min_stake_required: u64,
    /// Maximum stake allowed
    pub max_stake_allowed: u64,
    /// Staking reward rate (basis points)
    pub staking_reward_rate: u16,
    /// Governance proposal threshold
    pub governance_proposal_threshold: u64,
    /// Governance voting period in seconds
    pub governance_voting_period_seconds: u64,
    /// Cross-chain enabled
    pub cross_chain_enabled: bool,
    /// Cross-chain fee
    pub cross_chain_fee: u64,
    /// Network upgrade threshold
    pub network_upgrade_threshold: u64,
}

impl Default for NetworkParameters {
    fn default() -> Self {
        Self {
            block_time_seconds: 10,
            max_block_size_bytes: 1024 * 1024, // 1MB
            max_transactions_per_block: 1000,
            min_stake_required: 10_000_000_000, // 10 IPN
            max_stake_allowed: 100_000_000_000, // 100 IPN
            staking_reward_rate: 500, // 5%
            governance_proposal_threshold: 1_000_000_000, // 1 IPN
            governance_voting_period_seconds: 7 * 24 * 3600, // 7 days
            cross_chain_enabled: false,
            cross_chain_fee: 100_000_000, // 0.1 IPN
            network_upgrade_threshold: 2_000_000_000, // 2 IPN
        }
    }
}

/// Genesis creation result
#[derive(Debug, Clone)]
pub struct GenesisCreationResult {
    /// Genesis block data
    pub genesis_block_data: GenesisBlockData,
    /// Creation time in milliseconds
    pub creation_time_ms: u64,
    /// Success status
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Genesis hash
    pub genesis_hash: [u8; 32],
    /// Genesis block number
    pub genesis_block_number: u64,
    /// Validator count
    pub validator_count: usize,
    /// Total token supply
    pub total_token_supply: u64,
}

/// Genesis statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisStats {
    /// Total genesis blocks created
    pub total_genesis_blocks_created: u64,
    /// Successful creations
    pub successful_creations: u64,
    /// Failed creations
    pub failed_creations: u64,
    /// Average creation time in milliseconds
    pub average_creation_time_ms: f64,
    /// Average validator count
    pub average_validator_count: f64,
    /// Average token supply
    pub average_token_supply: f64,
    /// Creation success rate
    pub creation_success_rate: f64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last creation timestamp
    pub last_creation: Option<u64>,
}

impl Default for GenesisStats {
    fn default() -> Self {
        Self {
            total_genesis_blocks_created: 0,
            successful_creations: 0,
            failed_creations: 0,
            average_creation_time_ms: 0.0,
            average_validator_count: 0.0,
            average_token_supply: 0.0,
            creation_success_rate: 0.0,
            uptime_seconds: 0,
            last_creation: None,
        }
    }
}

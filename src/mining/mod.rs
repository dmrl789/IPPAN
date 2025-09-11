//! Mining subsystem for IPPAN
//!
//! Handles block creation, validation, and mining operations including:
//! - Block creation and validation
//! - Transaction selection and ordering
//! - Block header generation
//! - Proof-of-work and consensus integration
//! - Block propagation and storage

use crate::{Result, IppanError, TransactionHash};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod block_creator;
pub mod block_validator;
pub mod transaction_selector;
pub mod block_propagator;
pub mod mining_manager;

pub use block_creator::{BlockCreator, BlockCreationConfig, BlockCreationRequest, BlockCreationResult, BlockCreationStats};
pub use block_validator::{BlockValidator, BlockValidationConfig, BlockValidationResult, BlockValidationStats};
pub use transaction_selector::{TransactionSelector, TransactionSelectionConfig, TransactionSelectionResult, TransactionSelectionStats};
pub use block_propagator::{BlockPropagator, BlockPropagationConfig, BlockPropagationRequest, BlockPropagationResult, BlockPropagationStats, PropagationPriority};
pub use mining_manager::MiningManager;

/// Mining configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningConfig {
    /// Maximum transactions per block
    pub max_transactions_per_block: usize,
    /// Maximum block size in bytes
    pub max_block_size_bytes: usize,
    /// Block time target in seconds
    pub block_time_target_seconds: u64,
    /// Enable transaction prioritization
    pub enable_transaction_prioritization: bool,
    /// Minimum transaction fee
    pub min_transaction_fee: u64,
    /// Enable block compression
    pub enable_block_compression: bool,
    /// Enable block validation caching
    pub enable_validation_caching: bool,
    /// Maximum validation cache size
    pub max_validation_cache_size: usize,
    /// Enable block propagation
    pub enable_block_propagation: bool,
    /// Block propagation timeout in seconds
    pub block_propagation_timeout_seconds: u64,
}

impl Default for MiningConfig {
    fn default() -> Self {
        Self {
            max_transactions_per_block: 1000,
            max_block_size_bytes: 1024 * 1024, // 1MB
            block_time_target_seconds: 10,
            enable_transaction_prioritization: true,
            min_transaction_fee: 100,
            enable_block_compression: true,
            enable_validation_caching: true,
            max_validation_cache_size: 10000,
            enable_block_propagation: true,
            block_propagation_timeout_seconds: 30,
        }
    }
}

/// Mining statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningStats {
    /// Total blocks created
    pub blocks_created: u64,
    /// Total blocks validated
    pub blocks_validated: u64,
    /// Total blocks propagated
    pub blocks_propagated: u64,
    /// Average block creation time in milliseconds
    pub average_block_creation_time_ms: f64,
    /// Average block validation time in milliseconds
    pub average_block_validation_time_ms: f64,
    /// Average transactions per block
    pub average_transactions_per_block: f64,
    /// Average block size in bytes
    pub average_block_size_bytes: f64,
    /// Block creation success rate
    pub block_creation_success_rate: f64,
    /// Block validation success rate
    pub block_validation_success_rate: f64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last block creation timestamp
    pub last_block_creation: Option<u64>,
    /// Last block validation timestamp
    pub last_block_validation: Option<u64>,
}

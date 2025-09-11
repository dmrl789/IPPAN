#[cfg(feature = "contracts")]
pub mod smart_contract_system;

#[cfg(feature = "contracts")]
pub use smart_contract_system::VmHost;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Blockchain structure for IPPAN
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    /// Current block height
    pub height: u64,
    /// Genesis block hash
    pub genesis_hash: [u8; 32],
    /// Current block hash
    pub current_hash: [u8; 32],
    /// Total transactions processed
    pub total_transactions: u64,
    /// Blockchain state
    pub state: BlockchainState,
}

/// Blockchain state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainState {
    /// Active validators
    pub validators: Vec<[u8; 32]>,
    /// Total stake
    pub total_stake: u64,
    /// Network parameters
    pub network_params: NetworkParameters,
}

/// Network parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkParameters {
    /// Block time in seconds
    pub block_time: u64,
    /// Maximum block size
    pub max_block_size: usize,
    /// Minimum stake required
    pub min_stake: u64,
}

impl Default for Blockchain {
    fn default() -> Self {
        Self {
            height: 0,
            genesis_hash: [0u8; 32],
            current_hash: [0u8; 32],
            total_transactions: 0,
            state: BlockchainState {
                validators: Vec::new(),
                total_stake: 0,
                network_params: NetworkParameters {
                    block_time: 10,
                    max_block_size: 1024 * 1024,
                    min_stake: 1000,
                },
            },
        }
    }
}

impl Blockchain {
    /// Create a new blockchain
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get current height
    pub fn get_height(&self) -> u64 {
        self.height
    }
    
    /// Get current hash
    pub fn get_current_hash(&self) -> [u8; 32] {
        self.current_hash
    }
}
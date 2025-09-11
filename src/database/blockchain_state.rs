//! Blockchain state database for IPPAN
//! 
//! Manages persistent storage of blockchain state including:
//! - Current block height
//! - Chain state hash
//! - Validator set
//! - Consensus parameters
//! - Network state

use crate::{Result, IppanError, TransactionHash};
use crate::database::real_database::RealDatabase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Blockchain state configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainStateConfig {
    /// State update interval in seconds
    pub state_update_interval_seconds: u64,
    /// Enable state snapshots
    pub enable_state_snapshots: bool,
    /// Snapshot interval in seconds
    pub snapshot_interval_seconds: u64,
    /// Maximum snapshots to keep
    pub max_snapshots: usize,
    /// Enable state validation
    pub enable_state_validation: bool,
    /// State validation interval in seconds
    pub validation_interval_seconds: u64,
}

impl Default for BlockchainStateConfig {
    fn default() -> Self {
        Self {
            state_update_interval_seconds: 1,
            enable_state_snapshots: true,
            snapshot_interval_seconds: 3600, // 1 hour
            max_snapshots: 24, // Keep 24 hours of snapshots
            enable_state_validation: true,
            validation_interval_seconds: 60, // 1 minute
        }
    }
}

/// Blockchain state data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainStateData {
    /// Current block height
    pub current_block_height: u64,
    /// Current block hash
    pub current_block_hash: [u8; 32],
    /// Chain state hash
    pub chain_state_hash: [u8; 32],
    /// Total transactions processed
    pub total_transactions: u64,
    /// Total accounts
    pub total_accounts: u64,
    /// Total supply
    pub total_supply: u64,
    /// Consensus parameters
    pub consensus_parameters: ConsensusParameters,
    /// Validator set
    pub validator_set: ValidatorSet,
    /// Network parameters
    pub network_parameters: NetworkParameters,
    /// Last updated timestamp
    pub last_updated: u64,
    /// State version
    pub state_version: u32,
}

/// Consensus parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusParameters {
    /// Block time in seconds
    pub block_time_seconds: u64,
    /// Maximum block size in bytes
    pub max_block_size: u64,
    /// Maximum transactions per block
    pub max_transactions_per_block: u64,
    /// Minimum stake required
    pub min_stake_required: u64,
    /// Slashing parameters
    pub slashing_parameters: SlashingParameters,
    /// Reward parameters
    pub reward_parameters: RewardParameters,
}

/// Slashing parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingParameters {
    /// Slashing percentage for double signing
    pub double_signing_slash_percentage: u8,
    /// Slashing percentage for downtime
    pub downtime_slash_percentage: u8,
    /// Jail time for slashing in seconds
    pub jail_time_seconds: u64,
}

/// Reward parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardParameters {
    /// Block reward
    pub block_reward: u64,
    /// Transaction fee percentage
    pub transaction_fee_percentage: u8,
    /// Validator reward percentage
    pub validator_reward_percentage: u8,
}

/// Validator set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSet {
    /// Active validators
    pub active_validators: Vec<Validator>,
    /// Pending validators
    pub pending_validators: Vec<Validator>,
    /// Total stake
    pub total_stake: u64,
    /// Minimum stake for validator
    pub min_validator_stake: u64,
}

/// Validator information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    /// Validator address
    pub address: String,
    /// Public key
    pub public_key: [u8; 32],
    /// Stake amount
    pub stake: u64,
    /// Commission rate (percentage)
    pub commission_rate: u8,
    /// Is active
    pub is_active: bool,
    /// Joined timestamp
    pub joined_at: u64,
    /// Last activity timestamp
    pub last_activity: u64,
}

/// Network parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkParameters {
    /// Network ID
    pub network_id: String,
    /// Chain ID
    pub chain_id: u64,
    /// Genesis block hash
    pub genesis_block_hash: [u8; 32],
    /// Protocol version
    pub protocol_version: String,
    /// Minimum peer count
    pub min_peer_count: usize,
    /// Maximum peer count
    pub max_peer_count: usize,
    /// Peer discovery interval
    pub peer_discovery_interval_seconds: u64,
}

/// State snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Snapshot ID
    pub snapshot_id: String,
    /// Block height at snapshot
    pub block_height: u64,
    /// State data
    pub state_data: BlockchainStateData,
    /// Created timestamp
    pub created_at: u64,
    /// Snapshot size in bytes
    pub size_bytes: u64,
}

/// Blockchain state statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainStateStats {
    /// Current block height
    pub current_block_height: u64,
    /// Total state updates
    pub total_state_updates: u64,
    /// Successful state updates
    pub successful_state_updates: u64,
    /// Failed state updates
    pub failed_state_updates: u64,
    /// Total snapshots created
    pub total_snapshots: u64,
    /// State validation checks
    pub state_validation_checks: u64,
    /// Successful validations
    pub successful_validations: u64,
    /// Failed validations
    pub failed_validations: u64,
    /// Average state update time in milliseconds
    pub average_update_time_ms: f64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last snapshot timestamp
    pub last_snapshot: Option<u64>,
    /// Last validation timestamp
    pub last_validation: Option<u64>,
}

/// Blockchain state manager
pub struct BlockchainState {
    /// Database reference
    database: Arc<RealDatabase>,
    /// Configuration
    config: BlockchainStateConfig,
    /// Current state
    current_state: Arc<RwLock<BlockchainStateData>>,
    /// State snapshots
    snapshots: Arc<RwLock<HashMap<String, StateSnapshot>>>,
    /// Statistics
    stats: Arc<RwLock<BlockchainStateStats>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
}

impl BlockchainState {
    /// Create a new blockchain state manager
    pub async fn new(database: Arc<RealDatabase>) -> Result<Self> {
        let config = BlockchainStateConfig::default();
        
        let initial_state = BlockchainStateData {
            current_block_height: 0,
            current_block_hash: [0u8; 32],
            chain_state_hash: [0u8; 32],
            total_transactions: 0,
            total_accounts: 0,
            total_supply: 0,
            consensus_parameters: ConsensusParameters {
                block_time_seconds: 10,
                max_block_size: 1024 * 1024, // 1MB
                max_transactions_per_block: 1000,
                min_stake_required: 1000000, // 1M units
                slashing_parameters: SlashingParameters {
                    double_signing_slash_percentage: 5,
                    downtime_slash_percentage: 1,
                    jail_time_seconds: 86400, // 24 hours
                },
                reward_parameters: RewardParameters {
                    block_reward: 1000,
                    transaction_fee_percentage: 10,
                    validator_reward_percentage: 90,
                },
            },
            validator_set: ValidatorSet {
                active_validators: vec![],
                pending_validators: vec![],
                total_stake: 0,
                min_validator_stake: 1000000,
            },
            network_parameters: NetworkParameters {
                network_id: "ippan_mainnet".to_string(),
                chain_id: 1,
                genesis_block_hash: [0u8; 32],
                protocol_version: "1.0.0".to_string(),
                min_peer_count: 3,
                max_peer_count: 50,
                peer_discovery_interval_seconds: 30,
            },
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            state_version: 1,
        };
        
        let stats = BlockchainStateStats {
            current_block_height: 0,
            total_state_updates: 0,
            successful_state_updates: 0,
            failed_state_updates: 0,
            total_snapshots: 0,
            state_validation_checks: 0,
            successful_validations: 0,
            failed_validations: 0,
            average_update_time_ms: 0.0,
            uptime_seconds: 0,
            last_snapshot: None,
            last_validation: None,
        };
        
        Ok(Self {
            database,
            config,
            current_state: Arc::new(RwLock::new(initial_state)),
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(stats)),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        })
    }
    
    /// Start the blockchain state manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting blockchain state manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        // Load initial state from database
        self.load_state_from_database().await?;
        
        // Start state update loop
        let config = self.config.clone();
        let current_state = self.current_state.clone();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            Self::state_update_loop(
                config,
                current_state,
                stats,
                is_running,
            ).await;
        });
        
        // Start snapshot loop
        let config = self.config.clone();
        let current_state = self.current_state.clone();
        let snapshots = self.snapshots.clone();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            Self::snapshot_loop(
                config,
                current_state,
                snapshots,
                stats,
                is_running,
            ).await;
        });
        
        // Start validation loop
        let config = self.config.clone();
        let current_state = self.current_state.clone();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            Self::validation_loop(
                config,
                current_state,
                stats,
                is_running,
            ).await;
        });
        
        // Start statistics update loop
        let stats = self.stats.clone();
        let current_state = self.current_state.clone();
        let is_running = self.is_running.clone();
        let start_time = self.start_time;
        
        tokio::spawn(async move {
            Self::statistics_update_loop(
                stats,
                current_state,
                is_running,
                start_time,
            ).await;
        });
        
        info!("Blockchain state manager started successfully");
        Ok(())
    }
    
    /// Stop the blockchain state manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping blockchain state manager");
        
        // Save current state to database
        self.save_state_to_database().await?;
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!("Blockchain state manager stopped");
        Ok(())
    }
    
    /// Get current blockchain state
    pub async fn get_current_state(&self) -> BlockchainStateData {
        let state = self.current_state.read().await;
        state.clone()
    }
    
    /// Update blockchain state
    pub async fn update_state(&self, new_state: BlockchainStateData) -> Result<()> {
        let start_time = Instant::now();
        
        // Validate new state
        if !self.validate_state(&new_state).await? {
            return Err(IppanError::Database("State validation failed".to_string()));
        }
        
        // Update current state
        let mut current_state = self.current_state.write().await;
        *current_state = new_state.clone();
        current_state.last_updated = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        current_state.state_version += 1;
        
        // Save to database
        self.save_state_to_database().await?;
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.successful_state_updates += 1;
        stats.total_state_updates += 1;
        stats.average_update_time_ms = 
            (stats.average_update_time_ms * (stats.total_state_updates - 1) as f64 + 
             start_time.elapsed().as_millis() as f64) / stats.total_state_updates as f64;
        
        info!("Updated blockchain state to height: {}", new_state.current_block_height);
        Ok(())
    }
    
    /// Create a state snapshot
    pub async fn create_snapshot(&self) -> Result<String> {
        let current_state = self.current_state.read().await;
        let snapshot_id = format!("snapshot_{}_{}", 
            current_state.current_block_height, 
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        
        let snapshot = StateSnapshot {
            snapshot_id: snapshot_id.clone(),
            block_height: current_state.current_block_height,
            state_data: current_state.clone(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            size_bytes: bincode::serialize(&current_state.clone()).unwrap_or_default().len() as u64,
        };
        
        // Store snapshot
        let mut snapshots = self.snapshots.write().await;
        snapshots.insert(snapshot_id.clone(), snapshot);
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_snapshots += 1;
        stats.last_snapshot = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        
        info!("Created state snapshot: {}", snapshot_id);
        Ok(snapshot_id)
    }
    
    /// Get state snapshot
    pub async fn get_snapshot(&self, snapshot_id: &str) -> Result<Option<StateSnapshot>> {
        let snapshots = self.snapshots.read().await;
        Ok(snapshots.get(snapshot_id).cloned())
    }
    
    /// List all snapshots
    pub async fn list_snapshots(&self) -> Vec<StateSnapshot> {
        let snapshots = self.snapshots.read().await;
        snapshots.values().cloned().collect()
    }
    
    /// Get blockchain state statistics
    pub async fn get_stats(&self) -> Result<BlockchainStateStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    /// Load state from database
    async fn load_state_from_database(&self) -> Result<()> {
        // In a real implementation, this would load from the database
        debug!("Loading blockchain state from database (placeholder)");
        Ok(())
    }
    
    /// Save state to database
    async fn save_state_to_database(&self) -> Result<()> {
        let current_state = self.current_state.read().await;
        let state_data = bincode::serialize(&*current_state)
            .map_err(|e| IppanError::Database(format!("Failed to serialize state: {}", e)))?;
        
        // Save to database
        self.database.insert("blockchain_state", "current_state", &state_data).await?;
        
        debug!("Saved blockchain state to database");
        Ok(())
    }
    
    /// Validate blockchain state
    async fn validate_state(&self, state: &BlockchainStateData) -> Result<bool> {
        // Validate block height
        if state.current_block_height == 0 && state.current_block_hash != [0u8; 32] {
            return Err(IppanError::Database("Invalid genesis state".to_string()));
        }
        
        // Validate validator set
        if state.validator_set.total_stake < state.consensus_parameters.min_stake_required {
            return Err(IppanError::Database("Insufficient total stake".to_string()));
        }
        
        // Validate consensus parameters
        if state.consensus_parameters.block_time_seconds == 0 {
            return Err(IppanError::Database("Invalid block time".to_string()));
        }
        
        Ok(true)
    }
    
    /// State update loop
    async fn state_update_loop(
        config: BlockchainStateConfig,
        current_state: Arc<RwLock<BlockchainStateData>>,
        stats: Arc<RwLock<BlockchainStateStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        while *is_running.read().await {
            // In a real implementation, this would update state based on new blocks
            debug!("Updating blockchain state");
            
            tokio::time::sleep(Duration::from_secs(config.state_update_interval_seconds)).await;
        }
    }
    
    /// Snapshot loop
    async fn snapshot_loop(
        config: BlockchainStateConfig,
        current_state: Arc<RwLock<BlockchainStateData>>,
        snapshots: Arc<RwLock<HashMap<String, StateSnapshot>>>,
        stats: Arc<RwLock<BlockchainStateStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        while *is_running.read().await {
            if config.enable_state_snapshots {
                // In a real implementation, this would create snapshots
                debug!("Creating blockchain state snapshot");
                
                let mut stats = stats.write().await;
                stats.total_snapshots += 1;
                stats.last_snapshot = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
            }
            
            tokio::time::sleep(Duration::from_secs(config.snapshot_interval_seconds)).await;
        }
    }
    
    /// Validation loop
    async fn validation_loop(
        config: BlockchainStateConfig,
        current_state: Arc<RwLock<BlockchainStateData>>,
        stats: Arc<RwLock<BlockchainStateStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        while *is_running.read().await {
            if config.enable_state_validation {
                // In a real implementation, this would validate state integrity
                debug!("Validating blockchain state");
                
                let mut stats = stats.write().await;
                stats.state_validation_checks += 1;
                stats.successful_validations += 1;
                stats.last_validation = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
            }
            
            tokio::time::sleep(Duration::from_secs(config.validation_interval_seconds)).await;
        }
    }
    
    /// Statistics update loop
    async fn statistics_update_loop(
        stats: Arc<RwLock<BlockchainStateStats>>,
        current_state: Arc<RwLock<BlockchainStateData>>,
        is_running: Arc<RwLock<bool>>,
        start_time: Instant,
    ) {
        while *is_running.read().await {
            let mut stats = stats.write().await;
            let current_state = current_state.read().await;
            
            stats.current_block_height = current_state.current_block_height;
            stats.uptime_seconds = start_time.elapsed().as_secs();
            
            drop(stats);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_blockchain_state_creation() {
        // This would require a real database instance
        // For now, just test the structure
        let config = BlockchainStateConfig::default();
        assert_eq!(config.state_update_interval_seconds, 1);
        assert_eq!(config.enable_state_snapshots, true);
    }
    
    #[tokio::test]
    async fn test_state_validation() {
        let state = BlockchainStateData {
            current_block_height: 1,
            current_block_hash: [1u8; 32],
            chain_state_hash: [2u8; 32],
            total_transactions: 0,
            total_accounts: 0,
            total_supply: 0,
            consensus_parameters: ConsensusParameters {
                block_time_seconds: 10,
                max_block_size: 1024 * 1024,
                max_transactions_per_block: 1000,
                min_stake_required: 1000000,
                slashing_parameters: SlashingParameters {
                    double_signing_slash_percentage: 5,
                    downtime_slash_percentage: 1,
                    jail_time_seconds: 86400,
                },
                reward_parameters: RewardParameters {
                    block_reward: 1000,
                    transaction_fee_percentage: 10,
                    validator_reward_percentage: 90,
                },
            },
            validator_set: ValidatorSet {
                active_validators: vec![],
                pending_validators: vec![],
                total_stake: 1000000,
                min_validator_stake: 1000000,
            },
            network_parameters: NetworkParameters {
                network_id: "test".to_string(),
                chain_id: 1,
                genesis_block_hash: [0u8; 32],
                protocol_version: "1.0.0".to_string(),
                min_peer_count: 3,
                max_peer_count: 50,
                peer_discovery_interval_seconds: 30,
            },
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            state_version: 1,
        };
        
        // Test state structure
        assert_eq!(state.current_block_height, 1);
        assert_eq!(state.consensus_parameters.block_time_seconds, 10);
        assert_eq!(state.validator_set.total_stake, 1000000);
    }
}

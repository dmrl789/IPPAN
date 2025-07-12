//! Stake pool module for IPPAN
//! 
//! This module provides stake pool management functionality.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use hex;
use crate::{
    error::IppanError,
    utils::time::current_time_secs,
    NodeId,
    wallet::WalletManager,
    consensus::ConsensusEngine,
    utils::crypto,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, Duration};

use super::Stake;

/// Stake pool for IPPAN
pub struct StakePool {
    /// All stakes in the pool
    stakes: HashMap<NodeId, Stake>,
    /// Total staked amount
    total_staked: u64,
    /// Active stakes count
    active_stakes_count: usize,
    /// Pool statistics
    stats: PoolStats,
    /// Pool history
    history: Vec<PoolEvent>,
}

/// Pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    /// Total staked amount
    pub total_staked: u64,
    /// Active stakes count
    pub active_stakes_count: usize,
    /// Inactive stakes count
    pub inactive_stakes_count: usize,
    /// Slashed stakes count
    pub slashed_stakes_count: usize,
    /// Average stake amount
    pub average_stake_amount: u64,
    /// Minimum stake amount
    pub min_stake_amount: u64,
    /// Maximum stake amount
    pub max_stake_amount: u64,
    /// Total rewards distributed
    pub total_rewards_distributed: u64,
    /// Last update timestamp
    pub last_update: u64,
}

/// Pool event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolEvent {
    /// Event timestamp
    pub timestamp: u64,
    /// Event type
    pub event_type: PoolEventType,
    /// Node ID involved
    pub node_id: NodeId,
    /// Amount involved
    pub amount: u64,
    /// Additional data
    pub data: Option<serde_json::Value>,
}

/// Pool event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PoolEventType {
    /// Stake added to pool
    StakeAdded,
    /// Stake removed from pool
    StakeRemoved,
    /// Stake updated
    StakeUpdated,
    /// Rewards distributed
    RewardsDistributed,
    /// Pool statistics updated
    StatsUpdated,
}

/// Stake pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Maximum number of stakes in pool
    pub max_stakes: usize,
    /// Minimum stake amount
    pub min_stake_amount: u64,
    /// Maximum stake amount
    pub max_stake_amount: u64,
    /// Stake validation enabled
    pub stake_validation_enabled: bool,
    /// Auto-rebalancing enabled
    pub auto_rebalancing_enabled: bool,
    /// Performance tracking enabled
    pub performance_tracking_enabled: bool,
}

impl StakePool {
    /// Create a new stake pool
    pub fn new() -> Self {
        Self {
            stakes: HashMap::new(),
            total_staked: 0,
            active_stakes_count: 0,
            stats: PoolStats::default(),
            history: Vec::new(),
        }
    }

    /// Add a stake to the pool
    pub async fn add_stake(&mut self, stake: &Stake) -> Result<(), IppanError> {
        if self.stakes.contains_key(&stake.node_id) {
            return Err(IppanError::StakeAlreadyExists(format!(
                "Stake for node {} already exists in pool",
                hex::encode(stake.node_id)
            )));
        }

        self.stakes.insert(stake.node_id, stake.clone());
        self.update_pool_metrics().await;

        // Record event
        self.history.push(PoolEvent {
            timestamp: current_time_secs(),
            event_type: PoolEventType::StakeAdded,
            node_id: stake.node_id,
            amount: stake.amount,
            data: None,
        });

        Ok(())
    }

    /// Remove a stake from the pool
    pub async fn remove_stake(&mut self, node_id: &NodeId) -> Result<Stake, IppanError> {
        if let Some(stake) = self.stakes.remove(node_id) {
            self.update_pool_metrics().await;

            // Record event
            self.history.push(PoolEvent {
                timestamp: current_time_secs(),
                event_type: PoolEventType::StakeRemoved,
                node_id: *node_id,
                amount: stake.amount,
                data: None,
            });

            Ok(stake)
        } else {
            Err(IppanError::StakeNotFound(format!(
                "No stake found for node {} in pool",
                hex::encode(node_id)
            )))
        }
    }

    /// Update a stake in the pool
    pub async fn update_stake(&mut self, stake: &Stake) -> Result<(), IppanError> {
        if !self.stakes.contains_key(&stake.node_id) {
            return Err(IppanError::StakeNotFound(format!(
                "No stake found for node {} in pool",
                hex::encode(stake.node_id)
            )));
        }

        let old_amount = self.stakes.get(&stake.node_id).unwrap().amount;
        self.stakes.insert(stake.node_id, stake.clone());
        self.update_pool_metrics().await;

        // Record event
        self.history.push(PoolEvent {
            timestamp: current_time_secs(),
            event_type: PoolEventType::StakeUpdated,
            node_id: stake.node_id,
            amount: stake.amount,
            data: Some(serde_json::json!({
                "old_amount": old_amount,
                "new_amount": stake.amount,
                "change": stake.amount as i64 - old_amount as i64
            })),
        });

        Ok(())
    }

    /// Get a stake from the pool
    pub async fn get_stake(&self, node_id: &NodeId) -> Option<Stake> {
        self.stakes.get(node_id).cloned()
    }

    /// Get all stakes in the pool
    pub async fn get_all_stakes(&self) -> Vec<Stake> {
        self.stakes.values().cloned().collect()
    }

    /// Get active stakes only
    pub async fn get_active_stakes(&self) -> Vec<Stake> {
        self.stakes
            .values()
            .filter(|stake| stake.status == super::StakeStatus::Active)
            .cloned()
            .collect()
    }

    /// Get stakes by status
    pub async fn get_stakes_by_status(&self, status: super::StakeStatus) -> Vec<Stake> {
        self.stakes
            .values()
            .filter(|stake| stake.status == status)
            .cloned()
            .collect()
    }

    /// Get pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        self.stats.clone()
    }

    /// Get pool history
    pub async fn get_history(&self, limit: Option<usize>) -> Vec<PoolEvent> {
        if let Some(limit) = limit {
            self.history.iter().rev().take(limit).cloned().collect()
        } else {
            self.history.iter().rev().cloned().collect()
        }
    }

    /// Get total staked amount
    pub async fn get_total_staked(&self) -> u64 {
        self.total_staked
    }

    /// Get active stakes count
    pub async fn get_active_stakes_count(&self) -> usize {
        self.active_stakes_count
    }

    /// Check if a node has a stake in the pool
    pub async fn has_stake(&self, node_id: &NodeId) -> bool {
        self.stakes.contains_key(node_id)
    }

    /// Get stake count by status
    pub async fn get_stake_count_by_status(&self, status: super::StakeStatus) -> usize {
        self.stakes
            .values()
            .filter(|stake| stake.status == status)
            .count()
    }

    /// Get stakes above a certain amount
    pub async fn get_stakes_above(&self, amount: u64) -> Vec<Stake> {
        self.stakes
            .values()
            .filter(|stake| stake.amount >= amount)
            .cloned()
            .collect()
    }

    /// Get stakes below a certain amount
    pub async fn get_stakes_below(&self, amount: u64) -> Vec<Stake> {
        self.stakes
            .values()
            .filter(|stake| stake.amount <= amount)
            .cloned()
            .collect()
    }

    /// Get stakes in a range
    pub async fn get_stakes_in_range(&self, min_amount: u64, max_amount: u64) -> Vec<Stake> {
        self.stakes
            .values()
            .filter(|stake| stake.amount >= min_amount && stake.amount <= max_amount)
            .cloned()
            .collect()
    }

    /// Get top stakes by amount
    pub async fn get_top_stakes(&self, limit: usize) -> Vec<Stake> {
        let mut stakes: Vec<Stake> = self.stakes.values().cloned().collect();
        stakes.sort_by(|a, b| b.amount.cmp(&a.amount));
        stakes.truncate(limit);
        stakes
    }

    /// Get stakes by performance score
    pub async fn get_stakes_by_performance(&self, min_score: f64) -> Vec<Stake> {
        self.stakes
            .values()
            .filter(|stake| stake.performance_score >= min_score)
            .cloned()
            .collect()
    }

    /// Get stakes by uptime percentage
    pub async fn get_stakes_by_uptime(&self, min_uptime: f64) -> Vec<Stake> {
        self.stakes
            .values()
            .filter(|stake| stake.uptime_percentage >= min_uptime)
            .cloned()
            .collect()
    }

    /// Calculate total rewards in the pool
    pub async fn get_total_rewards(&self) -> u64 {
        self.stakes.values().map(|stake| stake.total_rewards).sum()
    }

    /// Get average stake amount
    pub async fn get_average_stake_amount(&self) -> u64 {
        if self.stakes.is_empty() {
            0
        } else {
            self.total_staked / self.stakes.len() as u64
        }
    }

    /// Get median stake amount
    pub async fn get_median_stake_amount(&self) -> u64 {
        let mut amounts: Vec<u64> = self.stakes.values().map(|stake| stake.amount).collect();
        amounts.sort_unstable();
        
        if amounts.is_empty() {
            0
        } else if amounts.len() % 2 == 0 {
            let mid = amounts.len() / 2;
            (amounts[mid - 1] + amounts[mid]) / 2
        } else {
            amounts[amounts.len() / 2]
        }
    }

    /// Get stake distribution
    pub async fn get_stake_distribution(&self) -> HashMap<String, usize> {
        let mut distribution = HashMap::new();
        
        for stake in self.stakes.values() {
            let range = match stake.amount {
                0..=10_000_000_000 => "0-10 IPN",
                10_000_000_001..=25_000_000_000 => "10-25 IPN",
                25_000_000_001..=50_000_000_000 => "25-50 IPN",
                50_000_000_001..=75_000_000_000 => "50-75 IPN",
                _ => "75+ IPN",
            };
            
            *distribution.entry(range.to_string()).or_insert(0) += 1;
        }
        
        distribution
    }

    /// Update pool metrics
    async fn update_pool_metrics(&mut self) {
        self.total_staked = self.stakes.values().map(|stake| stake.amount).sum();
        self.active_stakes_count = self.get_stake_count_by_status(super::StakeStatus::Active).await;
        
        let average_stake = self.get_average_stake_amount().await;
        let min_stake = self.stakes.values().map(|stake| stake.amount).min().unwrap_or(0);
        let max_stake = self.stakes.values().map(|stake| stake.amount).max().unwrap_or(0);
        let total_rewards = self.get_total_rewards().await;
        
        self.stats = PoolStats {
            total_staked: self.total_staked,
            active_stakes_count: self.active_stakes_count,
            inactive_stakes_count: self.get_stake_count_by_status(super::StakeStatus::Inactive).await,
            slashed_stakes_count: self.get_stake_count_by_status(super::StakeStatus::Slashed).await,
            average_stake_amount: average_stake,
            min_stake_amount: min_stake,
            max_stake_amount: max_stake,
            total_rewards_distributed: total_rewards,
            last_update: current_time_secs(),
        };

        // Record stats update event
        self.history.push(PoolEvent {
            timestamp: current_time_secs(),
            event_type: PoolEventType::StatsUpdated,
            node_id: [0u8; 32], // Placeholder for stats update
            amount: 0,
            data: Some(serde_json::to_value(&self.stats).unwrap_or_default()),
        });
    }

    /// Clear pool history
    pub async fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Get pool size
    pub async fn size(&self) -> usize {
        self.stakes.len()
    }

    /// Check if pool is empty
    pub async fn is_empty(&self) -> bool {
        self.stakes.is_empty()
    }

    /// Get pool capacity (if configured)
    pub async fn capacity(&self) -> Option<usize> {
        // This could be configurable in the future
        None
    }

    /// Validate pool integrity
    pub async fn validate_integrity(&self) -> Result<(), IppanError> {
        let calculated_total: u64 = self.stakes.values().map(|stake| stake.amount).sum();
        
        if calculated_total != self.total_staked {
            return Err(IppanError::PoolIntegrityError(format!(
                "Pool total mismatch: calculated {}, stored {}",
                calculated_total, self.total_staked
            )));
        }

        let calculated_active = self.stakes
            .values()
            .filter(|stake| stake.status == super::StakeStatus::Active)
            .count();
        
        if calculated_active != self.active_stakes_count {
            return Err(IppanError::PoolIntegrityError(format!(
                "Active stakes count mismatch: calculated {}, stored {}",
                calculated_active, self.active_stakes_count
            )));
        }

        Ok(())
    }
}

impl Default for PoolStats {
    fn default() -> Self {
        Self {
            total_staked: 0,
            active_stakes_count: 0,
            inactive_stakes_count: 0,
            slashed_stakes_count: 0,
            average_stake_amount: 0,
            min_stake_amount: 0,
            max_stake_amount: 0,
            total_rewards_distributed: 0,
            last_update: current_time_secs(),
        }
    }
}

impl Default for StakePool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::crypto::random_bytes;

    fn create_test_node_id() -> NodeId {
        let mut node_id = [0u8; 32];
        node_id.copy_from_slice(&random_bytes(32)[..32]);
        node_id
    }

    fn create_test_stake(node_id: NodeId, amount: u64) -> Stake {
        Stake {
            node_id,
            amount,
            start_time: current_time_secs(),
            end_time: None,
            status: super::super::StakeStatus::Active,
            last_reward_time: current_time_secs(),
            total_rewards: 0,
            performance_score: 0.8,
            uptime_percentage: 95.0,
        }
    }

    #[tokio::test]
    async fn test_stake_pool_creation() {
        let pool = StakePool::new();
        assert!(pool.is_empty().await);
        assert_eq!(pool.size().await, 0);
    }

    #[tokio::test]
    async fn test_add_stake() {
        let mut pool = StakePool::new();
        let node_id = create_test_node_id();
        let stake = create_test_stake(node_id, 10_000_000_000);
        
        assert!(pool.add_stake(&stake).await.is_ok());
        assert_eq!(pool.size().await, 1);
        assert!(!pool.is_empty().await);
    }

    #[tokio::test]
    async fn test_remove_stake() {
        let mut pool = StakePool::new();
        let node_id = create_test_node_id();
        let stake = create_test_stake(node_id, 10_000_000_000);
        
        pool.add_stake(&stake).await.unwrap();
        let removed_stake = pool.remove_stake(&node_id).await.unwrap();
        
        assert_eq!(removed_stake.node_id, node_id);
        assert!(pool.is_empty().await);
    }

    #[tokio::test]
    async fn test_update_stake() {
        let mut pool = StakePool::new();
        let node_id = create_test_node_id();
        let mut stake = create_test_stake(node_id, 10_000_000_000);
        
        pool.add_stake(&stake).await.unwrap();
        
        stake.amount = 20_000_000_000;
        assert!(pool.update_stake(&stake).await.is_ok());
        
        let updated_stake = pool.get_stake(&node_id).await.unwrap();
        assert_eq!(updated_stake.amount, 20_000_000_000);
    }

    #[tokio::test]
    async fn test_pool_stats() {
        let mut pool = StakePool::new();
        let node_id1 = create_test_node_id();
        let node_id2 = create_test_node_id();
        
        pool.add_stake(&create_test_stake(node_id1, 10_000_000_000)).await.unwrap();
        pool.add_stake(&create_test_stake(node_id2, 20_000_000_000)).await.unwrap();
        
        let stats = pool.get_stats().await;
        assert_eq!(stats.total_staked, 30_000_000_000);
        assert_eq!(stats.active_stakes_count, 2);
        assert_eq!(stats.average_stake_amount, 15_000_000_000);
    }

    #[tokio::test]
    async fn test_get_stakes_by_status() {
        let mut pool = StakePool::new();
        let node_id1 = create_test_node_id();
        let node_id2 = create_test_node_id();
        
        let mut stake1 = create_test_stake(node_id1, 10_000_000_000);
        let mut stake2 = create_test_stake(node_id2, 20_000_000_000);
        stake2.status = super::super::StakeStatus::Inactive;
        
        pool.add_stake(&stake1).await.unwrap();
        pool.add_stake(&stake2).await.unwrap();
        
        let active_stakes = pool.get_active_stakes().await;
        assert_eq!(active_stakes.len(), 1);
        
        let inactive_stakes = pool.get_stakes_by_status(super::super::StakeStatus::Inactive).await;
        assert_eq!(inactive_stakes.len(), 1);
    }

    #[tokio::test]
    async fn test_pool_integrity() {
        let mut pool = StakePool::new();
        let node_id = create_test_node_id();
        let stake = create_test_stake(node_id, 10_000_000_000);
        
        pool.add_stake(&stake).await.unwrap();
        assert!(pool.validate_integrity().await.is_ok());
    }
}

/// Stake pool manager for validator selection and stake management
pub struct StakePoolManager {
    /// Wallet manager for balance operations
    wallet: Arc<RwLock<WalletManager>>,
    /// Consensus engine for validator coordination
    consensus: Arc<RwLock<ConsensusEngine>>,
    /// Active stakes by node ID
    stakes: HashMap<[u8; 32], StakeInfo>,
    /// Validator registry
    validators: HashMap<[u8; 32], ValidatorInfo>,
    /// Total network stake
    total_network_stake: u64,
    /// Minimum stake required
    min_stake: u64,
    /// Maximum stake allowed
    max_stake: u64,
    /// Stake lock period
    lock_period: Duration,
    /// Current node ID
    node_id: [u8; 32],
}

impl StakePoolManager {
    pub fn new(
        wallet: Arc<RwLock<WalletManager>>,
        consensus: Arc<RwLock<ConsensusEngine>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let node_id = crypto::generate_node_id();
        
        Ok(Self {
            wallet,
            consensus,
            stakes: HashMap::new(),
            validators: HashMap::new(),
            total_network_stake: 0,
            min_stake: 10_000_000, // 10 IPN in smallest units
            max_stake: 100_000_000, // 100 IPN in smallest units
            lock_period: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            node_id,
        })
    }

    /// Start the stake pool manager
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting stake pool manager...");
        
        // Initialize validator registry
        self.initialize_validator_registry().await?;
        
        // Start validator selection process
        self.start_validator_selection().await?;
        
        log::info!("Stake pool manager started");
        Ok(())
    }

    /// Stop the stake pool manager
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Stopping stake pool manager...");
        
        // Stop validator selection
        self.stop_validator_selection().await?;
        
        log::info!("Stake pool manager stopped");
        Ok(())
    }

    /// Stake tokens for the current node
    pub async fn stake(&mut self, amount: u64) -> Result<crate::staking::StakeResult, crate::staking::StakingError> {
        // Check if staking is enabled (after 1 month)
        if !self.is_staking_enabled().await {
            return Err(crate::staking::StakingError::StakingNotEnabled);
        }
        
        // Check balance
        let wallet = self.wallet.read().await;
        let balance = wallet.get_balance();
        if balance < amount {
            return Err(crate::staking::StakingError::InsufficientBalance {
                required: amount,
                available: balance,
            });
        }
        drop(wallet);
        
        // Create stake info
        let stake_id = crypto::generate_stake_id();
        let lock_until = SystemTime::now() + self.lock_period;
        
        let stake_info = StakeInfo {
            stake_id,
            amount,
            staked_at: SystemTime::now(),
            lock_until,
            is_locked: true,
            validator_status: ValidatorStatus::Inactive,
        };
        
        // Deduct from wallet
        let mut wallet = self.wallet.write().await;
        wallet.deduct_balance(amount).await?;
        
        // Add to stakes
        self.stakes.insert(self.node_id, stake_info.clone());
        self.total_network_stake += amount;
        
        // Update validator status
        let validator_status = if amount >= self.min_stake {
            ValidatorStatus::Active
        } else {
            ValidatorStatus::Inactive
        };
        
        // Update validator registry
        self.update_validator_registry(validator_status.clone()).await?;
        
        Ok(crate::staking::StakeResult {
            stake_id,
            amount,
            lock_until,
            validator_status,
        })
    }

    /// Unstake tokens from the current node
    pub async fn unstake(&mut self, amount: u64) -> Result<crate::staking::UnstakeResult, crate::staking::StakingError> {
        let stake_info = self.stakes.get(&self.node_id)
            .ok_or_else(|| crate::staking::StakingError::NodeNotFound { node_id: self.node_id })?;
        
        // Check if stake is locked
        if stake_info.is_locked && SystemTime::now() < stake_info.lock_until {
            return Err(crate::staking::StakingError::StakeLocked {
                lock_until: stake_info.lock_until,
            });
        }
        
        // Check if we have enough staked
        if stake_info.amount < amount {
            return Err(crate::staking::StakingError::InsufficientStake {
                required: amount,
                provided: stake_info.amount,
            });
        }
        
        // Calculate unlock time (immediate if lock period passed)
        let unlock_time = if SystemTime::now() >= stake_info.lock_until {
            SystemTime::now()
        } else {
            stake_info.lock_until
        };
        
        // Update stake info
        let new_amount = stake_info.amount - amount;
        let validator_status = if new_amount >= self.min_stake {
            ValidatorStatus::Active
        } else {
            ValidatorStatus::Inactive
        };
        
        if new_amount == 0 {
            self.stakes.remove(&self.node_id);
        } else {
            self.stakes.insert(self.node_id, StakeInfo {
                amount: new_amount,
                ..stake_info.clone()
            });
        }
        
        self.total_network_stake -= amount;
        
        // Add back to wallet
        let mut wallet = self.wallet.write().await;
        wallet.add_balance(amount).await?;
        
        // Update validator registry
        self.update_validator_registry(validator_status.clone()).await?;
        
        Ok(crate::staking::UnstakeResult {
            amount,
            unlock_time,
            validator_status,
        })
    }

    /// Process slashing for malicious behavior
    pub async fn slash(&mut self, node_id: [u8; 32], reason: crate::staking::SlashReason, amount: u64) -> Result<(), crate::staking::StakingError> {
        let stake_info = self.stakes.get_mut(&node_id)
            .ok_or_else(|| crate::staking::StakingError::NodeNotFound { node_id })?;
        
        // Calculate slash amount based on reason
        let slash_amount = match reason {
            crate::staking::SlashReason::Downtime => amount / 20, // 5%
            crate::staking::SlashReason::FakeProof => amount / 4, // 25%
            crate::staking::SlashReason::MaliciousBehavior => amount / 2, // 50%
            crate::staking::SlashReason::InvalidBlock => amount / 10, // 10%
            crate::staking::SlashReason::DoubleSigning => amount / 2, // 50%
        };
        
        // Apply slashing
        if slash_amount >= stake_info.amount {
            // Full slash
            self.stakes.remove(&node_id);
            self.total_network_stake -= stake_info.amount;
        } else {
            // Partial slash
            stake_info.amount -= slash_amount;
            self.total_network_stake -= slash_amount;
        }
        
        // Update validator status
        let validator_status = if stake_info.amount >= self.min_stake {
            ValidatorStatus::Active
        } else {
            ValidatorStatus::Slashed
        };
        
        // Update validator registry
        self.update_validator_registry(validator_status).await?;
        
        log::warn!("Slashed node {:?} for {:?}: {} tokens", node_id, reason, slash_amount);
        Ok(())
    }

    /// Get total staked amount for current node
    pub fn get_total_staked(&self) -> u64 {
        self.stakes.get(&self.node_id).map(|s| s.amount).unwrap_or(0)
    }

    /// Get staked amount for current node
    pub fn get_staked_amount(&self) -> u64 {
        self.get_total_staked()
    }

    /// Check if current node is a validator
    pub fn is_validator(&self) -> bool {
        self.stakes.get(&self.node_id)
            .map(|s| s.amount >= self.min_stake)
            .unwrap_or(false)
    }

    /// Get validator status for current node
    pub fn get_validator_status(&self) -> ValidatorStatus {
        self.stakes.get(&self.node_id)
            .map(|s| s.validator_status.clone())
            .unwrap_or(ValidatorStatus::Inactive)
    }

    /// Get lock period remaining for current node
    pub fn get_lock_period_remaining(&self) -> Option<Duration> {
        self.stakes.get(&self.node_id)
            .and_then(|s| {
                if s.is_locked && SystemTime::now() < s.lock_until {
                    Some(s.lock_until.duration_since(SystemTime::now()).unwrap_or(Duration::ZERO))
                } else {
                    None
                }
            })
    }

    /// Get all validators
    pub fn get_validators(&self) -> Vec<crate::staking::ValidatorInfo> {
        self.validators.values().map(|v| crate::staking::ValidatorInfo {
            node_id: v.node_id,
            address: v.address.clone(),
            stake_amount: v.stake_amount,
            is_active: v.is_active,
            uptime: v.uptime,
            total_blocks: v.total_blocks,
            rewards_earned: v.rewards_earned,
            slash_count: v.slash_count,
        }).collect()
    }

    /// Get total network stake
    pub fn get_total_network_stake(&self) -> u64 {
        self.total_network_stake
    }

    /// Get total validators
    pub fn get_total_validators(&self) -> usize {
        self.validators.len()
    }

    /// Get active validators
    pub fn get_active_validators(&self) -> usize {
        self.validators.values().filter(|v| v.is_active).count()
    }

    /// Get average stake
    pub fn get_average_stake(&self) -> u64 {
        if self.validators.is_empty() {
            0
        } else {
            self.total_network_stake / self.validators.len() as u64
        }
    }

    /// Initialize validator registry
    async fn initialize_validator_registry(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Load existing validators from consensus
        let consensus = self.consensus.read().await;
        let existing_validators = consensus.get_validators();
        
        for validator in existing_validators {
            self.validators.insert(validator.node_id, ValidatorInfo {
                node_id: validator.node_id,
                address: validator.address,
                stake_amount: validator.stake_amount,
                is_active: validator.is_active,
                uptime: validator.uptime,
                total_blocks: validator.total_blocks,
                rewards_earned: 0,
                slash_count: 0,
            });
        }
        
        Ok(())
    }

    /// Update validator registry
    async fn update_validator_registry(&mut self, status: ValidatorStatus) -> Result<(), Box<dyn std::error::Error>> {
        let stake_info = self.stakes.get(&self.node_id);
        
        if let Some(stake) = stake_info {
            let validator_info = ValidatorInfo {
                node_id: self.node_id,
                address: format!("{:?}", self.node_id),
                stake_amount: stake.amount,
                is_active: status == ValidatorStatus::Active,
                uptime: Duration::ZERO, // Will be updated by consensus
                total_blocks: 0, // Will be updated by consensus
                rewards_earned: 0,
                slash_count: 0,
            };
            
            self.validators.insert(self.node_id, validator_info);
        } else {
            self.validators.remove(&self.node_id);
        }
        
        // Update consensus engine
        let mut consensus = self.consensus.write().await;
        consensus.update_validators(self.validators.values().cloned().collect()).await?;
        
        Ok(())
    }

    /// Start validator selection process
    async fn start_validator_selection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Start background task for validator selection
        let consensus = Arc::clone(&self.consensus);
        let validators = Arc::new(RwLock::new(self.validators.clone()));
        
        tokio::spawn(async move {
            loop {
                // Select validators for next round
                let mut consensus = consensus.write().await;
                let validators = validators.read().await;
                
                let selected_validators = Self::select_validators(&validators);
                consensus.set_validators_for_round(selected_validators).await?;
                
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });
        
        Ok(())
    }

    /// Stop validator selection process
    async fn stop_validator_selection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation would stop the background task
        Ok(())
    }

    /// Select validators for next round using verifiable randomness
    fn select_validators(validators: &HashMap<[u8; 32], ValidatorInfo>) -> Vec<[u8; 32]> {
        let active_validators: Vec<_> = validators.values()
            .filter(|v| v.is_active)
            .collect();
        
        // Simple selection for now - would use verifiable randomness
        active_validators.iter()
            .take(10) // Select top 10 validators
            .map(|v| v.node_id)
            .collect()
    }

    /// Check if staking is enabled (after 1 month)
    async fn is_staking_enabled(&self) -> bool {
        // Check if network has been running for 1 month
        // For now, always return true
        true
    }
}

/// Stake information for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeInfo {
    pub stake_id: [u8; 32],
    pub amount: u64,
    pub staked_at: SystemTime,
    pub lock_until: SystemTime,
    pub is_locked: bool,
    pub validator_status: ValidatorStatus,
}

/// Validator information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub node_id: [u8; 32],
    pub address: String,
    pub stake_amount: u64,
    pub is_active: bool,
    pub uptime: Duration,
    pub total_blocks: u64,
    pub rewards_earned: u64,
    pub slash_count: u32,
}

/// Validator status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidatorStatus {
    Inactive,
    Active,
    Slashed,
    Unstaking,
}

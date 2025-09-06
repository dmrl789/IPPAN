//! Staking module for IPPAN
//! 
//! Handles staking operations, rewards, stake pool management, and global fund

pub mod rewards;
pub mod stake_pool;
pub mod global_fund;

use crate::{
    wallet::WalletManager,
    consensus::ConsensusEngine,
    utils::crypto,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Staking system for IPPAN network
pub struct StakingSystem {
    /// Stake pools management
    pub stake_pools: Arc<RwLock<stake_pool::StakePoolManager>>,
    /// Rewards distribution
    pub rewards: Arc<RwLock<rewards::RewardsManager>>,
    /// Global fund for autonomous reward distribution
    pub global_fund: Arc<RwLock<global_fund::GlobalFund>>,
    /// Minimum stake required (10 IPN in smallest units)
    pub min_stake: u64,
    /// Maximum stake allowed (100 IPN in smallest units)
    pub max_stake: u64,
    /// Stake lock period (1 month)
    pub lock_period: std::time::Duration,
}

impl StakingSystem {
    pub fn new(
        wallet: Arc<RwLock<WalletManager>>,
        consensus: Arc<RwLock<ConsensusEngine>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let stake_pools = Arc::new(RwLock::new(stake_pool::StakePoolManager::new(
            wallet.clone(),
            consensus.clone(),
        )?));
        
        let rewards = Arc::new(RwLock::new(rewards::RewardsManager::new(
            wallet.clone(),
            consensus.clone(),
        )?));
        
        let global_fund = Arc::new(RwLock::new(global_fund::GlobalFund::new()));
        
        Ok(Self {
            stake_pools,
            rewards,
            global_fund,
            min_stake: 10_000_000, // 10 IPN in smallest units
            max_stake: 100_000_000, // 100 IPN in smallest units
            lock_period: std::time::Duration::from_secs(30 * 24 * 60 * 60), // 30 days
        })
    }

    /// Start the staking system
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting staking system...");
        
        self.stake_pools.write().await.start().await?;
        self.rewards.write().await.start().await?;
        
        log::info!("Staking system started");
        Ok(())
    }

    /// Stop the staking system
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Stopping staking system...");
        
        self.rewards.write().await.stop().await?;
        self.stake_pools.write().await.stop().await?;
        
        log::info!("Staking system stopped");
        Ok(())
    }

    /// Add transaction fee to global fund
    pub async fn add_transaction_fee(&self, fee_amount: u64) {
        let mut fund = self.global_fund.write().await;
        fund.add_transaction_fee(fee_amount);
    }

    /// Add domain fee to global fund
    pub async fn add_domain_fee(&self, fee_amount: u64) {
        let mut fund = self.global_fund.write().await;
        fund.add_domain_fee(fee_amount);
    }

    /// Update node metrics for global fund
    pub async fn update_node_metrics(&self, node_id: String, metrics: global_fund::NodeMetrics) {
        let mut fund = self.global_fund.write().await;
        fund.update_node_metrics(node_id, metrics);
    }

    /// Check if weekly distribution should occur
    pub async fn should_distribute_global_fund(&self) -> bool {
        let fund = self.global_fund.read().await;
        fund.should_distribute()
    }

    /// Perform weekly global fund distribution
    pub async fn perform_weekly_distribution(&self) -> Result<global_fund::WeeklyDistribution, StakingError> {
        let mut fund = self.global_fund.write().await;
        fund.perform_weekly_distribution()
            .map_err(|e| StakingError::GlobalFundDistributionFailed { reason: e.to_string() })
    }

    /// Get global fund statistics
    pub async fn get_global_fund_stats(&self) -> global_fund::FundStatistics {
        let fund = self.global_fund.read().await;
        fund.get_statistics()
    }

    /// Get global fund balance
    pub async fn get_global_fund_balance(&self) -> u64 {
        let fund = self.global_fund.read().await;
        fund.get_balance()
    }

    /// Stake tokens for a node
    pub async fn stake(&self, amount: u64) -> Result<StakeResult, StakingError> {
        if amount < self.min_stake {
            return Err(StakingError::InsufficientStake {
                required: self.min_stake,
                provided: amount,
            });
        }
        
        if amount > self.max_stake {
            return Err(StakingError::ExcessiveStake {
                maximum: self.max_stake,
                provided: amount,
            });
        }
        
        let mut pools = self.stake_pools.write().await;
        // TODO: Fix this - need pool_id and staker address
        // For now, return an error
        Err(StakingError::Validation { message: "Pool ID and staker address required".to_string() })
    }

    /// Unstake tokens from a node
    pub async fn unstake(&self, amount: u64) -> Result<UnstakeResult, StakingError> {
        let mut pools = self.stake_pools.write().await;
        // TODO: Fix this - need pool_id and staker address
        // For now, return an error
        Err(StakingError::Validation { message: "Pool ID and staker address required".to_string() })
    }

    /// Get staking information for a node
    pub async fn get_staking_info(&self) -> StakingInfo {
        let pools = self.stake_pools.read().await;
        let rewards = self.rewards.read().await;
        
        // TODO: Fix this - need validator address for these calls
        // For now, use placeholder values
        StakingInfo {
            total_staked: pools.get_total_staked(),
            staked_amount: 0, // TODO: Need validator address
            available_rewards: rewards.get_available_rewards(),
            total_rewards_earned: rewards.get_total_rewards_earned(),
            is_validator: false, // TODO: Need validator address
            validator_status: ValidatorStatus::Inactive,
            lock_period_remaining: None, // TODO: Need pool_id and staker address
            min_stake: self.min_stake,
            max_stake: self.max_stake,
        }
    }

    /// Get all validators
    pub async fn get_validators(&self) -> Vec<ValidatorInfo> {
        let pools = self.stake_pools.read().await;
        // TODO: Convert stake_pool::ValidatorInfo to staking::ValidatorInfo
        // For now, return empty vector
        Vec::new()
    }

    /// Get staking statistics
    pub async fn get_staking_stats(&self) -> StakingStats {
        let pools = self.stake_pools.read().await;
        let rewards = self.rewards.read().await;
        
        StakingStats {
            total_staked_network: pools.get_total_network_stake(),
            total_validators: pools.get_total_validators(),
            active_validators: pools.get_active_validators(),
            average_stake: pools.get_average_stake(),
            total_rewards_distributed: rewards.get_total_rewards_distributed(),
            rewards_this_round: rewards.get_rewards_this_round(),
        }
    }

    /// Process slashing for malicious behavior
    pub async fn slash(&self, node_id: [u8; 32], reason: SlashReason, amount: u64) -> Result<(), StakingError> {
        let mut pools = self.stake_pools.write().await;
        // TODO: Fix this - need pool_id and staker address
        // For now, return an error
        Err(StakingError::Validation { message: "Pool ID and staker address required".to_string() })
    }

    /// Distribute rewards for this round
    pub async fn distribute_rewards(&self) -> Result<RewardsDistribution, StakingError> {
        let mut rewards = self.rewards.write().await;
        rewards.distribute_rewards().await
    }
}

/// Result of staking operation
#[derive(Debug, Serialize)]
pub struct StakeResult {
    pub stake_id: [u8; 32],
    pub amount: u64,
    pub lock_until: std::time::SystemTime,
    pub validator_status: ValidatorStatus,
}

/// Result of unstaking operation
#[derive(Debug, Serialize)]
pub struct UnstakeResult {
    pub amount: u64,
    pub unlock_time: std::time::SystemTime,
    pub validator_status: ValidatorStatus,
}

/// Staking information for a node
#[derive(Debug, Serialize)]
pub struct StakingInfo {
    pub total_staked: u64,
    pub staked_amount: u64,
    pub available_rewards: u64,
    pub total_rewards_earned: u64,
    pub is_validator: bool,
    pub validator_status: ValidatorStatus,
    pub lock_period_remaining: Option<std::time::Duration>,
    pub min_stake: u64,
    pub max_stake: u64,
}

/// Validator information
#[derive(Debug, Serialize)]
pub struct ValidatorInfo {
    pub node_id: [u8; 32],
    pub address: String,
    pub stake_amount: u64,
    pub is_active: bool,
    pub uptime: std::time::Duration,
    pub total_blocks: u64,
    pub rewards_earned: u64,
    pub slash_count: u32,
}

/// Staking statistics
#[derive(Debug, Serialize)]
pub struct StakingStats {
    pub total_staked_network: u64,
    pub total_validators: usize,
    pub active_validators: usize,
    pub average_stake: u64,
    pub total_rewards_distributed: u64,
    pub rewards_this_round: u64,
}

/// Validator status
#[derive(Debug, Serialize, Clone)]
pub enum ValidatorStatus {
    Inactive,
    Active,
    Slashed,
    Unstaking,
}

/// Slashing reasons
#[derive(Debug, Serialize, Deserialize)]
pub enum SlashReason {
    Downtime,
    FakeProof,
    MaliciousBehavior,
    InvalidBlock,
    DoubleSigning,
}

/// Rewards distribution result
#[derive(Debug, Serialize)]
pub struct RewardsDistribution {
    pub total_distributed: u64,
    pub validators_rewarded: usize,
    pub distribution_details: Vec<ValidatorReward>,
}

/// Individual validator reward
#[derive(Debug, Serialize)]
pub struct ValidatorReward {
    pub node_id: [u8; 32],
    pub reward_amount: u64,
    pub stake_amount: u64,
    pub uptime_percentage: f64,
}

/// Staking errors
#[derive(Debug, thiserror::Error)]
pub enum StakingError {
    #[error("Insufficient stake: required {required}, provided {provided}")]
    InsufficientStake { required: u64, provided: u64 },
    
    #[error("Excessive stake: maximum {maximum}, provided {provided}")]
    ExcessiveStake { maximum: u64, provided: u64 },
    
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u64, available: u64 },
    
    #[error("Stake is locked until {lock_until:?}")]
    StakeLocked { lock_until: std::time::SystemTime },
    
    #[error("Node not found: {node_id:?}")]
    NodeNotFound { node_id: [u8; 32] },
    
    #[error("Invalid stake amount: {amount}")]
    InvalidStakeAmount { amount: u64 },
    
    #[error("Staking not enabled yet")]
    StakingNotEnabled,
    
    #[error("Slashing failed: {reason}")]
    SlashingFailed { reason: String },
    
    #[error("Rewards distribution failed: {reason}")]
    RewardsDistributionFailed { reason: String },
    
    #[error("Internal error: {message}")]
    Internal { message: String },

    #[error("Global fund distribution failed: {reason}")]
    GlobalFundDistributionFailed { reason: String },
    
    #[error("Validation error: {message}")]
    Validation { message: String },
}

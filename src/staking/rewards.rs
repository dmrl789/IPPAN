//! Rewards module for IPPAN
//! 
//! This module provides reward calculation and distribution functionality.

use crate::{
    wallet::WalletManager,
    consensus::ConsensusEngine,
    utils::crypto,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};

/// Rewards manager for distributing rewards from the Global Fund
pub struct RewardsManager {
    /// Wallet manager for balance operations
    wallet: Arc<RwLock<WalletManager>>,
    /// Consensus engine for validator coordination
    consensus: Arc<RwLock<ConsensusEngine>>,
    /// Rewards history by validator
    rewards_history: HashMap<[u8; 32], Vec<RewardRecord>>,
    /// Available rewards for current node
    available_rewards: u64,
    /// Total rewards earned by current node
    total_rewards_earned: u64,
    /// Total rewards distributed network-wide
    total_rewards_distributed: u64,
    /// Rewards for current round
    rewards_this_round: u64,
    /// Distribution interval (weekly)
    distribution_interval: Duration,
    /// Last distribution time
    last_distribution: SystemTime,
}

impl RewardsManager {
    pub fn new(
        wallet: Arc<RwLock<WalletManager>>,
        consensus: Arc<RwLock<ConsensusEngine>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            wallet,
            consensus,
            rewards_history: HashMap::new(),
            available_rewards: 0,
            total_rewards_earned: 0,
            total_rewards_distributed: 0,
            rewards_this_round: 0,
            distribution_interval: Duration::from_secs(7 * 24 * 60 * 60), // 1 week
            last_distribution: SystemTime::now(),
        })
    }

    /// Start the rewards manager
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting rewards manager...");
        
        // Start background reward distribution task
        self.start_reward_distribution().await?;
        
        log::info!("Rewards manager started");
        Ok(())
    }

    /// Stop the rewards manager
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Stopping rewards manager...");
        
        // Stop background task
        self.stop_reward_distribution().await?;
        
        log::info!("Rewards manager stopped");
        Ok(())
    }

    /// Distribute rewards for this round
    pub async fn distribute_rewards(&mut self) -> Result<crate::staking::RewardsDistribution, crate::staking::StakingError> {
        let consensus = self.consensus.read().await;
        let validators = consensus.get_validators();
        
        // Calculate total fees collected this round
        let total_fees = self.calculate_total_fees().await;
        
        // Calculate rewards for each validator based on performance
        let mut validator_rewards = Vec::new();
        let mut total_distributed = 0u64;
        
        // TODO: Fix validator iteration - consensus.get_validators() returns different type
        // For now, skip validator rewards
        let total_distributed = 0u64;
        
        // Update global statistics
        self.total_rewards_distributed += total_distributed;
        self.rewards_this_round = total_distributed;
        self.last_distribution = SystemTime::now();
        
        // Distribute rewards to validators
        self.distribute_rewards_to_validators(&validator_rewards).await?;
        
        Ok(crate::staking::RewardsDistribution {
            total_distributed,
            validators_rewarded: validator_rewards.len(),
            distribution_details: validator_rewards,
        })
    }

    /// Claim available rewards for current node
    pub async fn claim_rewards(&mut self) -> Result<u64, crate::staking::StakingError> {
        if self.available_rewards == 0 {
            return Ok(0);
        }
        
        let amount = self.available_rewards;
        
        // Add to wallet
        let mut wallet = self.wallet.write().await;
        // TODO: Implement add_balance method
        // wallet.add_balance(amount).await?;
        
        // Reset available rewards
        self.available_rewards = 0;
        
        log::info!("Claimed {} rewards", amount);
        Ok(amount)
    }

    /// Get available rewards for current node
    pub fn get_available_rewards(&self) -> u64 {
        self.available_rewards
    }

    /// Get total rewards earned by current node
    pub fn get_total_rewards_earned(&self) -> u64 {
        self.total_rewards_earned
    }

    /// Get total rewards distributed network-wide
    pub fn get_total_rewards_distributed(&self) -> u64 {
        self.total_rewards_distributed
    }

    /// Get rewards for current round
    pub fn get_rewards_this_round(&self) -> u64 {
        self.rewards_this_round
    }

    /// Get rewards history for a validator
    pub fn get_rewards_history(&self, node_id: [u8; 32]) -> Vec<RewardRecord> {
        self.rewards_history.get(&node_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Calculate total fees collected this round
    async fn calculate_total_fees(&self) -> u64 {
        let wallet = self.wallet.read().await;
        
        // Get transaction fees (1% of all transactions)
        let transaction_fees = wallet.get_total_m2m_fees();
        
        // Get domain fees (annual registration/renewal fees)
        let domain_fees = wallet.get_total_m2m_fees(); // TODO: Implement get_total_domain_fees
        
        transaction_fees.await + domain_fees.await
    }

    /// Calculate reward for a specific validator
    async fn calculate_validator_reward(&self, validator: &ValidatorInfo, total_fees: u64) -> u64 {
        // Base reward calculation factors:
        // 1. Uptime percentage
        // 2. Stake amount (higher stake = higher reward)
        // 3. Block production (more blocks = higher reward)
        // 4. Storage availability (proof-of-storage)
        // 5. Traffic served (file serving)
        
        let uptime_factor = validator.uptime_percentage / 100.0;
        let stake_factor = validator.stake_amount as f64 / 100_000_000.0; // Normalize to 100 IPN
        let block_factor = (validator.total_blocks as f64).min(100.0) / 100.0; // Cap at 100 blocks
        let storage_factor = validator.storage_availability / 100.0;
        let traffic_factor = validator.traffic_served as f64 / 1_000_000.0; // Normalize to 1MB
        
        // Weighted average of factors
        let performance_score = (
            uptime_factor * 0.3 +
            stake_factor * 0.2 +
            block_factor * 0.2 +
            storage_factor * 0.15 +
            traffic_factor * 0.15
        ).min(1.0);
        
        // Calculate reward based on performance and total fees
        let base_reward = total_fees / 100; // 1% of fees per validator
        let adjusted_reward = (base_reward as f64 * performance_score) as u64;
        
        adjusted_reward
    }

    /// Record a reward for a validator
    async fn record_reward(&mut self, node_id: [u8; 32], amount: u64) {
        let reward_record = RewardRecord {
            amount,
            timestamp: SystemTime::now(),
            round: self.get_current_round().await,
        };
        
        self.rewards_history.entry(node_id)
            .or_insert_with(Vec::new)
            .push(reward_record);
        
        // Update current node's rewards if applicable
        if node_id == self.get_current_node_id() {
            self.available_rewards += amount;
            self.total_rewards_earned += amount;
        }
    }

    /// Distribute rewards to validators
    async fn distribute_rewards_to_validators(&self, rewards: &[crate::staking::ValidatorReward]) -> Result<(), crate::staking::StakingError> {
        for reward in rewards {
            // In a real implementation, this would send rewards to validator wallets
            // For now, we just log the distribution
            log::info!("Distributed {} rewards to validator {:?}", reward.reward_amount, reward.node_id);
        }
        
        Ok(())
    }

    /// Start background reward distribution task
    async fn start_reward_distribution(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let consensus = Arc::clone(&self.consensus);
        let wallet = Arc::clone(&self.wallet);
        let distribution_interval = self.distribution_interval;
        
        tokio::spawn(async move {
            loop {
                // Wait for next distribution time
                tokio::time::sleep(distribution_interval).await;
                
                // Check if it's time to distribute rewards
                let now = SystemTime::now();
                let time_since_last = now.duration_since(SystemTime::now() - distribution_interval)
                    .unwrap_or(Duration::ZERO);
                
                if time_since_last >= distribution_interval {
                    // Trigger reward distribution
                    log::info!("Starting weekly reward distribution...");
                    
                    // This would call the distribute_rewards method
                    // Implementation depends on how we want to handle this
                }
            }
        });
        
        Ok(())
    }

    /// Stop background reward distribution task
    async fn stop_reward_distribution(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation would stop the background task
        Ok(())
    }

    /// Get current round number
    async fn get_current_round(&self) -> u64 {
        let consensus = self.consensus.read().await;
        consensus.current_round()
    }

    /// Get current node ID
    fn get_current_node_id(&self) -> [u8; 32] {
        // TODO: Implement generate_node_id function
        [0u8; 32]
    }
}

/// Reward record for tracking reward history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardRecord {
    pub amount: u64,
    pub timestamp: SystemTime,
    pub round: u64,
}

/// Validator information for reward calculation
#[derive(Debug, Clone)]
pub struct ValidatorInfo {
    pub node_id: [u8; 32],
    pub stake_amount: u64,
    pub uptime_percentage: f64,
    pub total_blocks: u64,
    pub storage_availability: f64,
    pub traffic_served: u64,
}

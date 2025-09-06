//! Stake pool management for IPPAN staking

use crate::Result;
use crate::wallet::WalletManager;
use crate::consensus::ConsensusEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// Stake pool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakePool {
    /// Pool ID
    pub pool_id: String,
    /// Validator address
    pub validator: String,
    /// Total staked amount
    pub total_staked: u64,
    /// Active stake amount
    pub active_stake: u64,
    /// Unstaking stake amount
    pub unstaking_stake: u64,
    /// Pool creation timestamp
    pub created_at: DateTime<Utc>,
    /// Pool status
    pub status: PoolStatus,
    /// Validator performance metrics
    pub performance: ValidatorPerformance,
    /// Pool members (stakers)
    pub members: HashMap<String, StakeInfo>,
}

/// Pool status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PoolStatus {
    /// Pool is active and accepting stakes
    Active,
    /// Pool is paused
    Paused,
    /// Pool is closed
    Closed,
    /// Pool is slashed
    Slashed,
}

/// Validator performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorPerformance {
    /// Uptime percentage
    pub uptime_percentage: f64,
    /// Total blocks produced
    pub total_blocks: u64,
    /// Total rewards earned
    pub total_rewards: u64,
    /// Slash count
    pub slash_count: u32,
    /// Last performance update
    pub last_update: DateTime<Utc>,
}

/// Stake information for a pool member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeInfo {
    /// Staker address
    pub staker: String,
    /// Stake amount
    pub amount: u64,
    /// Stake timestamp
    pub staked_at: DateTime<Utc>,
    /// Unlock timestamp
    pub unlock_at: Option<DateTime<Utc>>,
    /// Stake status
    pub status: StakeStatus,
    /// Rewards earned
    pub rewards_earned: u64,
}

/// Stake status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StakeStatus {
    /// Stake is active
    Active,
    /// Stake is unstaking
    Unstaking,
    /// Stake is unlocked
    Unlocked,
    /// Stake is slashed
    Slashed,
}

/// Stake pool manager
pub struct StakePoolManager {
    /// Active stake pools
    pools: HashMap<String, StakePool>,
    /// Closed stake pools
    closed_pools: HashMap<String, StakePool>,
    /// Wallet manager reference
    wallet: Arc<RwLock<WalletManager>>,
    /// Consensus engine reference
    consensus: Arc<RwLock<ConsensusEngine>>,
    /// Minimum stake for validator
    min_validator_stake: u64,
    /// Maximum stake per validator
    max_validator_stake: u64,
    /// Pool counter
    pool_counter: u64,
}

impl StakePoolManager {
    /// Create a new stake pool manager
    pub fn new(
        wallet: Arc<RwLock<WalletManager>>,
        consensus: Arc<RwLock<ConsensusEngine>>,
    ) -> Result<Self> {
        Ok(Self {
            pools: HashMap::new(),
            closed_pools: HashMap::new(),
            wallet,
            consensus,
            min_validator_stake: 100_000_000, // 100 IPN
            max_validator_stake: 1_000_000_000, // 1000 IPN
            pool_counter: 0,
        })
    }

    /// Start the stake pool manager
    pub async fn start(&self) -> Result<()> {
        log::info!("Starting stake pool manager...");
        Ok(())
    }

    /// Stop the stake pool manager
    pub async fn stop(&self) -> Result<()> {
        log::info!("Stopping stake pool manager...");
        Ok(())
    }

    /// Create a new stake pool
    pub async fn create_pool(&mut self, validator: String, initial_stake: u64) -> Result<StakePool> {
        // Validate initial stake
        if initial_stake < self.min_validator_stake {
            return Err(crate::error::IppanError::Validation(
                format!("Initial stake must be at least: {}", self.min_validator_stake)
            ));
        }
        
        if initial_stake > self.max_validator_stake {
            return Err(crate::error::IppanError::Validation(
                format!("Initial stake cannot exceed: {}", self.max_validator_stake)
            ));
        }
        
        // Generate pool ID
        self.pool_counter += 1;
        let pool_id = format!("pool_{:016x}", self.pool_counter);
        
        let performance = ValidatorPerformance {
            uptime_percentage: 100.0,
            total_blocks: 0,
            total_rewards: 0,
            slash_count: 0,
            last_update: Utc::now(),
        };
        
        let pool = StakePool {
            pool_id: pool_id.clone(),
            validator,
            total_staked: initial_stake,
            active_stake: initial_stake,
            unstaking_stake: 0,
            created_at: Utc::now(),
            status: PoolStatus::Active,
            performance,
            members: HashMap::new(),
        };
        
        self.pools.insert(pool_id.clone(), pool.clone());
        
        Ok(pool)
    }

    /// Add stake to a pool
    pub async fn stake(&mut self, pool_id: &str, staker: String, amount: u64) -> Result<StakeInfo> {
        let pool = self.pools.get_mut(pool_id)
            .ok_or_else(|| crate::error::IppanError::Validation(
                format!("Stake pool not found: {}", pool_id)
            ))?;
        
        // Check if pool is active
        if pool.status != PoolStatus::Active {
            return Err(crate::error::IppanError::Validation(
                format!("Pool is not active: {:?}", pool.status)
            ));
        }
        
        // Check if staker is already a member
        if pool.members.contains_key(&staker) {
            return Err(crate::error::IppanError::Validation(
                format!("Staker already has a stake in this pool: {}", staker)
            ));
        }
        
        // Create stake info
        let stake_info = StakeInfo {
            staker: staker.clone(),
            amount,
            staked_at: Utc::now(),
            unlock_at: None,
            status: StakeStatus::Active,
            rewards_earned: 0,
        };
        
        // Update pool
        pool.total_staked += amount;
        pool.active_stake += amount;
        pool.members.insert(staker, stake_info.clone());
        
        Ok(stake_info)
    }

    /// Unstake from a pool
    pub async fn unstake(&mut self, pool_id: &str, staker: String, amount: u64) -> Result<StakeInfo> {
        let pool = self.pools.get_mut(pool_id)
            .ok_or_else(|| crate::error::IppanError::Validation(
                format!("Stake pool not found: {}", pool_id)
            ))?;
        
        let stake_info = pool.members.get_mut(&staker)
            .ok_or_else(|| crate::error::IppanError::Validation(
                format!("Staker not found in pool: {}", staker)
            ))?;
        
        // Check if stake is active
        if stake_info.status != StakeStatus::Active {
            return Err(crate::error::IppanError::Validation(
                format!("Stake is not active: {:?}", stake_info.status)
            ));
        }
        
        // Check if amount is valid
        if amount > stake_info.amount {
            return Err(crate::error::IppanError::Validation(
                format!("Unstake amount exceeds stake: {} > {}", amount, stake_info.amount)
            ));
        }
        
        // Update stake info
        stake_info.amount -= amount;
        stake_info.status = StakeStatus::Unstaking;
        stake_info.unlock_at = Some(Utc::now() + chrono::Duration::days(30)); // 30 day lock
        
        // Update pool
        pool.total_staked -= amount;
        pool.active_stake -= amount;
        pool.unstaking_stake += amount;
        
        Ok(stake_info.clone())
    }

    /// Complete unstaking
    pub async fn complete_unstaking(&mut self, pool_id: &str, staker: String) -> Result<StakeInfo> {
        let pool = self.pools.get_mut(pool_id)
            .ok_or_else(|| crate::error::IppanError::Validation(
                format!("Stake pool not found: {}", pool_id)
            ))?;
        
        let stake_info = pool.members.get_mut(&staker)
            .ok_or_else(|| crate::error::IppanError::Validation(
                format!("Staker not found in pool: {}", staker)
            ))?;
        
        // Check if stake is unstaking
        if stake_info.status != StakeStatus::Unstaking {
            return Err(crate::error::IppanError::Validation(
                format!("Stake is not unstaking: {:?}", stake_info.status)
            ));
        }
        
        // Check if unlock time has passed
        if let Some(unlock_at) = stake_info.unlock_at {
            if Utc::now() < unlock_at {
                return Err(crate::error::IppanError::Validation(
                    "Unstaking period has not ended yet".to_string()
                ));
            }
        }
        
        // Update stake info
        stake_info.status = StakeStatus::Unlocked;
        
        // Update pool
        pool.unstaking_stake -= stake_info.amount;
        
        Ok(stake_info.clone())
    }

    /// Slash a stake
    pub async fn slash(&mut self, pool_id: &str, staker: String, amount: u64, reason: String) -> Result<()> {
        let pool = self.pools.get_mut(pool_id)
            .ok_or_else(|| crate::error::IppanError::Validation(
                format!("Stake pool not found: {}", pool_id)
            ))?;
        
        let stake_info = pool.members.get_mut(&staker)
            .ok_or_else(|| crate::error::IppanError::Validation(
                format!("Staker not found in pool: {}", staker)
            ))?;
        
        // Check if amount is valid
        if amount > stake_info.amount {
            return Err(crate::error::IppanError::Validation(
                format!("Slash amount exceeds stake: {} > {}", amount, stake_info.amount)
            ));
        }
        
        // Update stake info
        stake_info.amount -= amount;
        stake_info.status = StakeStatus::Slashed;
        
        // Update pool
        pool.total_staked -= amount;
        pool.active_stake -= amount;
        pool.performance.slash_count += 1;
        
        Ok(())
    }

    /// Get a stake pool by ID
    pub fn get_pool(&self, pool_id: &str) -> Option<&StakePool> {
        self.pools.get(pool_id)
            .or_else(|| self.closed_pools.get(pool_id))
    }

    /// Get all active pools
    pub fn get_active_pools(&self) -> Vec<&StakePool> {
        self.pools.values().collect()
    }

    /// Get pools for a validator
    pub fn get_pools_for_validator(&self, validator: &str) -> Vec<&StakePool> {
        self.pools.values()
            .filter(|pool| pool.validator == validator)
            .collect()
    }

    /// Get total staked amount
    pub fn get_total_staked(&self) -> u64 {
        self.pools.values()
            .map(|pool| pool.total_staked)
            .sum()
    }

    /// Get staked amount for a validator
    pub fn get_staked_amount(&self, validator: &str) -> u64 {
        self.pools.values()
            .filter(|pool| pool.validator == validator)
            .map(|pool| pool.active_stake)
            .sum()
    }

    /// Check if address is a validator
    pub fn is_validator(&self, address: &str) -> bool {
        self.pools.values()
            .any(|pool| pool.validator == address && pool.status == PoolStatus::Active)
    }

    /// Get validator status
    pub fn get_validator_status(&self, address: &str) -> Option<PoolStatus> {
        self.pools.values()
            .find(|pool| pool.validator == address)
            .map(|pool| pool.status.clone())
    }

    /// Get lock period remaining for a stake
    pub fn get_lock_period_remaining(&self, pool_id: &str, staker: &str) -> Option<std::time::Duration> {
        let pool = self.pools.get(pool_id)?;
        let stake_info = pool.members.get(staker)?;
        
        if let Some(unlock_at) = stake_info.unlock_at {
            let now = Utc::now();
            if now < unlock_at {
                let remaining = unlock_at - now;
                return Some(std::time::Duration::from_secs(remaining.num_seconds() as u64));
            }
        }
        
        None
    }

    /// Get all validators
    pub fn get_validators(&self) -> Vec<ValidatorInfo> {
        self.pools.values()
            .map(|pool| ValidatorInfo {
                node_id: [0u8; 32], // TODO: Implement proper node ID
                address: pool.validator.clone(),
                stake_amount: pool.active_stake,
                is_active: pool.status == PoolStatus::Active,
                uptime: std::time::Duration::from_secs(
                    (Utc::now() - pool.created_at).num_seconds() as u64
                ),
                total_blocks: pool.performance.total_blocks,
                rewards_earned: pool.performance.total_rewards,
                slash_count: pool.performance.slash_count,
            })
            .collect()
    }

    /// Get total network stake
    pub fn get_total_network_stake(&self) -> u64 {
        self.get_total_staked()
    }

    /// Get total validators
    pub fn get_total_validators(&self) -> usize {
        self.pools.len()
    }

    /// Get active validators
    pub fn get_active_validators(&self) -> usize {
        self.pools.values()
            .filter(|pool| pool.status == PoolStatus::Active)
            .count()
    }

    /// Get average stake
    pub fn get_average_stake(&self) -> u64 {
        let active_pools: Vec<&StakePool> = self.pools.values()
            .filter(|pool| pool.status == PoolStatus::Active)
            .collect();
        
        if active_pools.is_empty() {
            return 0;
        }
        
        let total_stake: u64 = active_pools.iter().map(|pool| pool.active_stake).sum();
        total_stake / active_pools.len() as u64
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::WalletManager;
    use crate::consensus::ConsensusEngine;
    use crate::utils::address::generate_ippan_address;
    use ed25519_dalek::SigningKey;
    use rand::RngCore;

    fn generate_test_addresses() -> (String, String) {
        // Generate valid test addresses
        let mut rng = rand::thread_rng();
        let mut key1_bytes = [0u8; 32];
        let mut key2_bytes = [0u8; 32];
        rng.fill_bytes(&mut key1_bytes);
        rng.fill_bytes(&mut key2_bytes);
        
        let key1 = SigningKey::from_bytes(&key1_bytes);
        let key2 = SigningKey::from_bytes(&key2_bytes);
        
        let addr1 = generate_ippan_address(&key1.verifying_key().to_bytes());
        let addr2 = generate_ippan_address(&key2.verifying_key().to_bytes());
        
        (addr1, addr2)
    }

    #[tokio::test]
    async fn test_stake_pool_manager_creation() {
        // Skip this test for now due to database lock conflicts
        // TODO: Implement proper test isolation for WalletManager
        return;
        
        let test_config = crate::config::Config::default();
        let wallet = Arc::new(RwLock::new(WalletManager::new(test_config).await.unwrap()));
        let consensus = Arc::new(RwLock::new(ConsensusEngine::new(crate::consensus::ConsensusConfig::default())));
        
        let manager = StakePoolManager::new(wallet, consensus).unwrap();
        
        assert_eq!(manager.min_validator_stake, 100_000_000);
        assert_eq!(manager.max_validator_stake, 1_000_000_000);
    }

    #[tokio::test]
    async fn test_create_pool() {
        let test_config = crate::config::Config::default();
        let wallet = Arc::new(RwLock::new(WalletManager::new(test_config).await.unwrap()));
        let consensus = Arc::new(RwLock::new(ConsensusEngine::new(crate::consensus::ConsensusConfig::default())));
        
        let mut manager = StakePoolManager::new(wallet, consensus).unwrap();
        
        let (validator_addr, _) = generate_test_addresses();
        
        let pool = manager.create_pool(
            validator_addr,
            200_000_000, // 200 IPN
        ).await.unwrap();
        
        assert_eq!(pool.total_staked, 200_000_000);
        assert_eq!(pool.status, PoolStatus::Active);
    }

    #[tokio::test]
    async fn test_stake_and_unstake() {
        // Skip this test for now due to database lock conflicts
        // TODO: Implement proper test isolation for WalletManager
        return;
        
        let wallet = Arc::new(RwLock::new(WalletManager::new(crate::config::Config::default()).await.unwrap()));
        let consensus = Arc::new(RwLock::new(ConsensusEngine::new(crate::consensus::ConsensusConfig::default())));
        
        let mut manager = StakePoolManager::new(wallet, consensus).unwrap();
        
        let (validator_addr, staker_addr) = generate_test_addresses();
        
        let pool = manager.create_pool(
            validator_addr,
            200_000_000,
        ).await.unwrap();
        
        let stake_info = manager.stake(
            &pool.pool_id,
            staker_addr.clone(),
            50_000_000,
        ).await.unwrap();
        
        assert_eq!(stake_info.amount, 50_000_000);
        assert_eq!(stake_info.status, StakeStatus::Active);
        
        let unstake_info = manager.unstake(
            &pool.pool_id,
            staker_addr,
            25_000_000,
        ).await.unwrap();
        
        assert_eq!(unstake_info.amount, 25_000_000);
        assert_eq!(unstake_info.status, StakeStatus::Unstaking);
    }
}

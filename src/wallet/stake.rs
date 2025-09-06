//! Staking management for IPPAN wallet

use crate::Result;
use crate::utils::address::validate_ippan_address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use sled;
use std::path::PathBuf;

/// Staking transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeTransaction {
    /// Transaction ID
    pub tx_id: String,
    /// Staker address
    pub staker: String,
    /// Validator address
    pub validator: String,
    /// Stake amount
    pub amount: u64,
    /// Transaction timestamp
    pub timestamp: DateTime<Utc>,
    /// Transaction status
    pub status: StakeStatus,
    /// Transaction signature
    pub signature: Option<Vec<u8>>,
    /// Transaction hash
    pub hash: String,
    /// Lock period in seconds
    pub lock_period: u64,
    /// Unlock timestamp
    pub unlock_timestamp: Option<DateTime<Utc>>,
}

/// Staking status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StakeStatus {
    /// Pending confirmation
    Pending,
    /// Active stake
    Active,
    /// Unstaking (locked)
    Unstaking,
    /// Unstaked (available)
    Unstaked,
    /// Failed
    Failed,
    /// Slashed
    Slashed,
}

/// Staking manager
pub struct StakeManager {
    /// Active stakes
    active_stakes: HashMap<String, StakeTransaction>,
    /// Unstaking stakes
    unstaking_stakes: HashMap<String, StakeTransaction>,
    /// Unstaked stakes
    unstaked_stakes: HashMap<String, StakeTransaction>,
    /// Failed stakes
    failed_stakes: HashMap<String, StakeTransaction>,
    /// Transaction counter
    tx_counter: u64,
    /// Minimum stake amount
    min_stake: u64,
    /// Maximum stake amount
    max_stake: u64,
    /// Default lock period (30 days)
    default_lock_period: u64,
    /// Database for persistence
    db: sled::Db,
}

impl StakeManager {
    /// Create a new staking manager
    pub fn new() -> Result<Self> {
        Self::new_with_db_path(None)
    }

    /// Create a new staking manager with custom database path (for testing)
    pub fn new_with_db_path(custom_path: Option<PathBuf>) -> Result<Self> {
        let db_path = if let Some(path) = custom_path {
            path
        } else {
            Self::get_db_path()?
        };
        
        let db = sled::open(&db_path)
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to open stake database: {}", e)))?;
        
        Ok(Self {
            active_stakes: HashMap::new(),
            unstaking_stakes: HashMap::new(),
            unstaked_stakes: HashMap::new(),
            failed_stakes: HashMap::new(),
            tx_counter: 0,
            min_stake: 10_000_000, // 10 IPN
            max_stake: 100_000_000, // 100 IPN
            default_lock_period: 30 * 24 * 60 * 60, // 30 days
            db,
        })
    }

    /// Get database path
    fn get_db_path() -> Result<PathBuf> {
        let mut path = dirs::data_dir()
            .ok_or_else(|| crate::error::IppanError::Storage("Could not determine data directory".to_string()))?;
        path.push("ippan");
        path.push("wallet");
        path.push("stakes");
        
        // Create directory if it doesn't exist
        std::fs::create_dir_all(&path)
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to create stake database directory: {}", e)))?;
        
        Ok(path)
    }

    /// Initialize the staking manager
    pub async fn initialize(&mut self) -> Result<()> {
        // Load existing stakes from storage
        self.load_stakes().await?;
        Ok(())
    }

    /// Shutdown the staking manager
    pub async fn shutdown(&mut self) -> Result<()> {
        // Save stakes to storage
        self.save_stakes().await?;
        Ok(())
    }

    /// Create a new stake transaction
    pub async fn create_stake(
        &mut self,
        staker: String,
        validator: String,
        amount: u64,
        lock_period: Option<u64>,
    ) -> Result<StakeTransaction> {
        // Validate addresses
        if validate_ippan_address(&staker).is_err() {
            return Err(crate::error::IppanError::Validation(
                format!("Invalid staker address: {}", staker)
            ));
        }
        
        if validate_ippan_address(&validator).is_err() {
            return Err(crate::error::IppanError::Validation(
                format!("Invalid validator address: {}", validator)
            ));
        }
        
        // Validate amount
        if amount < self.min_stake {
            return Err(crate::error::IppanError::Validation(
                format!("Stake amount must be at least: {}", self.min_stake)
            ));
        }
        
        if amount > self.max_stake {
            return Err(crate::error::IppanError::Validation(
                format!("Stake amount cannot exceed: {}", self.max_stake)
            ));
        }
        
        // Generate transaction ID
        self.tx_counter += 1;
        let tx_id = format!("stake_{:016x}", self.tx_counter);
        
        // Create transaction hash
        let hash_data = format!("{}:{}:{}:{}", staker, validator, amount, Utc::now().timestamp());
        let hash = crate::utils::crypto::sha256_hash(hash_data.as_bytes());
        let hash_string = hex::encode(hash);
        
        let lock_period_seconds = lock_period.unwrap_or(self.default_lock_period);
        let unlock_timestamp = Utc::now() + chrono::Duration::seconds(lock_period_seconds as i64);
        
        let transaction = StakeTransaction {
            tx_id: tx_id.clone(),
            staker,
            validator,
            amount,
            timestamp: Utc::now(),
            status: StakeStatus::Pending,
            signature: None,
            hash: hash_string,
            lock_period: lock_period_seconds,
            unlock_timestamp: Some(unlock_timestamp),
        };
        
        self.active_stakes.insert(tx_id.clone(), transaction.clone());
        
        Ok(transaction)
    }

    /// Confirm a stake transaction
    pub async fn confirm_stake(&mut self, tx_id: &str) -> Result<()> {
        if let Some(transaction) = self.active_stakes.get_mut(tx_id) {
            transaction.status = StakeStatus::Active;
            Ok(())
        } else {
            Err(crate::error::IppanError::Validation(
                format!("Stake transaction not found: {}", tx_id)
            ))
        }
    }

    /// Initiate unstaking
    pub async fn initiate_unstaking(&mut self, tx_id: &str) -> Result<()> {
        if let Some(transaction) = self.active_stakes.remove(tx_id) {
            let mut unstaking_tx = transaction;
            unstaking_tx.status = StakeStatus::Unstaking;
            self.unstaking_stakes.insert(tx_id.to_string(), unstaking_tx);
            Ok(())
        } else {
            Err(crate::error::IppanError::Validation(
                format!("Stake transaction not found: {}", tx_id)
            ))
        }
    }

    /// Complete unstaking
    pub async fn complete_unstaking(&mut self, tx_id: &str) -> Result<()> {
        if let Some(transaction) = self.unstaking_stakes.remove(tx_id) {
            let mut unstaked_tx = transaction;
            unstaked_tx.status = StakeStatus::Unstaked;
            self.unstaked_stakes.insert(tx_id.to_string(), unstaked_tx);
            Ok(())
        } else {
            Err(crate::error::IppanError::Validation(
                format!("Unstaking transaction not found: {}", tx_id)
            ))
        }
    }

    /// Slash a stake
    pub async fn slash_stake(&mut self, tx_id: &str, reason: String) -> Result<()> {
        if let Some(transaction) = self.active_stakes.remove(tx_id) {
            let mut slashed_tx = transaction;
            slashed_tx.status = StakeStatus::Slashed;
            self.failed_stakes.insert(tx_id.to_string(), slashed_tx);
            Ok(())
        } else {
            Err(crate::error::IppanError::Validation(
                format!("Stake transaction not found: {}", tx_id)
            ))
        }
    }

    /// Fail a stake transaction
    pub async fn fail_stake(&mut self, tx_id: &str, reason: String) -> Result<()> {
        if let Some(transaction) = self.active_stakes.remove(tx_id) {
            let mut failed_tx = transaction;
            failed_tx.status = StakeStatus::Failed;
            self.failed_stakes.insert(tx_id.to_string(), failed_tx);
            Ok(())
        } else {
            Err(crate::error::IppanError::Validation(
                format!("Stake transaction not found: {}", tx_id)
            ))
        }
    }

    /// Get a stake transaction by ID
    pub fn get_stake_transaction(&self, tx_id: &str) -> Option<&StakeTransaction> {
        self.active_stakes.get(tx_id)
            .or_else(|| self.unstaking_stakes.get(tx_id))
            .or_else(|| self.unstaked_stakes.get(tx_id))
            .or_else(|| self.failed_stakes.get(tx_id))
    }

    /// Get active stakes
    pub fn get_active_stakes(&self) -> Vec<&StakeTransaction> {
        self.active_stakes.values().collect()
    }

    /// Get unstaking stakes
    pub fn get_unstaking_stakes(&self) -> Vec<&StakeTransaction> {
        self.unstaking_stakes.values().collect()
    }

    /// Get unstaked stakes
    pub fn get_unstaked_stakes(&self) -> Vec<&StakeTransaction> {
        self.unstaked_stakes.values().collect()
    }

    /// Get failed stakes
    pub fn get_failed_stakes(&self) -> Vec<&StakeTransaction> {
        self.failed_stakes.values().collect()
    }

    /// Get stakes for a staker
    pub fn get_stakes_for_staker(&self, staker: &str) -> Vec<&StakeTransaction> {
        let mut stakes = Vec::new();
        
        // Check active stakes
        for stake in self.active_stakes.values() {
            if stake.staker == staker {
                stakes.push(stake);
            }
        }
        
        // Check unstaking stakes
        for stake in self.unstaking_stakes.values() {
            if stake.staker == staker {
                stakes.push(stake);
            }
        }
        
        // Check unstaked stakes
        for stake in self.unstaked_stakes.values() {
            if stake.staker == staker {
                stakes.push(stake);
            }
        }
        
        // Check failed stakes
        for stake in self.failed_stakes.values() {
            if stake.staker == staker {
                stakes.push(stake);
            }
        }
        
        stakes
    }

    /// Get stakes for a validator
    pub fn get_stakes_for_validator(&self, validator: &str) -> Vec<&StakeTransaction> {
        let mut stakes = Vec::new();
        
        // Check active stakes
        for stake in self.active_stakes.values() {
            if stake.validator == validator {
                stakes.push(stake);
            }
        }
        
        // Check unstaking stakes
        for stake in self.unstaking_stakes.values() {
            if stake.validator == validator {
                stakes.push(stake);
            }
        }
        
        stakes
    }

    /// Get total staked amount for a validator
    pub fn get_total_staked_for_validator(&self, validator: &str) -> u64 {
        let mut total = 0u64;
        
        // Sum active stakes
        for stake in self.active_stakes.values() {
            if stake.validator == validator && stake.status == StakeStatus::Active {
                total += stake.amount;
            }
        }
        
        total
    }

    /// Get total staked amount for a staker
    pub fn get_total_staked_for_staker(&self, staker: &str) -> u64 {
        let mut total = 0u64;
        
        // Sum active stakes
        for stake in self.active_stakes.values() {
            if stake.staker == staker && stake.status == StakeStatus::Active {
                total += stake.amount;
            }
        }
        
        total
    }

    /// Check if stake can be unstaked
    pub fn can_unstake(&self, tx_id: &str) -> bool {
        if let Some(stake) = self.active_stakes.get(tx_id) {
            if let Some(unlock_time) = stake.unlock_timestamp {
                return Utc::now() >= unlock_time;
            }
        }
        false
    }

    /// Get staking statistics
    pub fn get_staking_stats(&self) -> StakingStats {
        let total_active_stakes = self.active_stakes.len();
        let total_unstaking_stakes = self.unstaking_stakes.len();
        let total_unstaked_stakes = self.unstaked_stakes.len();
        let total_failed_stakes = self.failed_stakes.len();
        
        let total_active_amount: u64 = self.active_stakes.values()
            .filter(|s| s.status == StakeStatus::Active)
            .map(|s| s.amount)
            .sum();
        
        let total_unstaking_amount: u64 = self.unstaking_stakes.values()
            .map(|s| s.amount)
            .sum();
        
        let total_unstaked_amount: u64 = self.unstaked_stakes.values()
            .map(|s| s.amount)
            .sum();
        
        StakingStats {
            total_active_stakes,
            total_unstaking_stakes,
            total_unstaked_stakes,
            total_failed_stakes,
            total_active_amount,
            total_unstaking_amount,
            total_unstaked_amount,
            min_stake: self.min_stake,
            max_stake: self.max_stake,
            default_lock_period: self.default_lock_period,
        }
    }

    /// Load stakes from storage
    async fn load_stakes(&mut self) -> Result<()> {
        log::info!("Loading stakes from persistent storage...");
        
        // Load active stakes
        if let Ok(active_tree) = self.db.open_tree("active_stakes") {
            for result in active_tree.iter() {
                let (key, value) = result
                    .map_err(|e| crate::error::IppanError::Storage(format!("Failed to read active stake: {}", e)))?;
                
                let tx_id = String::from_utf8(key.to_vec())
                    .map_err(|e| crate::error::IppanError::Storage(format!("Invalid transaction ID: {}", e)))?;
                
                let stake: StakeTransaction = bincode::deserialize(&value)
                    .map_err(|e| crate::error::IppanError::Storage(format!("Failed to deserialize active stake: {}", e)))?;
                
                self.active_stakes.insert(tx_id, stake);
            }
        }
        
        // Load unstaking stakes
        if let Ok(unstaking_tree) = self.db.open_tree("unstaking_stakes") {
            for result in unstaking_tree.iter() {
                let (key, value) = result
                    .map_err(|e| crate::error::IppanError::Storage(format!("Failed to read unstaking stake: {}", e)))?;
                
                let tx_id = String::from_utf8(key.to_vec())
                    .map_err(|e| crate::error::IppanError::Storage(format!("Invalid transaction ID: {}", e)))?;
                
                let stake: StakeTransaction = bincode::deserialize(&value)
                    .map_err(|e| crate::error::IppanError::Storage(format!("Failed to deserialize unstaking stake: {}", e)))?;
                
                self.unstaking_stakes.insert(tx_id, stake);
            }
        }
        
        // Load unstaked stakes
        if let Ok(unstaked_tree) = self.db.open_tree("unstaked_stakes") {
            for result in unstaked_tree.iter() {
                let (key, value) = result
                    .map_err(|e| crate::error::IppanError::Storage(format!("Failed to read unstaked stake: {}", e)))?;
                
                let tx_id = String::from_utf8(key.to_vec())
                    .map_err(|e| crate::error::IppanError::Storage(format!("Invalid transaction ID: {}", e)))?;
                
                let stake: StakeTransaction = bincode::deserialize(&value)
                    .map_err(|e| crate::error::IppanError::Storage(format!("Failed to deserialize unstaked stake: {}", e)))?;
                
                self.unstaked_stakes.insert(tx_id, stake);
            }
        }
        
        // Load failed stakes
        if let Ok(failed_tree) = self.db.open_tree("failed_stakes") {
            for result in failed_tree.iter() {
                let (key, value) = result
                    .map_err(|e| crate::error::IppanError::Storage(format!("Failed to read failed stake: {}", e)))?;
                
                let tx_id = String::from_utf8(key.to_vec())
                    .map_err(|e| crate::error::IppanError::Storage(format!("Invalid transaction ID: {}", e)))?;
                
                let stake: StakeTransaction = bincode::deserialize(&value)
                    .map_err(|e| crate::error::IppanError::Storage(format!("Failed to deserialize failed stake: {}", e)))?;
                
                self.failed_stakes.insert(tx_id, stake);
            }
        }
        
        // Load transaction counter
        if let Ok(Some(counter_value)) = self.db.get("tx_counter") {
            self.tx_counter = bincode::deserialize(&counter_value)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to deserialize transaction counter: {}", e)))?;
        }
        
        log::info!("Loaded {} active, {} unstaking, {} unstaked, {} failed stakes", 
            self.active_stakes.len(), self.unstaking_stakes.len(), 
            self.unstaked_stakes.len(), self.failed_stakes.len());
        
        Ok(())
    }

    /// Save stakes to storage
    async fn save_stakes(&self) -> Result<()> {
        log::info!("Saving stakes to persistent storage...");
        
        // Save active stakes
        let active_tree = self.db.open_tree("active_stakes")
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to open active stakes tree: {}", e)))?;
        
        for (tx_id, stake) in &self.active_stakes {
            let key = tx_id.as_bytes();
            let value = bincode::serialize(stake)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to serialize active stake: {}", e)))?;
            active_tree.insert(key, value)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to save active stake: {}", e)))?;
        }
        
        // Save unstaking stakes
        let unstaking_tree = self.db.open_tree("unstaking_stakes")
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to open unstaking stakes tree: {}", e)))?;
        
        for (tx_id, stake) in &self.unstaking_stakes {
            let key = tx_id.as_bytes();
            let value = bincode::serialize(stake)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to serialize unstaking stake: {}", e)))?;
            unstaking_tree.insert(key, value)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to save unstaking stake: {}", e)))?;
        }
        
        // Save unstaked stakes
        let unstaked_tree = self.db.open_tree("unstaked_stakes")
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to open unstaked stakes tree: {}", e)))?;
        
        for (tx_id, stake) in &self.unstaked_stakes {
            let key = tx_id.as_bytes();
            let value = bincode::serialize(stake)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to serialize unstaked stake: {}", e)))?;
            unstaked_tree.insert(key, value)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to save unstaked stake: {}", e)))?;
        }
        
        // Save failed stakes
        let failed_tree = self.db.open_tree("failed_stakes")
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to open failed stakes tree: {}", e)))?;
        
        for (tx_id, stake) in &self.failed_stakes {
            let key = tx_id.as_bytes();
            let value = bincode::serialize(stake)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to serialize failed stake: {}", e)))?;
            failed_tree.insert(key, value)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to save failed stake: {}", e)))?;
        }
        
        // Save transaction counter
        let counter_value = bincode::serialize(&self.tx_counter)
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to serialize transaction counter: {}", e)))?;
        self.db.insert("tx_counter", counter_value)
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to save transaction counter: {}", e)))?;
        
        // Flush database to ensure data is written to disk
        self.db.flush()
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to flush database: {}", e)))?;
        
        log::info!("Saved {} active, {} unstaking, {} unstaked, {} failed stakes", 
            self.active_stakes.len(), self.unstaking_stakes.len(), 
            self.unstaked_stakes.len(), self.failed_stakes.len());
        
        Ok(())
    }
}

/// Staking statistics
#[derive(Debug, Serialize)]
pub struct StakingStats {
    pub total_active_stakes: usize,
    pub total_unstaking_stakes: usize,
    pub total_unstaked_stakes: usize,
    pub total_failed_stakes: usize,
    pub total_active_amount: u64,
    pub total_unstaking_amount: u64,
    pub total_unstaked_amount: u64,
    pub min_stake: u64,
    pub max_stake: u64,
    pub default_lock_period: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
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
    async fn test_stake_manager_creation() {
        let test_db_path = std::env::temp_dir().join(format!("test_stakes_{}", rand::random::<u64>()));
        let mut manager = StakeManager::new_with_db_path(Some(test_db_path)).unwrap();
        manager.initialize().await.unwrap();
        
        assert_eq!(manager.min_stake, 10_000_000);
        assert_eq!(manager.max_stake, 100_000_000);
    }

    #[tokio::test]
    async fn test_create_stake() {
        let test_db_path = std::env::temp_dir().join(format!("test_stakes_{}", rand::random::<u64>()));
        let mut manager = StakeManager::new_with_db_path(Some(test_db_path)).unwrap();
        manager.initialize().await.unwrap();
        
        let (staker_addr, validator_addr) = generate_test_addresses();
        
        let stake = manager.create_stake(
            staker_addr,
            validator_addr,
            50_000_000, // 50 IPN (within max limit)
            Some(30 * 24 * 60 * 60), // 30 days lock period
        ).await.unwrap();
        
        assert_eq!(stake.amount, 50_000_000);
        assert_eq!(stake.status, StakeStatus::Pending);
        assert!(stake.signature.is_none());
    }

    #[tokio::test]
    async fn test_confirm_stake() {
        let test_db_path = std::env::temp_dir().join(format!("test_stakes_{}", rand::random::<u64>()));
        let mut manager = StakeManager::new_with_db_path(Some(test_db_path)).unwrap();
        manager.initialize().await.unwrap();
        
        let (staker_addr, validator_addr) = generate_test_addresses();
        
        let stake = manager.create_stake(
            staker_addr,
            validator_addr,
            50_000_000, // 50 IPN (within max limit)
            None,
        ).await.unwrap();
        
        manager.confirm_stake(&stake.tx_id).await.unwrap();
        
        let confirmed_tx = manager.get_stake_transaction(&stake.tx_id).unwrap();
        assert_eq!(confirmed_tx.status, StakeStatus::Active);
    }

    #[tokio::test]
    async fn test_unstaking() {
        let test_db_path = std::env::temp_dir().join(format!("test_stakes_{}", rand::random::<u64>()));
        let mut manager = StakeManager::new_with_db_path(Some(test_db_path)).unwrap();
        manager.initialize().await.unwrap();
        
        let (staker_addr, validator_addr) = generate_test_addresses();
        
        let stake = manager.create_stake(
            staker_addr,
            validator_addr,
            50_000_000, // 50 IPN (within max limit)
            None,
        ).await.unwrap();
        
        manager.confirm_stake(&stake.tx_id).await.unwrap();
        
        manager.initiate_unstaking(&stake.tx_id).await.unwrap();
        
        let unstaking_stake = manager.get_stake_transaction(&stake.tx_id).unwrap();
        assert_eq!(unstaking_stake.status, StakeStatus::Unstaking);
    }

    #[tokio::test]
    async fn test_staking_stats() {
        let test_db_path = std::env::temp_dir().join(format!("test_stakes_{}", rand::random::<u64>()));
        let mut manager = StakeManager::new_with_db_path(Some(test_db_path)).unwrap();
        manager.initialize().await.unwrap();
        
        let (staker_addr, validator_addr) = generate_test_addresses();
        
        let stake = manager.create_stake(
            staker_addr,
            validator_addr,
            50_000_000, // 50 IPN (within max limit)
            None,
        ).await.unwrap();
        
        manager.confirm_stake(&stake.tx_id).await.unwrap();
        
        let stats = manager.get_staking_stats();
        assert_eq!(stats.total_active_stakes, 1);
        assert_eq!(stats.total_active_amount, 50_000_000);
    }

    #[tokio::test]
    async fn test_insufficient_stake() {
        let test_db_path = std::env::temp_dir().join(format!("test_stakes_{}", rand::random::<u64>()));
        let mut manager = StakeManager::new_with_db_path(Some(test_db_path)).unwrap();
        manager.initialize().await.unwrap();
        
        let (staker_addr, validator_addr) = generate_test_addresses();
        
        let result = manager.create_stake(
            staker_addr,
            validator_addr,
            5_000_000, // Below minimum stake (5 IPN < 10 IPN minimum)
            None,
        ).await;
        
        assert!(result.is_err());
    }
}

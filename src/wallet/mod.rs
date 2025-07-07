//! Wallet module for IPPAN
//! 
//! Handles Ed25519 keys, payments, and staking operations

pub mod ed25519;
pub mod payments;
pub mod stake;

pub use ed25519::Ed25519Wallet;
pub use payments::PaymentWallet;
pub use stake::StakingWallet;

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{
    error::{IppanError, Result},
    NodeId,
    utils::time::current_time_secs,
};

/// Main wallet that combines all wallet functionality
pub struct Wallet {
    /// Ed25519 key management
    ed25519: Ed25519Wallet,
    /// Payment operations
    payments: PaymentWallet,
    /// Staking operations
    staking: StakingWallet,
    /// Wallet configuration
    config: WalletConfig,
}

/// Wallet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    /// Wallet file path
    pub wallet_path: String,
    /// Whether to encrypt wallet file
    pub encrypt_wallet: bool,
    /// Minimum transaction fee
    pub min_fee: u64,
    /// Default transaction fee
    pub default_fee: u64,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            wallet_path: "./wallet.dat".to_string(),
            encrypt_wallet: true,
            min_fee: 1000, // 0.00001 IPN
            default_fee: 10000, // 0.0001 IPN
        }
    }
}

/// Wallet balance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    /// Available balance (not staked)
    pub available: u64,
    /// Staked balance
    pub staked: u64,
    /// Total balance
    pub total: u64,
    /// Pending rewards
    pub pending_rewards: u64,
}

/// Transaction information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction hash
    pub hash: [u8; 32],
    /// Transaction type
    pub tx_type: TransactionType,
    /// Amount (for payments)
    pub amount: Option<u64>,
    /// Fee
    pub fee: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Status
    pub status: TransactionStatus,
    /// Block height (if confirmed)
    pub block_height: Option<u64>,
}

/// Transaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    /// Payment transaction
    Payment {
        from: [u8; 32],
        to: [u8; 32],
    },
    /// Staking transaction
    Stake {
        amount: u64,
        validator_id: [u8; 32],
    },
    /// Unstaking transaction
    Unstake {
        amount: u64,
        validator_id: [u8; 32],
    },
    /// Reward claim
    RewardClaim {
        amount: u64,
    },
}

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Pending
    Pending,
    /// Confirmed
    Confirmed,
    /// Failed
    Failed,
    /// Rejected
    Rejected,
}

impl Wallet {
    /// Create a new wallet
    pub async fn new(config: WalletConfig) -> Result<Self> {
        let ed25519 = Ed25519Wallet::new(&config.wallet_path, config.encrypt_wallet).await?;
        let payments = PaymentWallet::new(&ed25519);
        let staking = StakingWallet::new(&ed25519);

        Ok(Self {
            ed25519,
            payments,
            staking,
            config,
        })
    }

    /// Get wallet balance
    pub async fn get_balance(&self) -> Result<WalletBalance> {
        let available = self.payments.get_available_balance().await?;
        let staked = self.staking.get_staked_balance().await?;
        let pending_rewards = self.staking.get_pending_rewards().await?;
        let total = available + staked;

        Ok(WalletBalance {
            available,
            staked,
            total,
            pending_rewards,
        })
    }

    /// Send a payment
    pub async fn send_payment(&self, to: [u8; 32], amount: u64, fee: Option<u64>) -> Result<[u8; 32]> {
        let fee = fee.unwrap_or(self.config.default_fee);
        if fee < self.config.min_fee {
            return Err(crate::IppanError::Wallet("Fee too low".to_string()));
        }

        self.payments.send_payment(to, amount, fee).await
    }

    /// Stake tokens
    pub async fn stake(&self, amount: u64, validator_id: [u8; 32]) -> Result<[u8; 32]> {
        self.staking.stake(amount, validator_id).await
    }

    /// Unstake tokens
    pub async fn unstake(&self, amount: u64, validator_id: [u8; 32]) -> Result<[u8; 32]> {
        self.staking.unstake(amount, validator_id).await
    }

    /// Claim rewards
    pub async fn claim_rewards(&self) -> Result<[u8; 32]> {
        self.staking.claim_rewards().await
    }

    /// Get transaction history
    pub async fn get_transactions(&self, limit: Option<usize>) -> Result<Vec<Transaction>> {
        let mut transactions = Vec::new();
        
        // Get payment transactions
        let payment_txs = self.payments.get_transactions(limit).await?;
        transactions.extend(payment_txs);
        
        // Get staking transactions
        let staking_txs = self.staking.get_transactions(limit).await?;
        transactions.extend(staking_txs);
        
        // Sort by timestamp (newest first)
        transactions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        // Apply limit if specified
        if let Some(limit) = limit {
            transactions.truncate(limit);
        }
        
        Ok(transactions)
    }

    /// Get public key
    pub fn get_public_key(&self) -> [u8; 32] {
        self.ed25519.get_public_key()
    }

    /// Get node ID (derived from public key)
    pub fn get_node_id(&self) -> [u8; 32] {
        self.ed25519.get_node_id()
    }

    /// Sign data
    pub async fn sign(&self, data: &[u8]) -> Result<[u8; 64]> {
        self.ed25519.sign(data).await
    }

    /// Verify signature
    pub fn verify(&self, data: &[u8], signature: &[u8; 64]) -> Result<bool> {
        self.ed25519.verify(data, signature)
    }

    /// Export wallet (for backup)
    pub async fn export(&self) -> Result<Vec<u8>> {
        self.ed25519.export().await
    }

    /// Import wallet
    pub async fn import(&mut self, data: &[u8]) -> Result<()> {
        self.ed25519.import(data).await
    }

    /// Backup wallet
    pub async fn backup(&self, backup_path: &str) -> Result<()> {
        let data = self.export().await?;
        std::fs::write(backup_path, data)
            .map_err(|e| crate::IppanError::Wallet(format!("Backup failed: {}", e)))?;
        Ok(())
    }

    /// Restore wallet from backup
    pub async fn restore(&mut self, backup_path: &str) -> Result<()> {
        let data = std::fs::read(backup_path)
            .map_err(|e| crate::IppanError::Wallet(format!("Restore failed: {}", e)))?;
        self.import(&data).await
    }

    /// Get staking information
    pub async fn get_staking_info(&self) -> Result<StakingInfo> {
        self.staking.get_staking_info().await
    }

    /// Get payment addresses
    pub async fn get_addresses(&self) -> Result<Vec<[u8; 32]>> {
        self.payments.get_addresses().await
    }

    /// Generate new address
    pub async fn generate_address(&self) -> Result<[u8; 32]> {
        self.payments.generate_address().await
    }
}

/// Staking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingInfo {
    /// Total staked amount
    pub total_staked: u64,
    /// Number of active stakes
    pub active_stakes: u64,
    /// Pending rewards
    pub pending_rewards: u64,
    /// Total rewards earned
    pub total_rewards_earned: u64,
    /// Validator information
    pub validators: Vec<ValidatorStake>,
}

/// Validator stake information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorStake {
    /// Validator ID
    pub validator_id: [u8; 32],
    /// Staked amount
    pub staked_amount: u64,
    /// Stake start time
    pub start_time: u64,
    /// Rewards earned
    pub rewards_earned: u64,
    /// Stake status
    pub status: StakeStatus,
}

/// Stake status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StakeStatus {
    /// Active stake
    Active,
    /// Unstaking (waiting period)
    Unstaking,
    /// Unstaked (ready to withdraw)
    Unstaked,
}

/// Wallet manager (stub implementation)
pub struct WalletManager {
    config: WalletConfig,
}

impl WalletManager {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            config: WalletConfig::default(),
        })
    }
    
    pub async fn start(&self) -> Result<()> {
        Ok(())
    }
    
    pub async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wallet_creation() {
        let config = WalletConfig::default();
        let wallet = Wallet::new(config).await.unwrap();
        
        let public_key = wallet.get_public_key();
        assert_ne!(public_key, [0u8; 32]);
        
        let node_id = wallet.get_node_id();
        assert_ne!(node_id, [0u8; 32]);
    }

    #[tokio::test]
    async fn test_wallet_balance() {
        let config = WalletConfig::default();
        let wallet = Wallet::new(config).await.unwrap();
        
        let balance = wallet.get_balance().await.unwrap();
        assert_eq!(balance.available, 0);
        assert_eq!(balance.staked, 0);
        assert_eq!(balance.total, 0);
    }

    #[tokio::test]
    async fn test_wallet_signing() {
        let config = WalletConfig::default();
        let wallet = Wallet::new(config).await.unwrap();
        
        let data = b"Test data for signing";
        let signature = wallet.sign(data).await.unwrap();
        
        let is_valid = wallet.verify(data, &signature).unwrap();
        assert!(is_valid);
    }
}

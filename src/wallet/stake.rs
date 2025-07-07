use crate::{Result, TransactionHash, MIN_STAKE_AMOUNT, MAX_STAKE_AMOUNT};
use super::ed25519::Ed25519Wallet;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use super::payments::SignatureWrapper;

/// Staking wallet for validator operations
pub struct StakingWallet {
    /// Ed25519 wallet for signing
    ed25519: Ed25519Wallet,
    /// Staked balance
    staked_balance: RwLock<u64>,
    /// Pending rewards
    pending_rewards: RwLock<u64>,
    /// Active stakes
    active_stakes: RwLock<HashMap<[u8; 32], StakeInfo>>,
    /// Staking transactions
    transactions: RwLock<Vec<StakingTransaction>>,
}

/// Stake information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeInfo {
    /// Validator ID
    pub validator_id: [u8; 32],
    /// Staked amount
    pub amount: u64,
    /// Stake start time
    pub start_time: u64,
    /// Rewards earned
    pub rewards_earned: u64,
    /// Stake status
    pub status: StakeStatus,
    /// Unstaking start time (if unstaking)
    pub unstaking_start: Option<u64>,
    /// Unstaking period in seconds
    pub unstaking_period: u64,
}

/// Staking transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingTransaction {
    /// Transaction hash
    pub hash: TransactionHash,
    /// Transaction type
    pub tx_type: StakingType,
    /// Amount
    pub amount: u64,
    /// Validator ID
    pub validator_id: [u8; 32],
    /// Timestamp
    pub timestamp: u64,
    /// Status
    pub status: TransactionStatus,
    /// Signature
    pub signature: SignatureWrapper,
    /// Block height (if confirmed)
    pub block_height: Option<u64>,
}

/// Staking transaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StakingType {
    /// Stake tokens
    Stake,
    /// Unstake tokens
    Unstake,
    /// Claim rewards
    ClaimRewards,
    /// Slash stake (penalty)
    Slash,
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
    /// Slashed (penalized)
    Slashed,
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

/// Stake transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeTransaction {
    /// Transaction hash
    pub hash: [u8; 32],
    /// Staker address
    pub staker: [u8; 32],
    /// Stake amount in smallest units
    pub amount: u64,
    /// Transaction fee
    pub fee: u64,
    /// Transaction timestamp
    pub timestamp: u64,
    /// Nonce
    pub nonce: u64,
    /// Signature
    pub signature: SignatureWrapper,
}

/// Stake manager
pub struct StakeManager {
    /// Ed25519 wallet
    ed25519: Ed25519Wallet,
    /// Stake transactions
    transactions: HashMap<[u8; 32], StakeTransaction>,
    /// Current stake amounts by address
    stake_amounts: HashMap<[u8; 32], u64>,
}

impl StakingWallet {
    /// Create a new staking wallet
    pub fn new(ed25519: &Ed25519Wallet) -> Self {
        Self {
            ed25519: ed25519.clone(),
            staked_balance: RwLock::new(0),
            pending_rewards: RwLock::new(0),
            active_stakes: RwLock::new(HashMap::new()),
            transactions: RwLock::new(Vec::new()),
        }
    }

    /// Get staked balance
    pub async fn get_staked_balance(&self) -> Result<u64> {
        Ok(*self.staked_balance.read().await)
    }

    /// Get pending rewards
    pub async fn get_pending_rewards(&self) -> Result<u64> {
        Ok(*self.pending_rewards.read().await)
    }

    /// Stake tokens with a validator
    pub async fn stake(&self, amount: u64, validator_id: [u8; 32]) -> Result<TransactionHash> {
        // Validate stake amount
        if amount < MIN_STAKE_AMOUNT {
            return Err(crate::IppanError::Wallet(format!(
                "Stake amount too low. Minimum: {}, Got: {}", 
                MIN_STAKE_AMOUNT, amount
            )));
        }

        if amount > MAX_STAKE_AMOUNT {
            return Err(crate::IppanError::Wallet(format!(
                "Stake amount too high. Maximum: {}, Got: {}", 
                MAX_STAKE_AMOUNT, amount
            )));
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create transaction data
        let tx_data = self.create_staking_data(StakingType::Stake, amount, validator_id, timestamp);
        let signature = self.ed25519.sign(&tx_data).await?;
        let hash = self.calculate_transaction_hash(&tx_data, &signature);

        let transaction = StakingTransaction {
            hash,
            tx_type: StakingType::Stake,
            amount,
            validator_id,
            timestamp,
            status: TransactionStatus::Pending,
            signature,
            block_height: None,
        };

        // Update staked balance
        {
            let mut staked_balance = self.staked_balance.write().await;
            *staked_balance += amount;
        }

        // Add or update stake info
        {
            let mut active_stakes = self.active_stakes.write().await;
            let stake_info = active_stakes.entry(validator_id).or_insert_with(|| StakeInfo {
                validator_id,
                amount: 0,
                start_time: timestamp,
                rewards_earned: 0,
                status: StakeStatus::Active,
                unstaking_start: None,
                unstaking_period: 7 * 24 * 60 * 60, // 7 days
            });

            stake_info.amount += amount;
            stake_info.status = StakeStatus::Active;
        }

        // Add to transaction history
        {
            let mut transactions = self.transactions.write().await;
            transactions.push(transaction);
        }

        Ok(hash)
    }

    /// Unstake tokens
    pub async fn unstake(&self, amount: u64, validator_id: [u8; 32]) -> Result<TransactionHash> {
        let active_stakes = self.active_stakes.read().await;
        let stake_info = active_stakes.get(&validator_id)
            .ok_or_else(|| crate::IppanError::Wallet("No active stake for this validator".to_string()))?;

        if stake_info.amount < amount {
            return Err(crate::IppanError::Wallet("Insufficient staked amount".to_string()));
        }

        if matches!(stake_info.status, StakeStatus::Unstaking | StakeStatus::Unstaked) {
            return Err(crate::IppanError::Wallet("Stake is already being unstaked".to_string()));
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create transaction data
        let tx_data = self.create_staking_data(StakingType::Unstake, amount, validator_id, timestamp);
        let signature = self.ed25519.sign(&tx_data).await?;
        let hash = self.calculate_transaction_hash(&tx_data, &signature);

        let transaction = StakingTransaction {
            hash,
            tx_type: StakingType::Unstake,
            amount,
            validator_id,
            timestamp,
            status: TransactionStatus::Pending,
            signature,
            block_height: None,
        };

        // Update stake info
        {
            let mut active_stakes = self.active_stakes.write().await;
            if let Some(stake_info) = active_stakes.get_mut(&validator_id) {
                stake_info.amount = stake_info.amount.saturating_sub(amount);
                stake_info.status = StakeStatus::Unstaking;
                stake_info.unstaking_start = Some(timestamp);
            }
        }

        // Add to transaction history
        {
            let mut transactions = self.transactions.write().await;
            transactions.push(transaction);
        }

        Ok(hash)
    }

    /// Claim rewards
    pub async fn claim_rewards(&self) -> Result<TransactionHash> {
        let pending_rewards = *self.pending_rewards.read().await;
        if pending_rewards == 0 {
            return Err(crate::IppanError::Wallet("No pending rewards to claim".to_string()));
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create transaction data
        let tx_data = self.create_staking_data(StakingType::ClaimRewards, pending_rewards, [0; 32], timestamp);
        let signature = self.ed25519.sign(&tx_data).await?;
        let hash = self.calculate_transaction_hash(&tx_data, &signature);

        let transaction = StakingTransaction {
            hash,
            tx_type: StakingType::ClaimRewards,
            amount: pending_rewards,
            validator_id: [0; 32],
            timestamp,
            status: TransactionStatus::Pending,
            signature,
            block_height: None,
        };

        // Reset pending rewards
        {
            let mut pending_rewards = self.pending_rewards.write().await;
            *pending_rewards = 0;
        }

        // Add to transaction history
        {
            let mut transactions = self.transactions.write().await;
            transactions.push(transaction);
        }

        Ok(hash)
    }

    /// Add rewards (called by the Global Fund)
    pub async fn add_rewards(&self, amount: u64) -> Result<()> {
        let mut pending_rewards = self.pending_rewards.write().await;
        *pending_rewards += amount;
        Ok(())
    }

    /// Get staking information
    pub async fn get_staking_info(&self) -> Result<super::StakingInfo> {
        let active_stakes = self.active_stakes.read().await;
        let staked_balance = *self.staked_balance.read().await;
        let pending_rewards = *self.pending_rewards.read().await;

        let mut total_rewards_earned = 0;
        let mut validators = Vec::new();

        for stake_info in active_stakes.values() {
            total_rewards_earned += stake_info.rewards_earned;
            validators.push(super::ValidatorStake {
                validator_id: stake_info.validator_id,
                staked_amount: stake_info.amount,
                start_time: stake_info.start_time,
                rewards_earned: stake_info.rewards_earned,
                status: match stake_info.status {
                    StakeStatus::Active => super::StakeStatus::Active,
                    StakeStatus::Unstaking => super::StakeStatus::Unstaking,
                    StakeStatus::Unstaked => super::StakeStatus::Unstaked,
                    StakeStatus::Slashed => super::StakeStatus::Active, // Map to active for compatibility
                },
            });
        }

        Ok(super::StakingInfo {
            total_staked: staked_balance,
            active_stakes: active_stakes.len() as u64,
            pending_rewards,
            total_rewards_earned,
            validators,
        })
    }

    /// Get transaction history
    pub async fn get_transactions(&self, limit: Option<usize>) -> Result<Vec<super::Transaction>> {
        let transactions = self.transactions.read().await;
        let mut result = Vec::new();

        for tx in transactions.iter() {
            let tx_type = match tx.tx_type {
                StakingType::Stake => super::TransactionType::Stake {
                    amount: tx.amount,
                    validator_id: tx.validator_id,
                },
                StakingType::Unstake => super::TransactionType::Unstake {
                    amount: tx.amount,
                    validator_id: tx.validator_id,
                },
                StakingType::ClaimRewards => super::TransactionType::RewardClaim {
                    amount: tx.amount,
                },
                StakingType::Slash => super::TransactionType::Stake {
                    amount: tx.amount,
                    validator_id: tx.validator_id,
                },
            };

            result.push(super::Transaction {
                hash: tx.hash,
                tx_type,
                amount: Some(tx.amount),
                fee: 0, // Staking transactions don't have fees
                timestamp: tx.timestamp,
                status: match tx.status {
                    TransactionStatus::Pending => super::TransactionStatus::Pending,
                    TransactionStatus::Confirmed => super::TransactionStatus::Confirmed,
                    TransactionStatus::Failed => super::TransactionStatus::Failed,
                    TransactionStatus::Rejected => super::TransactionStatus::Rejected,
                },
                block_height: tx.block_height,
            });
        }

        // Sort by timestamp (newest first)
        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit if specified
        if let Some(limit) = limit {
            result.truncate(limit);
        }

        Ok(result)
    }

    /// Check if stake can be withdrawn (unstaking period completed)
    pub async fn can_withdraw(&self, validator_id: [u8; 32]) -> Result<bool> {
        let active_stakes = self.active_stakes.read().await;
        if let Some(stake_info) = active_stakes.get(&validator_id) {
            if let Some(unstaking_start) = stake_info.unstaking_start {
                let current_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                return Ok(current_time >= unstaking_start + stake_info.unstaking_period);
            }
        }
        Ok(false)
    }

    /// Withdraw unstaked tokens
    pub async fn withdraw(&self, validator_id: [u8; 32]) -> Result<u64> {
        if !self.can_withdraw(validator_id).await? {
            return Err(crate::IppanError::Wallet("Unstaking period not completed".to_string()));
        }

        let mut active_stakes = self.active_stakes.write().await;
        if let Some(stake_info) = active_stakes.get_mut(&validator_id) {
            if matches!(stake_info.status, StakeStatus::Unstaking) {
                let amount = stake_info.amount;
                stake_info.amount = 0;
                stake_info.status = StakeStatus::Unstaked;

                // Update staked balance
                {
                    let mut staked_balance = self.staked_balance.write().await;
                    *staked_balance = staked_balance.saturating_sub(amount);
                }

                Ok(amount)
            } else {
                Err(crate::IppanError::Wallet("Stake is not in unstaking status".to_string()))
            }
        } else {
            Err(crate::IppanError::Wallet("No stake found for this validator".to_string()))
        }
    }

    /// Create staking transaction data for signing
    fn create_staking_data(&self, tx_type: StakingType, amount: u64, validator_id: [u8; 32], timestamp: u64) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&(tx_type as u8).to_le_bytes());
        data.extend_from_slice(&amount.to_le_bytes());
        data.extend_from_slice(&validator_id);
        data.extend_from_slice(&timestamp.to_le_bytes());
        data
    }

    /// Calculate transaction hash
    fn calculate_transaction_hash(&self, tx_data: &[u8], signature: &[u8; 64]) -> TransactionHash {
        let mut hasher = Sha256::new();
        hasher.update(tx_data);
        hasher.update(signature);
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::ed25519::Ed25519Wallet;

    #[tokio::test]
    async fn test_staking_wallet() {
        let ed25519 = Ed25519Wallet::new("./test_staking_wallet.dat", false).await.unwrap();
        let wallet = StakingWallet::new(&ed25519);

        let validator_id = [1u8; 32];
        let stake_amount = MIN_STAKE_AMOUNT;

        // Stake tokens
        let tx_hash = wallet.stake(stake_amount, validator_id).await.unwrap();
        assert_eq!(wallet.get_staked_balance().await.unwrap(), stake_amount);

        // Check staking info
        let staking_info = wallet.get_staking_info().await.unwrap();
        assert_eq!(staking_info.total_staked, stake_amount);
        assert_eq!(staking_info.active_stakes, 1);

        // Clean up
        let _ = std::fs::remove_file("./test_staking_wallet.dat");
    }

    #[tokio::test]
    async fn test_stake_amount_validation() {
        let ed25519 = Ed25519Wallet::new("./test_stake_validation.dat", false).await.unwrap();
        let wallet = StakingWallet::new(&ed25519);

        let validator_id = [1u8; 32];

        // Try to stake less than minimum
        let result = wallet.stake(MIN_STAKE_AMOUNT - 1, validator_id).await;
        assert!(result.is_err());

        // Try to stake more than maximum
        let result = wallet.stake(MAX_STAKE_AMOUNT + 1, validator_id).await;
        assert!(result.is_err());

        // Clean up
        let _ = std::fs::remove_file("./test_stake_validation.dat");
    }

    #[tokio::test]
    async fn test_rewards() {
        let ed25519 = Ed25519Wallet::new("./test_rewards.dat", false).await.unwrap();
        let wallet = StakingWallet::new(&ed25519);

        // Add rewards
        wallet.add_rewards(100000).await.unwrap();
        assert_eq!(wallet.get_pending_rewards().await.unwrap(), 100000);

        // Claim rewards
        let tx_hash = wallet.claim_rewards().await.unwrap();
        assert_eq!(wallet.get_pending_rewards().await.unwrap(), 0);

        // Clean up
        let _ = std::fs::remove_file("./test_rewards.dat");
    }
}

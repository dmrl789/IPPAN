//! M2M (Machine-to-Machine) payment system for IPPAN wallet

use crate::Result;
use crate::utils::address::validate_ippan_address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// Payment channel for M2M payments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentChannel {
    /// Channel ID
    pub channel_id: String,
    /// Sender address
    pub sender: String,
    /// Recipient address
    pub recipient: String,
    /// Total deposit amount
    pub deposit_amount: u64,
    /// Current balance
    pub current_balance: u64,
    /// Channel creation timestamp
    pub created_at: DateTime<Utc>,
    /// Channel expiration timestamp
    pub expires_at: DateTime<Utc>,
    /// Channel status
    pub status: ChannelStatus,
    /// Channel signature
    pub signature: Option<Vec<u8>>,
    /// Micro-transactions in this channel
    pub micro_transactions: Vec<MicroTransaction>,
}

/// Channel status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChannelStatus {
    /// Channel is open and active
    Open,
    /// Channel is closed
    Closed,
    /// Channel is expired
    Expired,
    /// Channel is disputed
    Disputed,
}

/// Micro-transaction within a payment channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroTransaction {
    /// Transaction ID
    pub tx_id: String,
    /// Amount
    pub amount: u64,
    /// Transaction type
    pub tx_type: MicroTransactionType,
    /// Transaction timestamp
    pub timestamp: DateTime<Utc>,
    /// Transaction signature
    pub signature: Option<Vec<u8>>,
    /// Transaction hash
    pub hash: String,
    /// Memo/note
    pub memo: Option<String>,
}

/// Micro-transaction type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MicroTransactionType {
    /// Payment from sender to recipient
    Payment,
    /// Fee payment
    Fee,
    /// Refund to sender
    Refund,
}

/// M2M payment system
pub struct M2MPaymentSystem {
    /// Active payment channels
    channels: HashMap<String, PaymentChannel>,
    /// Closed payment channels
    closed_channels: HashMap<String, PaymentChannel>,
    /// Transaction counter
    tx_counter: u64,
    /// Channel counter
    channel_counter: u64,
    /// M2M fee rate (1%)
    fee_rate: f64,
    /// Minimum channel deposit
    min_deposit: u64,
    /// Maximum channel deposit
    max_deposit: u64,
    /// Default channel timeout (24 hours)
    default_timeout_hours: u64,
}

impl M2MPaymentSystem {
    /// Create a new M2M payment system
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            closed_channels: HashMap::new(),
            tx_counter: 0,
            channel_counter: 0,
            fee_rate: 0.01, // 1%
            min_deposit: 100_000, // 0.1 IPN
            max_deposit: 10_000_000, // 10 IPN
            default_timeout_hours: 24,
        }
    }

    /// Create a new payment channel
    pub async fn create_payment_channel(
        &mut self,
        sender: String,
        recipient: String,
        deposit_amount: u64,
        timeout_hours: Option<u64>,
    ) -> Result<PaymentChannel> {
        // Validate addresses
        if !validate_ippan_address(&sender) {
            return Err(crate::error::IppanError::Validation(
                format!("Invalid sender address: {}", sender)
            ));
        }
        
        if !validate_ippan_address(&recipient) {
            return Err(crate::error::IppanError::Validation(
                format!("Invalid recipient address: {}", recipient)
            ));
        }
        
        // Validate deposit amount
        if deposit_amount < self.min_deposit {
            return Err(crate::error::IppanError::Validation(
                format!("Deposit amount must be at least: {}", self.min_deposit)
            ));
        }
        
        if deposit_amount > self.max_deposit {
            return Err(crate::error::IppanError::Validation(
                format!("Deposit amount cannot exceed: {}", self.max_deposit)
            ));
        }
        
        // Generate channel ID
        self.channel_counter += 1;
        let channel_id = format!("m2m_{:016x}", self.channel_counter);
        
        let timeout = timeout_hours.unwrap_or(self.default_timeout_hours);
        let expires_at = Utc::now() + chrono::Duration::hours(timeout as i64);
        
        let channel = PaymentChannel {
            channel_id: channel_id.clone(),
            sender,
            recipient,
            deposit_amount,
            current_balance: deposit_amount,
            created_at: Utc::now(),
            expires_at,
            status: ChannelStatus::Open,
            signature: None,
            micro_transactions: Vec::new(),
        };
        
        self.channels.insert(channel_id.clone(), channel.clone());
        
        Ok(channel)
    }

    /// Process a micro-payment within a channel
    pub async fn process_micro_payment(
        &mut self,
        channel_id: &str,
        amount: u64,
        tx_type: MicroTransactionType,
    ) -> Result<MicroTransaction> {
        let channel = self.channels.get_mut(channel_id)
            .ok_or_else(|| crate::error::IppanError::Validation(
                format!("Payment channel not found: {}", channel_id)
            ))?;
        
        // Check if channel is open
        if channel.status != ChannelStatus::Open {
            return Err(crate::error::IppanError::Validation(
                format!("Channel is not open: {:?}", channel.status)
            ));
        }
        
        // Check if channel is expired
        if Utc::now() > channel.expires_at {
            channel.status = ChannelStatus::Expired;
            return Err(crate::error::IppanError::Validation(
                "Channel has expired".to_string()
            ));
        }
        
        // Validate amount
        if amount == 0 {
            return Err(crate::error::IppanError::Validation(
                "Amount must be greater than 0".to_string()
            ));
        }
        
        // Check available balance
        if amount > channel.current_balance {
            return Err(crate::error::IppanError::Validation(
                format!("Insufficient balance: required {}, available {}", 
                    amount, channel.current_balance)
            ));
        }
        
        // Generate transaction ID
        self.tx_counter += 1;
        let tx_id = format!("micro_{:016x}", self.tx_counter);
        
        // Create transaction hash
        let hash_data = format!("{}:{}:{}:{}", channel_id, amount, tx_type as u8, Utc::now().timestamp());
        let hash = crate::utils::crypto::sha256_hash(hash_data.as_bytes());
        let hash_string = hex::encode(hash);
        
        let micro_tx = MicroTransaction {
            tx_id: tx_id.clone(),
            amount,
            tx_type,
            timestamp: Utc::now(),
            signature: None,
            hash: hash_string,
            memo: None,
        };
        
        // Update channel balance
        channel.current_balance -= amount;
        channel.micro_transactions.push(micro_tx.clone());
        
        Ok(micro_tx)
    }

    /// Close a payment channel
    pub async fn close_payment_channel(&mut self, channel_id: &str) -> Result<PaymentChannel> {
        if let Some(channel) = self.channels.remove(channel_id) {
            let mut closed_channel = channel;
            closed_channel.status = ChannelStatus::Closed;
            self.closed_channels.insert(channel_id.to_string(), closed_channel.clone());
            Ok(closed_channel)
        } else {
            Err(crate::error::IppanError::Validation(
                format!("Payment channel not found: {}", channel_id)
            ))
        }
    }

    /// Get a payment channel by ID
    pub fn get_payment_channel(&self, channel_id: &str) -> Option<&PaymentChannel> {
        self.channels.get(channel_id)
            .or_else(|| self.closed_channels.get(channel_id))
    }

    /// Get all payment channels for an address
    pub fn get_channels_for_address(&self, address: &str) -> Vec<&PaymentChannel> {
        let mut channels = Vec::new();
        
        // Check active channels
        for channel in self.channels.values() {
            if channel.sender == address || channel.recipient == address {
                channels.push(channel);
            }
        }
        
        // Check closed channels
        for channel in self.closed_channels.values() {
            if channel.sender == address || channel.recipient == address {
                channels.push(channel);
            }
        }
        
        channels
    }

    /// Get active payment channels
    pub fn get_active_channels(&self) -> Vec<&PaymentChannel> {
        self.channels.values().collect()
    }

    /// Get closed payment channels
    pub fn get_closed_channels(&self) -> Vec<&PaymentChannel> {
        self.closed_channels.values().collect()
    }

    /// Clean up expired channels
    pub fn cleanup_expired_channels(&mut self) -> usize {
        let mut expired_count = 0;
        let now = Utc::now();
        
        let expired_channels: Vec<String> = self.channels.iter()
            .filter(|(_, channel)| channel.expires_at < now)
            .map(|(id, _)| id.clone())
            .collect();
        
        for channel_id in expired_channels {
            if let Some(channel) = self.channels.remove(&channel_id) {
                let mut expired_channel = channel;
                expired_channel.status = ChannelStatus::Expired;
                self.closed_channels.insert(channel_id, expired_channel);
                expired_count += 1;
            }
        }
        
        expired_count
    }

    /// Get total fees collected from M2M payments
    pub fn get_total_fees_collected(&self) -> u64 {
        let mut total_fees = 0u64;
        
        for channel in self.channels.values() {
            for tx in &channel.micro_transactions {
                if matches!(tx.tx_type, MicroTransactionType::Fee) {
                    total_fees += tx.amount;
                }
            }
        }
        
        for channel in self.closed_channels.values() {
            for tx in &channel.micro_transactions {
                if matches!(tx.tx_type, MicroTransactionType::Fee) {
                    total_fees += tx.amount;
                }
            }
        }
        
        total_fees
    }

    /// Get payment statistics
    pub fn get_payment_statistics(&self) -> PaymentStatistics {
        let total_channels = self.channels.len() + self.closed_channels.len();
        let active_channels = self.channels.len();
        let closed_channels = self.closed_channels.len();
        
        let total_deposits: u64 = self.channels.values()
            .map(|c| c.deposit_amount)
            .sum();
        
        let total_balance: u64 = self.channels.values()
            .map(|c| c.current_balance)
            .sum();
        
        let total_micro_transactions: usize = self.channels.values()
            .map(|c| c.micro_transactions.len())
            .sum::<usize>() + self.closed_channels.values()
            .map(|c| c.micro_transactions.len())
            .sum::<usize>();
        
        PaymentStatistics {
            total_channels,
            active_channels,
            closed_channels,
            total_deposits,
            total_balance,
            total_micro_transactions,
            total_fees_collected: self.get_total_fees_collected(),
            fee_rate: self.fee_rate,
            min_deposit: self.min_deposit,
            max_deposit: self.max_deposit,
        }
    }
}

/// Payment statistics
#[derive(Debug, Serialize)]
pub struct PaymentStatistics {
    pub total_channels: usize,
    pub active_channels: usize,
    pub closed_channels: usize,
    pub total_deposits: u64,
    pub total_balance: u64,
    pub total_micro_transactions: usize,
    pub total_fees_collected: u64,
    pub fee_rate: f64,
    pub min_deposit: u64,
    pub max_deposit: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_m2m_system_creation() {
        let system = M2MPaymentSystem::new();
        
        assert_eq!(system.fee_rate, 0.01);
        assert_eq!(system.min_deposit, 100_000);
        assert_eq!(system.max_deposit, 10_000_000);
    }

    #[tokio::test]
    async fn test_create_payment_channel() {
        let mut system = M2MPaymentSystem::new();
        
        let channel = system.create_payment_channel(
            "i1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
            "i1B1zP1eP5QGefi2DMPTfTL5SLmv7DivfNb".to_string(),
            1_000_000, // 1 IPN
            Some(24),
        ).await.unwrap();
        
        assert_eq!(channel.deposit_amount, 1_000_000);
        assert_eq!(channel.current_balance, 1_000_000);
        assert_eq!(channel.status, ChannelStatus::Open);
    }

    #[tokio::test]
    async fn test_process_micro_payment() {
        let mut system = M2MPaymentSystem::new();
        
        let channel = system.create_payment_channel(
            "i1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
            "i1B1zP1eP5QGefi2DMPTfTL5SLmv7DivfNb".to_string(),
            1_000_000,
            None,
        ).await.unwrap();
        
        let micro_tx = system.process_micro_payment(
            &channel.channel_id,
            100_000, // 0.1 IPN
            MicroTransactionType::Payment,
        ).await.unwrap();
        
        assert_eq!(micro_tx.amount, 100_000);
        assert!(matches!(micro_tx.tx_type, MicroTransactionType::Payment));
        
        // Check that channel balance was updated
        let updated_channel = system.get_payment_channel(&channel.channel_id).unwrap();
        assert_eq!(updated_channel.current_balance, 900_000);
    }

    #[tokio::test]
    async fn test_close_payment_channel() {
        let mut system = M2MPaymentSystem::new();
        
        let channel = system.create_payment_channel(
            "i1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
            "i1B1zP1eP5QGefi2DMPTfTL5SLmv7DivfNb".to_string(),
            1_000_000,
            None,
        ).await.unwrap();
        
        let closed_channel = system.close_payment_channel(&channel.channel_id).await.unwrap();
        assert_eq!(closed_channel.status, ChannelStatus::Closed);
    }

    #[tokio::test]
    async fn test_payment_statistics() {
        let mut system = M2MPaymentSystem::new();
        
        // Create a channel
        let channel = system.create_payment_channel(
            "i1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
            "i1B1zP1eP5QGefi2DMPTfTL5SLmv7DivfNb".to_string(),
            1_000_000,
            None,
        ).await.unwrap();
        
        // Process a micro-payment
        system.process_micro_payment(
            &channel.channel_id,
            100_000,
            MicroTransactionType::Payment,
        ).await.unwrap();
        
        let stats = system.get_payment_statistics();
        assert_eq!(stats.total_channels, 1);
        assert_eq!(stats.active_channels, 1);
        assert_eq!(stats.total_deposits, 1_000_000);
        assert_eq!(stats.total_balance, 900_000);
        assert_eq!(stats.total_micro_transactions, 1);
    }
} 
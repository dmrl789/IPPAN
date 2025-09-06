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
    pub id: String,
    /// Sender address
    pub sender: String,
    /// Recipient address
    pub recipient: String,
    /// Total deposit amount (net after fee)
    pub deposit_amount: u64,
    /// Fee paid to Global Fund
    pub deposit_fee: u64,
    /// Sender's balance (net after fee)
    pub sender_balance: u64,
    /// Recipient's balance (net after fee)
    pub recipient_balance: u64,
    /// Channel status
    pub status: ChannelStatus,
    /// Channel creation timestamp
    pub created_at: DateTime<Utc>,
    /// Channel expiration timestamp
    pub timeout_at: DateTime<Utc>,
    /// Last update timestamp
    pub last_update: DateTime<Utc>,
    /// Last sequence number for off-chain updates
    pub last_sequence: u64,
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

/// Settlement result for a channel
#[derive(Debug, Serialize)]
pub struct SettlementResult {
    pub channel_id: String,
    pub settled_amount: u64,
    pub settlement_fee: u64,
    pub remaining_balance: u64,
}

/// Close result for a channel
#[derive(Debug, Serialize)]
pub struct CloseResult {
    pub channel_id: String,
    pub final_settlement: u64,
    pub final_fee: u64,
    pub sender_refund: u64,
    pub total_fees_paid: u64,
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
    /// M2M fee rate (1% per PRD)
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
            fee_rate: 0.01, // 1% per PRD
            min_deposit: 100_000, // 0.1 IPN
            max_deposit: 10_000_000, // 10 IPN
            default_timeout_hours: 24,
        }
    }

    /// Calculate 1% fee on amount in smallest units
    pub fn calc_fee_1pct(&self, amount_units: u64) -> u64 {
        let one_pct = amount_units / 100; // floor division
        one_pct.max(1) // dust guard: minimum 1 unit
    }

    /// Create a new payment channel (on-chain)
    /// Applies 1% fee on the deposit amount
    pub async fn create_payment_channel(
        &mut self,
        sender: String,
        recipient: String,
        deposit_amount: u64,
        timeout_hours: Option<u64>,
    ) -> Result<PaymentChannel> {
        // Validate deposit amount
        if deposit_amount < self.min_deposit {
            return Err(crate::error::IppanError::Validation(
                format!("Deposit must be at least {} units", self.min_deposit)
            ));
        }
        
        if deposit_amount > self.max_deposit {
            return Err(crate::error::IppanError::Validation(
                format!("Deposit cannot exceed {} units", self.max_deposit)
            ));
        }

        // Calculate 1% fee on deposit (PRD rule)
        let deposit_fee = self.calc_fee_1pct(deposit_amount);
        let net_deposit = deposit_amount - deposit_fee;

        // Generate channel ID
        self.channel_counter += 1;
        let channel_id = format!("CH_{:016x}", self.channel_counter);

        // Create channel
        let channel = PaymentChannel {
            id: channel_id.clone(),
            sender,
            recipient,
            deposit_amount: net_deposit, // Net after fee
            deposit_fee, // Fee paid to Global Fund
            sender_balance: net_deposit,
            recipient_balance: 0,
            status: ChannelStatus::Open,
            created_at: chrono::Utc::now(),
            timeout_at: chrono::Utc::now() + chrono::Duration::hours(timeout_hours.unwrap_or(self.default_timeout_hours) as i64),
            last_update: chrono::Utc::now(),
            last_sequence: 0, // Initialize sequence number
        };

        // Store channel
        self.channels.insert(channel_id.clone(), channel.clone());

        Ok(channel)
    }

    /// Process off-chain update (no fee)
    pub async fn process_off_chain_update(
        &mut self,
        channel_id: &str,
        amount: u64,
        sequence: u64,
        signature: Vec<u8>,
    ) -> Result<()> {
        let channel = self.channels.get_mut(channel_id)
            .ok_or_else(|| crate::error::IppanError::Validation("Channel not found".to_string()))?;

        if channel.status != ChannelStatus::Open {
            return Err(crate::error::IppanError::Validation("Channel is not open".to_string()));
        }

        // Validate sequence number
        if sequence <= channel.last_sequence {
            return Err(crate::error::IppanError::Validation("Invalid sequence number".to_string()));
        }

        // Validate amount
        if amount > channel.sender_balance {
            return Err(crate::error::IppanError::Validation("Insufficient balance".to_string()));
        }

        // Process transfer (off-chain, no fee)
        channel.sender_balance -= amount;
        channel.recipient_balance += amount;
        channel.last_sequence = sequence;
        channel.last_update = chrono::Utc::now();

        Ok(())
    }

    /// Settle channel (on-chain)
    /// Applies 1% fee on the net settled amount
    pub async fn settle_channel(
        &mut self,
        channel_id: &str,
        settle_amount: u64,
    ) -> Result<SettlementResult> {
        // Calculate 1% fee on settled amount (PRD rule) - do this before borrowing
        let settlement_fee = self.calc_fee_1pct(settle_amount);
        let net_settlement = settle_amount - settlement_fee;

        let channel = self.channels.get_mut(channel_id)
            .ok_or_else(|| crate::error::IppanError::Validation("Channel not found".to_string()))?;

        if channel.status != ChannelStatus::Open {
            return Err(crate::error::IppanError::Validation("Channel is not open".to_string()));
        }

        // Validate settle amount
        if settle_amount > channel.recipient_balance {
            return Err(crate::error::IppanError::Validation("Invalid settle amount".to_string()));
        }

        // Update balances
        channel.recipient_balance -= settle_amount;

        // Create settlement result
        let result = SettlementResult {
            channel_id: channel_id.to_string(),
            settled_amount: net_settlement,
            settlement_fee,
            remaining_balance: channel.recipient_balance,
        };

        Ok(result)
    }

    /// Close channel completely (on-chain)
    /// Applies 1% fee on final settlement
    pub async fn close_channel(
        &mut self,
        channel_id: &str,
    ) -> Result<CloseResult> {
        let channel = self.channels.remove(channel_id)
            .ok_or_else(|| crate::error::IppanError::Validation("Channel not found".to_string()))?;

        if channel.status != ChannelStatus::Open {
            return Err(crate::error::IppanError::Validation("Channel is not open".to_string()));
        }

        // Calculate final settlement
        let final_settlement = channel.recipient_balance;
        let final_fee = if final_settlement > 0 {
            self.calc_fee_1pct(final_settlement)
        } else {
            0 // No fee when there's no settlement
        };
        let net_final_settlement = if final_settlement > final_fee {
            final_settlement - final_fee
        } else {
            0 // If fee is larger than settlement, net settlement is 0
        };

        // Return remaining balances to sender
        let sender_refund = channel.sender_balance;

        // Store closed channel
        let deposit_fee = channel.deposit_fee; // Store before moving
        let mut closed_channel = channel;
        closed_channel.status = ChannelStatus::Closed;
        self.closed_channels.insert(channel_id.to_string(), closed_channel);

        let result = CloseResult {
            channel_id: channel_id.to_string(),
            final_settlement: net_final_settlement,
            final_fee,
            sender_refund,
            total_fees_paid: final_fee + deposit_fee, // Final settlement fee + initial deposit fee
        };

        Ok(result)
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
            .filter(|(_, channel)| channel.timeout_at < now)
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
            total_fees += channel.deposit_fee; // Deposit fees are paid at open
        }
        
        for channel in self.closed_channels.values() {
            total_fees += channel.deposit_fee; // Deposit fees are paid at open
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
            .map(|c| c.sender_balance) // Use sender_balance for total balance
            .sum();
        
        // TODO: Implement micro transaction tracking
        let total_micro_transactions: usize = 0;
        
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
        
        assert_eq!(channel.deposit_amount, 990_000); // Net after 1% fee
        assert_eq!(channel.deposit_fee, 10_000); // 1% fee
        assert_eq!(channel.sender_balance, 990_000); // Net after fee
        assert_eq!(channel.recipient_balance, 0);
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
        
        // TODO: Implement process_micro_payment method
        // For now, just test that channel was created correctly
        assert_eq!(channel.sender_balance, 990_000); // Net after fee
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
        
        let closed_channel = system.close_channel(&channel.id).await.unwrap();
        
        assert_eq!(closed_channel.final_settlement, 0); // No recipient balance
        assert_eq!(closed_channel.final_fee, 0); // No fee on 0 amount
        assert_eq!(closed_channel.sender_refund, 990_000); // Net after fee
        assert_eq!(closed_channel.total_fees_paid, 10_000); // Initial deposit fee (no final settlement fee since recipient balance is 0)
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
        
        let stats = system.get_payment_statistics();
        assert_eq!(stats.total_channels, 1);
        assert_eq!(stats.active_channels, 1);
        assert_eq!(stats.total_deposits, 990_000); // Net after fee
        assert_eq!(stats.total_balance, 990_000); // Net after fee
        assert_eq!(stats.total_micro_transactions, 0); // Not implemented yet
    }
} 
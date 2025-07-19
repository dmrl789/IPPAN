//! Payment processing for IPPAN wallet

use crate::Result;
use crate::utils::address::validate_ippan_address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// Payment transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentTransaction {
    /// Transaction ID
    pub tx_id: String,
    /// Sender address
    pub from: String,
    /// Recipient address
    pub to: String,
    /// Amount in smallest units
    pub amount: u64,
    /// Transaction fee
    pub fee: u64,
    /// Transaction timestamp
    pub timestamp: DateTime<Utc>,
    /// Transaction status
    pub status: PaymentStatus,
    /// Transaction signature
    pub signature: Option<Vec<u8>>,
    /// Transaction hash
    pub hash: String,
    /// Memo/note
    pub memo: Option<String>,
}

/// Payment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentStatus {
    /// Pending confirmation
    Pending,
    /// Confirmed on network
    Confirmed,
    /// Failed
    Failed,
    /// Cancelled
    Cancelled,
}

/// Payment processor
pub struct PaymentProcessor {
    /// Pending transactions
    pending_transactions: HashMap<String, PaymentTransaction>,
    /// Confirmed transactions
    confirmed_transactions: HashMap<String, PaymentTransaction>,
    /// Failed transactions
    failed_transactions: HashMap<String, PaymentTransaction>,
    /// Transaction counter
    tx_counter: u64,
    /// Minimum fee
    min_fee: u64,
    /// Maximum transaction amount
    max_amount: u64,
}

impl PaymentProcessor {
    /// Create a new payment processor
    pub fn new() -> Self {
        Self {
            pending_transactions: HashMap::new(),
            confirmed_transactions: HashMap::new(),
            failed_transactions: HashMap::new(),
            tx_counter: 0,
            min_fee: 1000, // 0.001 IPN
            max_amount: 1_000_000_000_000, // 1M IPN
        }
    }

    /// Initialize the payment processor
    pub async fn initialize(&mut self) -> Result<()> {
        // Load existing transactions from storage
        self.load_transactions().await?;
        Ok(())
    }

    /// Shutdown the payment processor
    pub async fn shutdown(&mut self) -> Result<()> {
        // Save transactions to storage
        self.save_transactions().await?;
        Ok(())
    }

    /// Create a new payment transaction
    pub async fn create_payment(
        &mut self,
        from: String,
        to: String,
        amount: u64,
        fee: u64,
        memo: Option<String>,
    ) -> Result<PaymentTransaction> {
        // Validate addresses
        if !validate_ippan_address(&from) {
            return Err(crate::error::IppanError::Validation(
                format!("Invalid sender address: {}", from)
            ));
        }
        
        if !validate_ippan_address(&to) {
            return Err(crate::error::IppanError::Validation(
                format!("Invalid recipient address: {}", to)
            ));
        }
        
        // Validate amount
        if amount == 0 {
            return Err(crate::error::IppanError::Validation(
                "Amount must be greater than 0".to_string()
            ));
        }
        
        if amount > self.max_amount {
            return Err(crate::error::IppanError::Validation(
                format!("Amount exceeds maximum: {}", self.max_amount)
            ));
        }
        
        // Validate fee
        if fee < self.min_fee {
            return Err(crate::error::IppanError::Validation(
                format!("Fee must be at least: {}", self.min_fee)
            ));
        }
        
        // Generate transaction ID
        self.tx_counter += 1;
        let tx_id = format!("tx_{:016x}", self.tx_counter);
        
        // Create transaction hash
        let hash_data = format!("{}:{}:{}:{}:{}", from, to, amount, fee, Utc::now().timestamp());
        let hash = crate::utils::crypto::sha256_hash(hash_data.as_bytes());
        let hash_string = hex::encode(hash);
        
        let transaction = PaymentTransaction {
            tx_id: tx_id.clone(),
            from,
            to,
            amount,
            fee,
            timestamp: Utc::now(),
            status: PaymentStatus::Pending,
            signature: None,
            hash: hash_string,
            memo,
        };
        
        self.pending_transactions.insert(tx_id.clone(), transaction.clone());
        
        Ok(transaction)
    }

    /// Sign a payment transaction
    pub async fn sign_payment(
        &mut self,
        tx_id: &str,
        signature: Vec<u8>,
    ) -> Result<()> {
        if let Some(transaction) = self.pending_transactions.get_mut(tx_id) {
            transaction.signature = Some(signature);
            Ok(())
        } else {
            Err(crate::error::IppanError::Validation(
                format!("Transaction not found: {}", tx_id)
            ))
        }
    }

    /// Confirm a payment transaction
    pub async fn confirm_payment(&mut self, tx_id: &str) -> Result<()> {
        if let Some(transaction) = self.pending_transactions.remove(tx_id) {
            let mut confirmed_tx = transaction;
            confirmed_tx.status = PaymentStatus::Confirmed;
            self.confirmed_transactions.insert(tx_id.to_string(), confirmed_tx);
            Ok(())
        } else {
            Err(crate::error::IppanError::Validation(
                format!("Transaction not found: {}", tx_id)
            ))
        }
    }

    /// Fail a payment transaction
    pub async fn fail_payment(&mut self, tx_id: &str, reason: String) -> Result<()> {
        if let Some(transaction) = self.pending_transactions.remove(tx_id) {
            let mut failed_tx = transaction;
            failed_tx.status = PaymentStatus::Failed;
            self.failed_transactions.insert(tx_id.to_string(), failed_tx);
            Ok(())
        } else {
            Err(crate::error::IppanError::Validation(
                format!("Transaction not found: {}", tx_id)
            ))
        }
    }

    /// Cancel a payment transaction
    pub async fn cancel_payment(&mut self, tx_id: &str) -> Result<()> {
        if let Some(transaction) = self.pending_transactions.remove(tx_id) {
            let mut cancelled_tx = transaction;
            cancelled_tx.status = PaymentStatus::Cancelled;
            self.failed_transactions.insert(tx_id.to_string(), cancelled_tx);
            Ok(())
        } else {
            Err(crate::error::IppanError::Validation(
                format!("Transaction not found: {}", tx_id)
            ))
        }
    }

    /// Get a transaction by ID
    pub fn get_transaction(&self, tx_id: &str) -> Option<&PaymentTransaction> {
        self.pending_transactions.get(tx_id)
            .or_else(|| self.confirmed_transactions.get(tx_id))
            .or_else(|| self.failed_transactions.get(tx_id))
    }

    /// Get pending transactions
    pub fn get_pending_transactions(&self) -> Vec<&PaymentTransaction> {
        self.pending_transactions.values().collect()
    }

    /// Get confirmed transactions
    pub fn get_confirmed_transactions(&self) -> Vec<&PaymentTransaction> {
        self.confirmed_transactions.values().collect()
    }

    /// Get failed transactions
    pub fn get_failed_transactions(&self) -> Vec<&PaymentTransaction> {
        self.failed_transactions.values().collect()
    }

    /// Get transactions for an address
    pub fn get_transactions_for_address(&self, address: &str) -> Vec<&PaymentTransaction> {
        let mut transactions = Vec::new();
        
        // Check pending transactions
        for tx in self.pending_transactions.values() {
            if tx.from == address || tx.to == address {
                transactions.push(tx);
            }
        }
        
        // Check confirmed transactions
        for tx in self.confirmed_transactions.values() {
            if tx.from == address || tx.to == address {
                transactions.push(tx);
            }
        }
        
        // Check failed transactions
        for tx in self.failed_transactions.values() {
            if tx.from == address || tx.to == address {
                transactions.push(tx);
            }
        }
        
        transactions
    }

    /// Get payment statistics
    pub fn get_payment_stats(&self) -> PaymentStats {
        let total_pending = self.pending_transactions.len();
        let total_confirmed = self.confirmed_transactions.len();
        let total_failed = self.failed_transactions.len();
        
        let total_amount_pending: u64 = self.pending_transactions.values()
            .map(|tx| tx.amount)
            .sum();
        
        let total_amount_confirmed: u64 = self.confirmed_transactions.values()
            .map(|tx| tx.amount)
            .sum();
        
        let total_fees_collected: u64 = self.confirmed_transactions.values()
            .map(|tx| tx.fee)
            .sum();
        
        PaymentStats {
            total_pending,
            total_confirmed,
            total_failed,
            total_amount_pending,
            total_amount_confirmed,
            total_fees_collected,
            min_fee: self.min_fee,
            max_amount: self.max_amount,
        }
    }

    /// Load transactions from storage
    async fn load_transactions(&mut self) -> Result<()> {
        // TODO: Implement transaction loading from persistent storage
        Ok(())
    }

    /// Save transactions to storage
    async fn save_transactions(&self) -> Result<()> {
        // TODO: Implement transaction saving to persistent storage
        Ok(())
    }
}

/// Payment statistics
#[derive(Debug, Serialize)]
pub struct PaymentStats {
    pub total_pending: usize,
    pub total_confirmed: usize,
    pub total_failed: usize,
    pub total_amount_pending: u64,
    pub total_amount_confirmed: u64,
    pub total_fees_collected: u64,
    pub min_fee: u64,
    pub max_amount: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_payment_processor_creation() {
        let mut processor = PaymentProcessor::new();
        processor.initialize().await.unwrap();
        
        assert_eq!(processor.min_fee, 1000);
        assert_eq!(processor.max_amount, 1_000_000_000_000);
    }

    #[tokio::test]
    async fn test_create_payment() {
        let mut processor = PaymentProcessor::new();
        processor.initialize().await.unwrap();
        
        let payment = processor.create_payment(
            "i1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
            "i1B1zP1eP5QGefi2DMPTfTL5SLmv7DivfNb".to_string(),
            1000000, // 1 IPN
            1000,    // 0.001 IPN fee
            Some("Test payment".to_string()),
        ).await.unwrap();
        
        assert_eq!(payment.amount, 1000000);
        assert_eq!(payment.fee, 1000);
        assert_eq!(payment.status, PaymentStatus::Pending);
        assert!(payment.signature.is_none());
    }

    #[tokio::test]
    async fn test_invalid_address() {
        let mut processor = PaymentProcessor::new();
        processor.initialize().await.unwrap();
        
        let result = processor.create_payment(
            "invalid_address".to_string(),
            "i1B1zP1eP5QGefi2DMPTfTL5SLmv7DivfNb".to_string(),
            1000000,
            1000,
            None,
        ).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_confirm_payment() {
        let mut processor = PaymentProcessor::new();
        processor.initialize().await.unwrap();
        
        let payment = processor.create_payment(
            "i1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
            "i1B1zP1eP5QGefi2DMPTfTL5SLmv7DivfNb".to_string(),
            1000000,
            1000,
            None,
        ).await.unwrap();
        
        processor.confirm_payment(&payment.tx_id).await.unwrap();
        
        let confirmed_tx = processor.get_transaction(&payment.tx_id).unwrap();
        assert_eq!(confirmed_tx.status, PaymentStatus::Confirmed);
    }

    #[tokio::test]
    async fn test_payment_stats() {
        let mut processor = PaymentProcessor::new();
        processor.initialize().await.unwrap();
        
        // Create and confirm a payment
        let payment = processor.create_payment(
            "i1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
            "i1B1zP1eP5QGefi2DMPTfTL5SLmv7DivfNb".to_string(),
            1000000,
            1000,
            None,
        ).await.unwrap();
        
        processor.confirm_payment(&payment.tx_id).await.unwrap();
        
        let stats = processor.get_payment_stats();
        assert_eq!(stats.total_confirmed, 1);
        assert_eq!(stats.total_amount_confirmed, 1000000);
        assert_eq!(stats.total_fees_collected, 1000);
    }
}

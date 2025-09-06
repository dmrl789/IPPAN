//! Payment management for IPPAN wallet

use crate::Result;
use crate::utils::address::validate_ippan_address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use sled;
use std::path::PathBuf;

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
    /// Database for persistence
    db: sled::Db,
}

impl PaymentProcessor {
    /// Create a new payment processor
    pub fn new() -> Result<Self> {
        Self::new_with_db_path(None)
    }

    /// Create a new payment processor with custom database path (for testing)
    pub fn new_with_db_path(custom_path: Option<PathBuf>) -> Result<Self> {
        let db_path = if let Some(path) = custom_path {
            path
        } else {
            Self::get_db_path()?
        };
        
        let db = sled::open(&db_path)
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to open payment database: {}", e)))?;
        
        Ok(Self {
            pending_transactions: HashMap::new(),
            confirmed_transactions: HashMap::new(),
            failed_transactions: HashMap::new(),
            tx_counter: 0,
            min_fee: 1000, // 0.001 IPN
            max_amount: 1_000_000_000_000, // 1M IPN
            db,
        })
    }

    /// Get database path
    fn get_db_path() -> Result<PathBuf> {
        let mut path = dirs::data_dir()
            .ok_or_else(|| crate::error::IppanError::Storage("Could not determine data directory".to_string()))?;
        path.push("ippan");
        path.push("wallet");
        path.push("payments");
        
        // Create directory if it doesn't exist
        std::fs::create_dir_all(&path)
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to create payment database directory: {}", e)))?;
        
        Ok(path)
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
        if validate_ippan_address(&from).is_err() {
            return Err(crate::error::IppanError::Validation(
                format!("Invalid sender address: {}", from)
            ));
        }
        
        if validate_ippan_address(&to).is_err() {
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
        log::info!("Loading payment transactions from persistent storage...");
        
        // Load pending transactions
        if let Ok(pending_tree) = self.db.open_tree("pending_transactions") {
            for result in pending_tree.iter() {
                let (key, value) = result
                    .map_err(|e| crate::error::IppanError::Storage(format!("Failed to read pending transaction: {}", e)))?;
                
                let tx_id = String::from_utf8(key.to_vec())
                    .map_err(|e| crate::error::IppanError::Storage(format!("Invalid transaction ID: {}", e)))?;
                
                let transaction: PaymentTransaction = bincode::deserialize(&value)
                    .map_err(|e| crate::error::IppanError::Storage(format!("Failed to deserialize pending transaction: {}", e)))?;
                
                self.pending_transactions.insert(tx_id, transaction);
            }
        }
        
        // Load confirmed transactions
        if let Ok(confirmed_tree) = self.db.open_tree("confirmed_transactions") {
            for result in confirmed_tree.iter() {
                let (key, value) = result
                    .map_err(|e| crate::error::IppanError::Storage(format!("Failed to read confirmed transaction: {}", e)))?;
                
                let tx_id = String::from_utf8(key.to_vec())
                    .map_err(|e| crate::error::IppanError::Storage(format!("Invalid transaction ID: {}", e)))?;
                
                let transaction: PaymentTransaction = bincode::deserialize(&value)
                    .map_err(|e| crate::error::IppanError::Storage(format!("Failed to deserialize confirmed transaction: {}", e)))?;
                
                self.confirmed_transactions.insert(tx_id, transaction);
            }
        }
        
        // Load failed transactions
        if let Ok(failed_tree) = self.db.open_tree("failed_transactions") {
            for result in failed_tree.iter() {
                let (key, value) = result
                    .map_err(|e| crate::error::IppanError::Storage(format!("Failed to read failed transaction: {}", e)))?;
                
                let tx_id = String::from_utf8(key.to_vec())
                    .map_err(|e| crate::error::IppanError::Storage(format!("Invalid transaction ID: {}", e)))?;
                
                let transaction: PaymentTransaction = bincode::deserialize(&value)
                    .map_err(|e| crate::error::IppanError::Storage(format!("Failed to deserialize failed transaction: {}", e)))?;
                
                self.failed_transactions.insert(tx_id, transaction);
            }
        }
        
        // Load transaction counter
        if let Ok(Some(counter_value)) = self.db.get("tx_counter") {
            self.tx_counter = bincode::deserialize(&counter_value)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to deserialize transaction counter: {}", e)))?;
        }
        
        log::info!("Loaded {} pending, {} confirmed, {} failed transactions", 
            self.pending_transactions.len(), self.confirmed_transactions.len(), 
            self.failed_transactions.len());
        
        Ok(())
    }

    /// Save transactions to storage
    async fn save_transactions(&self) -> Result<()> {
        log::info!("Saving payment transactions to persistent storage...");
        
        // Save pending transactions
        let pending_tree = self.db.open_tree("pending_transactions")
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to open pending transactions tree: {}", e)))?;
        
        for (tx_id, transaction) in &self.pending_transactions {
            let key = tx_id.as_bytes();
            let value = bincode::serialize(transaction)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to serialize pending transaction: {}", e)))?;
            pending_tree.insert(key, value)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to save pending transaction: {}", e)))?;
        }
        
        // Save confirmed transactions
        let confirmed_tree = self.db.open_tree("confirmed_transactions")
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to open confirmed transactions tree: {}", e)))?;
        
        for (tx_id, transaction) in &self.confirmed_transactions {
            let key = tx_id.as_bytes();
            let value = bincode::serialize(transaction)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to serialize confirmed transaction: {}", e)))?;
            confirmed_tree.insert(key, value)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to save confirmed transaction: {}", e)))?;
        }
        
        // Save failed transactions
        let failed_tree = self.db.open_tree("failed_transactions")
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to open failed transactions tree: {}", e)))?;
        
        for (tx_id, transaction) in &self.failed_transactions {
            let key = tx_id.as_bytes();
            let value = bincode::serialize(transaction)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to serialize failed transaction: {}", e)))?;
            failed_tree.insert(key, value)
                .map_err(|e| crate::error::IppanError::Storage(format!("Failed to save failed transaction: {}", e)))?;
        }
        
        // Save transaction counter
        let counter_value = bincode::serialize(&self.tx_counter)
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to serialize transaction counter: {}", e)))?;
        self.db.insert("tx_counter", counter_value)
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to save transaction counter: {}", e)))?;
        
        // Flush database to ensure data is written to disk
        self.db.flush()
            .map_err(|e| crate::error::IppanError::Storage(format!("Failed to flush database: {}", e)))?;
        
        log::info!("Saved {} pending, {} confirmed, {} failed transactions", 
            self.pending_transactions.len(), self.confirmed_transactions.len(), 
            self.failed_transactions.len());
        
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
    async fn test_payment_processor_creation() {
        let test_db_path = std::env::temp_dir().join(format!("test_payments_{}", rand::random::<u64>()));
        let mut processor = PaymentProcessor::new_with_db_path(Some(test_db_path)).unwrap();
        processor.initialize().await.unwrap();
        
        assert_eq!(processor.min_fee, 1000);
        assert_eq!(processor.max_amount, 1_000_000_000_000);
    }

    #[tokio::test]
    async fn test_create_payment() {
        let test_db_path = std::env::temp_dir().join(format!("test_payments_{}", rand::random::<u64>()));
        let mut processor = PaymentProcessor::new_with_db_path(Some(test_db_path)).unwrap();
        processor.initialize().await.unwrap();
        
        let (from_addr, to_addr) = generate_test_addresses();
        
        let payment = processor.create_payment(
            from_addr,
            to_addr,
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
        let test_db_path = std::env::temp_dir().join(format!("test_payments_{}", rand::random::<u64>()));
        let mut processor = PaymentProcessor::new_with_db_path(Some(test_db_path)).unwrap();
        processor.initialize().await.unwrap();
        
        let (_, to_addr) = generate_test_addresses();
        
        let result = processor.create_payment(
            "invalid_address".to_string(),
            to_addr,
            1000000,
            1000,
            None,
        ).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_confirm_payment() {
        let test_db_path = std::env::temp_dir().join(format!("test_payments_{}", rand::random::<u64>()));
        let mut processor = PaymentProcessor::new_with_db_path(Some(test_db_path)).unwrap();
        processor.initialize().await.unwrap();
        
        let (from_addr, to_addr) = generate_test_addresses();
        
        let payment = processor.create_payment(
            from_addr,
            to_addr,
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
        let test_db_path = std::env::temp_dir().join(format!("test_payments_{}", rand::random::<u64>()));
        let mut processor = PaymentProcessor::new_with_db_path(Some(test_db_path)).unwrap();
        processor.initialize().await.unwrap();
        
        let (from_addr, to_addr) = generate_test_addresses();
        
        // Create and confirm a payment
        let payment = processor.create_payment(
            from_addr,
            to_addr,
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

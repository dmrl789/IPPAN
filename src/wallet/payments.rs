use crate::{Result, TransactionHash};
use super::ed25519::Ed25519Wallet;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Custom signature wrapper for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureWrapper(#[serde(with = "signature_serde")] pub [u8; 64]);

mod signature_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(signature: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        signature.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom("Invalid signature length"));
        }
        let mut signature = [0u8; 64];
        signature.copy_from_slice(&bytes);
        Ok(signature)
    }
}

/// Payment wallet for handling IPN transactions
pub struct PaymentWallet {
    /// Ed25519 wallet for signing
    ed25519: Ed25519Wallet,
    /// Available balance
    balance: RwLock<u64>,
    /// Transaction history
    transactions: RwLock<Vec<PaymentTransaction>>,
    /// Addresses (UTXOs)
    addresses: RwLock<HashMap<[u8; 32], u64>>,
}

/// Payment transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentTransaction {
    /// Transaction hash
    pub hash: TransactionHash,
    /// Transaction type
    pub tx_type: PaymentType,
    /// Amount
    pub amount: u64,
    /// Fee
    pub fee: u64,
    /// From address
    pub from: [u8; 32],
    /// To address
    pub to: [u8; 32],
    /// Timestamp
    pub timestamp: u64,
    /// Status
    pub status: TransactionStatus,
    /// Signature
    pub signature: SignatureWrapper,
    /// Block height (if confirmed)
    pub block_height: Option<u64>,
}

/// Payment types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentType {
    /// Regular payment
    Payment,
    /// M2M payment
    M2MPayment,
    /// Fee payment
    Fee,
    /// Reward payment
    Reward,
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

impl PaymentWallet {
    /// Create a new payment wallet
    pub fn new(ed25519: &Ed25519Wallet) -> Self {
        Self {
            ed25519: ed25519.clone(),
            balance: RwLock::new(0),
            transactions: RwLock::new(Vec::new()),
            addresses: RwLock::new(HashMap::new()),
        }
    }

    /// Get available balance
    pub async fn get_available_balance(&self) -> Result<u64> {
        Ok(*self.balance.read().await)
    }

    /// Send a payment
    pub async fn send_payment(&self, to: [u8; 32], amount: u64, fee: u64) -> Result<TransactionHash> {
        let total_amount = amount + fee;
        let current_balance = *self.balance.read().await;
        
        if current_balance < total_amount {
            return Err(crate::IppanError::Wallet("Insufficient balance".to_string()));
        }

        // Create transaction
        let from = self.ed25519.get_public_key();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let tx_data = self.create_transaction_data(from, to, amount, fee, timestamp);
        let signature = self.ed25519.sign(&tx_data).await?;
        let hash = self.calculate_transaction_hash(&tx_data, &signature);

        let transaction = PaymentTransaction {
            hash,
            tx_type: PaymentType::Payment,
            amount,
            fee,
            from,
            to,
            timestamp,
            status: TransactionStatus::Pending,
            signature: SignatureWrapper(signature),
            block_height: None,
        };

        // Update balance
        {
            let mut balance = self.balance.write().await;
            *balance = balance.saturating_sub(total_amount);
        }

        // Add to transaction history
        {
            let mut transactions = self.transactions.write().await;
            transactions.push(transaction);
        }

        Ok(hash)
    }

    /// Send M2M payment (micro-payment)
    pub async fn send_m2m_payment(&self, to: [u8; 32], amount: u64) -> Result<TransactionHash> {
        // M2M payments have a 1% fee as specified in the PRD
        let fee = (amount * 1) / 100; // 1% fee
        self.send_payment(to, amount, fee).await
    }

    /// Receive a payment
    pub async fn receive_payment(&self, from: [u8; 32], amount: u64, tx_hash: TransactionHash) -> Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let transaction = PaymentTransaction {
            hash: tx_hash,
            tx_type: PaymentType::Payment,
            amount,
            fee: 0, // No fee for received payments
            from,
            to: self.ed25519.get_public_key(),
            timestamp,
            status: TransactionStatus::Confirmed,
            signature: SignatureWrapper([0; 64]), // Will be set by sender
            block_height: None,
        };

        // Update balance
        {
            let mut balance = self.balance.write().await;
            *balance += amount;
        }

        // Add to transaction history
        {
            let mut transactions = self.transactions.write().await;
            transactions.push(transaction);
        }

        Ok(())
    }

    /// Get transaction history
    pub async fn get_transactions(&self, limit: Option<usize>) -> Result<Vec<super::Transaction>> {
        let transactions = self.transactions.read().await;
        let mut result = Vec::new();

        for tx in transactions.iter() {
            let tx_type = super::TransactionType::Payment {
                from: tx.from,
                to: tx.to,
            };

            result.push(super::Transaction {
                hash: tx.hash,
                tx_type,
                amount: Some(tx.amount),
                fee: tx.fee,
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

    /// Get addresses (UTXOs)
    pub async fn get_addresses(&self) -> Result<Vec<[u8; 32]>> {
        let addresses = self.addresses.read().await;
        Ok(addresses.keys().cloned().collect())
    }

    /// Generate new address
    pub async fn generate_address(&self) -> Result<[u8; 32]> {
        // For simplicity, we'll use the public key as the address
        // In a real implementation, this would generate a new keypair
        let address = self.ed25519.get_public_key();
        
        // Add to addresses with 0 balance
        {
            let mut addresses = self.addresses.write().await;
            addresses.insert(address, 0);
        }

        Ok(address)
    }

    /// Add funds to wallet (for testing or mining rewards)
    pub async fn add_funds(&self, amount: u64) -> Result<()> {
        let mut balance = self.balance.write().await;
        *balance += amount;
        Ok(())
    }

    /// Get transaction by hash
    pub async fn get_transaction(&self, tx_hash: &TransactionHash) -> Option<PaymentTransaction> {
        let transactions = self.transactions.read().await;
        transactions.iter().find(|tx| tx.hash == *tx_hash).cloned()
    }

    /// Verify transaction signature
    pub async fn verify_transaction(&self, transaction: &PaymentTransaction) -> Result<bool> {
        let tx_data = self.create_transaction_data(
            transaction.from,
            transaction.to,
            transaction.amount,
            transaction.fee,
            transaction.timestamp,
        );

        self.ed25519.verify_with_key(&tx_data, &transaction.signature.0, &transaction.from).await
    }

    /// Create transaction data for signing
    fn create_transaction_data(&self, from: [u8; 32], to: [u8; 32], amount: u64, fee: u64, timestamp: u64) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&from);
        data.extend_from_slice(&to);
        data.extend_from_slice(&amount.to_le_bytes());
        data.extend_from_slice(&fee.to_le_bytes());
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

    /// Get payment statistics
    pub async fn get_payment_stats(&self) -> PaymentStats {
        let transactions = self.transactions.read().await;
        let balance = *self.balance.read().await;

        let mut total_sent = 0;
        let mut total_received = 0;
        let mut total_fees = 0;
        let mut m2m_payments = 0;

        for tx in transactions.iter() {
            if tx.from == self.ed25519.get_public_key() {
                // Outgoing transaction
                total_sent += tx.amount;
                total_fees += tx.fee;
                if matches!(tx.tx_type, PaymentType::M2MPayment) {
                    m2m_payments += 1;
                }
            } else {
                // Incoming transaction
                total_received += tx.amount;
            }
        }

        PaymentStats {
            balance,
            total_sent,
            total_received,
            total_fees,
            m2m_payments,
            transaction_count: transactions.len() as u64,
        }
    }
}

/// Payment statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentStats {
    /// Current balance
    pub balance: u64,
    /// Total amount sent
    pub total_sent: u64,
    /// Total amount received
    pub total_received: u64,
    /// Total fees paid
    pub total_fees: u64,
    /// Number of M2M payments
    pub m2m_payments: u64,
    /// Total transaction count
    pub transaction_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::ed25519::Ed25519Wallet;

    #[tokio::test]
    async fn test_payment_wallet() {
        let ed25519 = Ed25519Wallet::new("./test_payment_wallet.dat", false).await.unwrap();
        let wallet = PaymentWallet::new(&ed25519);

        // Add some funds
        wallet.add_funds(1000000).await.unwrap();
        assert_eq!(wallet.get_available_balance().await.unwrap(), 1000000);

        // Send payment
        let to = [1u8; 32];
        let tx_hash = wallet.send_payment(to, 100000, 1000).await.unwrap();
        assert_eq!(wallet.get_available_balance().await.unwrap(), 899000);

        // Check transaction
        let transaction = wallet.get_transaction(&tx_hash).await.unwrap();
        assert_eq!(transaction.amount, 100000);
        assert_eq!(transaction.fee, 1000);

        // Clean up
        let _ = std::fs::remove_file("./test_payment_wallet.dat");
    }

    #[tokio::test]
    async fn test_m2m_payment() {
        let ed25519 = Ed25519Wallet::new("./test_m2m_wallet.dat", false).await.unwrap();
        let wallet = PaymentWallet::new(&ed25519);

        wallet.add_funds(1000000).await.unwrap();

        let to = [1u8; 32];
        let tx_hash = wallet.send_m2m_payment(to, 100000).await.unwrap();

        let transaction = wallet.get_transaction(&tx_hash).await.unwrap();
        assert_eq!(transaction.amount, 100000);
        assert_eq!(transaction.fee, 1000); // 1% of 100000

        // Clean up
        let _ = std::fs::remove_file("./test_m2m_wallet.dat");
    }

    #[tokio::test]
    async fn test_insufficient_balance() {
        let ed25519 = Ed25519Wallet::new("./test_insufficient_wallet.dat", false).await.unwrap();
        let wallet = PaymentWallet::new(&ed25519);

        // Try to send more than available
        let to = [1u8; 32];
        let result = wallet.send_payment(to, 1000000, 1000).await;
        assert!(result.is_err());

        // Clean up
        let _ = std::fs::remove_file("./test_insufficient_wallet.dat");
    }
}

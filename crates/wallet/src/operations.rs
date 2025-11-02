use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::crypto::*;
use crate::errors::*;
use crate::storage::WalletStorage;
use crate::types::*;
use chrono::TimeZone;
use ippan_types::address::{decode_address, encode_address};
use ippan_types::{Amount, Transaction};
use std::convert::TryFrom;

/// Main wallet operations manager
pub struct WalletManager {
    storage: Arc<WalletStorage>,
    rpc_client: Option<Arc<dyn RpcClient + Send + Sync>>,
    transaction_cache: RwLock<HashMap<String, WalletTransaction>>,
}

/// RPC client trait for blockchain interaction
pub trait RpcClient {
    fn get_balance(&self, address: &str) -> Result<u64>;
    fn get_nonce(&self, address: &str) -> Result<u64>;
    fn send_transaction(&self, transaction: &Transaction) -> Result<String>;
    fn get_transaction(&self, tx_hash: &str) -> Result<Option<Transaction>>;
    fn get_transactions_by_address(&self, address: &str) -> Result<Vec<Transaction>>;
}

impl WalletManager {
    /// Create a new wallet manager
    pub fn new(
        storage: Arc<WalletStorage>,
        rpc_client: Option<Arc<dyn RpcClient + Send + Sync>>,
    ) -> Self {
        Self {
            storage,
            rpc_client,
            transaction_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Initialize a new wallet
    pub fn create_wallet(&self, name: String, password: Option<&str>) -> Result<()> {
        self.storage.initialize(name, password)?;
        Ok(())
    }

    /// Load existing wallet
    pub fn load_wallet(&self, password: Option<&str>) -> Result<()> {
        self.storage.load_wallet(password)?;
        Ok(())
    }

    /// Generate a new address
    pub fn generate_address(
        &self,
        label: Option<String>,
        password: Option<&str>,
    ) -> Result<String> {
        let (address, private_key, _) = generate_new_address()?;

        // Encrypt private key
        let encrypted_key = if let Some(pwd) = password {
            let (ciphertext, nonce, salt) = encrypt_data(&private_key, pwd)?;
            EncryptedKey {
                ciphertext,
                nonce,
                salt,
            }
        } else {
            let (ciphertext, nonce, salt) = encrypt_data(&private_key, "default_password")?;
            EncryptedKey {
                ciphertext,
                nonce,
                salt,
            }
        };

        let wallet_address = WalletAddress::new(address.clone(), encrypted_key, label);

        self.storage.update_wallet_state(|state| {
            state.add_address(wallet_address);
            Ok(())
        })?;

        Ok(address)
    }

    /// Generate multiple addresses
    pub fn generate_addresses(
        &self,
        count: usize,
        label_prefix: Option<String>,
        password: Option<&str>,
    ) -> Result<Vec<String>> {
        let mut addresses = Vec::new();
        for i in 0..count {
            let label = label_prefix
                .as_ref()
                .map(|prefix| format!("{}_{}", prefix, i + 1));
            let address = self.generate_address(label, password)?;
            addresses.push(address);
        }
        Ok(addresses)
    }

    /// Get all addresses
    pub fn list_addresses(&self) -> Result<Vec<WalletAddress>> {
        let state = self.storage.get_wallet_state()?;
        Ok(state.list_addresses())
    }

    /// Get address by string
    pub fn get_address(&self, address: &str) -> Result<WalletAddress> {
        let state = self.storage.get_wallet_state()?;
        state
            .get_address(address)
            .cloned()
            .ok_or_else(|| WalletError::AddressNotFound(address.to_string()))
    }

    /// Get balance for a specific address
    pub fn get_address_balance(&self, address: &str) -> Result<u64> {
        let state = self.storage.get_wallet_state()?;
        if let Some(addr) = state.get_address(address) {
            if let Some(ref rpc) = self.rpc_client {
                let balance = rpc.get_balance(address)?;
                self.storage.update_wallet_state(|state| {
                    state.update_balance(address, balance)?;
                    Ok(())
                })?;
                Ok(balance)
            } else {
                Ok(addr.balance)
            }
        } else {
            Err(WalletError::AddressNotFound(address.to_string()))
        }
    }

    /// Get total wallet balance
    pub fn get_total_balance(&self) -> Result<u64> {
        let state = self.storage.get_wallet_state()?;
        let mut total = 0u64;
        for address in state.list_addresses() {
            if let Some(ref rpc) = self.rpc_client {
                total += rpc.get_balance(&address.address)?;
            } else {
                total += address.balance;
            }
        }
        Ok(total)
    }

    /// Send transaction from one address to another
    pub fn send_transaction(
        &self,
        from_address: &str,
        to_address: &str,
        amount: u64,
        password: Option<&str>,
    ) -> Result<String> {
        if !validate_address(from_address) {
            return Err(WalletError::InvalidAddress(format!(
                "Invalid from address: {}",
                from_address
            )));
        }
        if !validate_address(to_address) {
            return Err(WalletError::InvalidAddress(format!(
                "Invalid to address: {}",
                to_address
            )));
        }

        let wallet_address = self.get_address(from_address)?;
        let balance = self.get_address_balance(from_address)?;
        if balance < amount {
            return Err(WalletError::InsufficientBalance {
                required: amount,
                available: balance,
            });
        }

        let private_key = if let Some(pwd) = password {
            decrypt_data(
                &wallet_address.encrypted_private_key.ciphertext,
                &wallet_address.encrypted_private_key.nonce,
                pwd,
            )?
        } else {
            decrypt_data(
                &wallet_address.encrypted_private_key.ciphertext,
                &wallet_address.encrypted_private_key.nonce,
                "default_password",
            )?
        };

        if private_key.len() != 32 {
            return Err(WalletError::InvalidPrivateKey(
                "Invalid private key length".into(),
            ));
        }

        let mut private_key_bytes = [0u8; 32];
        private_key_bytes.copy_from_slice(&private_key);

        let nonce = if let Some(ref rpc) = self.rpc_client {
            rpc.get_nonce(from_address)?
        } else {
            wallet_address.nonce
        };

        let from_bytes = decode_address(from_address)
            .map_err(|e| WalletError::InvalidAddress(format!("Invalid from address: {}", e)))?;
        let to_bytes = decode_address(to_address)
            .map_err(|e| WalletError::InvalidAddress(format!("Invalid to address: {}", e)))?;
        let mut transaction = Transaction::new(
            from_bytes,
            to_bytes,
            Amount::from_atomic(amount as u128),
            nonce,
        );

        transaction
            .sign(&private_key_bytes)
            .map_err(WalletError::TransactionError)?;

        // Deterministic, mempool-aligned fee
        let estimated_fee = self.estimate_fee(&transaction);

        let tx_hash = if let Some(ref rpc) = self.rpc_client {
            rpc.send_transaction(&transaction)?
        } else {
            hex::encode(transaction.hash())
        };

        self.storage.update_wallet_state(|state| {
            state.update_nonce(from_address, nonce + 1)?;
            state.mark_address_used(from_address)?;
            Ok(())
        })?;

        let wallet_tx = WalletTransaction {
            id: uuid::Uuid::new_v4(),
            tx_hash: tx_hash.clone(),
            from_address: Some(from_address.to_string()),
            to_address: Some(to_address.to_string()),
            amount,
            fee: estimated_fee,
            timestamp: chrono::Utc::now(),
            status: TransactionStatus::Pending,
            label: None,
        };

        self.transaction_cache
            .write()
            .insert(tx_hash.clone(), wallet_tx);
        Ok(tx_hash)
    }

    /// Estimate a transaction fee consistent with mempool admission rules
    fn estimate_fee(&self, tx: &Transaction) -> u64 {
        let base_fee = 1000u64;
        let mut estimated_size = 0usize;

        estimated_size += 32 + 32 + 32 + 8 + 8 + 64;
        estimated_size += std::mem::size_of_val(&tx.hashtimer.timestamp_us);
        estimated_size += tx.hashtimer.entropy.len();
        estimated_size += std::mem::size_of_val(&tx.timestamp.0);
        estimated_size += tx.topics.iter().map(|t| t.len()).sum::<usize>();

        if let Some(env) = &tx.confidential {
            estimated_size += env.enc_algo.len() + env.iv.len() + env.ciphertext.len();
            estimated_size += env
                .access_keys
                .iter()
                .map(|k| k.recipient_pub.len() + k.enc_key.len())
                .sum::<usize>();
        }

        if let Some(proof) = &tx.zk_proof {
            estimated_size += proof.proof.len();
            estimated_size += proof
                .public_inputs
                .iter()
                .map(|(k, v)| k.len() + v.len())
                .sum::<usize>();
        }

        let size_fee = (estimated_size as u64).saturating_mul(10);
        let mut fee = base_fee.saturating_add(size_fee);
        const MAX_FEE_PER_TX: u64 = 10_000_000;
        if fee > MAX_FEE_PER_TX {
            fee = MAX_FEE_PER_TX;
        }
        fee
    }

    /// Get transaction history for an address
    pub fn get_address_transactions(&self, address: &str) -> Result<Vec<WalletTransaction>> {
        if let Some(ref rpc) = self.rpc_client {
            let transactions = rpc.get_transactions_by_address(address)?;
            let wallet_transactions = transactions
                .into_iter()
                .map(|tx| {
                    let fee = self.estimate_fee(&tx);
                    let amount_atomic = tx.amount.atomic();
                    let amount = u64::try_from(amount_atomic).unwrap_or(u64::MAX);
                    let micros = tx.timestamp.0;
                    let seconds = (micros / 1_000_000) as i64;
                    let nanos = ((micros % 1_000_000) * 1_000) as u32;
                    let timestamp = chrono::Utc
                        .timestamp_opt(seconds, nanos)
                        .single()
                        .unwrap_or_else(|| chrono::Utc::now());

                    WalletTransaction {
                        id: uuid::Uuid::new_v4(),
                        tx_hash: hex::encode(tx.hash()),
                        from_address: Some(encode_address(&tx.from)),
                        to_address: Some(encode_address(&tx.to)),
                        amount,
                        fee,
                        timestamp,
                        status: TransactionStatus::Confirmed,
                        label: None,
                    }
                })
                .collect();
            Ok(wallet_transactions)
        } else {
            let cache = self.transaction_cache.read();
            Ok(cache
                .values()
                .filter(|tx| {
                    tx.from_address.as_deref() == Some(address)
                        || tx.to_address.as_deref() == Some(address)
                })
                .cloned()
                .collect())
        }
    }

    /// Get all wallet transactions
    pub fn get_all_transactions(&self) -> Result<Vec<WalletTransaction>> {
        let state = self.storage.get_wallet_state()?;
        let mut all_transactions = Vec::new();
        for address in state.list_addresses() {
            all_transactions.extend(self.get_address_transactions(&address.address)?);
        }
        all_transactions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(all_transactions)
    }

    /// Update address label
    pub fn update_address_label(&self, address: &str, label: Option<String>) -> Result<()> {
        self.storage.update_wallet_state(|state| {
            if let Some(addr) = state.get_address_mut(address) {
                addr.label = label;
                Ok(())
            } else {
                Err(WalletError::AddressNotFound(address.to_string()))
            }
        })
    }

    /// Remove address
    pub fn remove_address(&self, address: &str) -> Result<()> {
        self.storage.update_wallet_state(|state| {
            state.remove_address(address);
            Ok(())
        })
    }

    /// Create backup
    pub fn create_backup(&self) -> Result<PathBuf> {
        self.storage.create_backup()
    }

    /// Restore from backup
    pub fn restore_from_backup(&self, backup_path: &Path, password: Option<&str>) -> Result<()> {
        self.storage.restore_from_backup(backup_path, password)
    }

    /// List backups
    pub fn list_backups(&self) -> Result<Vec<PathBuf>> {
        self.storage.list_backups()
    }

    /// Export wallet
    pub fn export_wallet(&self) -> Result<WalletBackup> {
        self.storage.export_wallet()
    }

    /// Import wallet
    pub fn import_wallet(&self, backup: WalletBackup, password: Option<&str>) -> Result<()> {
        self.storage.import_wallet(backup, password)
    }

    /// Sync wallet with blockchain
    pub fn sync_wallet(&self) -> Result<()> {
        if let Some(ref rpc) = self.rpc_client {
            let state = self.storage.get_wallet_state()?;
            for address in state.list_addresses() {
                let balance = rpc.get_balance(&address.address)?;
                let nonce = rpc.get_nonce(&address.address)?;
                self.storage.update_wallet_state(|state| {
                    state.update_balance(&address.address, balance)?;
                    state.update_nonce(&address.address, nonce)?;
                    Ok(())
                })?;
            }
            self.storage.update_wallet_state(|state| {
                state.last_sync = Some(chrono::Utc::now());
                Ok(())
            })?;
        }
        Ok(())
    }

    /// Get wallet statistics
    pub fn get_wallet_stats(&self) -> Result<WalletStats> {
        let state = self.storage.get_wallet_state()?;
        let total_balance = self.get_total_balance()?;
        let address_count = state.addresses.len();
        let transaction_count = self.get_all_transactions()?.len();

        Ok(WalletStats {
            name: state.config.name.clone(),
            address_count,
            total_balance,
            transaction_count,
            last_sync: state.last_sync,
            created_at: state.config.created_at,
        })
    }
}

/// Wallet statistics
#[derive(Debug, Clone)]
pub struct WalletStats {
    pub name: String,
    pub address_count: usize,
    pub total_balance: u64,
    pub transaction_count: usize,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;
    use tempfile::tempdir;

    struct MockRpcClient;

    impl RpcClient for MockRpcClient {
        fn get_balance(&self, _address: &str) -> Result<u64> {
            Ok(1000)
        }
        fn get_nonce(&self, _address: &str) -> Result<u64> {
            Ok(0)
        }
        fn send_transaction(&self, _transaction: &Transaction) -> Result<String> {
            Ok("mock_tx_hash".into())
        }
        fn get_transaction(&self, _tx_hash: &str) -> Result<Option<Transaction>> {
            Ok(None)
        }
        fn get_transactions_by_address(&self, _address: &str) -> Result<Vec<Transaction>> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_wallet_creation() {
        let temp_dir = tempdir().unwrap();
        let storage = Arc::new(WalletStorage::new(temp_dir.path()));
        let wallet = WalletManager::new(storage, None);
        wallet
            .create_wallet("Test Wallet".into(), Some("password123"))
            .unwrap();
        let stats = wallet.get_wallet_stats().unwrap();
        assert_eq!(stats.name, "Test Wallet");
        assert_eq!(stats.address_count, 0);
    }

    #[test]
    fn test_address_generation() {
        let temp_dir = tempdir().unwrap();
        let storage = Arc::new(WalletStorage::new(temp_dir.path()));
        let wallet = WalletManager::new(storage, None);
        wallet
            .create_wallet("Test Wallet".into(), Some("password123"))
            .unwrap();
        let address = wallet
            .generate_address(Some("Test Address".into()), Some("password123"))
            .unwrap();
        assert!(address.starts_with('i'));
        assert_eq!(address.len(), 65);
    }

    #[test]
    fn test_multiple_address_generation() {
        let temp_dir = tempdir().unwrap();
        let storage = Arc::new(WalletStorage::new(temp_dir.path()));
        let wallet = WalletManager::new(storage, None);
        wallet
            .create_wallet("Test Wallet".into(), Some("password123"))
            .unwrap();
        let addresses = wallet
            .generate_addresses(5, Some("Test".into()), Some("password123"))
            .unwrap();
        assert_eq!(addresses.len(), 5);
    }
}

use crate::crypto::{self, KeyPair, PublicKeyBytes};
use crate::transaction::Transaction;
use crate::time::IppanTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub keypair: KeyPair,
    pub address: String,
    pub balance: u64,
    pub nonce: u64,
}

impl Wallet {
    pub fn new() -> Self {
        let keypair = KeyPair::generate();
        let address = crypto::derive_address(&keypair.public_key);
        
        Self {
            keypair,
            address,
            balance: 0,
            nonce: 0,
        }
    }

    pub fn from_secret_key(secret_key_bytes: &[u8]) -> Result<Self, crate::Error> {
        let keypair = KeyPair::from_secret_key(secret_key_bytes)?;
        let address = crypto::derive_address(&keypair.public_key);
        
        Ok(Self {
            keypair,
            address,
            balance: 0,
            nonce: 0,
        })
    }

    pub fn get_address(&self) -> &str {
        &self.address
    }

    pub fn get_balance(&self) -> u64 {
        self.balance
    }

    pub fn get_nonce(&self) -> u64 {
        self.nonce
    }

    pub fn create_payment_tx(
        &self,
        to_address: &str,
        amount: u64,
        ippan_time: Arc<IppanTime>,
    ) -> Result<Transaction, crate::Error> {
        // Validate amount
        if amount == 0 {
            return Err(crate::Error::Wallet("Amount must be greater than 0".to_string()));
        }

        if amount > self.balance {
            return Err(crate::Error::Wallet(format!(
                "Insufficient balance: {} (required: {})",
                self.balance, amount
            )));
        }

        // Parse recipient address (for now, assume it's a public key in hex)
        let to_pub = if to_address.starts_with('i') {
            // This is a simplified address parsing - in a real implementation,
            // you'd need to reverse the address derivation process
            return Err(crate::Error::Wallet("Address parsing not implemented".to_string()));
        } else {
            // Assume it's a hex-encoded public key
            hex::decode(to_address)
                .map_err(|e| crate::Error::Wallet(format!("Invalid address format: {}", e)))?
        };

        if to_pub.len() != 32 {
            return Err(crate::Error::Wallet("Invalid public key length".to_string()));
        }

        let mut to_pub_bytes = [0u8; 32];
        to_pub_bytes.copy_from_slice(&to_pub);

        // Create transaction
        let tx = Transaction::new(
            &self.keypair,
            to_pub_bytes,
            amount,
            self.nonce,
            ippan_time,
        )?;

        Ok(tx)
    }

    pub fn update_balance(&mut self, new_balance: u64) {
        self.balance = new_balance;
    }

    pub fn increment_nonce(&mut self) {
        self.nonce += 1;
    }

    pub fn export_secret_key(&self) -> Vec<u8> {
        self.keypair.secret_key.clone()
    }

    pub fn export_public_key(&self) -> PublicKeyBytes {
        self.keypair.public_key
    }
}

pub struct WalletManager {
    wallets: Arc<RwLock<HashMap<String, Wallet>>>,
    default_wallet: Arc<RwLock<Option<String>>>,
}

impl WalletManager {
    pub fn new() -> Self {
        Self {
            wallets: Arc::new(RwLock::new(HashMap::new())),
            default_wallet: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn create_wallet(&self, name: String) -> Result<(), crate::Error> {
        let mut wallets = self.wallets.write().await;
        
        if wallets.contains_key(&name) {
            return Err(crate::Error::Wallet(format!("Wallet '{}' already exists", name)));
        }

        let wallet = Wallet::new();
        wallets.insert(name.clone(), wallet);
        
        // Set as default if it's the first wallet
        if wallets.len() == 1 {
            *self.default_wallet.write().await = Some(name);
        }

        Ok(())
    }

    pub async fn import_wallet(&self, name: String, secret_key: &[u8]) -> Result<(), crate::Error> {
        let mut wallets = self.wallets.write().await;
        
        if wallets.contains_key(&name) {
            return Err(crate::Error::Wallet(format!("Wallet '{}' already exists", name)));
        }

        let wallet = Wallet::from_secret_key(secret_key)?;
        wallets.insert(name.clone(), wallet);
        
        // Set as default if it's the first wallet
        if wallets.len() == 1 {
            *self.default_wallet.write().await = Some(name);
        }

        Ok(())
    }

    pub async fn get_wallet(&self, name: Option<&str>) -> Result<Wallet, crate::Error> {
        let wallets = self.wallets.read().await;
        
        let wallet_name = if let Some(name) = name {
            name.to_string()
        } else {
            let default = self.default_wallet.read().await;
            default.clone().ok_or_else(|| {
                crate::Error::Wallet("No default wallet set".to_string())
            })?
        };

        wallets
            .get(&wallet_name)
            .cloned()
            .ok_or_else(|| crate::Error::Wallet(format!("Wallet '{}' not found", wallet_name)))
    }

    pub async fn get_wallet_mut(&self, name: Option<&str>) -> Result<tokio::sync::RwLockWriteGuard<HashMap<String, Wallet>>, crate::Error> {
        let wallet_name = if let Some(name) = name {
            name.to_string()
        } else {
            let default = self.default_wallet.read().await;
            default.clone().ok_or_else(|| {
                crate::Error::Wallet("No default wallet set".to_string())
            })?
        };

        let mut wallets = self.wallets.write().await;
        
        if !wallets.contains_key(&wallet_name) {
            return Err(crate::Error::Wallet(format!("Wallet '{}' not found", wallet_name)));
        }

        Ok(wallets)
    }

    pub async fn list_wallets(&self) -> Vec<String> {
        let wallets = self.wallets.read().await;
        wallets.keys().cloned().collect()
    }

    pub async fn set_default_wallet(&self, name: &str) -> Result<(), crate::Error> {
        let wallets = self.wallets.read().await;
        
        if !wallets.contains_key(name) {
            return Err(crate::Error::Wallet(format!("Wallet '{}' not found", name)));
        }

        *self.default_wallet.write().await = Some(name.to_string());
        Ok(())
    }

    pub async fn get_default_wallet_name(&self) -> Option<String> {
        self.default_wallet.read().await.clone()
    }

    pub async fn update_wallet_balance(&self, name: &str, new_balance: u64) -> Result<(), crate::Error> {
        let mut wallets = self.wallets.write().await;
        
        if let Some(wallet) = wallets.get_mut(name) {
            wallet.update_balance(new_balance);
            Ok(())
        } else {
            Err(crate::Error::Wallet(format!("Wallet '{}' not found", name)))
        }
    }

    pub async fn increment_wallet_nonce(&self, name: &str) -> Result<(), crate::Error> {
        let mut wallets = self.wallets.write().await;
        
        if let Some(wallet) = wallets.get_mut(name) {
            wallet.increment_nonce();
            Ok(())
        } else {
            Err(crate::Error::Wallet(format!("Wallet '{}' not found", name)))
        }
    }
}

impl Default for WalletManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wallet_creation() {
        let wallet = Wallet::new();
        assert!(wallet.address.starts_with('i'));
        assert_eq!(wallet.balance, 0);
        assert_eq!(wallet.nonce, 0);
    }

    #[tokio::test]
    async fn test_wallet_manager() {
        let manager = WalletManager::new();
        
        // Create wallet
        manager.create_wallet("test_wallet".to_string()).await.unwrap();
        
        // List wallets
        let wallets = manager.list_wallets().await;
        assert_eq!(wallets.len(), 1);
        assert_eq!(wallets[0], "test_wallet");
        
        // Get wallet
        let wallet = manager.get_wallet(Some("test_wallet")).await.unwrap();
        assert_eq!(wallet.get_address(), "test_wallet");
    }

    #[tokio::test]
    async fn test_wallet_balance_update() {
        let manager = WalletManager::new();
        manager.create_wallet("test_wallet".to_string()).await.unwrap();
        
        manager.update_wallet_balance("test_wallet", 1000).await.unwrap();
        
        let wallet = manager.get_wallet(Some("test_wallet")).await.unwrap();
        assert_eq!(wallet.get_balance(), 1000);
    }

    #[tokio::test]
    async fn test_wallet_nonce_increment() {
        let manager = WalletManager::new();
        manager.create_wallet("test_wallet".to_string()).await.unwrap();
        
        manager.increment_wallet_nonce("test_wallet").await.unwrap();
        
        let wallet = manager.get_wallet(Some("test_wallet")).await.unwrap();
        assert_eq!(wallet.get_nonce(), 1);
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

use crate::errors::*;
use ippan_types::{Address, Amount};

/// Encrypted private key with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedKey {
    pub ciphertext: String,
    pub nonce: String,
    pub salt: String,
}

/// A single address in the wallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletAddress {
    pub id: Uuid,
    pub address: String,
    pub encrypted_private_key: EncryptedKey,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub balance: u64,
    pub nonce: u64,
}

impl WalletAddress {
    pub fn new(
        address: String,
        encrypted_private_key: EncryptedKey,
        label: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            address,
            encrypted_private_key,
            label,
            created_at: Utc::now(),
            last_used: None,
            balance: 0,
            nonce: 0,
        }
    }
}

/// Wallet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub name: String,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub last_backup: Option<DateTime<Utc>>,
    pub auto_backup: bool,
    pub encryption_enabled: bool,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            name: "IPPAN Wallet".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: Utc::now(),
            last_backup: None,
            auto_backup: true,
            encryption_enabled: true,
        }
    }
}

/// Wallet state and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletState {
    pub config: WalletConfig,
    pub addresses: HashMap<String, WalletAddress>,
    pub master_seed: Option<String>, // Encrypted master seed for HD wallets
    pub address_counter: u64,
    pub is_locked: bool,
    pub last_sync: Option<DateTime<Utc>>,
}

impl WalletState {
    pub fn new(name: String) -> Self {
        Self {
            config: WalletConfig {
                name,
                ..Default::default()
            },
            addresses: HashMap::new(),
            master_seed: None,
            address_counter: 0,
            is_locked: false,
            last_sync: None,
        }
    }

    pub fn add_address(&mut self, address: WalletAddress) {
        self.addresses.insert(address.address.clone(), address);
        self.address_counter += 1;
    }

    pub fn get_address(&self, address: &str) -> Option<&WalletAddress> {
        self.addresses.get(address)
    }

    pub fn get_address_mut(&mut self, address: &str) -> Option<&mut WalletAddress> {
        self.addresses.get_mut(address)
    }

    pub fn remove_address(&mut self, address: &str) -> Option<WalletAddress> {
        self.addresses.remove(address)
    }

    pub fn list_addresses(&self) -> Vec<&WalletAddress> {
        self.addresses.values().collect()
    }

    pub fn get_total_balance(&self) -> u64 {
        self.addresses.values().map(|addr| addr.balance).sum()
    }

    pub fn update_balance(&mut self, address: &str, balance: u64) -> Result<()> {
        if let Some(addr) = self.addresses.get_mut(address) {
            addr.balance = balance;
            Ok(())
        } else {
            Err(WalletError::AddressNotFound(address.to_string()))
        }
    }

    pub fn update_nonce(&mut self, address: &str, nonce: u64) -> Result<()> {
        if let Some(addr) = self.addresses.get_mut(address) {
            addr.nonce = nonce;
            Ok(())
        } else {
            Err(WalletError::AddressNotFound(address.to_string()))
        }
    }

    pub fn mark_address_used(&mut self, address: &str) -> Result<()> {
        if let Some(addr) = self.addresses.get_mut(address) {
            addr.last_used = Some(Utc::now());
            Ok(())
        } else {
            Err(WalletError::AddressNotFound(address.to_string()))
        }
    }
}

/// Transaction metadata for wallet tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransaction {
    pub id: Uuid,
    pub tx_hash: String,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: DateTime<Utc>,
    pub status: TransactionStatus,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
    Cancelled,
}

/// Wallet backup data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBackup {
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub wallet_state: WalletState,
    pub checksum: String,
}

impl WalletBackup {
    pub fn new(wallet_state: WalletState) -> Self {
        let checksum = Self::calculate_checksum(&wallet_state);
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: Utc::now(),
            wallet_state,
            checksum,
        }
    }

    fn calculate_checksum(state: &WalletState) -> String {
        // Simple checksum for integrity verification
        let data = serde_json::to_string(state).unwrap_or_default();
        blake3::hash(data.as_bytes()).to_hex()[..16].to_string()
    }

    pub fn verify_checksum(&self) -> bool {
        let calculated = Self::calculate_checksum(&self.wallet_state);
        calculated == self.checksum
    }
}

// TODO: Implement state management
// - In-memory KV: balances + nonces, deterministic apply by HashTimer
// - Periodic snapshot (e.g., every N rounds)

use std::collections::HashMap;
use ippan_common::{Address, Result, Transaction, PublicKeyBytes, crypto::derive_address};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Account state
#[derive(Debug, Clone)]
pub struct AccountState {
    pub balance: u64,
    pub nonce: u64,
    pub last_updated: u64,
}

/// State manager implementation
pub struct StateManager {
    balances: DashMap<PublicKeyBytes, u64>,
    nonces: DashMap<PublicKeyBytes, u64>,
    accounts: DashMap<PublicKeyBytes, AccountState>,
    snapshots: Arc<RwLock<Vec<StateSnapshot>>>,
    max_snapshots: usize,
}

/// State snapshot for periodic backups
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub timestamp: u64,
    pub balances: HashMap<PublicKeyBytes, u64>,
    pub nonces: HashMap<PublicKeyBytes, u64>,
    pub total_accounts: usize,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            balances: DashMap::new(),
            nonces: DashMap::new(),
            accounts: DashMap::new(),
            snapshots: Arc::new(RwLock::new(Vec::new())),
            max_snapshots: 100,
        }
    }

    /// Apply a transaction to the state
    pub async fn apply_transaction(&mut self, tx: &Transaction) -> Result<bool> {
        // Verify transaction signature
        if !tx.verify()? {
            return Ok(false);
        }

        // Check sender balance
        let sender_balance = self.balances.get(&tx.from_pub).map(|b| *b).unwrap_or(0);
        if sender_balance < tx.amount {
            return Ok(false);
        }

        // Check sender nonce
        let sender_nonce = self.nonces.get(&tx.from_pub).map(|n| *n).unwrap_or(0);
        if tx.nonce != sender_nonce + 1 {
            return Ok(false);
        }

        // Apply transaction atomically
        self.balances.insert(tx.from_pub, sender_balance - tx.amount);
        self.balances.insert(tx.to_addr, self.balances.get(&tx.to_addr).map(|b| *b).unwrap_or(0) + tx.amount);
        self.nonces.insert(tx.from_pub, tx.nonce);

        // Update account state
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        let sender_account = AccountState {
            balance: sender_balance - tx.amount,
            nonce: tx.nonce,
            last_updated: current_time,
        };
        self.accounts.insert(tx.from_pub, sender_account);

        let receiver_balance = self.balances.get(&tx.to_addr).map(|b| *b).unwrap_or(0) + tx.amount;
        let receiver_account = AccountState {
            balance: receiver_balance,
            nonce: self.nonces.get(&tx.to_addr).map(|n| *n).unwrap_or(0),
            last_updated: current_time,
        };
        self.accounts.insert(tx.to_addr, receiver_account);

        tracing::debug!("Applied transaction: {} -> {} (amount: {})", 
            derive_address(&tx.from_pub), 
            derive_address(&tx.to_addr), 
            tx.amount);

        Ok(true)
    }

    /// Get account balance
    pub fn get_balance(&self, pub_key: &PublicKeyBytes) -> u64 {
        self.balances.get(pub_key).map(|b| *b).unwrap_or(0)
    }

    /// Get account nonce
    pub fn get_nonce(&self, pub_key: &PublicKeyBytes) -> u64 {
        self.nonces.get(pub_key).map(|n| *n).unwrap_or(0)
    }

    /// Get account state
    pub fn get_account_state(&self, pub_key: &PublicKeyBytes) -> Option<AccountState> {
        self.accounts.get(pub_key).map(|acc| acc.clone())
    }

    /// Create a new account with initial balance
    pub fn create_account(&mut self, pub_key: PublicKeyBytes, initial_balance: u64) {
        self.balances.insert(pub_key, initial_balance);
        self.nonces.insert(pub_key, 0);
        
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        let account = AccountState {
            balance: initial_balance,
            nonce: 0,
            last_updated: current_time,
        };
        self.accounts.insert(pub_key, account);
    }

    /// Take a snapshot of the current state
    pub async fn take_snapshot(&mut self) -> Result<()> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut balances = HashMap::new();
        let mut nonces = HashMap::new();

        for entry in self.balances.iter() {
            balances.insert(*entry.key(), *entry.value());
        }

        for entry in self.nonces.iter() {
            nonces.insert(*entry.key(), *entry.value());
        }

        let snapshot = StateSnapshot {
            timestamp: current_time,
            balances,
            nonces,
            total_accounts: self.accounts.len(),
        };

        let mut snapshots = self.snapshots.write();
        snapshots.push(snapshot);

        // Keep only the most recent snapshots
        if snapshots.len() > self.max_snapshots {
            snapshots.remove(0);
        }

        tracing::info!("State snapshot taken: {} accounts", self.accounts.len());

        Ok(())
    }

    /// Get the latest snapshot
    pub fn get_latest_snapshot(&self) -> Option<StateSnapshot> {
        let snapshots = self.snapshots.read();
        snapshots.last().cloned()
    }

    /// Get state statistics
    pub fn get_stats(&self) -> StateStats {
        let total_balance: u64 = self.balances.iter().map(|entry| *entry.value()).sum();
        let total_accounts = self.accounts.len();

        StateStats {
            total_balance,
            total_accounts,
            total_transactions: 0, // TODO: Track transaction count
        }
    }

    /// Validate state consistency
    pub fn validate_state(&self) -> Result<bool> {
        // Check that all accounts have consistent balances and nonces
        for entry in self.accounts.iter() {
            let pub_key = entry.key();
            let account = entry.value();

            let stored_balance = self.balances.get(pub_key).map(|b| *b).unwrap_or(0);
            let stored_nonce = self.nonces.get(pub_key).map(|n| *n).unwrap_or(0);

            if stored_balance != account.balance || stored_nonce != account.nonce {
                tracing::error!("State inconsistency detected for account: {}", derive_address(pub_key));
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Get all accounts
    pub fn get_all_accounts(&self) -> Vec<(PublicKeyBytes, AccountState)> {
        self.accounts.iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }

    /// Get total supply
    pub fn get_total_supply(&self) -> u64 {
        self.balances.iter().map(|entry| *entry.value()).sum()
    }
}

#[derive(Debug)]
pub struct StateStats {
    pub total_balance: u64,
    pub total_accounts: usize,
    pub total_transactions: u64,
}

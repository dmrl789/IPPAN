use anyhow::Result;
use ippan_types::{Block, L2Commit, L2ExitRecord, L2Network, Transaction};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sled::{Db, Tree};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Storage errors
#[derive(thiserror::Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] sled::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Block not found")]
    BlockNotFound,
    #[error("Transaction not found")]
    TransactionNotFound,
    #[error("Account not found")]
    AccountNotFound,
}

/// Account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: [u8; 32],
    pub balance: u64,
    pub nonce: u64,
}

/// Storage interface for IPPAN blockchain
pub trait Storage {
    /// Store a block
    fn store_block(&self, block: Block) -> Result<()>;

    /// Get a block by hash
    fn get_block(&self, hash: &[u8; 32]) -> Result<Option<Block>>;

    /// Get a block by height
    fn get_block_by_height(&self, height: u64) -> Result<Option<Block>>;

    /// Store a transaction
    fn store_transaction(&self, tx: Transaction) -> Result<()>;

    /// Get a transaction by hash
    fn get_transaction(&self, hash: &[u8; 32]) -> Result<Option<Transaction>>;

    /// Get the latest block height
    fn get_latest_height(&self) -> Result<u64>;

    /// Get account information
    fn get_account(&self, address: &[u8; 32]) -> Result<Option<Account>>;

    /// Update account information
    fn update_account(&self, account: Account) -> Result<()>;

    /// Get all accounts (for debugging/testing)
    fn get_all_accounts(&self) -> Result<Vec<Account>>;

    /// Get all transactions involving the provided address
    fn get_transactions_by_address(&self, address: &[u8; 32]) -> Result<Vec<Transaction>>;

    /// Total number of stored transactions
    fn get_transaction_count(&self) -> Result<u64>;

    /// Store or update metadata for an L2 network
    fn put_l2_network(&self, network: L2Network) -> Result<()>;

    /// Fetch an L2 network by identifier
    fn get_l2_network(&self, id: &str) -> Result<Option<L2Network>>;

    /// List all registered L2 networks
    fn list_l2_networks(&self) -> Result<Vec<L2Network>>;

    /// Persist a new L2 state commitment
    fn store_l2_commit(&self, commit: L2Commit) -> Result<()>;

    /// List stored L2 commitments, optionally filtered by L2 identifier
    fn list_l2_commits(&self, l2_id: Option<&str>) -> Result<Vec<L2Commit>>;

    /// Persist or update an L2 exit record
    fn store_l2_exit(&self, exit: L2ExitRecord) -> Result<()>;

    /// List L2 exit records, optionally filtered by L2 identifier
    fn list_l2_exits(&self, l2_id: Option<&str>) -> Result<Vec<L2ExitRecord>>;
}

/// Sled-backed persistent storage implementation
pub struct SledStorage {
    db: Db,
    blocks: Tree,
    transactions: Tree,
    accounts: Tree,
    metadata: Tree,
    l2_networks: Tree,
    l2_commits: Tree,
    l2_exits: Tree,
}

impl SledStorage {
    /// Create a new Sled storage instance
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path)?;

        let blocks = db.open_tree("blocks")?;
        let transactions = db.open_tree("transactions")?;
        let accounts = db.open_tree("accounts")?;
        let metadata = db.open_tree("metadata")?;
        let l2_networks = db.open_tree("l2_networks")?;
        let l2_commits = db.open_tree("l2_commits")?;
        let l2_exits = db.open_tree("l2_exits")?;

        Ok(Self {
            db,
            blocks,
            transactions,
            accounts,
            metadata,
            l2_networks,
            l2_commits,
            l2_exits,
        })
    }

    /// Initialize with genesis block if needed
    pub fn initialize(&self) -> Result<()> {
        // Check if we already have blocks
        if self.get_latest_height()? == 0 {
            // Create genesis block
            let genesis_block = Block::new(
                Vec::new(), // No parents in genesis
                vec![],     // No transactions
                0,          // Genesis round
                [0u8; 32],  // Genesis proposer
            );

            self.store_block(genesis_block)?;

            // Create genesis account with initial balance
            let genesis_account = Account {
                address: [0u8; 32],
                balance: 1_000_000_000, // 1 billion tokens
                nonce: 0,
            };
            self.update_account(genesis_account)?;

            tracing::info!("Initialized with genesis block and account");
        }

        Ok(())
    }

    /// Flush all pending writes to disk
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }
}

impl Storage for SledStorage {
    fn store_block(&self, block: Block) -> Result<()> {
        let hash = block.hash();
        let height = block.header.round;

        // Serialize block
        let block_data = serde_json::to_vec(&block)?;

        // Store by hash
        self.blocks.insert(&hash[..], block_data.as_slice())?;

        // Store by height for quick lookup
        let height_key = height.to_be_bytes();
        let height_prefix = b"height_";
        let mut height_key_bytes = Vec::with_capacity(height_prefix.len() + height_key.len());
        height_key_bytes.extend_from_slice(height_prefix);
        height_key_bytes.extend_from_slice(&height_key);
        self.blocks.insert(&height_key_bytes, &hash[..])?;

        // Update latest height
        let latest_height = self.get_latest_height()?.max(height);
        self.metadata
            .insert(b"latest_height", &latest_height.to_be_bytes())?;

        tracing::debug!("Stored block {} at height {}", hex::encode(hash), height);
        Ok(())
    }

    fn get_block(&self, hash: &[u8; 32]) -> Result<Option<Block>> {
        if let Some(data) = self.blocks.get(&hash[..])? {
            let block: Block = serde_json::from_slice(&data)?;
            Ok(Some(block))
        } else {
            Ok(None)
        }
    }

    fn get_block_by_height(&self, height: u64) -> Result<Option<Block>> {
        let height_key = height.to_be_bytes();
        let height_prefix = b"height_";
        let mut height_key_bytes = Vec::with_capacity(height_prefix.len() + height_key.len());
        height_key_bytes.extend_from_slice(height_prefix);
        height_key_bytes.extend_from_slice(&height_key);

        if let Some(hash_data) = self.blocks.get(&height_key_bytes)? {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&hash_data);
            self.get_block(&hash)
        } else {
            Ok(None)
        }
    }

    fn store_transaction(&self, tx: Transaction) -> Result<()> {
        let hash = tx.hash();
        let tx_data = serde_json::to_vec(&tx)?;

        self.transactions.insert(&hash[..], tx_data.as_slice())?;

        tracing::debug!("Stored transaction {}", hex::encode(hash));
        Ok(())
    }

    fn get_transaction(&self, hash: &[u8; 32]) -> Result<Option<Transaction>> {
        if let Some(data) = self.transactions.get(&hash[..])? {
            let tx: Transaction = serde_json::from_slice(&data)?;
            Ok(Some(tx))
        } else {
            Ok(None)
        }
    }

    fn get_latest_height(&self) -> Result<u64> {
        if let Some(data) = self.metadata.get(b"latest_height")? {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&data);
            Ok(u64::from_be_bytes(bytes))
        } else {
            Ok(0)
        }
    }

    fn get_account(&self, address: &[u8; 32]) -> Result<Option<Account>> {
        if let Some(data) = self.accounts.get(&address[..])? {
            let account: Account = serde_json::from_slice(&data)?;
            Ok(Some(account))
        } else {
            Ok(None)
        }
    }

    fn update_account(&self, account: Account) -> Result<()> {
        let account_data = serde_json::to_vec(&account)?;
        self.accounts
            .insert(&account.address[..], account_data.as_slice())?;

        tracing::debug!(
            "Updated account {} with balance {}",
            hex::encode(account.address),
            account.balance
        );
        Ok(())
    }

    fn get_all_accounts(&self) -> Result<Vec<Account>> {
        let mut accounts = Vec::new();

        for result in self.accounts.iter() {
            let (_, data) = result?;
            let account: Account = serde_json::from_slice(&data)?;
            accounts.push(account);
        }

        Ok(accounts)
    }

    fn get_transactions_by_address(&self, address: &[u8; 32]) -> Result<Vec<Transaction>> {
        let mut results = Vec::new();

        for item in self.transactions.iter() {
            let (_, data) = item?;
            let tx: Transaction = serde_json::from_slice(&data)?;
            if tx.from == *address || tx.to == *address {
                results.push(tx);
            }
        }

        Ok(results)
    }

    fn get_transaction_count(&self) -> Result<u64> {
        Ok(self.transactions.len() as u64)
    }

    fn put_l2_network(&self, network: L2Network) -> Result<()> {
        let data = serde_json::to_vec(&network)?;
        self.l2_networks
            .insert(network.id.as_bytes(), data.as_slice())?;
        Ok(())
    }

    fn get_l2_network(&self, id: &str) -> Result<Option<L2Network>> {
        if let Some(value) = self.l2_networks.get(id.as_bytes())? {
            let network: L2Network = serde_json::from_slice(&value)?;
            Ok(Some(network))
        } else {
            Ok(None)
        }
    }

    fn list_l2_networks(&self) -> Result<Vec<L2Network>> {
        let mut networks = Vec::new();
        for entry in self.l2_networks.iter() {
            let (_, value) = entry?;
            let network: L2Network = serde_json::from_slice(&value)?;
            networks.push(network);
        }
        networks.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(networks)
    }

    fn store_l2_commit(&self, commit: L2Commit) -> Result<()> {
        let data = serde_json::to_vec(&commit)?;
        self.l2_commits
            .insert(commit.id.as_bytes(), data.as_slice())?;
        Ok(())
    }

    fn list_l2_commits(&self, l2_id: Option<&str>) -> Result<Vec<L2Commit>> {
        let mut commits = Vec::new();
        for entry in self.l2_commits.iter() {
            let (_, value) = entry?;
            let commit: L2Commit = serde_json::from_slice(&value)?;
            if l2_id.map(|id| id == commit.l2_id).unwrap_or(true) {
                commits.push(commit);
            }
        }
        commits.sort_by(|a, b| a.epoch.cmp(&b.epoch));
        Ok(commits)
    }

    fn store_l2_exit(&self, exit: L2ExitRecord) -> Result<()> {
        let data = serde_json::to_vec(&exit)?;
        self.l2_exits.insert(exit.id.as_bytes(), data.as_slice())?;
        Ok(())
    }

    fn list_l2_exits(&self, l2_id: Option<&str>) -> Result<Vec<L2ExitRecord>> {
        let mut exits = Vec::new();
        for entry in self.l2_exits.iter() {
            let (_, value) = entry?;
            let exit: L2ExitRecord = serde_json::from_slice(&value)?;
            if l2_id.map(|id| id == exit.l2_id).unwrap_or(true) {
                exits.push(exit);
            }
        }
        exits.sort_by(|a, b| a.submitted_at.cmp(&b.submitted_at));
        Ok(exits)
    }
}

/// In-memory storage implementation (for testing/development)
pub struct MemoryStorage {
    blocks: Arc<RwLock<HashMap<String, Block>>>,
    transactions: Arc<RwLock<HashMap<String, Transaction>>>,
    accounts: Arc<RwLock<HashMap<String, Account>>>,
    latest_height: Arc<RwLock<u64>>,
    l2_networks: Arc<RwLock<HashMap<String, L2Network>>>,
    l2_commits: Arc<RwLock<HashMap<String, L2Commit>>>,
    l2_exits: Arc<RwLock<HashMap<String, L2ExitRecord>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            blocks: Arc::new(RwLock::new(HashMap::new())),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            accounts: Arc::new(RwLock::new(HashMap::new())),
            latest_height: Arc::new(RwLock::new(0)),
            l2_networks: Arc::new(RwLock::new(HashMap::new())),
            l2_commits: Arc::new(RwLock::new(HashMap::new())),
            l2_exits: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage for MemoryStorage {
    fn store_block(&self, block: Block) -> Result<()> {
        let hash = block.hash();
        let hash_str = hex::encode(hash);

        self.blocks.write().insert(hash_str, block);
        *self.latest_height.write() = self.blocks.read().len() as u64 - 1;

        Ok(())
    }

    fn get_block(&self, hash: &[u8; 32]) -> Result<Option<Block>> {
        let hash_str = hex::encode(hash);
        Ok(self.blocks.read().get(&hash_str).cloned())
    }

    fn get_block_by_height(&self, height: u64) -> Result<Option<Block>> {
        Ok(self
            .blocks
            .read()
            .values()
            .find(|b| b.header.round == height)
            .cloned())
    }

    fn store_transaction(&self, tx: Transaction) -> Result<()> {
        let hash = tx.hash();
        let hash_str = hex::encode(hash);

        self.transactions.write().insert(hash_str, tx);
        Ok(())
    }

    fn get_transaction(&self, hash: &[u8; 32]) -> Result<Option<Transaction>> {
        let hash_str = hex::encode(hash);
        Ok(self.transactions.read().get(&hash_str).cloned())
    }

    fn get_latest_height(&self) -> Result<u64> {
        Ok(*self.latest_height.read())
    }

    fn get_account(&self, address: &[u8; 32]) -> Result<Option<Account>> {
        let addr_str = hex::encode(address);
        Ok(self.accounts.read().get(&addr_str).cloned())
    }

    fn update_account(&self, account: Account) -> Result<()> {
        let addr_str = hex::encode(account.address);
        self.accounts.write().insert(addr_str, account);
        Ok(())
    }

    fn get_all_accounts(&self) -> Result<Vec<Account>> {
        Ok(self.accounts.read().values().cloned().collect())
    }

    fn get_transactions_by_address(&self, address: &[u8; 32]) -> Result<Vec<Transaction>> {
        let transactions = self.transactions.read();

        Ok(transactions
            .values()
            .filter(|tx| tx.from == *address || tx.to == *address)
            .cloned()
            .collect())
    }

    fn get_transaction_count(&self) -> Result<u64> {
        Ok(self.transactions.read().len() as u64)
    }

    fn put_l2_network(&self, network: L2Network) -> Result<()> {
        self.l2_networks.write().insert(network.id.clone(), network);
        Ok(())
    }

    fn get_l2_network(&self, id: &str) -> Result<Option<L2Network>> {
        Ok(self.l2_networks.read().get(id).cloned())
    }

    fn list_l2_networks(&self) -> Result<Vec<L2Network>> {
        let mut networks: Vec<L2Network> = self.l2_networks.read().values().cloned().collect();
        networks.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(networks)
    }

    fn store_l2_commit(&self, commit: L2Commit) -> Result<()> {
        self.l2_commits.write().insert(commit.id.clone(), commit);
        Ok(())
    }

    fn list_l2_commits(&self, l2_id: Option<&str>) -> Result<Vec<L2Commit>> {
        let mut commits: Vec<L2Commit> = self
            .l2_commits
            .read()
            .values()
            .filter(|commit| l2_id.map(|id| id == commit.l2_id).unwrap_or(true))
            .cloned()
            .collect();
        commits.sort_by(|a, b| a.epoch.cmp(&b.epoch));
        Ok(commits)
    }

    fn store_l2_exit(&self, exit: L2ExitRecord) -> Result<()> {
        self.l2_exits.write().insert(exit.id.clone(), exit);
        Ok(())
    }

    fn list_l2_exits(&self, l2_id: Option<&str>) -> Result<Vec<L2ExitRecord>> {
        let mut exits: Vec<L2ExitRecord> = self
            .l2_exits
            .read()
            .values()
            .filter(|exit| l2_id.map(|id| id == exit.l2_id).unwrap_or(true))
            .cloned()
            .collect();
        exits.sort_by(|a, b| a.submitted_at.cmp(&b.submitted_at));
        Ok(exits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::Block;
    use tempfile::tempdir;

    #[test]
    fn test_sled_storage() {
        let temp_dir = tempdir().unwrap();
        let storage = SledStorage::new(temp_dir.path()).unwrap();
        storage.initialize().unwrap();

        // Test storing and retrieving a block
        let block = Block::new(vec![[1u8; 32]], vec![], 1, [2u8; 32]);
        let block_hash = block.hash();

        storage.store_block(block.clone()).unwrap();
        let retrieved_block = storage.get_block(&block_hash).unwrap();

        assert!(retrieved_block.is_some());
        assert_eq!(retrieved_block.unwrap().header.round, 1);

        // Test height lookup
        let block_by_height = storage.get_block_by_height(1).unwrap();
        assert!(block_by_height.is_some());
        assert_eq!(block_by_height.unwrap().hash(), block_hash);

        // Test latest height
        assert_eq!(storage.get_latest_height().unwrap(), 1);
    }

    #[test]
    fn test_account_storage() {
        let temp_dir = tempdir().unwrap();
        let storage = SledStorage::new(temp_dir.path()).unwrap();
        storage.initialize().unwrap();

        let account = Account {
            address: [1u8; 32],
            balance: 1000,
            nonce: 5,
        };

        storage.update_account(account.clone()).unwrap();
        let retrieved = storage.get_account(&account.address).unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().balance, 1000);
    }
}

use anyhow::Result;
use ippan_types::{Block, Transaction};
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
}

/// Sled-backed persistent storage implementation
pub struct SledStorage {
    db: Db,
    blocks: Tree,
    transactions: Tree,
    accounts: Tree,
    metadata: Tree,
}

impl SledStorage {
    /// Create a new Sled storage instance
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path)?;

        let blocks = db.open_tree("blocks")?;
        let transactions = db.open_tree("transactions")?;
        let accounts = db.open_tree("accounts")?;
        let metadata = db.open_tree("metadata")?;

        Ok(Self {
            db,
            blocks,
            transactions,
            accounts,
            metadata,
        })
    }

    /// Initialize with genesis block if needed
    pub fn initialize(&self) -> Result<()> {
        // Check if we already have blocks
        if self.get_latest_height()? == 0 {
            // Create genesis block
            let genesis_block = Block::new(
                [0u8; 32], // No previous block
                vec![],    // No transactions
                0,         // Genesis round
                [0u8; 32], // Genesis proposer
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
        let height = block.header.round_id;

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
}

/// In-memory storage implementation (for testing/development)
pub struct MemoryStorage {
    blocks: Arc<RwLock<HashMap<String, Block>>>,
    transactions: Arc<RwLock<HashMap<String, Transaction>>>,
    accounts: Arc<RwLock<HashMap<String, Account>>>,
    latest_height: Arc<RwLock<u64>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            blocks: Arc::new(RwLock::new(HashMap::new())),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            accounts: Arc::new(RwLock::new(HashMap::new())),
            latest_height: Arc::new(RwLock::new(0)),
        }
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
            .find(|b| b.header.round_id == height)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::{Block, Transaction};
    use tempfile::tempdir;

    #[test]
    fn test_sled_storage() {
        let temp_dir = tempdir().unwrap();
        let storage = SledStorage::new(temp_dir.path()).unwrap();
        storage.initialize().unwrap();

        // Test storing and retrieving a block
        let block = Block::new([1u8; 32], vec![], 1, [2u8; 32]);
        let block_hash = block.hash();

        storage.store_block(block.clone()).unwrap();
        let retrieved_block = storage.get_block(&block_hash).unwrap();

        assert!(retrieved_block.is_some());
        assert_eq!(retrieved_block.unwrap().header.round_id, 1);

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

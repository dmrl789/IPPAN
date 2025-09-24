use anyhow::Result;
use ippan_types::{Block, IppanTimeMicros, L2Exit, L2ExitStatus, L2StateCommit, Transaction};
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
    #[error("L2 commitment not found")]
    L2CommitNotFound,
    #[error("L2 exit not found")]
    L2ExitNotFound,
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

    /// Persist an L2 state commitment anchor.
    fn store_l2_commit(&self, commit: L2StateCommit) -> Result<()>;

    /// Fetch a commitment by identifier.
    fn get_l2_commit(&self, id: &[u8; 32]) -> Result<Option<L2StateCommit>>;

    /// List commitments, optionally filtered by rollup identifier.
    fn get_l2_commits(&self, l2_id: Option<&str>) -> Result<Vec<L2StateCommit>>;

    /// Persist an L2 exit request.
    fn store_l2_exit(&self, exit: L2Exit) -> Result<()>;

    /// Fetch an exit request by identifier.
    fn get_l2_exit(&self, id: &[u8; 32]) -> Result<Option<L2Exit>>;

    /// List exit requests, optionally filtered by rollup identifier.
    fn get_l2_exits(&self, l2_id: Option<&str>) -> Result<Vec<L2Exit>>;

    /// Update the status of an exit request.
    fn update_l2_exit_status(
        &self,
        id: &[u8; 32],
        status: L2ExitStatus,
        finalized_at: Option<IppanTimeMicros>,
        rejection_reason: Option<String>,
    ) -> Result<()>;
}

/// Sled-backed persistent storage implementation
pub struct SledStorage {
    db: Db,
    blocks: Tree,
    transactions: Tree,
    accounts: Tree,
    metadata: Tree,
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
        let l2_commits = db.open_tree("l2_commits")?;
        let l2_exits = db.open_tree("l2_exits")?;

        Ok(Self {
            db,
            blocks,
            transactions,
            accounts,
            metadata,
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

    fn store_l2_commit(&self, commit: L2StateCommit) -> Result<()> {
        let key = commit.id;
        let data = serde_json::to_vec(&commit)?;
        self.l2_commits.insert(&key[..], data)?;
        Ok(())
    }

    fn get_l2_commit(&self, id: &[u8; 32]) -> Result<Option<L2StateCommit>> {
        if let Some(data) = self.l2_commits.get(&id[..])? {
            let commit: L2StateCommit = serde_json::from_slice(&data)?;
            Ok(Some(commit))
        } else {
            Ok(None)
        }
    }

    fn get_l2_commits(&self, l2_id: Option<&str>) -> Result<Vec<L2StateCommit>> {
        let mut commits = Vec::new();

        for item in self.l2_commits.iter() {
            let (_, data) = item?;
            let commit: L2StateCommit = serde_json::from_slice(&data)?;
            if l2_id
                .map(|expected| commit.l2_id == expected)
                .unwrap_or(true)
            {
                commits.push(commit);
            }
        }

        commits.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(commits)
    }

    fn store_l2_exit(&self, exit: L2Exit) -> Result<()> {
        let key = exit.id;
        let data = serde_json::to_vec(&exit)?;
        self.l2_exits.insert(&key[..], data)?;
        Ok(())
    }

    fn get_l2_exit(&self, id: &[u8; 32]) -> Result<Option<L2Exit>> {
        if let Some(data) = self.l2_exits.get(&id[..])? {
            let exit: L2Exit = serde_json::from_slice(&data)?;
            Ok(Some(exit))
        } else {
            Ok(None)
        }
    }

    fn get_l2_exits(&self, l2_id: Option<&str>) -> Result<Vec<L2Exit>> {
        let mut exits = Vec::new();

        for item in self.l2_exits.iter() {
            let (_, data) = item?;
            let exit: L2Exit = serde_json::from_slice(&data)?;
            if l2_id.map(|expected| exit.l2_id == expected).unwrap_or(true) {
                exits.push(exit);
            }
        }

        exits.sort_by(|a, b| b.submitted_at.cmp(&a.submitted_at));
        Ok(exits)
    }

    fn update_l2_exit_status(
        &self,
        id: &[u8; 32],
        status: L2ExitStatus,
        finalized_at: Option<IppanTimeMicros>,
        rejection_reason: Option<String>,
    ) -> Result<()> {
        let mut exit = self.get_l2_exit(id)?.ok_or(StorageError::L2ExitNotFound)?;

        exit.status = status;
        exit.finalized_at = finalized_at.or_else(|| {
            if matches!(
                exit.status,
                L2ExitStatus::Finalized | L2ExitStatus::Rejected
            ) {
                Some(IppanTimeMicros::now())
            } else {
                None
            }
        });
        exit.rejection_reason = rejection_reason;
        exit.refresh_id();

        // Use original identifier as the key to preserve references.
        let data = serde_json::to_vec(&exit)?;
        self.l2_exits.insert(&id[..], data)?;
        Ok(())
    }
}

/// In-memory storage implementation (for testing/development)
pub struct MemoryStorage {
    blocks: Arc<RwLock<HashMap<String, Block>>>,
    transactions: Arc<RwLock<HashMap<String, Transaction>>>,
    accounts: Arc<RwLock<HashMap<String, Account>>>,
    latest_height: Arc<RwLock<u64>>,
    l2_commits: Arc<RwLock<HashMap<String, L2StateCommit>>>,
    l2_exits: Arc<RwLock<HashMap<String, L2Exit>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            blocks: Arc::new(RwLock::new(HashMap::new())),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            accounts: Arc::new(RwLock::new(HashMap::new())),
            latest_height: Arc::new(RwLock::new(0)),
            l2_commits: Arc::new(RwLock::new(HashMap::new())),
            l2_exits: Arc::new(RwLock::new(HashMap::new())),
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

    fn store_l2_commit(&self, commit: L2StateCommit) -> Result<()> {
        let key = hex::encode(commit.id);
        self.l2_commits.write().insert(key, commit);
        Ok(())
    }

    fn get_l2_commit(&self, id: &[u8; 32]) -> Result<Option<L2StateCommit>> {
        let key = hex::encode(id);
        Ok(self.l2_commits.read().get(&key).cloned())
    }

    fn get_l2_commits(&self, l2_id: Option<&str>) -> Result<Vec<L2StateCommit>> {
        let mut commits: Vec<_> = self
            .l2_commits
            .read()
            .values()
            .filter(|commit| {
                l2_id
                    .map(|expected| commit.l2_id == expected)
                    .unwrap_or(true)
            })
            .cloned()
            .collect();
        commits.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(commits)
    }

    fn store_l2_exit(&self, exit: L2Exit) -> Result<()> {
        let key = hex::encode(exit.id);
        self.l2_exits.write().insert(key, exit);
        Ok(())
    }

    fn get_l2_exit(&self, id: &[u8; 32]) -> Result<Option<L2Exit>> {
        let key = hex::encode(id);
        Ok(self.l2_exits.read().get(&key).cloned())
    }

    fn get_l2_exits(&self, l2_id: Option<&str>) -> Result<Vec<L2Exit>> {
        let mut exits: Vec<_> = self
            .l2_exits
            .read()
            .values()
            .filter(|exit| l2_id.map(|expected| exit.l2_id == expected).unwrap_or(true))
            .cloned()
            .collect();
        exits.sort_by(|a, b| b.submitted_at.cmp(&a.submitted_at));
        Ok(exits)
    }

    fn update_l2_exit_status(
        &self,
        id: &[u8; 32],
        status: L2ExitStatus,
        finalized_at: Option<IppanTimeMicros>,
        rejection_reason: Option<String>,
    ) -> Result<()> {
        let key = hex::encode(id);
        let mut exits = self.l2_exits.write();
        let exit = exits.get_mut(&key).ok_or(StorageError::L2ExitNotFound)?;
        exit.status = status;
        exit.finalized_at = finalized_at.or_else(|| {
            if matches!(
                exit.status,
                L2ExitStatus::Finalized | L2ExitStatus::Rejected
            ) {
                Some(IppanTimeMicros::now())
            } else {
                None
            }
        });
        exit.rejection_reason = rejection_reason;
        exit.refresh_id();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::{Block, L2Exit, L2ExitStatus, L2ProofType, L2StateCommit};
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

    #[test]
    fn test_l2_commit_storage_roundtrip() {
        let temp_dir = tempdir().unwrap();
        let storage = SledStorage::new(temp_dir.path()).unwrap();
        storage.initialize().unwrap();

        let commit = L2StateCommit::new(
            "rollup-1",
            42,
            "0xabc",
            None,
            L2ProofType::Optimistic,
            Some("proof".to_string()),
            None,
            b"node",
        );

        storage.store_l2_commit(commit.clone()).unwrap();
        let stored = storage.get_l2_commit(&commit.id).unwrap();
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().epoch, 42);

        let filtered = storage.get_l2_commits(Some("rollup-1")).unwrap();
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_l2_exit_storage_and_update() {
        let storage = MemoryStorage::new();

        let exit = L2Exit::new("rollup-1", 7, "0xabc", 10_000, 1, "proof", b"node");
        let exit_id = exit.id;
        storage.store_l2_exit(exit).unwrap();

        let fetched = storage.get_l2_exit(&exit_id).unwrap().unwrap();
        assert_eq!(fetched.status, L2ExitStatus::Pending);

        storage
            .update_l2_exit_status(&exit_id, L2ExitStatus::ChallengeWindow, None, None)
            .unwrap();

        let updated = storage.get_l2_exit(&exit_id).unwrap().unwrap();
        assert_eq!(updated.status, L2ExitStatus::ChallengeWindow);
    }
}

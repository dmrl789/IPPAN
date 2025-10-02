mod retention;

use anyhow::Result;
use ippan_types::{
    Block, BlockReceipts, IppanTimeMicros, L2Commit, L2ExitRecord, L2Network, StateSnapshot,
    Transaction,
};
use parking_lot::RwLock;
pub use retention::{PruneReport, RetentionPolicies, RetentionPolicy, RetentionTarget};
use serde::{Deserialize, Serialize};
use sled::{Db, Tree};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;

const HEIGHT_PREFIX: &[u8] = b"height_";
const SNAPSHOT_PREFIX: &[u8] = b"snapshot_";

fn make_height_key(height: u64) -> Vec<u8> {
    let mut key = Vec::with_capacity(HEIGHT_PREFIX.len() + 8);
    key.extend_from_slice(HEIGHT_PREFIX);
    key.extend_from_slice(&height.to_be_bytes());
    key
}

fn parse_height_from_key(key: &[u8]) -> Option<u64> {
    if key.len() != HEIGHT_PREFIX.len() + 8 || !key.starts_with(HEIGHT_PREFIX) {
        return None;
    }

    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&key[HEIGHT_PREFIX.len()..]);
    Some(u64::from_be_bytes(bytes))
}

fn make_snapshot_key(height: u64) -> Vec<u8> {
    let mut key = Vec::with_capacity(SNAPSHOT_PREFIX.len() + 8);
    key.extend_from_slice(SNAPSHOT_PREFIX);
    key.extend_from_slice(&height.to_be_bytes());
    key
}

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

    /// Store receipts for a block
    fn store_block_receipts(&self, receipts: BlockReceipts) -> Result<()>;

    /// Retrieve receipts for a block hash
    fn get_block_receipts(&self, hash: &[u8; 32]) -> Result<Option<BlockReceipts>>;

    /// Persist a signed state snapshot checkpoint
    fn store_state_snapshot(&self, snapshot: StateSnapshot) -> Result<()>;

    /// Fetch the state snapshot at a specific height if available
    fn get_state_snapshot(&self, height: u64) -> Result<Option<StateSnapshot>>;

    /// Fetch the latest available snapshot
    fn get_latest_snapshot(&self) -> Result<Option<StateSnapshot>>;

    /// List recent snapshots, returning up to `limit` entries ordered from newest to oldest
    fn list_state_snapshots(&self, limit: usize) -> Result<Vec<StateSnapshot>>;

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

    /// Apply retention policies across the tracked data stores and return pruning stats
    fn apply_retention(&self, policies: &RetentionPolicies) -> Result<Vec<PruneReport>>;
}

/// Sled-backed persistent storage implementation
pub struct SledStorage {
    db: Db,
    blocks: Tree,
    transactions: Tree,
    accounts: Tree,
    metadata: Tree,
    receipts: Tree,
    snapshots: Tree,
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
        let receipts = db.open_tree("receipts")?;
        let snapshots = db.open_tree("snapshots")?;
        let l2_networks = db.open_tree("l2_networks")?;
        let l2_commits = db.open_tree("l2_commits")?;
        let l2_exits = db.open_tree("l2_exits")?;

        Ok(Self {
            db,
            blocks,
            transactions,
            accounts,
            metadata,
            receipts,
            snapshots,
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

    fn refresh_latest_height(&self) -> Result<()> {
        let mut max_height = 0;
        for entry in self.blocks.scan_prefix(HEIGHT_PREFIX) {
            let (key, _) = entry?;
            if let Some(height) = parse_height_from_key(&key) {
                if height > max_height {
                    max_height = height;
                }
            }
        }

        self.metadata
            .insert(b"latest_height", &max_height.to_be_bytes())?;
        Ok(())
    }

    fn prune_blocks(
        &self,
        policy: RetentionPolicy,
        now_us: u64,
        latest_height: u64,
    ) -> Result<PruneReport> {
        let mut report = PruneReport::new(RetentionTarget::BlockBodies);

        for entry in self.blocks.scan_prefix(HEIGHT_PREFIX) {
            let (key, value) = entry?;
            let Some(height) = parse_height_from_key(&key) else {
                continue;
            };

            let mut hash = [0u8; 32];
            if value.len() == 32 {
                hash.copy_from_slice(&value);
            } else {
                continue;
            }

            let block = self.get_block(&hash)?;
            let timestamp_us = block
                .as_ref()
                .map(|b| b.header.hashtimer.time().0)
                .unwrap_or(0);

            if policy.should_prune(height, timestamp_us, latest_height, now_us) {
                self.blocks.remove(&hash[..])?;
                self.blocks.remove(&key)?;
                report.pruned_entries += 1;
            } else {
                report.retained_entries += 1;
            }
        }

        self.refresh_latest_height()?;
        Ok(report)
    }

    fn prune_receipts(
        &self,
        policy: RetentionPolicy,
        now_us: u64,
        latest_height: u64,
    ) -> Result<PruneReport> {
        let mut report = PruneReport::new(RetentionTarget::Receipts);

        for entry in self.receipts.iter() {
            let (key, value) = entry?;
            let receipts: BlockReceipts = serde_json::from_slice(&value)?;
            let timestamp_us = receipts.timestamp.0;

            if policy.should_prune(receipts.block_height, timestamp_us, latest_height, now_us) {
                self.receipts.remove(&key)?;
                report.pruned_entries += 1;
            } else {
                report.retained_entries += 1;
            }
        }

        Ok(report)
    }

    fn prune_snapshots(
        &self,
        policy: RetentionPolicy,
        now_us: u64,
        latest_height: u64,
    ) -> Result<PruneReport> {
        let mut report = PruneReport::new(RetentionTarget::Snapshots);

        for entry in self.snapshots.iter() {
            let (key, value) = entry?;
            let snapshot: StateSnapshot = serde_json::from_slice(&value)?;
            let timestamp_us = snapshot.produced_at.0;

            if policy.should_prune(snapshot.block_height, timestamp_us, latest_height, now_us) {
                self.snapshots.remove(&key)?;
                report.pruned_entries += 1;
            } else {
                report.retained_entries += 1;
            }
        }

        Ok(report)
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
        let height_key = make_height_key(height);
        self.blocks.insert(height_key, &hash[..])?;

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
        let height_key = make_height_key(height);

        if let Some(hash_data) = self.blocks.get(height_key)? {
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

    fn store_block_receipts(&self, receipts: BlockReceipts) -> Result<()> {
        let key = receipts.block_hash;
        let data = serde_json::to_vec(&receipts)?;
        self.receipts.insert(&key[..], data)?;
        Ok(())
    }

    fn get_block_receipts(&self, hash: &[u8; 32]) -> Result<Option<BlockReceipts>> {
        if let Some(data) = self.receipts.get(&hash[..])? {
            let receipts: BlockReceipts = serde_json::from_slice(&data)?;
            Ok(Some(receipts))
        } else {
            Ok(None)
        }
    }

    fn store_state_snapshot(&self, snapshot: StateSnapshot) -> Result<()> {
        let key = make_snapshot_key(snapshot.block_height);
        let data = serde_json::to_vec(&snapshot)?;
        self.snapshots.insert(key, data)?;
        Ok(())
    }

    fn get_state_snapshot(&self, height: u64) -> Result<Option<StateSnapshot>> {
        let key = make_snapshot_key(height);
        if let Some(value) = self.snapshots.get(key)? {
            let snapshot: StateSnapshot = serde_json::from_slice(&value)?;
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    fn get_latest_snapshot(&self) -> Result<Option<StateSnapshot>> {
        if let Some(Ok((_, value))) = self.snapshots.iter().next_back() {
            let snapshot: StateSnapshot = serde_json::from_slice(&value)?;
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    fn list_state_snapshots(&self, limit: usize) -> Result<Vec<StateSnapshot>> {
        let mut snapshots = Vec::new();
        for entry in self.snapshots.iter().rev().take(limit) {
            let (_, value) = entry?;
            let snapshot: StateSnapshot = serde_json::from_slice(&value)?;
            snapshots.push(snapshot);
        }
        Ok(snapshots)
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

    fn apply_retention(&self, policies: &RetentionPolicies) -> Result<Vec<PruneReport>> {
        let mut reports = Vec::new();
        let now_us = IppanTimeMicros::now().0;
        let latest_height = self.get_latest_height()?;

        if let Some(policy) = policies.block_bodies {
            if !policy.is_disabled() {
                reports.push(self.prune_blocks(policy, now_us, latest_height)?);
            }
        }

        if let Some(policy) = policies.receipts {
            if !policy.is_disabled() {
                reports.push(self.prune_receipts(policy, now_us, latest_height)?);
            }
        }

        if let Some(policy) = policies.snapshots {
            if !policy.is_disabled() {
                reports.push(self.prune_snapshots(policy, now_us, latest_height)?);
            }
        }

        Ok(reports)
    }
}

/// In-memory storage implementation (for testing/development)
pub struct MemoryStorage {
    blocks: Arc<RwLock<HashMap<String, Block>>>,
    block_heights: Arc<RwLock<BTreeMap<u64, String>>>,
    transactions: Arc<RwLock<HashMap<String, Transaction>>>,
    accounts: Arc<RwLock<HashMap<String, Account>>>,
    latest_height: Arc<RwLock<u64>>,
    receipts: Arc<RwLock<HashMap<String, BlockReceipts>>>,
    snapshots: Arc<RwLock<BTreeMap<u64, StateSnapshot>>>,
    l2_networks: Arc<RwLock<HashMap<String, L2Network>>>,
    l2_commits: Arc<RwLock<HashMap<String, L2Commit>>>,
    l2_exits: Arc<RwLock<HashMap<String, L2ExitRecord>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            blocks: Arc::new(RwLock::new(HashMap::new())),
            block_heights: Arc::new(RwLock::new(BTreeMap::new())),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            accounts: Arc::new(RwLock::new(HashMap::new())),
            latest_height: Arc::new(RwLock::new(0)),
            receipts: Arc::new(RwLock::new(HashMap::new())),
            snapshots: Arc::new(RwLock::new(BTreeMap::new())),
            l2_networks: Arc::new(RwLock::new(HashMap::new())),
            l2_commits: Arc::new(RwLock::new(HashMap::new())),
            l2_exits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn prune_blocks(
        &self,
        policy: RetentionPolicy,
        now_us: u64,
        latest_height: u64,
    ) -> PruneReport {
        let entries: Vec<(u64, String, u64)> = {
            let heights_snapshot: Vec<(u64, String)> = self
                .block_heights
                .read()
                .iter()
                .map(|(height, hash)| (*height, hash.clone()))
                .collect();
            let blocks = self.blocks.read();
            heights_snapshot
                .into_iter()
                .filter_map(|(height, hash)| {
                    blocks
                        .get(&hash)
                        .map(|block| (height, hash, block.header.hashtimer.time().0))
                })
                .collect()
        };

        let mut report = PruneReport::new(RetentionTarget::BlockBodies);
        let mut to_remove = Vec::new();

        for (height, hash, timestamp) in entries.iter() {
            if policy.should_prune(*height, *timestamp, latest_height, now_us) {
                to_remove.push((*height, hash.clone()));
            }
        }

        if !to_remove.is_empty() {
            let mut block_heights = self.block_heights.write();
            let mut blocks = self.blocks.write();
            for (height, hash) in &to_remove {
                block_heights.remove(height);
                blocks.remove(hash);
            }
        }

        report.pruned_entries = to_remove.len() as u64;
        report.retained_entries = self.block_heights.read().len() as u64;

        let new_latest = self.block_heights.read().keys().copied().max().unwrap_or(0);
        *self.latest_height.write() = new_latest;

        report
    }

    fn prune_receipts(
        &self,
        policy: RetentionPolicy,
        now_us: u64,
        latest_height: u64,
    ) -> PruneReport {
        let receipts_snapshot: Vec<(String, BlockReceipts)> = self
            .receipts
            .read()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();

        let mut to_remove = Vec::new();
        for (key, receipts) in receipts_snapshot.iter() {
            if policy.should_prune(
                receipts.block_height,
                receipts.timestamp.0,
                latest_height,
                now_us,
            ) {
                to_remove.push(key.clone());
            }
        }

        if !to_remove.is_empty() {
            let mut receipts = self.receipts.write();
            for key in &to_remove {
                receipts.remove(key);
            }
        }

        let mut report = PruneReport::new(RetentionTarget::Receipts);
        report.pruned_entries = to_remove.len() as u64;
        report.retained_entries = self.receipts.read().len() as u64;
        report
    }

    fn prune_snapshots(
        &self,
        policy: RetentionPolicy,
        now_us: u64,
        latest_height: u64,
    ) -> PruneReport {
        let snapshot_keys: Vec<u64> = self
            .snapshots
            .read()
            .iter()
            .filter_map(|(height, snapshot)| {
                if policy.should_prune(*height, snapshot.produced_at.0, latest_height, now_us) {
                    Some(*height)
                } else {
                    None
                }
            })
            .collect();

        if !snapshot_keys.is_empty() {
            let mut snapshots = self.snapshots.write();
            for height in &snapshot_keys {
                snapshots.remove(height);
            }
        }

        let mut report = PruneReport::new(RetentionTarget::Snapshots);
        report.pruned_entries = snapshot_keys.len() as u64;
        report.retained_entries = self.snapshots.read().len() as u64;
        report
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
        let height = block.header.round;

        self.blocks.write().insert(hash_str.clone(), block);
        self.block_heights.write().insert(height, hash_str);

        let mut latest_height = self.latest_height.write();
        if height > *latest_height {
            *latest_height = height;
        }

        Ok(())
    }

    fn get_block(&self, hash: &[u8; 32]) -> Result<Option<Block>> {
        let hash_str = hex::encode(hash);
        Ok(self.blocks.read().get(&hash_str).cloned())
    }

    fn get_block_by_height(&self, height: u64) -> Result<Option<Block>> {
        if let Some(hash) = self.block_heights.read().get(&height) {
            Ok(self.blocks.read().get(hash).cloned())
        } else {
            Ok(None)
        }
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

    fn store_block_receipts(&self, receipts: BlockReceipts) -> Result<()> {
        let key = hex::encode(receipts.block_hash);
        self.receipts.write().insert(key, receipts);
        Ok(())
    }

    fn get_block_receipts(&self, hash: &[u8; 32]) -> Result<Option<BlockReceipts>> {
        let key = hex::encode(hash);
        Ok(self.receipts.read().get(&key).cloned())
    }

    fn store_state_snapshot(&self, snapshot: StateSnapshot) -> Result<()> {
        self.snapshots
            .write()
            .insert(snapshot.block_height, snapshot);
        Ok(())
    }

    fn get_state_snapshot(&self, height: u64) -> Result<Option<StateSnapshot>> {
        Ok(self.snapshots.read().get(&height).cloned())
    }

    fn get_latest_snapshot(&self) -> Result<Option<StateSnapshot>> {
        Ok(self
            .snapshots
            .read()
            .iter()
            .next_back()
            .map(|(_, v)| v.clone()))
    }

    fn list_state_snapshots(&self, limit: usize) -> Result<Vec<StateSnapshot>> {
        let snapshots = self.snapshots.read();
        let mut result = Vec::new();
        for (_, snapshot) in snapshots.iter().rev().take(limit) {
            result.push(snapshot.clone());
        }
        Ok(result)
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

    fn apply_retention(&self, policies: &RetentionPolicies) -> Result<Vec<PruneReport>> {
        let mut reports = Vec::new();
        let now_us = IppanTimeMicros::now().0;
        let latest_height = self.get_latest_height()?;

        if let Some(policy) = policies.block_bodies {
            if !policy.is_disabled() {
                reports.push(self.prune_blocks(policy, now_us, latest_height));
            }
        }

        if let Some(policy) = policies.receipts {
            if !policy.is_disabled() {
                reports.push(self.prune_receipts(policy, now_us, latest_height));
            }
        }

        if let Some(policy) = policies.snapshots {
            if !policy.is_disabled() {
                reports.push(self.prune_snapshots(policy, now_us, latest_height));
            }
        }

        Ok(reports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::{
        ippan_time_init, Block, BlockReceipts, IppanTimeMicros, SnapshotValidator, StateSnapshot,
        TransactionReceipt,
    };
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

    #[test]
    fn test_receipts_and_snapshots_storage() {
        ippan_time_init();
        let temp_dir = tempdir().unwrap();
        let storage = SledStorage::new(temp_dir.path()).unwrap();
        storage.initialize().unwrap();

        let receipts = BlockReceipts::new(
            [9u8; 32],
            12,
            [7u8; 32],
            IppanTimeMicros::now(),
            vec![TransactionReceipt {
                transaction_hash: [1u8; 32],
                success: true,
                gas_used: 42,
                logs: vec!["transfer".to_string()],
            }],
        );

        storage.store_block_receipts(receipts.clone()).unwrap();
        let fetched = storage.get_block_receipts(&receipts.block_hash).unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().receipts.len(), 1);

        let snapshot = StateSnapshot::unsigned(
            [5u8; 32],
            [4u8; 32],
            12,
            99,
            vec![SnapshotValidator::new([2u8; 32], 1)],
            IppanTimeMicros::now(),
        )
        .with_signature(vec![1, 2, 3]);

        storage.store_state_snapshot(snapshot.clone()).unwrap();

        let fetched_snapshot = storage.get_state_snapshot(snapshot.block_height).unwrap();
        assert!(fetched_snapshot.is_some());
        assert_eq!(fetched_snapshot.unwrap().signature, vec![1, 2, 3]);

        let latest = storage.get_latest_snapshot().unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().block_height, 12);
    }

    #[test]
    fn test_retention_policy_prunes_old_blocks() {
        ippan_time_init();
        let storage = MemoryStorage::new();

        let mut parent_ids = Vec::new();
        for round in 0..3u64 {
            let block = Block::new(parent_ids.clone(), vec![], round, [1u8; 32]);
            parent_ids = vec![block.hash()];
            storage.store_block(block).unwrap();
        }

        let policies = RetentionPolicies {
            block_bodies: Some(RetentionPolicy {
                retain_latest_heights: Some(1),
                retain_duration: None,
            }),
            receipts: None,
            snapshots: None,
        };

        let reports = storage.apply_retention(&policies).unwrap();
        let block_report = reports
            .into_iter()
            .find(|r| matches!(r.target, RetentionTarget::BlockBodies))
            .unwrap();

        assert_eq!(block_report.pruned_entries, 2);
        assert_eq!(block_report.retained_entries, 1);
        assert!(storage.get_block_by_height(0).unwrap().is_none());
        assert!(storage.get_block_by_height(1).unwrap().is_none());
        assert!(storage.get_block_by_height(2).unwrap().is_some());
        assert_eq!(storage.get_latest_height().unwrap(), 2);
    }
}

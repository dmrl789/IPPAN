//! IPPAN persistent storage abstraction layer. Defines the `Storage` trait,
//! Sled-backed node database, and in-memory test backend used across consensus,
//! mempool, and AI telemetry pipelines. Handles blocks, accounts, L2 anchors,
//! and validator telemetry with deterministic serialization.
//!
use anyhow::{anyhow, Result};
use ippan_types::{
    ippan_time_now, Address, Block, ChainState, FileDescriptor, FileDescriptorId, HashTimer,
    L2Commit, L2ExitRecord, L2Network, RoundCertificate, RoundFinalizationRecord, RoundId,
    Transaction,
};
use parking_lot::RwLock;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sled::{Db, Tree};
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
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

/// Errors raised when exporting or importing snapshots.
#[derive(thiserror::Error, Debug)]
pub enum SnapshotError {
    #[error("storage error: {0}")]
    Storage(#[from] anyhow::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("snapshot directory must be empty: {0}")]
    DirectoryNotEmpty(String),
    #[error("snapshot path missing: {0}")]
    MissingSnapshot(String),
    #[error("snapshot manifest invalid: {0}")]
    InvalidManifest(String),
    #[error("network mismatch (expected {expected}, found {actual})")]
    NetworkMismatch { expected: String, actual: String },
    #[error("storage not empty; start from a clean database before importing")]
    StorageNotEmpty,
    #[error("requested snapshot height {requested} does not match storage tip {available}")]
    HeightMismatch { requested: u64, available: u64 },
}

/// Versioned manifest describing snapshot metadata and record counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifest {
    pub version: u32,
    pub network_id: String,
    pub height: u64,
    pub last_round_id: Option<String>,
    pub timestamp_us: u64,
    pub accounts_count: u64,
    pub payments_count: u64,
    pub blocks_count: u64,
    pub handles_count: u64,
    pub files_count: u64,
    pub ai_model_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tip_block_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hashtimer_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hashtimer_end: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_start_us: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_end_us: Option<u64>,
}

/// Minimal @handle representation for snapshot exports.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HandleSnapshotRecord {
    pub handle: String,
    pub owner: String,
    #[serde(default)]
    pub expires_at: u64,
}

const SNAPSHOT_MANIFEST_VERSION: u32 = 2;
const MANIFEST_FILE: &str = "manifest.json";
const BLOCKS_FILE: &str = "blocks.jsonl";
const PAYMENTS_FILE: &str = "payments.jsonl";
const ACCOUNTS_FILE: &str = "accounts.jsonl";
const HANDLES_FILE: &str = "handles.jsonl";
const FILES_FILE: &str = "files.jsonl";
const ROUNDS_FILE: &str = "rounds.jsonl";
const CHAIN_STATE_FILE: &str = "chain_state.json";

impl SnapshotManifest {
    pub fn new_from_storage(storage: &impl StorageLike) -> Result<Self, SnapshotError> {
        let collections = collect_snapshot_data(storage)?;
        let height = storage.get_latest_height()?;
        build_manifest_from_collections(storage, &collections, height)
    }

    pub fn validate_against_storage(
        &self,
        storage: &impl StorageLike,
    ) -> Result<(), SnapshotError> {
        let collections = collect_snapshot_data(storage)?;
        let height = storage.get_latest_height()?;
        if self.network_id != storage.snapshot_network_id() {
            return Err(SnapshotError::NetworkMismatch {
                expected: self.network_id.clone(),
                actual: storage.snapshot_network_id(),
            });
        }
        if self.height != height {
            return Err(SnapshotError::InvalidManifest(format!(
                "height mismatch: manifest={}, storage={}",
                self.height, height
            )));
        }
        if self.blocks_count != collections.blocks.len() as u64 {
            return Err(SnapshotError::InvalidManifest(format!(
                "blocks mismatch: manifest={}, storage={}",
                self.blocks_count,
                collections.blocks.len()
            )));
        }
        if self.accounts_count != collections.accounts.len() as u64 {
            return Err(SnapshotError::InvalidManifest(format!(
                "accounts mismatch: manifest={}, storage={}",
                self.accounts_count,
                collections.accounts.len()
            )));
        }
        if self.payments_count != collections.transactions.len() as u64 {
            return Err(SnapshotError::InvalidManifest(format!(
                "payments mismatch: manifest={}, storage={}",
                self.payments_count,
                collections.transactions.len()
            )));
        }
        if self.handles_count != collections.handles.len() as u64 {
            return Err(SnapshotError::InvalidManifest(format!(
                "handles mismatch: manifest={}, storage={}",
                self.handles_count,
                collections.handles.len()
            )));
        }
        if self.files_count != collections.files.len() as u64 {
            return Err(SnapshotError::InvalidManifest(format!(
                "files mismatch: manifest={}, storage={}",
                self.files_count,
                collections.files.len()
            )));
        }
        if self.ai_model_hash != storage.snapshot_ai_model_hash()? {
            return Err(SnapshotError::InvalidManifest(
                "AI model hash mismatch".to_string(),
            ));
        }
        if self.version >= 2 {
            let bounds = compute_snapshot_bounds(&collections.blocks, self.height);
            if self.tip_block_hash != bounds.tip_block_hash {
                return Err(SnapshotError::InvalidManifest(
                    "tip block hash mismatch".to_string(),
                ));
            }
            if self.hashtimer_start != bounds.hashtimer_start {
                return Err(SnapshotError::InvalidManifest(
                    "hashtimer_start mismatch".to_string(),
                ));
            }
            if self.hashtimer_end != bounds.hashtimer_end {
                return Err(SnapshotError::InvalidManifest(
                    "hashtimer_end mismatch".to_string(),
                ));
            }
            if self.timestamp_start_us != bounds.timestamp_start_us {
                return Err(SnapshotError::InvalidManifest(
                    "timestamp_start_us mismatch".to_string(),
                ));
            }
            if self.timestamp_end_us != bounds.timestamp_end_us {
                return Err(SnapshotError::InvalidManifest(
                    "timestamp_end_us mismatch".to_string(),
                ));
            }
        }
        Ok(())
    }
}

/// Account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: [u8; 32],
    pub balance: u64,
    pub nonce: u64,
}

/// Extended storage interface for snapshot export/import helpers.
pub trait StorageLike: Storage + Send + Sync {
    fn snapshot_network_id(&self) -> String;
    fn snapshot_blocks(&self) -> Result<Vec<Block>>;
    fn snapshot_transactions(&self) -> Result<Vec<Transaction>>;
    fn snapshot_accounts(&self) -> Result<Vec<Account>>;
    fn snapshot_handles(&self) -> Result<Vec<HandleSnapshotRecord>> {
        Ok(Vec::new())
    }
    fn snapshot_file_descriptors(&self) -> Result<Vec<FileDescriptor>>;
    fn snapshot_round_finalizations(&self) -> Result<Vec<RoundFinalizationRecord>>;
    fn snapshot_chain_state(&self) -> Result<ChainState>;
    fn snapshot_ai_model_hash(&self) -> Result<Option<String>> {
        Ok(None)
    }
    fn apply_ai_model_hash(&self, _hash: Option<&str>) -> Result<()> {
        Ok(())
    }
    fn flush_storage(&self) -> Result<()> {
        Ok(())
    }
}

struct SnapshotCollections {
    accounts: Vec<Account>,
    blocks: Vec<Block>,
    transactions: Vec<Transaction>,
    handles: Vec<HandleSnapshotRecord>,
    files: Vec<FileDescriptor>,
    rounds: Vec<RoundFinalizationRecord>,
    chain_state: ChainState,
}

struct SnapshotBounds {
    tip_block_hash: Option<String>,
    hashtimer_start: Option<String>,
    hashtimer_end: Option<String>,
    timestamp_start_us: Option<u64>,
    timestamp_end_us: Option<u64>,
}

fn collect_snapshot_data(storage: &impl StorageLike) -> Result<SnapshotCollections, SnapshotError> {
    storage.flush_storage()?;
    let mut accounts = storage.snapshot_accounts()?;
    sort_accounts(&mut accounts);
    let mut blocks = storage.snapshot_blocks()?;
    sort_blocks(&mut blocks);
    let mut transactions = storage.snapshot_transactions()?;
    sort_transactions(&mut transactions);
    let mut handles = storage.snapshot_handles()?;
    handles.sort_by(|a, b| a.handle.cmp(&b.handle));
    let mut files = storage.snapshot_file_descriptors()?;
    sort_file_descriptors(&mut files);
    let mut rounds = storage.snapshot_round_finalizations()?;
    sort_rounds(&mut rounds);
    Ok(SnapshotCollections {
        accounts,
        blocks,
        transactions,
        handles,
        files,
        rounds,
        chain_state: storage.snapshot_chain_state()?,
    })
}

fn compute_snapshot_bounds(blocks: &[Block], snapshot_height: u64) -> SnapshotBounds {
    let tip_block_hash = blocks
        .iter()
        .rev()
        .find(|block| block.header.round == snapshot_height)
        .map(|block| hex::encode(block.hash()));
    let hashtimer_start = blocks
        .first()
        .map(|block| hex::encode(block.header.hashtimer.digest()));
    let hashtimer_end = blocks
        .last()
        .map(|block| hex::encode(block.header.hashtimer.digest()));
    let timestamp_start_us = blocks
        .first()
        .map(|block| hashtimer_timestamp_us(&block.header.hashtimer));
    let timestamp_end_us = blocks
        .last()
        .map(|block| hashtimer_timestamp_us(&block.header.hashtimer));

    SnapshotBounds {
        tip_block_hash,
        hashtimer_start,
        hashtimer_end,
        timestamp_start_us,
        timestamp_end_us,
    }
}

fn hashtimer_timestamp_us(timer: &HashTimer) -> u64 {
    if timer.timestamp_us.is_negative() {
        0
    } else {
        timer.timestamp_us as u64
    }
}

fn build_manifest_from_collections(
    storage: &impl StorageLike,
    collections: &SnapshotCollections,
    snapshot_height: u64,
) -> Result<SnapshotManifest, SnapshotError> {
    let bounds = compute_snapshot_bounds(&collections.blocks, snapshot_height);
    Ok(SnapshotManifest {
        version: SNAPSHOT_MANIFEST_VERSION,
        network_id: storage.snapshot_network_id(),
        height: snapshot_height,
        last_round_id: collections
            .rounds
            .last()
            .map(|record| record.round.to_string()),
        timestamp_us: ippan_time_now(),
        accounts_count: collections.accounts.len() as u64,
        payments_count: collections.transactions.len() as u64,
        blocks_count: collections.blocks.len() as u64,
        handles_count: collections.handles.len() as u64,
        files_count: collections.files.len() as u64,
        ai_model_hash: storage.snapshot_ai_model_hash()?,
        tip_block_hash: bounds.tip_block_hash,
        hashtimer_start: bounds.hashtimer_start,
        hashtimer_end: bounds.hashtimer_end,
        timestamp_start_us: bounds.timestamp_start_us,
        timestamp_end_us: bounds.timestamp_end_us,
    })
}

pub fn export_snapshot(
    storage: &impl StorageLike,
    path: &Path,
    height_hint: Option<u64>,
) -> Result<SnapshotManifest, SnapshotError> {
    ensure_export_directory(path)?;
    let latest_height = storage.get_latest_height()?;
    let snapshot_height = height_hint.unwrap_or(latest_height);
    if snapshot_height != latest_height {
        return Err(SnapshotError::HeightMismatch {
            requested: snapshot_height,
            available: latest_height,
        });
    }
    let collections = collect_snapshot_data(storage)?;
    write_jsonl(&path.join(BLOCKS_FILE), &collections.blocks)?;
    write_jsonl(&path.join(PAYMENTS_FILE), &collections.transactions)?;
    write_jsonl(&path.join(ACCOUNTS_FILE), &collections.accounts)?;
    write_jsonl(&path.join(HANDLES_FILE), &collections.handles)?;
    write_jsonl(&path.join(FILES_FILE), &collections.files)?;
    write_jsonl(&path.join(ROUNDS_FILE), &collections.rounds)?;
    write_json_file(&path.join(CHAIN_STATE_FILE), &collections.chain_state)?;

    let manifest = build_manifest_from_collections(storage, &collections, snapshot_height)?;
    write_json_file(&path.join(MANIFEST_FILE), &manifest)?;
    Ok(manifest)
}

pub fn import_snapshot(
    storage: &mut impl StorageLike,
    path: &Path,
) -> Result<SnapshotManifest, SnapshotError> {
    ensure_import_directory(path)?;
    let manifest_path = path.join(MANIFEST_FILE);
    let manifest: SnapshotManifest = read_json_file(&manifest_path)?;
    if manifest.version != SNAPSHOT_MANIFEST_VERSION {
        return Err(SnapshotError::InvalidManifest(format!(
            "unsupported manifest version {}",
            manifest.version
        )));
    }
    let storage_network = storage.snapshot_network_id();
    if storage_network != manifest.network_id {
        return Err(SnapshotError::NetworkMismatch {
            expected: manifest.network_id.clone(),
            actual: storage_network,
        });
    }
    ensure_storage_empty(storage)?;

    let blocks: Vec<Block> = read_optional_jsonl(&path.join(BLOCKS_FILE))?;
    let transactions: Vec<Transaction> = read_optional_jsonl(&path.join(PAYMENTS_FILE))?;
    let accounts: Vec<Account> = read_optional_jsonl(&path.join(ACCOUNTS_FILE))?;
    let handles: Vec<HandleSnapshotRecord> = read_optional_jsonl(&path.join(HANDLES_FILE))?;
    let files: Vec<FileDescriptor> = read_optional_jsonl(&path.join(FILES_FILE))?;
    let rounds: Vec<RoundFinalizationRecord> = read_optional_jsonl(&path.join(ROUNDS_FILE))?;
    let chain_state: Option<ChainState> = read_optional_json(&path.join(CHAIN_STATE_FILE))?;

    for block in blocks {
        storage.store_block(block)?;
    }
    for tx in transactions {
        storage.store_transaction(tx)?;
    }
    for account in accounts {
        storage.update_account(account)?;
    }
    for record in rounds {
        storage.store_round_finalization(record)?;
    }
    for descriptor in files {
        storage.store_file_descriptor(descriptor)?;
    }
    if let Some(state) = chain_state {
        storage.update_chain_state(&state)?;
    }

    // Handles are documented but currently in-memory only. We parse the file to
    // ensure deterministic exports and surface the data to operators. The
    // storage layer intentionally no-ops because handle persistence lives in
    // `l2_handle_registry` today.
    if !handles.is_empty() {
        tracing::warn!(
            "handle snapshot contained {} records but the registry is in-memory; data not restored",
            handles.len()
        );
    }

    storage.apply_ai_model_hash(manifest.ai_model_hash.as_deref())?;
    storage.flush_storage()?;
    manifest.validate_against_storage(storage)?;
    Ok(manifest)
}

fn ensure_export_directory(path: &Path) -> Result<(), SnapshotError> {
    if path.exists() {
        if !path.is_dir() {
            return Err(SnapshotError::InvalidManifest(format!(
                "snapshot path {} is not a directory",
                path.display()
            )));
        }
        if path.read_dir()?.next().is_some() {
            return Err(SnapshotError::DirectoryNotEmpty(path.display().to_string()));
        }
    } else {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

fn ensure_import_directory(path: &Path) -> Result<(), SnapshotError> {
    if !path.exists() || !path.is_dir() {
        return Err(SnapshotError::MissingSnapshot(path.display().to_string()));
    }
    Ok(())
}

fn write_jsonl<T: Serialize>(path: &Path, records: &[T]) -> Result<(), SnapshotError> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    for record in records {
        serde_json::to_writer(&mut writer, record)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    Ok(())
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), SnapshotError> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, value)?;
    Ok(())
}

fn read_json_file<T: DeserializeOwned>(path: &Path) -> Result<T, SnapshotError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
}

fn read_optional_jsonl<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>, SnapshotError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        records.push(serde_json::from_str(&line)?);
    }
    Ok(records)
}

fn read_optional_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, SnapshotError> {
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(read_json_file(path)?))
}

fn ensure_storage_empty(storage: &impl StorageLike) -> Result<(), SnapshotError> {
    if storage.get_latest_height()? > 0 {
        return Err(SnapshotError::StorageNotEmpty);
    }
    if storage.get_transaction_count()? > 0 {
        return Err(SnapshotError::StorageNotEmpty);
    }
    if !storage.snapshot_accounts()?.is_empty() {
        return Err(SnapshotError::StorageNotEmpty);
    }
    if !storage.snapshot_file_descriptors()?.is_empty() {
        return Err(SnapshotError::StorageNotEmpty);
    }
    Ok(())
}

fn sort_blocks(blocks: &mut [Block]) {
    blocks.sort_by(|a, b| {
        a.header
            .round
            .cmp(&b.header.round)
            .then_with(|| a.hash().cmp(&b.hash()))
    });
}

fn sort_transactions(transactions: &mut [Transaction]) {
    transactions.sort_by_key(|tx| tx.hash());
}

fn sort_accounts(accounts: &mut [Account]) {
    accounts.sort_by(|a, b| a.address.cmp(&b.address));
}

fn sort_file_descriptors(files: &mut [FileDescriptor]) {
    files.sort_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes()));
}

fn sort_rounds(rounds: &mut [RoundFinalizationRecord]) {
    rounds.sort_by(|a, b| a.round.cmp(&b.round));
}

/// Abstract storage trait
pub trait Storage {
    fn store_block(&self, block: Block) -> Result<()>;
    fn get_block(&self, hash: &[u8; 32]) -> Result<Option<Block>>;
    fn get_block_by_height(&self, height: u64) -> Result<Option<Block>>;
    fn store_transaction(&self, tx: Transaction) -> Result<()>;
    fn get_transaction(&self, hash: &[u8; 32]) -> Result<Option<Transaction>>;
    fn get_latest_height(&self) -> Result<u64>;
    fn get_account(&self, address: &[u8; 32]) -> Result<Option<Account>>;
    fn update_account(&self, account: Account) -> Result<()>;
    fn get_all_accounts(&self) -> Result<Vec<Account>>;
    fn get_transactions_by_address(&self, address: &[u8; 32]) -> Result<Vec<Transaction>>;
    fn get_transaction_count(&self) -> Result<u64>;
    fn put_l2_network(&self, network: L2Network) -> Result<()>;
    fn get_l2_network(&self, id: &str) -> Result<Option<L2Network>>;
    fn list_l2_networks(&self) -> Result<Vec<L2Network>>;
    fn store_l2_commit(&self, commit: L2Commit) -> Result<()>;
    fn list_l2_commits(&self, l2_id: Option<&str>) -> Result<Vec<L2Commit>>;
    fn store_l2_exit(&self, exit: L2ExitRecord) -> Result<()>;
    fn list_l2_exits(&self, l2_id: Option<&str>) -> Result<Vec<L2ExitRecord>>;
    fn store_round_certificate(&self, certificate: RoundCertificate) -> Result<()>;
    fn get_round_certificate(&self, round: RoundId) -> Result<Option<RoundCertificate>>;
    fn store_round_finalization(&self, record: RoundFinalizationRecord) -> Result<()>;
    fn get_round_finalization(&self, round: RoundId) -> Result<Option<RoundFinalizationRecord>>;
    fn get_latest_round_finalization(&self) -> Result<Option<RoundFinalizationRecord>>;

    /// Chain-state persistence for DAG-Fair emission tracking
    fn get_chain_state(&self) -> Result<ChainState>;
    fn update_chain_state(&self, state: &ChainState) -> Result<()>;

    /// Validator telemetry storage for AI consensus
    fn store_validator_telemetry(
        &self,
        validator_id: &[u8; 32],
        telemetry: &ValidatorTelemetry,
    ) -> Result<()>;
    fn get_validator_telemetry(
        &self,
        validator_id: &[u8; 32],
    ) -> Result<Option<ValidatorTelemetry>>;
    fn get_all_validator_telemetry(&self) -> Result<HashMap<[u8; 32], ValidatorTelemetry>>;

    /// File descriptor metadata (off-chain file hash registry)
    fn store_file_descriptor(&self, descriptor: FileDescriptor) -> Result<()>;
    fn get_file_descriptor(&self, id: &FileDescriptorId) -> Result<Option<FileDescriptor>>;
    fn list_file_descriptors_by_owner(&self, owner: &Address) -> Result<Vec<FileDescriptor>>;
}

/// Validator telemetry for AI consensus
/// All percentage/performance fields are scaled integers (0-10000 = 0%-100%)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorTelemetry {
    pub validator_id: [u8; 32],
    pub blocks_proposed: u64,
    pub blocks_verified: u64,
    pub rounds_active: u64,
    pub avg_latency_us: u64,
    pub slash_count: u32,
    pub stake: u64,
    pub age_rounds: u64,
    pub last_active_round: u64,
    /// Uptime percentage scaled by 100 (0-10000 = 0%-100%)
    pub uptime_percentage_scaled: i64,
    /// Recent performance scaled by 10000 (0-10000 = 0%-100%)
    pub recent_performance_scaled: i64,
    /// Network contribution scaled by 10000 (0-10000 = 0%-100%)
    pub network_contribution_scaled: i64,
}

#[derive(Default, Clone)]
pub struct MemoryStorage {
    inner: Arc<MemoryStorageInner>,
}

#[derive(Default)]
struct MemoryStorageInner {
    blocks: RwLock<HashMap<[u8; 32], Block>>,
    blocks_by_height: RwLock<BTreeMap<u64, [u8; 32]>>,
    transactions: RwLock<HashMap<[u8; 32], Transaction>>,
    accounts: RwLock<HashMap<[u8; 32], Account>>,
    l2_networks: RwLock<HashMap<String, L2Network>>,
    l2_commits: RwLock<HashMap<String, L2Commit>>,
    l2_exits: RwLock<HashMap<String, L2ExitRecord>>,
    round_certificates: RwLock<HashMap<RoundId, RoundCertificate>>,
    round_finalizations: RwLock<HashMap<RoundId, RoundFinalizationRecord>>,
    validator_telemetry: RwLock<HashMap<[u8; 32], ValidatorTelemetry>>,
    chain_state: RwLock<ChainState>,
    latest_height: RwLock<u64>,
    latest_finalized_round: RwLock<Option<RoundId>>,
    file_descriptors: RwLock<HashMap<FileDescriptorId, FileDescriptor>>,
    files_by_owner: RwLock<HashMap<[u8; 32], Vec<FileDescriptorId>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Storage for MemoryStorage {
    fn store_block(&self, block: Block) -> Result<()> {
        let hash = block.hash();
        {
            let mut blocks = self.inner.blocks.write();
            blocks.insert(hash, block.clone());
        }
        {
            let mut blocks_by_height = self.inner.blocks_by_height.write();
            blocks_by_height.insert(block.header.round, hash);
        }
        {
            let mut latest_height = self.inner.latest_height.write();
            if block.header.round > *latest_height {
                *latest_height = block.header.round;
            }
        }
        Ok(())
    }

    fn get_block(&self, hash: &[u8; 32]) -> Result<Option<Block>> {
        Ok(self.inner.blocks.read().get(hash).cloned())
    }

    fn get_block_by_height(&self, height: u64) -> Result<Option<Block>> {
        let hash = self.inner.blocks_by_height.read().get(&height).copied();
        Ok(hash.and_then(|id| self.inner.blocks.read().get(&id).cloned()))
    }

    fn store_transaction(&self, tx: Transaction) -> Result<()> {
        let hash = tx.hash();
        self.inner.transactions.write().insert(hash, tx);
        Ok(())
    }

    fn get_transaction(&self, hash: &[u8; 32]) -> Result<Option<Transaction>> {
        Ok(self.inner.transactions.read().get(hash).cloned())
    }

    fn get_latest_height(&self) -> Result<u64> {
        Ok(*self.inner.latest_height.read())
    }

    fn get_account(&self, address: &[u8; 32]) -> Result<Option<Account>> {
        Ok(self.inner.accounts.read().get(address).cloned())
    }

    fn update_account(&self, account: Account) -> Result<()> {
        self.inner.accounts.write().insert(account.address, account);
        Ok(())
    }

    fn get_all_accounts(&self) -> Result<Vec<Account>> {
        Ok(self.inner.accounts.read().values().cloned().collect())
    }

    fn get_transactions_by_address(&self, address: &[u8; 32]) -> Result<Vec<Transaction>> {
        let transactions = self.inner.transactions.read();
        Ok(transactions
            .values()
            .filter(|tx| tx.from == *address || tx.to == *address)
            .cloned()
            .collect())
    }

    fn get_transaction_count(&self) -> Result<u64> {
        Ok(self.inner.transactions.read().len() as u64)
    }

    fn put_l2_network(&self, network: L2Network) -> Result<()> {
        self.inner
            .l2_networks
            .write()
            .insert(network.id.clone(), network);
        Ok(())
    }

    fn get_l2_network(&self, id: &str) -> Result<Option<L2Network>> {
        Ok(self.inner.l2_networks.read().get(id).cloned())
    }

    fn list_l2_networks(&self) -> Result<Vec<L2Network>> {
        let mut networks: Vec<L2Network> =
            self.inner.l2_networks.read().values().cloned().collect();
        networks.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(networks)
    }

    fn store_l2_commit(&self, commit: L2Commit) -> Result<()> {
        self.inner
            .l2_commits
            .write()
            .insert(commit.id.clone(), commit);
        Ok(())
    }

    fn list_l2_commits(&self, l2_id: Option<&str>) -> Result<Vec<L2Commit>> {
        Ok(self
            .inner
            .l2_commits
            .read()
            .values()
            .filter(|commit| l2_id.map(|id| commit.l2_id == id).unwrap_or(true))
            .cloned()
            .collect())
    }

    fn store_l2_exit(&self, exit: L2ExitRecord) -> Result<()> {
        self.inner.l2_exits.write().insert(exit.id.clone(), exit);
        Ok(())
    }

    fn list_l2_exits(&self, l2_id: Option<&str>) -> Result<Vec<L2ExitRecord>> {
        Ok(self
            .inner
            .l2_exits
            .read()
            .values()
            .filter(|exit| l2_id.map(|id| exit.l2_id == id).unwrap_or(true))
            .cloned()
            .collect())
    }

    fn store_round_certificate(&self, certificate: RoundCertificate) -> Result<()> {
        self.inner
            .round_certificates
            .write()
            .insert(certificate.round, certificate);
        Ok(())
    }

    fn get_round_certificate(&self, round: RoundId) -> Result<Option<RoundCertificate>> {
        Ok(self.inner.round_certificates.read().get(&round).cloned())
    }

    fn store_round_finalization(&self, record: RoundFinalizationRecord) -> Result<()> {
        let round = record.round;
        self.inner.round_finalizations.write().insert(round, record);
        {
            let mut latest = self.inner.latest_finalized_round.write();
            if latest.map(|current| round > current).unwrap_or(true) {
                *latest = Some(round);
            }
        }
        Ok(())
    }

    fn get_round_finalization(&self, round: RoundId) -> Result<Option<RoundFinalizationRecord>> {
        Ok(self.inner.round_finalizations.read().get(&round).cloned())
    }

    fn get_latest_round_finalization(&self) -> Result<Option<RoundFinalizationRecord>> {
        let latest = *self.inner.latest_finalized_round.read();
        Ok(latest.and_then(|round| self.inner.round_finalizations.read().get(&round).cloned()))
    }

    fn get_chain_state(&self) -> Result<ChainState> {
        Ok(self.inner.chain_state.read().clone())
    }

    fn update_chain_state(&self, state: &ChainState) -> Result<()> {
        *self.inner.chain_state.write() = state.clone();
        Ok(())
    }

    fn store_validator_telemetry(
        &self,
        validator_id: &[u8; 32],
        telemetry: &ValidatorTelemetry,
    ) -> Result<()> {
        self.inner
            .validator_telemetry
            .write()
            .insert(*validator_id, telemetry.clone());
        Ok(())
    }

    fn get_validator_telemetry(
        &self,
        validator_id: &[u8; 32],
    ) -> Result<Option<ValidatorTelemetry>> {
        Ok(self
            .inner
            .validator_telemetry
            .read()
            .get(validator_id)
            .cloned())
    }

    fn get_all_validator_telemetry(&self) -> Result<HashMap<[u8; 32], ValidatorTelemetry>> {
        Ok(self.inner.validator_telemetry.read().clone())
    }

    fn store_file_descriptor(&self, descriptor: FileDescriptor) -> Result<()> {
        let mut descriptors = self.inner.file_descriptors.write();
        if descriptors.contains_key(&descriptor.id) {
            return Err(anyhow!(
                "file descriptor already exists: {}",
                descriptor.id.to_hex()
            ));
        }
        let owner_key = descriptor.owner.clone().0;
        descriptors.insert(descriptor.id, descriptor.clone());
        self.inner
            .files_by_owner
            .write()
            .entry(owner_key)
            .or_default()
            .push(descriptor.id);
        Ok(())
    }

    fn get_file_descriptor(&self, id: &FileDescriptorId) -> Result<Option<FileDescriptor>> {
        Ok(self.inner.file_descriptors.read().get(id).cloned())
    }

    fn list_file_descriptors_by_owner(&self, owner: &Address) -> Result<Vec<FileDescriptor>> {
        let owner_key = owner.clone().0;
        let ids = self
            .inner
            .files_by_owner
            .read()
            .get(&owner_key)
            .cloned()
            .unwrap_or_default();
        let descriptors = self.inner.file_descriptors.read();
        Ok(ids
            .iter()
            .filter_map(|id| descriptors.get(id).cloned())
            .collect())
    }
}

impl StorageLike for MemoryStorage {
    fn snapshot_network_id(&self) -> String {
        "memory-devnet".to_string()
    }

    fn snapshot_blocks(&self) -> Result<Vec<Block>> {
        let mut blocks: Vec<Block> = self.inner.blocks.read().values().cloned().collect();
        sort_blocks(&mut blocks);
        Ok(blocks)
    }

    fn snapshot_transactions(&self) -> Result<Vec<Transaction>> {
        let mut txs: Vec<Transaction> = self.inner.transactions.read().values().cloned().collect();
        sort_transactions(&mut txs);
        Ok(txs)
    }

    fn snapshot_accounts(&self) -> Result<Vec<Account>> {
        let mut accounts: Vec<Account> = self.inner.accounts.read().values().cloned().collect();
        sort_accounts(&mut accounts);
        Ok(accounts)
    }

    fn snapshot_file_descriptors(&self) -> Result<Vec<FileDescriptor>> {
        let mut files: Vec<FileDescriptor> = self
            .inner
            .file_descriptors
            .read()
            .values()
            .cloned()
            .collect();
        sort_file_descriptors(&mut files);
        Ok(files)
    }

    fn snapshot_round_finalizations(&self) -> Result<Vec<RoundFinalizationRecord>> {
        let mut rounds: Vec<RoundFinalizationRecord> = self
            .inner
            .round_finalizations
            .read()
            .values()
            .cloned()
            .collect();
        sort_rounds(&mut rounds);
        Ok(rounds)
    }

    fn snapshot_chain_state(&self) -> Result<ChainState> {
        Ok(self.inner.chain_state.read().clone())
    }
}

/// Sled-backed implementation
pub struct SledStorage {
    db: Db,
    blocks: Tree,
    transactions: Tree,
    accounts: Tree,
    metadata: Tree,
    l2_networks: Tree,
    l2_commits: Tree,
    l2_exits: Tree,
    round_certificates: Tree,
    round_finalizations: Tree,
    validator_telemetry: Tree,
    file_descriptors: Tree,
    file_owner_index: Tree,
    chain_state: Arc<RwLock<ChainState>>,
    network_id: Arc<RwLock<String>>,
}

impl SledStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path)?;
        let blocks = db.open_tree("blocks")?;
        let transactions = db.open_tree("transactions")?;
        let accounts = db.open_tree("accounts")?;
        let metadata = db.open_tree("metadata")?;
        let l2_networks = db.open_tree("l2_networks")?;
        let l2_commits = db.open_tree("l2_commits")?;
        let l2_exits = db.open_tree("l2_exits")?;
        let round_certificates = db.open_tree("round_certificates")?;
        let validator_telemetry = db.open_tree("validator_telemetry")?;
        let round_finalizations = db.open_tree("round_finalizations")?;
        let file_descriptors = db.open_tree("file_descriptors")?;
        let file_owner_index = db.open_tree("file_owner_index")?;

        let chain_state = if let Some(v) = metadata.get(b"chain_state")? {
            serde_json::from_slice(&v).unwrap_or_default()
        } else {
            ChainState::default()
        };
        let network_id = if let Some(bytes) = metadata.get(b"network_id")? {
            String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| "unknown".to_string())
        } else {
            "unknown".to_string()
        };

        Ok(Self {
            db,
            blocks,
            transactions,
            accounts,
            metadata,
            l2_networks,
            l2_commits,
            l2_exits,
            round_certificates,
            round_finalizations,
            validator_telemetry,
            file_descriptors,
            file_owner_index,
            chain_state: Arc::new(RwLock::new(chain_state)),
            network_id: Arc::new(RwLock::new(network_id)),
        })
    }

    pub fn initialize(&self) -> Result<()> {
        if self.get_latest_height()? == 0 {
            let genesis_block = Block::new(Vec::new(), vec![], 0, [0u8; 32]);
            self.store_block(genesis_block)?;
            let genesis_account = Account {
                address: [0u8; 32],
                balance: 1_000_000,
                nonce: 0,
            };
            self.update_account(genesis_account)?;
            tracing::info!("Initialized genesis block + account");
        }
        Ok(())
    }

    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }

    pub fn set_network_id(&self, network_id: &str) -> Result<()> {
        self.metadata.insert(b"network_id", network_id.as_bytes())?;
        *self.network_id.write() = network_id.to_string();
        Ok(())
    }
}

impl Storage for SledStorage {
    fn store_block(&self, block: Block) -> Result<()> {
        let hash = block.hash();
        let data = serde_json::to_vec(&block)?;
        self.blocks.insert(&hash[..], data)?;
        let height = block.header.round;

        let latest_height = self.get_latest_height()?;
        debug_assert!(
            height >= latest_height,
            "latest height cannot decrease (current={}, new={})",
            latest_height,
            height
        );

        if height >= latest_height {
            self.metadata
                .insert(b"latest_height", &height.to_be_bytes())?;
        }
        Ok(())
    }

    fn get_block(&self, hash: &[u8; 32]) -> Result<Option<Block>> {
        self.blocks
            .get(&hash[..])?
            .map(|v| serde_json::from_slice(&v))
            .transpose()
            .map_err(Into::into)
    }

    fn get_block_by_height(&self, height: u64) -> Result<Option<Block>> {
        for item in self.blocks.iter() {
            let (_, val) = item?;
            let b: Block = serde_json::from_slice(&val)?;
            if b.header.round == height {
                return Ok(Some(b));
            }
        }
        Ok(None)
    }

    fn store_transaction(&self, tx: Transaction) -> Result<()> {
        let h = tx.hash();
        let data = serde_json::to_vec(&tx)?;
        self.transactions.insert(&h[..], data)?;
        Ok(())
    }

    fn get_transaction(&self, hash: &[u8; 32]) -> Result<Option<Transaction>> {
        self.transactions
            .get(&hash[..])?
            .map(|v| serde_json::from_slice(&v))
            .transpose()
            .map_err(Into::into)
    }

    fn get_latest_height(&self) -> Result<u64> {
        Ok(self
            .metadata
            .get(b"latest_height")?
            .and_then(|v| v.as_ref().try_into().ok().map(u64::from_be_bytes))
            .unwrap_or(0))
    }

    fn get_account(&self, addr: &[u8; 32]) -> Result<Option<Account>> {
        self.accounts
            .get(&addr[..])?
            .map(|v| serde_json::from_slice(&v))
            .transpose()
            .map_err(Into::into)
    }

    fn update_account(&self, acc: Account) -> Result<()> {
        let data = serde_json::to_vec(&acc)?;
        self.accounts.insert(&acc.address[..], data)?;
        Ok(())
    }

    fn get_all_accounts(&self) -> Result<Vec<Account>> {
        self.accounts
            .iter()
            .map(|r| {
                let (_, v) = r?;
                Ok(serde_json::from_slice::<Account>(&v)?)
            })
            .collect()
    }

    fn get_transactions_by_address(&self, addr: &[u8; 32]) -> Result<Vec<Transaction>> {
        let mut v = Vec::new();
        for r in self.transactions.iter() {
            let (_, data) = r?;
            let tx: Transaction = serde_json::from_slice(&data)?;
            if tx.from == *addr || tx.to == *addr {
                v.push(tx);
            }
        }
        Ok(v)
    }

    fn get_transaction_count(&self) -> Result<u64> {
        Ok(self.transactions.len() as u64)
    }

    fn put_l2_network(&self, n: L2Network) -> Result<()> {
        self.l2_networks
            .insert(n.id.as_bytes(), serde_json::to_vec(&n)?)?;
        Ok(())
    }

    fn get_l2_network(&self, id: &str) -> Result<Option<L2Network>> {
        self.l2_networks
            .get(id.as_bytes())?
            .map(|v| serde_json::from_slice(&v))
            .transpose()
            .map_err(Into::into)
    }

    fn list_l2_networks(&self) -> Result<Vec<L2Network>> {
        let mut nets: Vec<L2Network> = Vec::new();
        for e in self.l2_networks.iter() {
            let (_, v) = e?;
            nets.push(serde_json::from_slice(&v)?);
        }
        nets.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(nets)
    }

    fn store_l2_commit(&self, c: L2Commit) -> Result<()> {
        self.l2_commits
            .insert(c.id.as_bytes(), serde_json::to_vec(&c)?)?;
        Ok(())
    }

    fn list_l2_commits(&self, f: Option<&str>) -> Result<Vec<L2Commit>> {
        let mut list = Vec::new();
        for e in self.l2_commits.iter() {
            let (_, v) = e?;
            let c: L2Commit = serde_json::from_slice(&v)?;
            if f.map(|id| id == c.l2_id).unwrap_or(true) {
                list.push(c);
            }
        }
        Ok(list)
    }

    fn store_l2_exit(&self, x: L2ExitRecord) -> Result<()> {
        self.l2_exits
            .insert(x.id.as_bytes(), serde_json::to_vec(&x)?)?;
        Ok(())
    }

    fn list_l2_exits(&self, f: Option<&str>) -> Result<Vec<L2ExitRecord>> {
        let mut xs = Vec::new();
        for e in self.l2_exits.iter() {
            let (_, v) = e?;
            let x: L2ExitRecord = serde_json::from_slice(&v)?;
            if f.map(|id| id == x.l2_id).unwrap_or(true) {
                xs.push(x);
            }
        }
        Ok(xs)
    }

    fn store_round_certificate(&self, cert: RoundCertificate) -> Result<()> {
        self.round_certificates
            .insert(cert.round.to_be_bytes(), serde_json::to_vec(&cert)?)?;
        Ok(())
    }

    fn get_round_certificate(&self, r: RoundId) -> Result<Option<RoundCertificate>> {
        self.round_certificates
            .get(r.to_be_bytes())?
            .map(|v| serde_json::from_slice(&v))
            .transpose()
            .map_err(Into::into)
    }

    fn store_round_finalization(&self, rec: RoundFinalizationRecord) -> Result<()> {
        let key = rec.round.to_be_bytes();
        self.round_finalizations
            .insert(key, serde_json::to_vec(&rec)?)?;
        self.metadata.insert(b"latest_finalized_round", &key)?;
        Ok(())
    }

    fn get_round_finalization(&self, r: RoundId) -> Result<Option<RoundFinalizationRecord>> {
        self.round_finalizations
            .get(r.to_be_bytes())?
            .map(|v| serde_json::from_slice(&v))
            .transpose()
            .map_err(Into::into)
    }

    fn get_latest_round_finalization(&self) -> Result<Option<RoundFinalizationRecord>> {
        if let Some(v) = self.metadata.get(b"latest_finalized_round")? {
            let mut b = [0u8; 8];
            b.copy_from_slice(&v);
            let r = u64::from_be_bytes(b);
            self.get_round_finalization(r)
        } else {
            Ok(None)
        }
    }

    fn get_chain_state(&self) -> Result<ChainState> {
        Ok(self.chain_state.read().clone())
    }

    fn update_chain_state(&self, s: &ChainState) -> Result<()> {
        *self.chain_state.write() = s.clone();
        self.metadata
            .insert(b"chain_state", serde_json::to_vec(s)?)?;
        Ok(())
    }

    fn store_validator_telemetry(
        &self,
        validator_id: &[u8; 32],
        telemetry: &ValidatorTelemetry,
    ) -> Result<()> {
        let key = &validator_id[..];
        let value = serde_json::to_vec(telemetry)?;
        self.validator_telemetry.insert(key, value)?;
        Ok(())
    }

    fn get_validator_telemetry(
        &self,
        validator_id: &[u8; 32],
    ) -> Result<Option<ValidatorTelemetry>> {
        self.validator_telemetry
            .get(&validator_id[..])?
            .map(|v| serde_json::from_slice(&v))
            .transpose()
            .map_err(Into::into)
    }

    fn get_all_validator_telemetry(&self) -> Result<HashMap<[u8; 32], ValidatorTelemetry>> {
        let mut telemetry = HashMap::new();
        for record in self.validator_telemetry.iter() {
            let (key, value) = record?;
            if key.len() != 32 {
                continue;
            }
            let mut validator_id = [0u8; 32];
            validator_id.copy_from_slice(&key);
            let telemetry_record: ValidatorTelemetry = serde_json::from_slice(&value)?;
            telemetry.insert(validator_id, telemetry_record);
        }
        Ok(telemetry)
    }

    fn store_file_descriptor(&self, descriptor: FileDescriptor) -> Result<()> {
        let key = descriptor.id.to_bytes();
        if self.file_descriptors.contains_key(&key[..])? {
            return Err(anyhow!(
                "file descriptor already exists: {}",
                descriptor.id.to_hex()
            ));
        }
        let data = serde_json::to_vec(&descriptor)?;
        self.file_descriptors.insert(&key[..], data)?;
        let owner_key = descriptor.owner.clone().0;
        let index_key = build_owner_index_key(&owner_key, &key);
        self.file_owner_index.insert(index_key, key.to_vec())?;
        Ok(())
    }

    fn get_file_descriptor(&self, id: &FileDescriptorId) -> Result<Option<FileDescriptor>> {
        self.file_descriptors
            .get(id.as_bytes())?
            .map(|v| serde_json::from_slice(&v))
            .transpose()
            .map_err(Into::into)
    }

    fn list_file_descriptors_by_owner(&self, owner: &Address) -> Result<Vec<FileDescriptor>> {
        let owner_key = owner.clone().0;
        let mut descriptors = Vec::new();
        for entry in self.file_owner_index.scan_prefix(owner_key) {
            let (_key, value) = entry?;
            if value.len() != 32 {
                continue;
            }
            let mut id_bytes = [0u8; 32];
            id_bytes.copy_from_slice(&value);
            if let Some(descriptor) =
                self.get_file_descriptor(&FileDescriptorId::from_bytes(id_bytes))?
            {
                descriptors.push(descriptor);
            }
        }
        Ok(descriptors)
    }
}

impl StorageLike for SledStorage {
    fn snapshot_network_id(&self) -> String {
        self.network_id.read().clone()
    }

    fn snapshot_blocks(&self) -> Result<Vec<Block>> {
        let mut blocks = Vec::new();
        for entry in self.blocks.iter() {
            let (_, value) = entry?;
            blocks.push(serde_json::from_slice(&value)?);
        }
        sort_blocks(&mut blocks);
        Ok(blocks)
    }

    fn snapshot_transactions(&self) -> Result<Vec<Transaction>> {
        let mut txs = Vec::new();
        for entry in self.transactions.iter() {
            let (_, value) = entry?;
            txs.push(serde_json::from_slice(&value)?);
        }
        sort_transactions(&mut txs);
        Ok(txs)
    }

    fn snapshot_accounts(&self) -> Result<Vec<Account>> {
        let mut accounts = Vec::new();
        for entry in self.accounts.iter() {
            let (_, value) = entry?;
            accounts.push(serde_json::from_slice(&value)?);
        }
        sort_accounts(&mut accounts);
        Ok(accounts)
    }

    fn snapshot_file_descriptors(&self) -> Result<Vec<FileDescriptor>> {
        let mut files = Vec::new();
        for entry in self.file_descriptors.iter() {
            let (_, value) = entry?;
            files.push(serde_json::from_slice(&value)?);
        }
        sort_file_descriptors(&mut files);
        Ok(files)
    }

    fn snapshot_round_finalizations(&self) -> Result<Vec<RoundFinalizationRecord>> {
        let mut rounds = Vec::new();
        for entry in self.round_finalizations.iter() {
            let (_, value) = entry?;
            rounds.push(serde_json::from_slice(&value)?);
        }
        sort_rounds(&mut rounds);
        Ok(rounds)
    }

    fn snapshot_chain_state(&self) -> Result<ChainState> {
        self.get_chain_state()
    }

    fn snapshot_ai_model_hash(&self) -> Result<Option<String>> {
        Ok(self
            .metadata
            .get(b"ai_model_hash")?
            .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
            .filter(|value| !value.is_empty()))
    }

    fn apply_ai_model_hash(&self, hash: Option<&str>) -> Result<()> {
        match hash {
            Some(value) if !value.is_empty() => {
                self.metadata.insert(b"ai_model_hash", value.as_bytes())?;
            }
            _ => {
                self.metadata.remove(b"ai_model_hash")?;
            }
        }
        Ok(())
    }

    fn flush_storage(&self) -> Result<()> {
        self.flush()
    }
}

fn build_owner_index_key(owner: &[u8; 32], descriptor: &[u8; 32]) -> Vec<u8> {
    let mut key = Vec::with_capacity(64);
    key.extend_from_slice(owner);
    key.extend_from_slice(descriptor);
    key
}

// =====================================================================
// In-memory backend for testing (MemoryStorage) and exhaustive tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::{
        Amount, Block, BlockId, ChainState, HashTimer, IppanTimeMicros, RoundCertificate,
        RoundFinalizationRecord, RoundId, RoundWindow, Transaction,
    };
    use tempfile::tempdir;

    fn sample_cert(round: RoundId) -> RoundCertificate {
        let first: BlockId = [round as u8; 32];
        let second: BlockId = [42u8; 32];
        RoundCertificate {
            round,
            block_ids: vec![first, second],
            agg_sig: vec![1, 2, 3, 4],
        }
    }

    #[test]
    fn memory_round_certificate_round_trip() {
        let storage = MemoryStorage::new();
        let cert = sample_cert(7);

        storage.store_round_certificate(cert.clone()).unwrap();
        let fetched = storage.get_round_certificate(7).unwrap();

        assert_eq!(fetched, Some(cert));
    }

    #[test]
    fn sled_round_certificate_round_trip() {
        let dir = tempdir().expect("temp dir");
        let storage = SledStorage::new(dir.path()).expect("sled storage");
        storage.initialize().expect("init");

        let cert = sample_cert(3);
        storage.store_round_certificate(cert.clone()).unwrap();

        let fetched = storage.get_round_certificate(3).unwrap();
        assert_eq!(fetched, Some(cert));

        let missing = storage.get_round_certificate(99).unwrap();
        assert!(missing.is_none());
    }

    fn sample_descriptor(seed: u8) -> FileDescriptor {
        let owner = Address([seed; 32]);
        let time = IppanTimeMicros(1_000 + seed as u64);
        let domain = [seed; 8];
        let payload = [seed; 16];
        let nonce = [seed; 32];
        let node = [seed.wrapping_add(1); 32];
        let hashtimer = HashTimer::derive("file", time, &domain, &payload, &nonce, &node);
        FileDescriptor::new(
            hashtimer,
            owner,
            [seed; 32],
            2048,
            Some("application/test".into()),
            vec!["tag".into()],
        )
    }

    fn sample_block(round: RoundId) -> Block {
        let mut tx = Transaction::new([1u8; 32], [2u8; 32], Amount::from_atomic(1_000), round);
        tx.refresh_id();
        Block::new(vec![], vec![tx], round, [3u8; 32])
    }

    #[test]
    fn memory_file_descriptor_round_trip() {
        let storage = MemoryStorage::new();
        let descriptor = sample_descriptor(7);
        storage
            .store_file_descriptor(descriptor.clone())
            .expect("store descriptor");

        let fetched = storage
            .get_file_descriptor(&descriptor.id)
            .expect("fetch descriptor")
            .expect("descriptor exists");
        assert_eq!(fetched.owner, descriptor.owner);
        assert_eq!(fetched.content_hash, descriptor.content_hash);

        let owner_files = storage
            .list_file_descriptors_by_owner(&descriptor.owner)
            .expect("list by owner");
        assert_eq!(owner_files.len(), 1);
        assert_eq!(owner_files[0].id, descriptor.id);
    }

    #[test]
    fn sled_file_descriptor_round_trip() {
        let dir = tempdir().expect("tempdir");
        let storage = SledStorage::new(dir.path()).expect("sled storage");
        storage.initialize().expect("init");

        let descriptor = sample_descriptor(9);
        storage
            .store_file_descriptor(descriptor.clone())
            .expect("store descriptor");

        let fetched = storage
            .get_file_descriptor(&descriptor.id)
            .expect("fetch descriptor")
            .expect("descriptor exists");
        assert_eq!(fetched.owner, descriptor.owner);

        let owner_files = storage
            .list_file_descriptors_by_owner(&descriptor.owner)
            .expect("list owner");
        assert_eq!(owner_files.len(), 1);
        assert_eq!(owner_files[0].id, descriptor.id);
    }

    #[test]
    fn snapshot_manifest_counts_memory_state() {
        let storage = MemoryStorage::new();

        let mut tx = Transaction::new([1u8; 32], [2u8; 32], Amount::from_atomic(1_000), 1);
        tx.refresh_id();
        storage.store_transaction(tx.clone()).expect("store tx");

        let block = Block::new(vec![], vec![tx.clone()], 1, [9u8; 32]);
        storage.store_block(block).expect("store block");

        let account = Account {
            address: [42u8; 32],
            balance: 5_000,
            nonce: 2,
        };
        storage.update_account(account).expect("store account");

        let descriptor = sample_descriptor(3);
        storage
            .store_file_descriptor(descriptor)
            .expect("store file");

        let round_record = RoundFinalizationRecord {
            round: 1,
            window: RoundWindow {
                id: 1,
                start_us: IppanTimeMicros(10),
                end_us: IppanTimeMicros(20),
            },
            ordered_tx_ids: vec![tx.hash()],
            fork_drops: vec![],
            state_root: [0u8; 32],
            proof: sample_cert(1),
            total_fees_atomic: Some(10),
            treasury_fees_atomic: Some(2),
            applied_payments: Some(1),
            rejected_payments: Some(0),
        };
        storage
            .store_round_finalization(round_record)
            .expect("store finalization");

        let manifest = SnapshotManifest::new_from_storage(&storage).expect("manifest");
        assert_eq!(manifest.accounts_count, 1);
        assert_eq!(manifest.payments_count, 1);
        assert_eq!(manifest.blocks_count, 1);
        assert_eq!(manifest.files_count, 1);
        assert_eq!(manifest.handles_count, 0);
    }

    #[test]
    fn storing_same_block_twice_is_idempotent() {
        let storage = MemoryStorage::new();
        let block = sample_block(5);

        storage.store_block(block.clone()).expect("store block");
        let first_height = storage.get_latest_height().expect("height");

        storage
            .store_block(block.clone())
            .expect("idempotent store");

        assert_eq!(first_height, storage.get_latest_height().expect("height"));
        let fetched = storage
            .get_block(&block.header.id)
            .expect("fetch")
            .expect("block exists");
        assert_eq!(fetched.header.id, block.header.id);
        assert_eq!(fetched.transactions.len(), block.transactions.len());
    }

    #[test]
    fn snapshot_round_trip_restores_state() {
        let storage = MemoryStorage::new();

        let mut chain_state = ChainState::with_initial(42, 2, 3);
        chain_state.set_state_root([9u8; 32]);
        chain_state.set_last_updated(777);
        chain_state
            .metadata
            .insert("network".into(), "devnet".into());
        storage
            .update_chain_state(&chain_state)
            .expect("store chain state");

        let mut tx = Transaction::new([9u8; 32], [8u8; 32], Amount::from_atomic(500), 1);
        tx.refresh_id();
        storage
            .store_transaction(tx.clone())
            .expect("store transaction");

        let block = Block::new(vec![], vec![tx.clone()], 1, [1u8; 32]);
        storage.store_block(block.clone()).expect("store block");

        let account_a = Account {
            address: [7u8; 32],
            balance: 1_500,
            nonce: 0,
        };
        let account_b = Account {
            address: [6u8; 32],
            balance: 2_500,
            nonce: 1,
        };
        storage
            .update_account(account_a.clone())
            .expect("store account a");
        storage
            .update_account(account_b.clone())
            .expect("store account b");

        let descriptor = sample_descriptor(5);
        storage
            .store_file_descriptor(descriptor.clone())
            .expect("store descriptor");

        let round_record = RoundFinalizationRecord {
            round: 1,
            window: RoundWindow {
                id: 1,
                start_us: IppanTimeMicros(10),
                end_us: IppanTimeMicros(20),
            },
            ordered_tx_ids: vec![tx.hash()],
            fork_drops: vec![],
            state_root: [1u8; 32],
            proof: sample_cert(1),
            total_fees_atomic: Some(10),
            treasury_fees_atomic: Some(2),
            applied_payments: Some(1),
            rejected_payments: Some(0),
        };
        storage
            .store_round_finalization(round_record.clone())
            .expect("store finalization");

        let snapshot_dir = tempdir().expect("temp snapshot dir");
        let manifest =
            export_snapshot(&storage, snapshot_dir.path(), None).expect("export snapshot");

        let mut replay_storage = MemoryStorage::new();
        let imported =
            import_snapshot(&mut replay_storage, snapshot_dir.path()).expect("import snapshot");

        assert_eq!(manifest.height, imported.height);
        assert_eq!(manifest.blocks_count, imported.blocks_count);

        let normalize_accounts = |accounts: Vec<Account>| {
            accounts
                .into_iter()
                .map(|acc| (acc.address, acc.balance, acc.nonce))
                .collect::<Vec<_>>()
        };
        let original_accounts = normalize_accounts(storage.snapshot_accounts().expect("accounts"));
        let replay_accounts =
            normalize_accounts(replay_storage.snapshot_accounts().expect("accounts replay"));
        assert_eq!(original_accounts, replay_accounts);

        let tx_ids: Vec<[u8; 32]> = storage
            .snapshot_transactions()
            .expect("txs")
            .into_iter()
            .map(|tx| tx.hash())
            .collect();
        let replay_tx_ids: Vec<[u8; 32]> = replay_storage
            .snapshot_transactions()
            .expect("txs replay")
            .into_iter()
            .map(|tx| tx.hash())
            .collect();
        assert_eq!(tx_ids, replay_tx_ids);

        let block_ids: Vec<[u8; 32]> = storage
            .snapshot_blocks()
            .expect("blocks")
            .into_iter()
            .map(|block| block.header.id)
            .collect();
        let replay_block_ids: Vec<[u8; 32]> = replay_storage
            .snapshot_blocks()
            .expect("blocks replay")
            .into_iter()
            .map(|block| block.header.id)
            .collect();
        assert_eq!(block_ids, replay_block_ids);

        let round_summaries: Vec<(RoundId, [u8; 32])> = storage
            .snapshot_round_finalizations()
            .expect("rounds")
            .into_iter()
            .map(|record| (record.round, record.state_root))
            .collect();
        let replay_round_summaries: Vec<(RoundId, [u8; 32])> = replay_storage
            .snapshot_round_finalizations()
            .expect("rounds replay")
            .into_iter()
            .map(|record| (record.round, record.state_root))
            .collect();
        assert_eq!(round_summaries, replay_round_summaries);

        let original_state = storage.snapshot_chain_state().expect("chain state");
        let replay_state = replay_storage
            .snapshot_chain_state()
            .expect("chain state replay");
        assert_eq!(
            original_state.total_issued_micro,
            replay_state.total_issued_micro
        );
        assert_eq!(original_state.current_height, replay_state.current_height);
        assert_eq!(original_state.current_round, replay_state.current_round);
        assert_eq!(original_state.state_root, replay_state.state_root);
        assert_eq!(original_state.last_updated, replay_state.last_updated);

        // Balances must remain non-negative and totals preserved
        let original_sum: u64 = storage
            .snapshot_accounts()
            .expect("accounts")
            .into_iter()
            .map(|acc| acc.balance)
            .sum();
        let replay_sum: u64 = replay_storage
            .snapshot_accounts()
            .expect("accounts replay")
            .into_iter()
            .map(|acc| acc.balance)
            .sum();
        assert_eq!(original_sum, replay_sum);
        assert!(replay_sum > 0);
    }
}

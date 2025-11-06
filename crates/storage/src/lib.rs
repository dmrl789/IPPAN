//! IPPAN persistent storage abstraction layer. Defines the `Storage` trait,
//! Sled-backed node database, and in-memory test backend used across consensus,
//! mempool, and AI telemetry pipelines. Handles blocks, accounts, L2 anchors,
//! and validator telemetry with deterministic serialization.
//!
use anyhow::Result;
use ippan_types::{
    Block, L2Commit, L2ExitRecord, L2Network, RoundCertificate, RoundFinalizationRecord, RoundId,
    Transaction,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sled::{Db, Tree};
use std::collections::{BTreeMap, HashMap};
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

/// Chain state (economic + round metadata)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChainState {
    pub total_issued_micro: u128,
    pub last_updated_round: u64,
}

impl ChainState {
    pub fn add_issued_micro(&mut self, amt: u128) {
        self.total_issued_micro = self.total_issued_micro.saturating_add(amt);
    }
    pub fn update_round(&mut self, round: u64) {
        self.last_updated_round = round;
    }
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
}

/// Validator telemetry for AI consensus
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
    pub uptime_percentage: f64,
    pub recent_performance: f64,
    pub network_contribution: f64,
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
    chain_state: Arc<RwLock<ChainState>>,
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

        let chain_state = if let Some(v) = metadata.get(b"chain_state")? {
            serde_json::from_slice(&v).unwrap_or_default()
        } else {
            ChainState::default()
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
            chain_state: Arc::new(RwLock::new(chain_state)),
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
}

impl Storage for SledStorage {
    fn store_block(&self, block: Block) -> Result<()> {
        let hash = block.hash();
        let data = serde_json::to_vec(&block)?;
        self.blocks.insert(&hash[..], data)?;
        let height = block.header.round;
        self.metadata
            .insert(b"latest_height", &height.to_be_bytes())?;
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
}

/// In-memory testing backend
pub struct MemoryStorage {
    blocks: Arc<RwLock<HashMap<String, Block>>>,
    txs: Arc<RwLock<HashMap<String, Transaction>>>,
    accounts: Arc<RwLock<HashMap<String, Account>>>,
    chain_state: Arc<RwLock<ChainState>>,
    latest_height: Arc<RwLock<u64>>,
    validator_telemetry: Arc<RwLock<HashMap<[u8; 32], ValidatorTelemetry>>>,
    l2_networks: Arc<RwLock<BTreeMap<String, L2Network>>>,
    l2_commits: Arc<RwLock<BTreeMap<String, L2Commit>>>,
    l2_exits: Arc<RwLock<BTreeMap<String, L2ExitRecord>>>,
    round_certificates: Arc<RwLock<BTreeMap<RoundId, RoundCertificate>>>,
    round_finalizations: Arc<RwLock<BTreeMap<RoundId, RoundFinalizationRecord>>>,
    latest_finalized_round: Arc<RwLock<Option<RoundId>>>,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self {
            blocks: Arc::new(RwLock::new(HashMap::new())),
            txs: Arc::new(RwLock::new(HashMap::new())),
            accounts: Arc::new(RwLock::new(HashMap::new())),
            chain_state: Arc::new(RwLock::new(ChainState::default())),
            latest_height: Arc::new(RwLock::new(0)),
            validator_telemetry: Arc::new(RwLock::new(HashMap::new())),
            l2_networks: Arc::new(RwLock::new(BTreeMap::new())),
            l2_commits: Arc::new(RwLock::new(BTreeMap::new())),
            l2_exits: Arc::new(RwLock::new(BTreeMap::new())),
            round_certificates: Arc::new(RwLock::new(BTreeMap::new())),
            round_finalizations: Arc::new(RwLock::new(BTreeMap::new())),
            latest_finalized_round: Arc::new(RwLock::new(None)),
        }
    }
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Storage for MemoryStorage {
    fn store_block(&self, b: Block) -> Result<()> {
        let round = b.header.round;
        let h = hex::encode(b.hash());
        self.blocks.write().insert(h, b);
        *self.latest_height.write() = round;
        Ok(())
    }

    fn get_block(&self, hash: &[u8; 32]) -> Result<Option<Block>> {
        Ok(self.blocks.read().get(&hex::encode(hash)).cloned())
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
        self.txs.write().insert(hex::encode(tx.hash()), tx);
        Ok(())
    }

    fn get_transaction(&self, hash: &[u8; 32]) -> Result<Option<Transaction>> {
        Ok(self.txs.read().get(&hex::encode(hash)).cloned())
    }

    fn get_latest_height(&self) -> Result<u64> {
        Ok(*self.latest_height.read())
    }

    fn get_account(&self, a: &[u8; 32]) -> Result<Option<Account>> {
        Ok(self.accounts.read().get(&hex::encode(a)).cloned())
    }

    fn update_account(&self, acc: Account) -> Result<()> {
        self.accounts.write().insert(hex::encode(acc.address), acc);
        Ok(())
    }

    fn get_all_accounts(&self) -> Result<Vec<Account>> {
        Ok(self.accounts.read().values().cloned().collect())
    }

    fn get_transactions_by_address(&self, a: &[u8; 32]) -> Result<Vec<Transaction>> {
        Ok(self
            .txs
            .read()
            .values()
            .filter(|t| t.from == *a || t.to == *a)
            .cloned()
            .collect())
    }

    fn get_transaction_count(&self) -> Result<u64> {
        Ok(self.txs.read().len() as u64)
    }

    // L2 network operations
    fn put_l2_network(&self, n: L2Network) -> Result<()> {
        self.l2_networks.write().insert(n.id.clone(), n);
        Ok(())
    }
    fn get_l2_network(&self, id: &str) -> Result<Option<L2Network>> {
        Ok(self.l2_networks.read().get(id).cloned())
    }

    fn list_l2_networks(&self) -> Result<Vec<L2Network>> {
        Ok(self.l2_networks.read().values().cloned().collect())
    }
    // L2 commit operations
    fn store_l2_commit(&self, c: L2Commit) -> Result<()> {
        self.l2_commits.write().insert(c.id.clone(), c);
        Ok(())
    }
    fn list_l2_commits(&self, filter: Option<&str>) -> Result<Vec<L2Commit>> {
        Ok(self
            .l2_commits
            .read()
            .values()
            .filter(|commit| filter.map(|id| id == commit.l2_id.as_str()).unwrap_or(true))
            .cloned()
            .collect())
    }
    fn store_l2_exit(&self, exit: L2ExitRecord) -> Result<()> {
        self.l2_exits.write().insert(exit.id.clone(), exit);
        Ok(())
    }
    fn list_l2_exits(&self, filter: Option<&str>) -> Result<Vec<L2ExitRecord>> {
        Ok(self
            .l2_exits
            .read()
            .values()
            .filter(|exit| filter.map(|id| id == exit.l2_id.as_str()).unwrap_or(true))
            .cloned()
            .collect())
    }
    fn store_round_certificate(&self, certificate: RoundCertificate) -> Result<()> {
        self.round_certificates
            .write()
            .insert(certificate.round, certificate);
        Ok(())
    }
    fn get_round_certificate(&self, round: RoundId) -> Result<Option<RoundCertificate>> {
        Ok(self.round_certificates.read().get(&round).cloned())
    }
    fn store_round_finalization(&self, record: RoundFinalizationRecord) -> Result<()> {
        let round = record.round;
        {
            let mut finalizations = self.round_finalizations.write();
            finalizations.insert(round, record);
        }
        let mut latest = self.latest_finalized_round.write();
        if latest.is_none_or(|current| round >= current) {
            *latest = Some(round);
        }
        Ok(())
    }
    fn get_round_finalization(&self, round: RoundId) -> Result<Option<RoundFinalizationRecord>> {
        Ok(self.round_finalizations.read().get(&round).cloned())
    }

    fn get_latest_round_finalization(&self) -> Result<Option<RoundFinalizationRecord>> {
        if let Some(round) = *self.latest_finalized_round.read() {
            self.get_round_finalization(round)
        } else {
            Ok(None)
        }
    }

    fn get_chain_state(&self) -> Result<ChainState> {
        Ok(self.chain_state.read().clone())
    }

    fn update_chain_state(&self, s: &ChainState) -> Result<()> {
        *self.chain_state.write() = s.clone();
        Ok(())
    }

    fn store_validator_telemetry(
        &self,
        validator_id: &[u8; 32],
        telemetry: &ValidatorTelemetry,
    ) -> Result<()> {
        self.validator_telemetry
            .write()
            .insert(*validator_id, telemetry.clone());
        Ok(())
    }

    fn get_validator_telemetry(
        &self,
        validator_id: &[u8; 32],
    ) -> Result<Option<ValidatorTelemetry>> {
        Ok(self.validator_telemetry.read().get(validator_id).cloned())
    }

    fn get_all_validator_telemetry(&self) -> Result<HashMap<[u8; 32], ValidatorTelemetry>> {
        Ok(self.validator_telemetry.read().clone())
    }
}

// =====================================================================
// Storage Integration Tests - P0 Critical Path Coverage
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::l2::L2NetworkStatus;
    use ippan_types::round::RoundWindow;
    use ippan_types::{Amount, IppanTimeMicros, Transaction};
    use tempfile::tempdir;

    #[test]
    fn initialize_creates_genesis_block_and_account() {
        let dir = tempdir().expect("tempdir");
        let storage = SledStorage::new(dir.path()).expect("sled storage");
        storage.initialize().expect("initialize");

        let latest_height = storage.get_latest_height().expect("height");
        assert_eq!(latest_height, 0, "genesis block should set height to 0");

        let genesis_block = storage.get_block_by_height(0).expect("fetch genesis block");
        assert!(genesis_block.is_some(), "genesis block present after init");

        let genesis_account = storage
            .get_account(&[0u8; 32])
            .expect("fetch genesis account");
        let account = genesis_account.expect("account exists");
        assert_eq!(account.balance, 1_000_000);
        assert_eq!(account.nonce, 0);
    }

    #[test]
    fn transaction_round_trip() {
        let dir = tempdir().expect("tempdir");
        let storage = SledStorage::new(dir.path()).expect("sled storage");
        storage.initialize().expect("initialize");

        let tx = Transaction::new([1u8; 32], [2u8; 32], Amount::from_micro_ipn(42), 7);
        let tx_hash = tx.hash();
        storage
            .store_transaction(tx.clone())
            .expect("store transaction");

        let fetched = storage
            .get_transaction(&tx_hash)
            .expect("fetch transaction")
            .expect("transaction present");
        assert_eq!(fetched.hash(), tx_hash);
        assert_eq!(fetched.from, tx.from);
        assert_eq!(fetched.to, tx.to);
        assert_eq!(fetched.amount, tx.amount);
        assert_eq!(fetched.nonce, tx.nonce);
        assert_eq!(storage.get_transaction_count().unwrap(), 1);
    }

    #[test]
    fn chain_state_updates_persist() {
        let dir = tempdir().expect("tempdir");
        {
            let storage = SledStorage::new(dir.path()).expect("sled storage");
            storage.initialize().expect("initialize");

            let mut state = storage.get_chain_state().expect("initial state");
            state.add_issued_micro(500);
            state.update_round(5);
            storage
                .update_chain_state(&state)
                .expect("update chain state");
            storage.flush().expect("flush");
        }

        let storage = SledStorage::new(dir.path()).expect("reopen storage");
        let persisted = storage.get_chain_state().expect("persisted state");
        assert_eq!(persisted.total_issued_micro, 500);
        assert_eq!(persisted.last_updated_round, 5);
    }

    fn create_test_block(round: u64, creator: [u8; 32]) -> Block {
        Block::new(vec![], vec![], round, creator)
    }

    fn create_test_transaction(
        from: [u8; 32],
        to: [u8; 32],
        amount: u64,
        nonce: u64,
    ) -> Transaction {
        Transaction::new(from, to, Amount::from_atomic(amount.into()), nonce)
    }

    fn create_test_account(address: [u8; 32], balance: u64, nonce: u64) -> Account {
        Account {
            address,
            balance,
            nonce,
        }
    }

    #[test]
    fn storage_block_round_trip() {
        let storage = MemoryStorage::new();
        let creator = [1u8; 32];
        let block = create_test_block(1, creator);
        let hash = block.hash();

        storage.store_block(block.clone()).expect("store block");
        let retrieved = storage
            .get_block(&hash)
            .expect("get block")
            .expect("block exists");

        assert_eq!(retrieved.header.round, block.header.round);
        assert_eq!(retrieved.header.creator, block.header.creator);
    }

    #[test]
    fn storage_transaction_round_trip() {
        let storage = MemoryStorage::new();
        let from = [1u8; 32];
        let to = [2u8; 32];
        let tx = create_test_transaction(from, to, 1000, 1);
        let hash = tx.hash();

        storage.store_transaction(tx.clone()).expect("store tx");
        let retrieved = storage
            .get_transaction(&hash)
            .expect("get tx")
            .expect("tx exists");

        assert_eq!(retrieved.from, tx.from);
        assert_eq!(retrieved.to, tx.to);
        assert_eq!(retrieved.nonce, tx.nonce);
    }

    #[test]
    fn storage_account_update_and_retrieval() {
        let storage = MemoryStorage::new();
        let address = [3u8; 32];
        let account = create_test_account(address, 5000, 0);

        storage
            .update_account(account.clone())
            .expect("update account");
        let retrieved = storage
            .get_account(&address)
            .expect("get account")
            .expect("account exists");

        assert_eq!(retrieved.balance, 5000);
        assert_eq!(retrieved.nonce, 0);
    }

    #[test]
    fn storage_account_balance_update() {
        let storage = MemoryStorage::new();
        let address = [4u8; 32];

        let account1 = create_test_account(address, 1000, 0);
        storage.update_account(account1).expect("initial update");

        let account2 = create_test_account(address, 2000, 1);
        storage.update_account(account2).expect("balance update");

        let retrieved = storage
            .get_account(&address)
            .expect("get account")
            .expect("account exists");
        assert_eq!(retrieved.balance, 2000);
        assert_eq!(retrieved.nonce, 1);
    }

    #[test]
    fn storage_latest_height_tracking() {
        let storage = MemoryStorage::new();

        assert_eq!(storage.get_latest_height().expect("height"), 0);

        let block1 = create_test_block(1, [1u8; 32]);
        storage.store_block(block1).expect("store block 1");
        assert_eq!(storage.get_latest_height().expect("height"), 1);

        let block5 = create_test_block(5, [2u8; 32]);
        storage.store_block(block5).expect("store block 5");
        assert_eq!(storage.get_latest_height().expect("height"), 5);
    }

    #[test]
    fn storage_get_block_by_height() {
        let storage = MemoryStorage::new();
        let creator = [5u8; 32];
        let block3 = create_test_block(3, creator);

        storage.store_block(block3.clone()).expect("store block");

        let retrieved = storage
            .get_block_by_height(3)
            .expect("get by height")
            .expect("block exists");
        assert_eq!(retrieved.header.round, 3);
        assert_eq!(retrieved.header.creator, creator);
    }

    #[test]
    fn storage_get_nonexistent_block() {
        let storage = MemoryStorage::new();
        let hash = [99u8; 32];

        let result = storage.get_block(&hash).expect("query should succeed");
        assert!(result.is_none(), "Nonexistent block should return None");
    }

    #[test]
    fn storage_get_nonexistent_transaction() {
        let storage = MemoryStorage::new();
        let hash = [88u8; 32];

        let result = storage
            .get_transaction(&hash)
            .expect("query should succeed");
        assert!(
            result.is_none(),
            "Nonexistent transaction should return None"
        );
    }

    #[test]
    fn storage_get_nonexistent_account() {
        let storage = MemoryStorage::new();
        let address = [77u8; 32];

        let result = storage.get_account(&address).expect("query should succeed");
        assert!(result.is_none(), "Nonexistent account should return None");
    }

    #[test]
    fn storage_transaction_count() {
        let storage = MemoryStorage::new();

        assert_eq!(storage.get_transaction_count().expect("count"), 0);

        let tx1 = create_test_transaction([1u8; 32], [2u8; 32], 100, 1);
        storage.store_transaction(tx1).expect("store tx1");
        assert_eq!(storage.get_transaction_count().expect("count"), 1);

        let tx2 = create_test_transaction([2u8; 32], [3u8; 32], 200, 1);
        storage.store_transaction(tx2).expect("store tx2");
        assert_eq!(storage.get_transaction_count().expect("count"), 2);
    }

    #[test]
    fn storage_get_transactions_by_address() {
        let storage = MemoryStorage::new();
        let addr1 = [10u8; 32];
        let addr2 = [20u8; 32];
        let addr3 = [30u8; 32];

        let tx1 = create_test_transaction(addr1, addr2, 100, 1);
        let tx2 = create_test_transaction(addr2, addr3, 200, 1);
        let tx3 = create_test_transaction(addr1, addr3, 300, 2);

        storage.store_transaction(tx1).expect("store tx1");
        storage.store_transaction(tx2).expect("store tx2");
        storage.store_transaction(tx3).expect("store tx3");

        let addr1_txs = storage
            .get_transactions_by_address(&addr1)
            .expect("get txs");
        assert_eq!(addr1_txs.len(), 2, "addr1 should have 2 transactions");

        let addr2_txs = storage
            .get_transactions_by_address(&addr2)
            .expect("get txs");
        assert_eq!(addr2_txs.len(), 2, "addr2 should have 2 transactions");

        let addr3_txs = storage
            .get_transactions_by_address(&addr3)
            .expect("get txs");
        assert_eq!(addr3_txs.len(), 2, "addr3 should have 2 transactions");
    }

    #[test]
    fn storage_get_all_accounts() {
        let storage = MemoryStorage::new();

        let acc1 = create_test_account([1u8; 32], 1000, 0);
        let acc2 = create_test_account([2u8; 32], 2000, 0);
        let acc3 = create_test_account([3u8; 32], 3000, 0);

        storage.update_account(acc1).expect("update acc1");
        storage.update_account(acc2).expect("update acc2");
        storage.update_account(acc3).expect("update acc3");

        let all_accounts = storage.get_all_accounts().expect("get all accounts");
        assert_eq!(all_accounts.len(), 3);
    }

    #[test]
    fn storage_chain_state_persistence() {
        let storage = MemoryStorage::new();

        let state = storage.get_chain_state().expect("get state");
        assert_eq!(state.total_issued_micro, 0);
        assert_eq!(state.last_updated_round, 0);

        let mut new_state = ChainState::default();
        new_state.add_issued_micro(1_000_000);
        new_state.update_round(10);

        storage
            .update_chain_state(&new_state)
            .expect("update state");

        let retrieved = storage.get_chain_state().expect("get state");
        assert_eq!(retrieved.total_issued_micro, 1_000_000);
        assert_eq!(retrieved.last_updated_round, 10);
    }

    #[test]
    fn storage_chain_state_saturation() {
        let mut state = ChainState::default();
        state.add_issued_micro(u128::MAX - 100);
        state.add_issued_micro(200);

        assert_eq!(
            state.total_issued_micro,
            u128::MAX,
            "Should saturate at MAX"
        );
    }

    #[test]
    fn storage_validator_telemetry_round_trip() {
        let storage = MemoryStorage::new();
        let validator_id = [42u8; 32];

        let telemetry = ValidatorTelemetry {
            validator_id,
            blocks_proposed: 10,
            blocks_verified: 50,
            rounds_active: 100,
            avg_latency_us: 1000,
            slash_count: 0,
            stake: 10_000_000,
            age_rounds: 100,
            last_active_round: 100,
            uptime_percentage: 99.5,
            recent_performance: 1.0,
            network_contribution: 0.8,
        };

        storage
            .store_validator_telemetry(&validator_id, &telemetry)
            .expect("store telemetry");

        let retrieved = storage
            .get_validator_telemetry(&validator_id)
            .expect("get telemetry")
            .expect("telemetry exists");

        assert_eq!(retrieved.blocks_proposed, 10);
        assert_eq!(retrieved.uptime_percentage, 99.5);
    }

    #[test]
    fn storage_multiple_validators_telemetry() {
        let storage = MemoryStorage::new();

        for i in 0..5u8 {
            let validator_id = [i; 32];
            let telemetry = ValidatorTelemetry {
                validator_id,
                blocks_proposed: i as u64,
                blocks_verified: (i * 5) as u64,
                rounds_active: 100,
                avg_latency_us: 1000,
                slash_count: 0,
                stake: 10_000_000,
                age_rounds: 100,
                last_active_round: 100,
                uptime_percentage: 99.0,
                recent_performance: 1.0,
                network_contribution: 0.5,
            };
            storage
                .store_validator_telemetry(&validator_id, &telemetry)
                .expect("store");
        }

        let all_telemetry = storage.get_all_validator_telemetry().expect("get all");
        assert_eq!(all_telemetry.len(), 5);
    }

    #[test]
    fn storage_round_certificate_operations() {
        let storage = MemoryStorage::new();
        let round = 42u64;

        let cert = RoundCertificate {
            round,
            block_ids: vec![[0u8; 32]],
            agg_sig: Vec::new(),
        };

        storage
            .store_round_certificate(cert.clone())
            .expect("store cert");

        let retrieved = storage
            .get_round_certificate(round)
            .expect("get cert")
            .expect("cert exists");

        assert_eq!(retrieved.round, round);
    }

    #[test]
    fn storage_round_finalization_tracking() {
        let storage = MemoryStorage::new();

        let window1 = RoundWindow {
            id: 10,
            start_us: IppanTimeMicros(1_000_000),
            end_us: IppanTimeMicros(2_000_000),
        };
        let proof1 = RoundCertificate {
            round: 10,
            block_ids: vec![[0u8; 32]],
            agg_sig: Vec::new(),
        };
        let rec1 = RoundFinalizationRecord {
            round: 10,
            window: window1,
            ordered_tx_ids: vec![[1u8; 32]],
            fork_drops: vec![],
            state_root: [0u8; 32],
            proof: proof1,
        };

        storage
            .store_round_finalization(rec1.clone())
            .expect("store rec1");

        let latest = storage
            .get_latest_round_finalization()
            .expect("get latest")
            .expect("latest exists");
        assert_eq!(latest.round, 10);

        let window2 = RoundWindow {
            id: 20,
            start_us: IppanTimeMicros(3_000_000),
            end_us: IppanTimeMicros(4_000_000),
        };
        let proof2 = RoundCertificate {
            round: 20,
            block_ids: vec![[2u8; 32]],
            agg_sig: vec![],
        };
        let rec2 = RoundFinalizationRecord {
            round: 20,
            window: window2,
            ordered_tx_ids: vec![[2u8; 32]],
            fork_drops: vec![],
            state_root: [1u8; 32],
            proof: proof2,
        };

        storage
            .store_round_finalization(rec2.clone())
            .expect("store rec2");

        let latest = storage
            .get_latest_round_finalization()
            .expect("get latest")
            .expect("latest exists");
        assert_eq!(latest.round, 20, "Latest should update to round 20");
    }

    #[test]
    fn storage_concurrent_account_updates() {
        use std::sync::Arc;
        use std::thread;

        let storage = Arc::new(MemoryStorage::new());
        let address = [99u8; 32];

        let account = create_test_account(address, 1000, 0);
        storage.update_account(account).expect("initial account");

        let mut handles = vec![];
        for i in 0..10 {
            let storage_clone = Arc::clone(&storage);
            let handle = thread::spawn(move || {
                let acc = create_test_account(address, 1000 + i * 100, i);
                storage_clone.update_account(acc).expect("update");
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("thread join");
        }

        let final_account = storage
            .get_account(&address)
            .expect("get account")
            .expect("account exists");

        assert!(final_account.balance >= 1000, "Balance should be updated");
    }

    #[test]
    fn storage_l2_network_operations() {
        let storage = MemoryStorage::new();

        let network = L2Network {
            id: "test-l2".to_string(),
            proof_type: "zk-rollup".to_string(),
            da_mode: "inline".to_string(),
            status: L2NetworkStatus::Active,
            last_epoch: 0,
            total_commits: 0,
            total_exits: 0,
            last_commit_time: Some(1_500_000),
            registered_at: 1_000_000,
            challenge_window_ms: Some(60_000),
        };

        storage
            .put_l2_network(network.clone())
            .expect("put network");

        let retrieved = storage
            .get_l2_network("test-l2")
            .expect("get network")
            .expect("network exists");
        assert!(matches!(retrieved.status, L2NetworkStatus::Active));

        let all_networks = storage.list_l2_networks().expect("list networks");
        assert_eq!(all_networks.len(), 1);
    }

    #[test]
    fn storage_l2_commit_filtering() {
        let storage = MemoryStorage::new();

        let commit1 = L2Commit {
            id: "commit1".to_string(),
            l2_id: "l2-a".to_string(),
            epoch: 100,
            state_root: "root1".to_string(),
            da_hash: "hash1".to_string(),
            proof_type: "zk".to_string(),
            proof: None,
            inline_data: None,
            submitted_at: 1_000_000,
            hashtimer: "ht1".to_string(),
        };

        let commit2 = L2Commit {
            id: "commit2".to_string(),
            l2_id: "l2-b".to_string(),
            epoch: 200,
            state_root: "root2".to_string(),
            da_hash: "hash2".to_string(),
            proof_type: "zk".to_string(),
            proof: None,
            inline_data: None,
            submitted_at: 2_000_000,
            hashtimer: "ht2".to_string(),
        };

        storage.store_l2_commit(commit1).expect("store commit1");
        storage.store_l2_commit(commit2).expect("store commit2");

        let l2a_commits = storage.list_l2_commits(Some("l2-a")).expect("list commits");
        assert_eq!(l2a_commits.len(), 1);

        let all_commits = storage.list_l2_commits(None).expect("list all");
        assert_eq!(all_commits.len(), 2);
    }
}

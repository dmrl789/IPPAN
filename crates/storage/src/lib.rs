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
            .map(|v| u64::from_be_bytes(v.as_ref().try_into().unwrap()))
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
        if latest.map_or(true, |current| round >= current) {
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

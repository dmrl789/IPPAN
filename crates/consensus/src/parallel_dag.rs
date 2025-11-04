use parking_lot::{Mutex, RwLock};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::task::spawn_blocking;

use ippan_crypto::{validate_confidential_block, validate_confidential_transaction};
use ippan_storage::Storage;
use ippan_types::{Block, BlockId, RoundId, Transaction, ValidatorId};
use tracing::error;

/// Configuration knobs controlling the behaviour of the [`ParallelDag`].
#[derive(Debug, Clone)]
pub struct ParallelDagConfig {
    /// Maximum number of parents a vertex is allowed to reference.
    pub max_parents: usize,
    /// Bound applied to the ready queue. When the queue is full the oldest
    /// entry is dropped and a metric is emitted.
    pub ready_queue_bound: usize,
}

impl Default for ParallelDagConfig {
    fn default() -> Self {
        Self {
            max_parents: 16,
            ready_queue_bound: 4096,
        }
    }
}

/// Error returned when inserting or mutating vertices inside the DAG.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum DagError {
    #[error("vertex already exists")]
    DuplicateVertex,
    #[error("vertex references too many parents: {count} > {max}")]
    TooManyParents { count: usize, max: usize },
    #[error("vertex references itself as a parent")]
    SelfParent,
    #[error("vertex contains duplicate parent references")]
    DuplicateParent,
    #[error("cycle detected through ancestor {0:02x?}")]
    CycleDetected(BlockId),
}

/// Outcome returned when a vertex insertion succeeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertionOutcome {
    pub block_id: BlockId,
    pub missing_parents: Vec<BlockId>,
    pub was_ready: bool,
}

/// Snapshot of the current DAG state, providing insights for telemetry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagSnapshot {
    pub vertices: usize,
    pub committed: usize,
    pub ready: usize,
    pub depth_estimate: usize,
    pub width_estimate: usize,
    pub metrics: DagMetricsSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagMetricsSnapshot {
    pub inserted: u64,
    pub became_ready: u64,
    pub committed: u64,
    pub queue_overflows: u64,
    pub duplicates: u64,
    pub orphan_commits: u64,
}

#[derive(Debug, Default)]
struct DagMetrics {
    inserted: AtomicU64,
    became_ready: AtomicU64,
    committed: AtomicU64,
    queue_overflows: AtomicU64,
    duplicates: AtomicU64,
    orphan_commits: AtomicU64,
}

impl DagMetrics {
    fn on_insert(&self) {
        self.inserted.fetch_add(1, Ordering::Relaxed);
    }

    fn on_ready(&self) {
        self.became_ready.fetch_add(1, Ordering::Relaxed);
    }

    fn on_committed(&self) {
        self.committed.fetch_add(1, Ordering::Relaxed);
    }

    fn on_queue_overflow(&self) {
        self.queue_overflows.fetch_add(1, Ordering::Relaxed);
    }

    fn on_duplicate(&self) {
        self.duplicates.fetch_add(1, Ordering::Relaxed);
    }

    fn on_orphan_commit(&self) {
        self.orphan_commits.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> DagMetricsSnapshot {
        DagMetricsSnapshot {
            inserted: self.inserted.load(Ordering::Relaxed),
            became_ready: self.became_ready.load(Ordering::Relaxed),
            committed: self.committed.load(Ordering::Relaxed),
            queue_overflows: self.queue_overflows.load(Ordering::Relaxed),
            duplicates: self.duplicates.load(Ordering::Relaxed),
            orphan_commits: self.orphan_commits.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug)]
struct DagNode {
    block: Block,
    missing_parents: RwLock<HashSet<BlockId>>,
    children: RwLock<HashSet<BlockId>>,
}

impl DagNode {
    fn new(block: Block, missing_parents: HashSet<BlockId>) -> Self {
        Self {
            block,
            missing_parents: RwLock::new(missing_parents),
            children: RwLock::new(HashSet::new()),
        }
    }

    fn block(&self) -> &Block {
        &self.block
    }

    fn is_ready(&self) -> bool {
        self.missing_parents.read().is_empty()
    }

    fn remove_parent(&self, parent: &BlockId) -> bool {
        let mut guard = self.missing_parents.write();
        guard.remove(parent);
        guard.is_empty()
    }

    fn attach_child(&self, child: BlockId) {
        self.children.write().insert(child);
    }
}

/// Lock-free ready queue with metadata tracking for [`ParallelDag`].
#[derive(Debug)]
struct ReadyQueue {
    queue: Mutex<VecDeque<BlockId>>,
    queued: RwLock<HashSet<BlockId>>,
    bound: usize,
    metrics: Arc<DagMetrics>,
}

impl ReadyQueue {
    fn new(bound: usize, metrics: Arc<DagMetrics>) -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            queued: RwLock::new(HashSet::new()),
            bound,
            metrics,
        }
    }

    fn push(&self, block_id: BlockId) {
        {
            let mut queued = self.queued.write();
            if !queued.insert(block_id) {
                return;
            }
        }

        let mut queue = self.queue.lock();
        if self.bound > 0 && queue.len() >= self.bound {
            if let Some(removed) = queue.pop_front() {
                self.metrics.on_queue_overflow();
                self.queued.write().remove(&removed);
            }
        }
        queue.push_back(block_id);
        self.metrics.on_ready();
    }

    fn drain(&self, limit: usize) -> Vec<BlockId> {
        let mut queue = self.queue.lock();
        let mut drained = Vec::with_capacity(limit.min(queue.len()));
        while drained.len() < limit {
            match queue.pop_front() {
                Some(id) => {
                    self.queued.write().remove(&id);
                    drained.push(id);
                }
                None => break,
            }
        }
        drained
    }

    fn len(&self) -> usize {
        self.queue.lock().len()
    }
}

/// Concurrent DAG that exposes ready vertices as soon as all dependencies are
/// fulfilled. The implementation favours determinism and telemetry to aid in
/// debugging future high-throughput scenarios.
#[derive(Debug)]
pub struct ParallelDag {
    config: ParallelDagConfig,
    nodes: RwLock<HashMap<BlockId, Arc<DagNode>>>,
    committed: RwLock<HashSet<BlockId>>,
    waiters: RwLock<HashMap<BlockId, HashSet<BlockId>>>,
    ready: ReadyQueue,
    metrics: Arc<DagMetrics>,
}

impl Default for ParallelDag {
    fn default() -> Self {
        Self::new(ParallelDagConfig::default())
    }
}

impl ParallelDag {
    /// Create a new [`ParallelDag`] with the supplied configuration.
    pub fn new(config: ParallelDagConfig) -> Self {
        let metrics = Arc::new(DagMetrics::default());
        Self {
            ready: ReadyQueue::new(config.ready_queue_bound, metrics.clone()),
            config,
            nodes: RwLock::new(HashMap::new()),
            committed: RwLock::new(HashSet::new()),
            waiters: RwLock::new(HashMap::new()),
            metrics,
        }
    }

    /// Convenience constructor that relies on [`ParallelDagConfig::default`].
    pub fn with_defaults() -> Self {
        Self::default()
    }

    /// Insert a block into the DAG. The returned [`InsertionOutcome`] describes
    /// whether the vertex is ready for scheduling and which parents were
    /// missing at insertion time.
    pub fn insert_block(&self, block: Block) -> Result<InsertionOutcome, DagError> {
        let block_id = block.hash();
        let parents = block.header.parent_ids.clone();

        if parents.len() > self.config.max_parents {
            return Err(DagError::TooManyParents {
                count: parents.len(),
                max: self.config.max_parents,
            });
        }

        if parents.iter().any(|parent| parent == &block_id) {
            return Err(DagError::SelfParent);
        }

        if has_duplicates(&parents) {
            return Err(DagError::DuplicateParent);
        }

        if self.detect_cycle(&block_id, &parents) {
            return Err(DagError::CycleDetected(block_id));
        }

        let missing = self.compute_missing_parents(&parents);
        let node = Arc::new(DagNode::new(block, missing.clone()));

        {
            let mut guard = self.nodes.write();
            if guard.contains_key(&block_id) {
                self.metrics.on_duplicate();
                return Err(DagError::DuplicateVertex);
            }
            guard.insert(block_id, node.clone());
        }

        self.metrics.on_insert();

        // Attach the node as a child to known parents and register waiters for
        // missing ones. Holding the read lock for the attachment keeps the
        // critical path short.
        if !parents.is_empty() {
            let guard = self.nodes.read();
            for parent in &parents {
                if let Some(parent_node) = guard.get(parent) {
                    parent_node.attach_child(block_id);
                }
            }
        }

        if !missing.is_empty() {
            let mut waiters = self.waiters.write();
            for parent in &missing {
                waiters.entry(*parent).or_default().insert(block_id);
            }
        } else if node.is_ready() {
            self.ready.push(block_id);
        }

        let was_ready = node.is_ready();
        let mut missing_parents: Vec<_> = missing.into_iter().collect();
        missing_parents.sort();

        Ok(InsertionOutcome {
            block_id,
            missing_parents,
            was_ready,
        })
    }

    /// Drain up to `limit` ready vertices.
    pub fn drain_ready(&self, limit: usize) -> Vec<Block> {
        let ready_ids = self.ready.drain(limit);
        if ready_ids.is_empty() {
            return Vec::new();
        }
        let nodes = self.nodes.read();
        ready_ids
            .into_iter()
            .filter_map(|id| nodes.get(&id).map(|node| node.block().clone()))
            .collect()
    }

    /// Mark a vertex as committed/finalized and update the readiness of waiting
    /// children. Returns the committed block when present.
    pub fn mark_committed(&self, block_id: &BlockId) -> Option<Block> {
        let node = {
            let mut guard = self.nodes.write();
            guard.remove(block_id)
        };

        let Some(node) = node else {
            self.metrics.on_orphan_commit();
            return None;
        };

        self.metrics.on_committed();
        self.committed.write().insert(*block_id);

        let waiting_children = {
            let mut waiters = self.waiters.write();
            waiters.remove(block_id).unwrap_or_default()
        };

        if !waiting_children.is_empty() {
            let guard = self.nodes.read();
            for child_id in waiting_children {
                if let Some(child) = guard.get(&child_id) {
                    if child.remove_parent(block_id) {
                        self.ready.push(child_id);
                    }
                }
            }
        }

        Some(node.block().clone())
    }

    /// Return the number of known vertices.
    pub fn len(&self) -> usize {
        self.nodes.read().len()
    }

    /// Returns `true` if the DAG is currently empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Collect a [`DagSnapshot`] for telemetry and debugging.
    pub fn snapshot(&self) -> DagSnapshot {
        let nodes = self.nodes.read();
        let committed = self.committed.read();

        let mut round_counts: HashMap<u64, usize> = HashMap::new();
        let mut max_round = 0u64;
        for node in nodes.values() {
            let round = node.block.header.round;
            *round_counts.entry(round).or_insert(0) += 1;
            max_round = max_round.max(round);
        }
        let width_estimate = round_counts.values().copied().max().unwrap_or(0);

        DagSnapshot {
            vertices: nodes.len(),
            committed: committed.len(),
            ready: self.ready.len(),
            depth_estimate: max_round as usize,
            width_estimate,
            metrics: self.metrics.snapshot(),
        }
    }

    fn compute_missing_parents(&self, parents: &[BlockId]) -> HashSet<BlockId> {
        let committed = self.committed.read();
        parents
            .iter()
            .copied()
            .filter(|parent| !committed.contains(parent))
            .collect()
    }

    fn detect_cycle(&self, block_id: &BlockId, parents: &[BlockId]) -> bool {
        if parents.is_empty() {
            return false;
        }

        let nodes = self.nodes.read();
        for parent in parents {
            if self.reachable(block_id, parent, &nodes) {
                return true;
            }
        }
        false
    }

    fn reachable(
        &self,
        target: &BlockId,
        start: &BlockId,
        nodes: &HashMap<BlockId, Arc<DagNode>>,
    ) -> bool {
        if target == start {
            return true;
        }

        let mut stack = vec![*start];
        let mut visited: HashSet<BlockId> = HashSet::new();

        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }

            if &current == target {
                return true;
            }

            if let Some(node) = nodes.get(&current) {
                for parent in &node.block.header.parent_ids {
                    if !visited.contains(parent) {
                        stack.push(*parent);
                    }
                }
            }
        }

        false
    }
}

fn has_duplicates(values: &[BlockId]) -> bool {
    let mut set = HashSet::with_capacity(values.len());
    for value in values {
        if !set.insert(*value) {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    Valid,
    Invalid(Vec<String>),
}

/// Parallel DAG engine enabling concurrent block creation, validation, and persistence.
pub struct ParallelDagEngine {
    storage: Arc<dyn Storage + Send + Sync>,
}

impl ParallelDagEngine {
    /// Create a new parallel DAG engine backed by the supplied storage.
    pub fn new(storage: Arc<dyn Storage + Send + Sync>) -> Self {
        Self { storage }
    }

    /// Creates blocks in parallel by splitting the pending transactions into
    /// fixed-size chunks.
    pub fn create_blocks_parallel(
        &self,
        pending_txs: Vec<Transaction>,
        max_txs_per_block: usize,
        round: RoundId,
        parent_ids: Vec<BlockId>,
        proposer: ValidatorId,
    ) -> Vec<Block> {
        if pending_txs.is_empty() || max_txs_per_block == 0 {
            return Vec::new();
        }

        let parent_ids = Arc::new(parent_ids);

        let chunks: Vec<Vec<Transaction>> = pending_txs
            .chunks(max_txs_per_block)
            .map(|chunk| chunk.to_vec())
            .collect();

        chunks
            .into_par_iter()
            .map(|tx_chunk| Block::new((*parent_ids).clone(), tx_chunk, round, proposer))
            .collect()
    }

    /// Validates a block by checking structural invariants and the confidential
    /// proofs for every transaction in parallel.
    #[allow(clippy::unused_self)]
    pub fn validate_block_parallel(&self, block: &Block) -> ValidationResult {
        let mut errors = Vec::new();

        if !block.is_valid() {
            errors.push("block structure invalid".to_string());
        }

        if let Err(err) = validate_confidential_block(&block.transactions) {
            errors.push(format!("confidential block validation failed: {err}"));
        }

        let tx_errors: Vec<String> = block
            .transactions
            .par_iter()
            .enumerate()
            .filter_map(|(idx, tx)| {
                if !tx.is_valid() {
                    return Some(format!("transaction #{idx} failed structural validation"));
                }

                if let Err(err) = validate_confidential_transaction(tx) {
                    return Some(format!(
                        "transaction #{idx} failed confidential validation: {err}"
                    ));
                }

                None
            })
            .collect();

        errors.extend(tx_errors);

        if errors.is_empty() {
            ValidationResult::Valid
        } else {
            ValidationResult::Invalid(errors)
        }
    }

    /// Persists blocks concurrently using Tokio's blocking pool to isolate
    /// storage writes from the async scheduler.
    pub async fn persist_blocks_parallel(&self, blocks: Vec<Block>) {
        let handles: Vec<_> = blocks
            .into_iter()
            .map(|block| {
                let storage = Arc::clone(&self.storage);
                spawn_blocking(move || storage.store_block(block))
            })
            .collect();

        for handle in handles {
            match handle.await {
                Ok(Ok(())) => {}
                Ok(Err(err)) => error!("Parallel persist error: {err:?}"),
                Err(join_err) => error!("Parallel persist join error: {join_err}"),
            }
        }
    }

    /// Full end-to-end pipeline that creates, validates, and persists blocks
    /// derived from the supplied transactions.
    pub async fn process_pending_transactions(
        &self,
        pending_txs: Vec<Transaction>,
        max_txs_per_block: usize,
        round: RoundId,
        parent_ids: Vec<BlockId>,
        proposer: ValidatorId,
    ) {
        let blocks = self.create_blocks_parallel(
            pending_txs,
            max_txs_per_block,
            round,
            parent_ids,
            proposer,
        );

        if blocks.is_empty() {
            return;
        }

        let validated: Vec<(Block, ValidationResult)> = blocks
            .into_par_iter()
            .map(|block| {
                let result = self.validate_block_parallel(&block);
                (block, result)
            })
            .collect();

        let mut valid_blocks = Vec::new();

        for (block, result) in validated {
            match result {
                ValidationResult::Valid => valid_blocks.push(block),
                ValidationResult::Invalid(errors) => {
                    error!(
                        "Invalid block {} skipped: {}",
                        hex::encode(block.hash()),
                        errors.join("; ")
                    );
                }
            }
        }

        if !valid_blocks.is_empty() {
            self.persist_blocks_parallel(valid_blocks).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use ippan_storage::MemoryStorage;
    use ippan_types::{
        AccessKey,
        Block,
        ConfidentialEnvelope,
        ConfidentialProof,
        ConfidentialProofType,
        RoundId,
        Transaction,
        ValidatorId,
    };
    use rand::{rngs::StdRng, RngCore, SeedableRng};
    use std::sync::Arc;
    use std::thread;

    fn random_validator(rng: &mut StdRng) -> ValidatorId {
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        bytes
    }

    fn build_transaction(rng: &mut StdRng, nonce: u64) -> Transaction {
        let from = random_validator(rng);
        let mut to = random_validator(rng);
        if to == from {
            to[0] ^= 0xFF;
        }
        let mut tx = Transaction::new(from, to, ippan_types::Amount::from_atomic(1), nonce);
        tx.id = tx.hash();
        tx
    }

    fn signed_confidential_transaction(
        signing_key: &SigningKey,
        envelope: ConfidentialEnvelope,
    ) -> Transaction {
        let mut tx = Transaction::new(
            signing_key.verifying_key().to_bytes(),
            [8u8; 32],
            ippan_types::Amount::from_micro_ipn(5_000),
            42,
        );
        tx.set_confidential_envelope(envelope);
        tx.set_confidential_proof(ConfidentialProof {
            proof_type: ConfidentialProofType::Stark,
            proof: "proof-bytes".into(),
            public_inputs: Default::default(),
        });
        let signing_bytes = signing_key.to_bytes();
        tx.sign(&signing_bytes).expect("sign transaction");
        tx
    }

    fn build_block(rng: &mut StdRng, round: RoundId, parents: Vec<BlockId>) -> (Block, BlockId) {
        let tx = build_transaction(rng, round + 1);
        let creator = random_validator(rng);
        let block = Block::new(parents, vec![tx], round, creator);
        let id = block.hash();
        (block, id)
    }

    #[test]
    fn inserts_ready_block_without_parents() {
        let dag = ParallelDag::with_defaults();
        let mut rng = StdRng::seed_from_u64(42);
        let (block, block_id) = build_block(&mut rng, 1, Vec::new());

        let outcome = dag
            .insert_block(block.clone())
            .expect("insert should succeed");
        assert!(outcome.was_ready);
        assert!(outcome.missing_parents.is_empty());

        let ready = dag.drain_ready(10);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].hash(), block_id);
    }

    #[test]
    fn child_becomes_ready_after_parent_commit() {
        let dag = ParallelDag::with_defaults();
        let mut rng = StdRng::seed_from_u64(7);
        let (parent, parent_id) = build_block(&mut rng, 1, Vec::new());
        dag.insert_block(parent.clone()).unwrap();
        let _ = dag.drain_ready(1);

        let (child, _) = build_block(&mut rng, 2, vec![parent_id]);
        let outcome = dag.insert_block(child.clone()).unwrap();
        assert!(!outcome.was_ready);
        assert_eq!(outcome.missing_parents, vec![parent_id]);

        dag.mark_committed(&parent_id);
        let ready = dag.drain_ready(4);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].hash(), child.hash());
    }

    #[test]
    fn concurrent_inserts_share_ready_queue() {
        let dag = Arc::new(ParallelDag::with_defaults());
        let mut rng = StdRng::seed_from_u64(1337);
        let (parent, parent_id) = build_block(&mut rng, 1, Vec::new());
        dag.insert_block(parent.clone()).unwrap();
        let _ = dag.drain_ready(1);
        dag.mark_committed(&parent_id);

        let mut handles = Vec::new();
        for round in 2..6 {
            let dag_clone = dag.clone();
            let mut local_rng = StdRng::seed_from_u64(round * 11);
            let parent_id_copy = parent_id;
            handles.push(thread::spawn(move || {
                let (block, _) = build_block(&mut local_rng, round, vec![parent_id_copy]);
                dag_clone.insert_block(block).unwrap();
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }

        let ready = dag.drain_ready(10);
        assert_eq!(ready.len(), 4);
        let snapshot = dag.snapshot();
        assert_eq!(snapshot.vertices, 4);
        assert_eq!(snapshot.ready, 0);
    }

    #[test]
    fn duplicate_detection_is_reported() {
        let dag = ParallelDag::with_defaults();
        let mut rng = StdRng::seed_from_u64(999);
        let (block, _block_id) = build_block(&mut rng, 1, Vec::new());
        dag.insert_block(block.clone()).unwrap();
        let err = dag.insert_block(block).unwrap_err();
        assert_eq!(err, DagError::DuplicateVertex);

        let snapshot = dag.snapshot();
        assert_eq!(snapshot.metrics.duplicates, 1);
        assert_eq!(snapshot.vertices, 1);
        assert_eq!(snapshot.ready, 1);
    }

    #[test]
    fn validate_block_parallel_rejects_invalid_confidential_tx() {
        let storage = Arc::new(MemoryStorage::default());
        let engine = ParallelDagEngine::new(storage);

        let signing_key = SigningKey::from_bytes(&[21u8; 32]);
        let invalid_tx = signed_confidential_transaction(
            &signing_key,
            ConfidentialEnvelope {
                enc_algo: String::new(),
                iv: "iv".into(),
                ciphertext: "cipher".into(),
                access_keys: vec![AccessKey {
                    recipient_pub: "alice".into(),
                    enc_key: "ak".into(),
                }],
            },
        );
        let block = Block::new(vec![], vec![invalid_tx], 1, [3u8; 32]);

        match engine.validate_block_parallel(&block) {
            ValidationResult::Invalid(errors) => {
                assert!(errors.iter().any(|e| e.contains("confidential")));
            }
            ValidationResult::Valid => panic!("expected confidential validation failure"),
        }
    }

    #[test]
    fn validate_block_parallel_accepts_valid_confidential_tx() {
        let storage = Arc::new(MemoryStorage::default());
        let engine = ParallelDagEngine::new(storage);

        let signing_key = SigningKey::from_bytes(&[22u8; 32]);
        let valid_tx = signed_confidential_transaction(
            &signing_key,
            ConfidentialEnvelope {
                enc_algo: "AES-256-GCM".into(),
                iv: "iv".into(),
                ciphertext: "cipher".into(),
                access_keys: vec![AccessKey {
                    recipient_pub: "bob".into(),
                    enc_key: "bk".into(),
                }],
            },
        );
        let block = Block::new(vec![], vec![valid_tx], 1, [4u8; 32]);

        assert_eq!(engine.validate_block_parallel(&block), ValidationResult::Valid);
    }
}

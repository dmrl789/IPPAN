use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};

/// Represents a transaction in the DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub timestamp: u64, // Local or IPPAN time
    pub hash: String,
}

/// Represents a block in the BlockDAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub hash: String,
    pub parents: Vec<String>,    // Parent block hashes
    pub proposer: String,        // Validator pubkey/address
    pub transactions: Vec<Transaction>,
    pub approvals: HashSet<String>, // Set of validator pubkeys that approved this block
    pub timestamp: u64,             // Median network time (IPPAN time)
    pub reported_peer_times: Vec<u64>, // All peer times used (optional, for audit)
}

/// Approval (attestation) for a block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockApproval {
    pub block_hash: String,
    pub validator: String,      // Pubkey
    pub signature: String,      // Signature of (block_hash + timestamp)
    pub reported_time: u64,     // Validator's local time
}

/// DAG state
pub struct BlockDAG {
    pub blocks: HashMap<String, Block>,            // All blocks by hash
    pub approvals: HashMap<String, HashSet<String>>, // block_hash -> set of validator pubkeys
    pub finalized: HashSet<String>,                // Finalized block hashes
    pub validators: Vec<String>,                   // Current validator set (pubkeys)
    pub peer_times: HashMap<String, u64>,          // peer_id/pubkey -> last seen time
}

/// Utilities

/// Get the current system time (UTC, in seconds)
fn current_system_time() -> u64 {
    chrono::Utc::now().timestamp() as u64
}

/// Calculate HashTimer: cryptographic hash of data + median time
pub fn calc_hashtimer(data: &[u8], ippan_time: u64) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.update(ippan_time.to_be_bytes());
    hex::encode(hasher.finalize())
}

/// Compute IPPAN (network median) time
pub fn compute_ippan_time(peer_times: &[u64]) -> u64 {
    if peer_times.is_empty() {
        return current_system_time();
    }
    let mut times = peer_times.to_vec();
    times.sort();
    let mid = times.len() / 2;
    if times.len() % 2 == 0 {
        (times[mid - 1] + times[mid]) / 2
    } else {
        times[mid]
    }
}

impl BlockDAG {
    /// Initialize a new BlockDAG
    pub fn new(validators: Vec<String>) -> Self {
        Self {
            blocks: HashMap::new(),
            approvals: HashMap::new(),
            finalized: HashSet::new(),
            validators,
            peer_times: HashMap::new(),
        }
    }

    /// Propose a new block, using IPPAN time as timestamp
    pub fn propose_block(
        &self,
        proposer: &str,
        transactions: Vec<Transaction>,
        parents: Vec<String>,
    ) -> Block {
        // Gather known peer times and self
        let mut times: Vec<u64> = self.peer_times.values().copied().collect();
        let local_time = current_system_time();
        times.push(local_time);

        let median_time = compute_ippan_time(&times);

        // Create block (hash calculated after serialization)
        let temp_block = Block {
            hash: String::new(), // fill after
            parents,
            proposer: proposer.to_string(),
            transactions: transactions.clone(),
            approvals: HashSet::new(),
            timestamp: median_time,
            reported_peer_times: times.clone(),
        };
        // Hash all block fields except hash and approvals
        let mut temp_bytes = bincode::serialize(&(
            &temp_block.parents,
            &temp_block.proposer,
            &temp_block.transactions,
            &temp_block.timestamp,
            &temp_block.reported_peer_times,
        ))
        .unwrap();
        // HashTimer as block hash
        let hash = calc_hashtimer(&temp_bytes, median_time);

        Block {
            hash,
            ..temp_block
        }
    }

    /// Validate block: check time is within drift, parents exist, etc.
    pub fn validate_block(&self, block: &Block, max_drift: u64) -> bool {
        // Check timestamp is within `max_drift` seconds of median time
        let mut times: Vec<u64> = self.peer_times.values().copied().collect();
        times.push(current_system_time());
        let median = compute_ippan_time(&times);
        let dt = if block.timestamp > median {
            block.timestamp - median
        } else {
            median - block.timestamp
        };
        if dt > max_drift {
            println!("Reject block {}: timestamp drift {} > {}", block.hash, dt, max_drift);
            return false;
        }
        // All parents must exist (unless genesis)
        for parent in &block.parents {
            if !self.blocks.contains_key(parent) && parent != "GENESIS" {
                println!("Reject block {}: missing parent {}", block.hash, parent);
                return false;
            }
        }
        // TODO: more checks (signatures, tx validity)
        true
    }

    /// Add a new block to the DAG (after validation)
    pub fn add_block(&mut self, block: Block) {
        self.blocks.insert(block.hash.clone(), block);
    }

    /// Handle a received block approval
    pub fn handle_approval(&mut self, approval: BlockApproval) -> bool {
        let set = self.approvals.entry(approval.block_hash.clone()).or_insert_with(HashSet::new);
        set.insert(approval.validator.clone());
        // Mark as finalized if enough unique validator signatures (e.g. 2/3+)
        let total = self.validators.len();
        if set.len() * 3 > total * 2 && !self.finalized.contains(&approval.block_hash) {
            self.finalized.insert(approval.block_hash.clone());
            println!("Block {} finalized with {} approvals", approval.block_hash, set.len());
            return true;
        }
        false
    }

    /// Update peer time from received gossip
    pub fn update_peer_time(&mut self, peer_id: &str, reported_time: u64) {
        self.peer_times.insert(peer_id.to_string(), reported_time);
    }
}

// Example usage:
// (Wire these up to your networking, e.g. gossipsub handler.)

/*
let mut dag = BlockDAG::new(validators);
// On receiving any block/approval from peer:
dag.update_peer_time(&peer_id, reported_time);

// To propose a new block:
let block = dag.propose_block(&my_pubkey, txs, parent_hashes);
// Send block over network

// To validate a received block:
if dag.validate_block(&block, 120) { // max drift: 120 sec
    dag.add_block(block);
}
*/

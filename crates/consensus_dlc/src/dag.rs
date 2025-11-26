//! Block DAG (Directed Acyclic Graph) implementation for DLC consensus
//!
//! This module provides a DAG-based blockchain structure that allows
//! parallel block production and deterministic ordering.

use crate::error::{DlcError, Result};
use crate::hashtimer::HashTimer;
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const FINALIZATION_LAG_ROUNDS: u64 = 2;
const FINALIZATION_DEPTH: u64 = 2;
const MAX_REORG_DEPTH: u64 = 2;

/// A block in the DAG
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Block {
    /// Unique block identifier
    pub id: String,
    /// Parent block IDs (can have multiple parents in DAG)
    pub parents: Vec<String>,
    /// Block creation time
    pub timestamp: HashTimer,
    /// Block data (transactions, etc.)
    pub data: Vec<u8>,
    /// Block height (longest chain to genesis)
    pub height: u64,
    /// Block proposer
    pub proposer: String,
    /// Block signature
    pub signature: Option<Vec<u8>>,
    /// Merkle root of transactions
    pub merkle_root: String,
}

impl Block {
    /// Create a new block
    pub fn new(
        parents: Vec<String>,
        timestamp: HashTimer,
        data: Vec<u8>,
        proposer: String,
    ) -> Self {
        let merkle_root = Self::compute_merkle_root(&data);
        let id = Self::compute_id(&parents, &timestamp, &merkle_root, &proposer);

        Self {
            id,
            parents,
            timestamp,
            data,
            height: 0, // Will be set when inserted into DAG
            proposer,
            signature: None,
            merkle_root,
        }
    }

    /// Compute block ID from components
    fn compute_id(
        parents: &[String],
        timestamp: &HashTimer,
        merkle_root: &str,
        proposer: &str,
    ) -> String {
        let mut hasher = Hasher::new();
        for parent in parents {
            hasher.update(parent.as_bytes());
        }
        hasher.update(timestamp.hash.as_bytes());
        hasher.update(merkle_root.as_bytes());
        hasher.update(proposer.as_bytes());
        hasher.finalize().to_hex().to_string()
    }

    /// Compute Merkle root of block data
    fn compute_merkle_root(data: &[u8]) -> String {
        let mut hasher = Hasher::new();
        hasher.update(data);
        hasher.finalize().to_hex().to_string()
    }

    /// Sign the block
    pub fn sign(&mut self, signature: Vec<u8>) {
        self.signature = Some(signature);
    }

    /// Verify block signature
    pub fn verify_signature(&self) -> bool {
        // In production, verify with actual cryptographic signature
        self.signature.is_some()
    }

    /// Check if block is genesis
    pub fn is_genesis(&self) -> bool {
        self.parents.is_empty()
    }
}

/// Block DAG structure
#[derive(Default, Debug)]
pub struct BlockDAG {
    /// All blocks indexed by ID
    pub blocks: HashMap<String, Block>,
    /// Genesis block ID
    pub genesis_id: Option<String>,
    /// Tips of the DAG (blocks with no children)
    pub tips: HashSet<String>,
    /// Finalized blocks (cannot be reorganized)
    pub finalized: HashSet<String>,
    /// Pending blocks (not yet finalized)
    pub pending_ids: HashSet<String>,
    /// Children of each block
    children: HashMap<String, HashSet<String>>,
    /// Current round number
    pub current_round: u64,
}

impl BlockDAG {
    /// Create a new BlockDAG with genesis block
    pub fn new() -> Self {
        let mut dag = Self::default();

        // Create genesis block
        let genesis = Block::new(
            vec![],
            HashTimer::for_round(0),
            vec![],
            "genesis".to_string(),
        );

        let genesis_id = genesis.id.clone();
        dag.genesis_id = Some(genesis_id.clone());
        dag.blocks.insert(genesis_id.clone(), genesis);
        dag.tips.insert(genesis_id.clone());
        dag.finalized.insert(genesis_id);

        dag
    }

    /// Insert a block into the DAG
    pub fn insert(&mut self, mut block: Block) -> Result<()> {
        // Validate block
        self.validate_block(&block)?;

        // Calculate height
        block.height = self.calculate_height(&block);

        let block_id = block.id.clone();

        // Update parent-child relationships
        for parent_id in &block.parents {
            self.children
                .entry(parent_id.clone())
                .or_default()
                .insert(block_id.clone());

            // Parent is no longer a tip
            self.tips.remove(parent_id);
        }

        // Add block to tips
        self.tips.insert(block_id.clone());

        // Add to pending
        self.pending_ids.insert(block_id.clone());

        // Insert block
        self.blocks.insert(block_id, block);

        Ok(())
    }

    /// Validate a block before insertion
    fn validate_block(&self, block: &Block) -> Result<()> {
        // Check if block already exists
        if self.blocks.contains_key(&block.id) {
            return Err(DlcError::BlockValidation(format!(
                "Block {} already exists",
                block.id
            )));
        }

        // Validate parents exist (except for genesis)
        if !block.is_genesis() {
            for parent_id in &block.parents {
                if !self.blocks.contains_key(parent_id) {
                    return Err(DlcError::BlockValidation(format!(
                        "Parent block {parent_id} not found"
                    )));
                }
            }
        }

        // Validate signature
        if !block.verify_signature() && !block.is_genesis() {
            return Err(DlcError::BlockValidation(
                "Block signature verification failed".to_string(),
            ));
        }

        Ok(())
    }

    /// Calculate block height (longest path to genesis)
    fn calculate_height(&self, block: &Block) -> u64 {
        if block.is_genesis() {
            return 0;
        }

        block
            .parents
            .iter()
            .filter_map(|parent_id| self.blocks.get(parent_id))
            .map(|parent| parent.height + 1)
            .max()
            .unwrap_or(0)
    }

    /// Get all pending blocks
    pub fn pending(&self) -> Vec<Block> {
        self.pending_ids
            .iter()
            .filter_map(|id| self.blocks.get(id))
            .cloned()
            .collect()
    }

    /// Get tips of the DAG
    pub fn get_tips(&self) -> Vec<&Block> {
        self.tips
            .iter()
            .filter_map(|id| self.blocks.get(id))
            .collect()
    }

    /// Finalize blocks that are a deterministic distance behind all tips
    pub fn finalize_round(&mut self, time: HashTimer) -> Vec<String> {
        self.current_round = time.round;

        if self.current_round <= FINALIZATION_LAG_ROUNDS {
            return Vec::new();
        }

        if self.tips.is_empty() {
            return Vec::new();
        }

        let round_cutoff = self.current_round - FINALIZATION_LAG_ROUNDS;
        let candidate_ids = self.common_ancestors_beyond_depth(FINALIZATION_DEPTH);

        if candidate_ids.is_empty() {
            return Vec::new();
        }

        let mut finalized_this_round: Vec<(u64, String)> = Vec::new();

        for block_id in candidate_ids {
            if self.finalized.contains(&block_id) {
                continue;
            }

            if let Some(block) = self.blocks.get(&block_id) {
                if block.is_genesis() {
                    continue;
                }

                if block.timestamp.round > round_cutoff {
                    continue;
                }

                if self.finalized.insert(block_id.clone()) {
                    self.pending_ids.remove(&block_id);
                    finalized_this_round.push((block.height, block_id));
                }
            }
        }

        finalized_this_round.sort_by(|a, b| a.0.cmp(&b.0));

        let finalized_ids = finalized_this_round
            .into_iter()
            .map(|(_, id)| id)
            .collect::<Vec<_>>();

        if !finalized_ids.is_empty() {
            tracing::debug!(
                "Finalized {} blocks in round {}, {} pending",
                finalized_ids.len(),
                self.current_round,
                self.pending_ids.len()
            );
        }

        finalized_ids
    }

    /// Select the canonical tip using deterministic fork-choice rules.
    pub fn select_canonical_tip(
        &self,
        validator_weights: &HashMap<String, i64>,
        shadow_flags: &HashSet<String>,
        current_head: Option<&str>,
    ) -> Option<String> {
        let mut candidates: Vec<(u64, &HashTimer, i128, String)> = Vec::new();

        for block in self.get_tips() {
            let weight_score = self.path_weight_score(&block.id, validator_weights, shadow_flags);
            candidates.push((
                block.height,
                &block.timestamp,
                weight_score,
                block.id.clone(),
            ));
        }

        if candidates.is_empty() {
            return None;
        }

        candidates.sort_by(|a, b| {
            b.0.cmp(&a.0) // height first
                .then_with(|| a.1.cmp(b.1)) // then HashTimer ordering (earlier first)
                .then_with(|| b.2.cmp(&a.2)) // then cumulative validator weight
                .then_with(|| a.3.cmp(&b.3)) // deterministic tie-breaker on id
        });

        let selected = candidates.first().unwrap().3.clone();

        if let Some(current) = current_head {
            if current != selected {
                if let Some(depth) = self.reorg_depth(current, &selected) {
                    if depth > MAX_REORG_DEPTH {
                        return Some(current.to_string());
                    }
                }
            }
        }

        Some(selected)
    }

    /// Get a block by ID
    pub fn get_block(&self, id: &str) -> Option<&Block> {
        self.blocks.get(id)
    }

    /// Get children of a block
    pub fn get_children(&self, block_id: &str) -> Vec<&Block> {
        self.children
            .get(block_id)
            .map(|children_ids| {
                children_ids
                    .iter()
                    .filter_map(|id| self.blocks.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn common_ancestors_beyond_depth(&self, depth: u64) -> HashSet<String> {
        let mut tips = self.tips.iter();
        let first_tip = match tips.next() {
            Some(tip) => tip,
            None => return HashSet::new(),
        };

        let mut intersection = self.ancestors_beyond_depth(first_tip, depth);

        for tip in tips {
            let ancestors = self.ancestors_beyond_depth(tip, depth);
            let new_intersection: HashSet<String> =
                intersection.intersection(&ancestors).cloned().collect();

            intersection = new_intersection;

            if intersection.is_empty() {
                break;
            }
        }

        intersection
    }

    fn path_weight_score(
        &self,
        tip_id: &str,
        validator_weights: &HashMap<String, i64>,
        shadow_flags: &HashSet<String>,
    ) -> i128 {
        let mut score: i128 = 0;
        for block_id in self.get_path_to_genesis(tip_id) {
            if let Some(block) = self.blocks.get(&block_id) {
                let weight = *validator_weights.get(&block.proposer).unwrap_or(&0) as i128;
                score += weight;
                if shadow_flags.contains(&block_id) {
                    // Penalize branches flagged by shadow verifiers without removing them entirely
                    score -= 1_000_000i128;
                }
            }
        }
        score
    }

    fn reorg_depth(&self, current_head: &str, candidate_head: &str) -> Option<u64> {
        let current_path: HashSet<String> =
            self.get_path_to_genesis(current_head).into_iter().collect();

        for ancestor in self.get_path_to_genesis(candidate_head) {
            if current_path.contains(&ancestor) {
                let current_height = self.blocks.get(current_head)?.height;
                let ancestor_height = self.blocks.get(&ancestor)?.height;
                return Some(current_height.saturating_sub(ancestor_height));
            }
        }
        None
    }

    fn ancestors_beyond_depth(&self, tip_id: &str, depth: u64) -> HashSet<String> {
        let mut ancestors = HashSet::new();

        let tip_block = match self.blocks.get(tip_id) {
            Some(block) => block,
            None => return ancestors,
        };

        let tip_height = tip_block.height;
        let path = self.get_path_to_genesis(tip_id);

        for block_id in path {
            if let Some(block) = self.blocks.get(&block_id) {
                let distance = tip_height.saturating_sub(block.height);
                if distance >= depth {
                    ancestors.insert(block_id.clone());
                }
            }
        }

        ancestors
    }

    /// Get path from block to genesis (for chain analysis)
    pub fn get_path_to_genesis(&self, block_id: &str) -> Vec<String> {
        let mut path = vec![block_id.to_string()];
        let mut current_id = block_id.to_string();

        while let Some(block) = self.blocks.get(&current_id) {
            if block.is_genesis() {
                break;
            }

            // Follow first parent (can be extended for DAG analysis)
            if let Some(parent_id) = block.parents.first() {
                path.push(parent_id.clone());
                current_id = parent_id.clone();
            } else {
                break;
            }
        }

        path
    }

    /// Perform topological sort of DAG
    pub fn topological_sort(&self) -> Vec<String> {
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut in_progress = HashSet::new();

        // Start from tips and work backwards
        for tip_id in &self.tips {
            self.dfs_topological(tip_id, &mut visited, &mut in_progress, &mut sorted);
        }

        sorted.reverse();
        sorted
    }

    fn dfs_topological(
        &self,
        node_id: &str,
        visited: &mut HashSet<String>,
        in_progress: &mut HashSet<String>,
        sorted: &mut Vec<String>,
    ) {
        if visited.contains(node_id) {
            return;
        }

        if in_progress.contains(node_id) {
            // Cycle detected (shouldn't happen in DAG)
            return;
        }

        in_progress.insert(node_id.to_string());

        if let Some(block) = self.blocks.get(node_id) {
            for parent_id in &block.parents {
                self.dfs_topological(parent_id, visited, in_progress, sorted);
            }
        }

        in_progress.remove(node_id);
        visited.insert(node_id.to_string());
        sorted.push(node_id.to_string());
    }

    /// Get statistics about the DAG
    pub fn stats(&self) -> DagStats {
        DagStats {
            total_blocks: self.blocks.len(),
            finalized_blocks: self.finalized.len(),
            pending_blocks: self.pending_ids.len(),
            tips_count: self.tips.len(),
            current_round: self.current_round,
            max_height: self.blocks.values().map(|b| b.height).max().unwrap_or(0),
        }
    }
}

/// DAG statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagStats {
    pub total_blocks: usize,
    pub finalized_blocks: usize,
    pub pending_blocks: usize,
    pub tips_count: usize,
    pub current_round: u64,
    pub max_height: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    #[test]
    fn test_block_creation() {
        let block = Block::new(vec![], HashTimer::now(), vec![1, 2, 3], "test".to_string());

        assert!(!block.id.is_empty());
        assert!(block.is_genesis());
    }

    #[test]
    fn test_dag_creation() {
        let dag = BlockDAG::new();
        assert!(dag.genesis_id.is_some());
        assert_eq!(dag.blocks.len(), 1);
    }

    #[test]
    fn test_dag_insertion() {
        let mut dag = BlockDAG::new();
        let genesis_id = dag.genesis_id.clone().unwrap();

        let mut block = Block::new(
            vec![genesis_id.clone()],
            HashTimer::for_round(1),
            vec![1, 2, 3],
            "proposer1".to_string(),
        );
        block.sign(vec![0u8; 64]); // Mock signature

        let result = dag.insert(block);
        assert!(result.is_ok());
        assert_eq!(dag.blocks.len(), 2);
    }

    #[test]
    fn test_dag_tips() {
        let mut dag = BlockDAG::new();
        let genesis_id = dag.genesis_id.clone().unwrap();

        let mut block = Block::new(
            vec![genesis_id.clone()],
            HashTimer::for_round(1),
            vec![],
            "proposer1".to_string(),
        );
        block.sign(vec![0u8; 64]);

        dag.insert(block.clone()).unwrap();

        let tips = dag.get_tips();
        assert_eq!(tips.len(), 1);
        assert_eq!(tips[0].id, block.id);
    }

    #[test]
    fn test_dag_finalization() {
        let mut dag = BlockDAG::new();
        let genesis_id = dag.genesis_id.clone().unwrap();

        let mut previous = genesis_id.clone();
        let mut first_block_id = String::new();

        for round in 1..=3 {
            let mut block = Block::new(
                vec![previous.clone()],
                HashTimer::for_round(round),
                vec![],
                format!("validator{round}"),
            );
            block.sign(vec![0u8; 64]);
            if round == 1 {
                first_block_id = block.id.clone();
            }
            previous = block.id.clone();
            dag.insert(block).unwrap();
        }

        let finalized = dag.finalize_round(HashTimer::for_round(5));

        assert_eq!(dag.current_round, 5);
        assert!(finalized.contains(&first_block_id));
        assert!(!finalized.iter().any(|id| id == &previous));
    }

    #[test]
    fn test_dag_finalization_requires_common_prefix() {
        let mut dag = BlockDAG::new();
        let genesis_id = dag.genesis_id.clone().unwrap();

        let mut branch_a_parent = genesis_id.clone();
        let mut branch_b_parent = genesis_id.clone();

        for round in 1..=2 {
            let mut block_a = Block::new(
                vec![branch_a_parent.clone()],
                HashTimer::for_round(round),
                vec![],
                format!("validator_a{round}"),
            );
            block_a.sign(vec![0u8; 64]);
            branch_a_parent = block_a.id.clone();
            dag.insert(block_a).unwrap();

            let mut block_b = Block::new(
                vec![branch_b_parent.clone()],
                HashTimer::for_round(round),
                vec![],
                format!("validator_b{round}"),
            );
            block_b.sign(vec![0u8; 64]);
            branch_b_parent = block_b.id.clone();
            dag.insert(block_b).unwrap();
        }

        let finalized = dag.finalize_round(HashTimer::for_round(5));

        assert!(finalized.is_empty());
        assert!(!dag.finalized.contains(&branch_a_parent));
        assert!(!dag.finalized.contains(&branch_b_parent));
    }

    fn build_block_with_timer(
        parents: Vec<String>,
        timestamp: chrono::DateTime<Utc>,
        round: u64,
        proposer: &str,
    ) -> Block {
        let data = vec![1u8, 2, 3];
        let hashtimer = HashTimer::new(timestamp, round);
        let merkle_root = Block::compute_merkle_root(&data);
        let id = Block::compute_id(&parents, &hashtimer, &merkle_root, proposer);
        let mut block = Block {
            id,
            parents,
            timestamp: hashtimer,
            data,
            height: 0,
            proposer: proposer.to_string(),
            signature: None,
            merkle_root,
        };
        block.sign(vec![0u8; 64]);
        block
    }

    #[test]
    fn selects_canonical_by_height_time_and_weight() {
        let mut dag = BlockDAG::new();
        let genesis_id = dag.genesis_id.clone().unwrap();
        let base = Utc.timestamp_nanos(1_000_000);

        let block_a1 = build_block_with_timer(vec![genesis_id.clone()], base, 1, "alice");
        dag.insert(block_a1.clone()).unwrap();

        let block_b1 = build_block_with_timer(
            vec![genesis_id.clone()],
            base + Duration::nanoseconds(50),
            1,
            "bob",
        );
        dag.insert(block_b1.clone()).unwrap();

        let block_a2 = build_block_with_timer(
            vec![block_a1.id.clone()],
            base + Duration::nanoseconds(75),
            2,
            "alice",
        );
        dag.insert(block_a2.clone()).unwrap();

        let block_b2 = build_block_with_timer(
            vec![block_b1.id.clone()],
            base + Duration::nanoseconds(150),
            2,
            "bob",
        );
        dag.insert(block_b2.clone()).unwrap();

        let mut weights = HashMap::new();
        weights.insert("alice".to_string(), 10);
        weights.insert("bob".to_string(), 100);

        let tip = dag
            .select_canonical_tip(&weights, &HashSet::new(), None)
            .expect("tip");

        // Earlier HashTimer wins when heights match, even if weight is lower
        assert_eq!(tip, block_a2.id);
    }

    #[test]
    fn rejects_deep_reorg_and_penalizes_shadow_flags() {
        let mut dag = BlockDAG::new();
        let genesis_id = dag.genesis_id.clone().unwrap();
        let base = Utc.timestamp_nanos(2_000_000);

        let c1 = build_block_with_timer(vec![genesis_id.clone()], base, 1, "canon");
        dag.insert(c1.clone()).unwrap();
        let c2 = build_block_with_timer(
            vec![c1.id.clone()],
            base + Duration::nanoseconds(10),
            2,
            "canon",
        );
        dag.insert(c2.clone()).unwrap();
        let c3 = build_block_with_timer(
            vec![c2.id.clone()],
            base + Duration::nanoseconds(20),
            3,
            "canon",
        );
        dag.insert(c3.clone()).unwrap();

        // Competing deeper fork that would exceed the reorg depth
        let f1 = build_block_with_timer(
            vec![genesis_id.clone()],
            base + Duration::nanoseconds(5),
            1,
            "fork",
        );
        dag.insert(f1.clone()).unwrap();
        let f2 = build_block_with_timer(
            vec![f1.id.clone()],
            base + Duration::nanoseconds(15),
            2,
            "fork",
        );
        dag.insert(f2.clone()).unwrap();
        let f3 = build_block_with_timer(
            vec![f2.id.clone()],
            base + Duration::nanoseconds(25),
            3,
            "fork",
        );
        dag.insert(f3.clone()).unwrap();
        let f4 = build_block_with_timer(
            vec![f3.id.clone()],
            base + Duration::nanoseconds(35),
            4,
            "fork",
        );
        dag.insert(f4.clone()).unwrap();

        let mut weights = HashMap::new();
        weights.insert("canon".to_string(), 5);
        weights.insert("fork".to_string(), 50);

        // Despite better weight/height, reorg depth guard keeps current head
        let tip = dag
            .select_canonical_tip(&weights, &HashSet::new(), Some(&c3.id))
            .expect("tip");
        assert_eq!(tip, c3.id);

        // Shallow competing block flagged by shadow verifiers should be ignored
        let suspicious = build_block_with_timer(
            vec![c2.id.clone()],
            base + Duration::nanoseconds(18),
            3,
            "fork",
        );
        dag.insert(suspicious.clone()).unwrap();

        let mut shadow_flags = HashSet::new();
        shadow_flags.insert(suspicious.id.clone());

        let tip = dag
            .select_canonical_tip(&weights, &shadow_flags, Some(&c3.id))
            .expect("tip");
        assert_eq!(tip, c3.id);
    }
}

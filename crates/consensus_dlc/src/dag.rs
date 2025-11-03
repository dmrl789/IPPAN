//! Block DAG (Directed Acyclic Graph) implementation for DLC consensus
//! 
//! This module provides a DAG-based blockchain structure that allows
//! parallel block production and deterministic ordering.

use crate::error::{DlcError, Result};
use crate::hashtimer::HashTimer;
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
    fn compute_id(parents: &[String], timestamp: &HashTimer, merkle_root: &str, proposer: &str) -> String {
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
                .or_insert_with(HashSet::new)
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
            return Err(DlcError::BlockValidation(
                format!("Block {} already exists", block.id)
            ));
        }
        
        // Validate parents exist (except for genesis)
        if !block.is_genesis() {
            for parent_id in &block.parents {
                if !self.blocks.contains_key(parent_id) {
                    return Err(DlcError::BlockValidation(
                        format!("Parent block {} not found", parent_id)
                    ));
                }
            }
        }
        
        // Validate signature
        if !block.verify_signature() && !block.is_genesis() {
            return Err(DlcError::BlockValidation(
                "Block signature verification failed".to_string()
            ));
        }
        
        Ok(())
    }

    /// Calculate block height (longest path to genesis)
    fn calculate_height(&self, block: &Block) -> u64 {
        if block.is_genesis() {
            return 0;
        }
        
        block.parents
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

    /// Finalize blocks up to a certain round
    pub fn finalize_round(&mut self, time: HashTimer) {
        self.current_round = time.round;
        
        // Find blocks to finalize (older than current round - finalization_lag)
        let finalization_lag = 2; // Finalize blocks from 2 rounds ago
        
        if self.current_round <= finalization_lag {
            return;
        }
        
        let finalize_round = self.current_round - finalization_lag;
        
        let to_finalize: Vec<String> = self.pending_ids
            .iter()
            .filter(|id| {
                if let Some(block) = self.blocks.get(*id) {
                    block.timestamp.round <= finalize_round
                } else {
                    false
                }
            })
            .cloned()
            .collect();
        
        for block_id in to_finalize {
            self.finalized.insert(block_id.clone());
            self.pending_ids.remove(&block_id);
        }
        
        tracing::debug!(
            "Finalized {} blocks in round {}, {} pending",
            self.finalized.len(),
            self.current_round,
            self.pending_ids.len()
        );
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
            self.dfs_topological(
                tip_id,
                &mut visited,
                &mut in_progress,
                &mut sorted,
            );
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

    #[test]
    fn test_block_creation() {
        let block = Block::new(
            vec![],
            HashTimer::now(),
            vec![1, 2, 3],
            "test".to_string(),
        );
        
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
        
        dag.finalize_round(HashTimer::for_round(5));
        
        assert_eq!(dag.current_round, 5);
    }
}

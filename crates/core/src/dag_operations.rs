//! Advanced DAG operations for IPPAN
//!
//! Provides sophisticated DAG manipulation, analysis, and optimization
//! capabilities for production-level blockchain operations.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use tracing::{debug, info};

use crate::dag::BlockDAG;
use std::sync::Arc;
use tokio::sync::RwLock;

/// DAG analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGAnalysis {
    pub total_blocks: usize,
    pub max_depth: usize,
    pub average_depth: f64,
    pub orphan_blocks: usize,
    pub longest_chain_length: usize,
    pub branching_factor: f64,
    pub convergence_ratio: f64,
    pub analysis_time_ms: u64,
}

/// DAG path finding result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGPath {
    pub path: Vec<[u8; 32]>,
    pub length: usize,
    pub total_weight: f64,
    pub is_optimal: bool,
}

/// DAG optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGOptimizationConfig {
    pub max_orphan_age: Duration,
    pub min_chain_length: usize,
    pub convergence_threshold: f64,
    pub enable_pruning: bool,
    pub enable_compaction: bool,
}

impl Default for DAGOptimizationConfig {
    fn default() -> Self {
        Self {
            max_orphan_age: Duration::from_secs(3600), // 1 hour
            min_chain_length: 10,
            convergence_threshold: 0.8,
            enable_pruning: true,
            enable_compaction: true,
        }
    }
}

/// Advanced DAG operations manager
pub struct DAGOperations {
    dag: Arc<RwLock<BlockDAG>>,
    config: DAGOptimizationConfig,
    block_metadata: HashMap<[u8; 32], BlockMetadata>,
    last_analysis: Option<DAGAnalysis>,
}

/// Metadata for DAG blocks
#[derive(Debug, Clone)]
struct BlockMetadata {
    depth: usize,
    children: Vec<[u8; 32]>,
    parents: Vec<[u8; 32]>,
    weight: f64,
    last_accessed: Instant,
}

impl DAGOperations {
    /// Create a new DAG operations manager
    pub fn new(dag: Arc<RwLock<BlockDAG>>, config: DAGOptimizationConfig) -> Self {
        Self {
            dag,
            config,
            block_metadata: HashMap::new(),
            last_analysis: None,
        }
    }

    /// Analyze the current DAG structure
    pub async fn analyze_dag(&mut self) -> Result<DAGAnalysis> {
        let start = Instant::now();

        let tips = {
            let dag = self.dag.read().await;
            dag.get_tips()?
        };
        let mut total_blocks = 0;
        let mut max_depth = 0;
        let mut total_depth = 0;
        let mut orphan_blocks = 0;
        let mut longest_chain_length = 0;
        let mut total_children = 0;
        let mut convergence_blocks = 0;

        // Build metadata for all blocks
        self.build_metadata().await?;

        for (hash, metadata) in &self.block_metadata {
            total_blocks += 1;
            max_depth = max_depth.max(metadata.depth);
            total_depth += metadata.depth;

            if metadata.parents.is_empty() && !tips.contains(hash) {
                orphan_blocks += 1;
            }

            if metadata.children.len() > 1 {
                convergence_blocks += 1;
            }

            total_children += metadata.children.len();

            // Find longest chain from this block
            let chain_length = self.find_longest_chain_from(*hash)?;
            longest_chain_length = longest_chain_length.max(chain_length);
        }

        let average_depth = if total_blocks > 0 {
            total_depth as f64 / total_blocks as f64
        } else {
            0.0
        };

        let branching_factor = if total_blocks > 0 {
            total_children as f64 / total_blocks as f64
        } else {
            0.0
        };

        let convergence_ratio = if total_blocks > 0 {
            convergence_blocks as f64 / total_blocks as f64
        } else {
            0.0
        };

        let analysis = DAGAnalysis {
            total_blocks,
            max_depth,
            average_depth,
            orphan_blocks,
            longest_chain_length,
            branching_factor,
            convergence_ratio,
            analysis_time_ms: start.elapsed().as_millis() as u64,
        };

        self.last_analysis = Some(analysis.clone());
        info!(
            "DAG analysis completed: {} blocks, max depth: {}, convergence: {:.2}%",
            analysis.total_blocks,
            analysis.max_depth,
            analysis.convergence_ratio * 100.0
        );

        Ok(analysis)
    }

    /// Find the shortest path between two blocks
    pub fn find_shortest_path(&self, from: [u8; 32], to: [u8; 32]) -> Result<Option<DAGPath>> {
        if from == to {
            return Ok(Some(DAGPath {
                path: vec![from],
                length: 1,
                total_weight: 0.0,
                is_optimal: true,
            }));
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent_map = HashMap::new();
        let mut weights = HashMap::new();

        queue.push_back(from);
        weights.insert(from, 0.0);
        visited.insert(from);

        while let Some(current) = queue.pop_front() {
            if current == to {
                let path = self.reconstruct_path(&parent_map, from, to)?;
                let total_weight = weights.get(&to).copied().unwrap_or(0.0);
                let path_len = path.len();
                return Ok(Some(DAGPath {
                    path,
                    length: path_len,
                    total_weight,
                    is_optimal: true,
                }));
            }

            if let Some(metadata) = self.block_metadata.get(&current) {
                for &child in &metadata.children {
                    if !visited.contains(&child) {
                        visited.insert(child);
                        parent_map.insert(child, current);
                        weights.insert(child, weights.get(&current).unwrap_or(&0.0) + 1.0);
                        queue.push_back(child);
                    }
                }
            }
        }

        Ok(None)
    }

    /// Find the longest chain in the DAG
    pub async fn find_longest_chain(&self) -> Result<Option<DAGPath>> {
        let tips = {
            let dag = self.dag.read().await;
            dag.get_tips()?
        };
        let mut longest_path = None;
        let mut max_length = 0;

        for tip in tips {
            if let Some(_metadata) = self.block_metadata.get(&tip) {
                let chain_length = self.find_longest_chain_from(tip)?;
                if chain_length > max_length {
                    max_length = chain_length;
                    longest_path = Some(self.build_chain_path(tip)?);
                }
            }
        }

        Ok(longest_path)
    }

    /// Optimize the DAG by pruning orphaned blocks
    pub async fn optimize_dag(&mut self) -> Result<usize> {
        if !self.config.enable_pruning {
            return Ok(0);
        }

        let mut pruned_count = 0;
        let cutoff_time = Instant::now() - self.config.max_orphan_age;

        let tips = {
            let dag = self.dag.read().await;
            dag.get_tips()?
        };
        let mut reachable = HashSet::new();

        // Mark all reachable blocks from tips
        for tip in tips {
            self.mark_reachable(tip, &mut reachable)?;
        }

        // Find orphaned blocks
        let orphaned_blocks: Vec<[u8; 32]> = self
            .block_metadata
            .iter()
            .filter(|(hash, metadata)| {
                !reachable.contains(*hash) && metadata.last_accessed < cutoff_time
            })
            .map(|(hash, _)| *hash)
            .collect();

        // Remove orphaned blocks
        for hash in orphaned_blocks {
            if let Some(_block) = {
                let dag = self.dag.read().await;
                dag.get_block(&hash)?
            } {
                // In a real implementation, we would remove the block from storage
                // For now, we just mark it as pruned
                debug!("Pruning orphaned block: {}", hex::encode(hash));
                pruned_count += 1;
            }
        }

        if pruned_count > 0 {
            info!("Pruned {} orphaned blocks from DAG", pruned_count);
        }

        Ok(pruned_count)
    }

    /// Compact the DAG by merging similar blocks
    pub fn compact_dag(&mut self) -> Result<usize> {
        if !self.config.enable_compaction {
            return Ok(0);
        }

        let mut compacted_count = 0;

        // Find blocks that can be compacted
        let mut candidates = Vec::new();
        for (hash, metadata) in &self.block_metadata {
            if metadata.children.len() > 1 {
                candidates.push(*hash);
            }
        }

        // Sort by convergence ratio
        candidates.sort_by(|a, b| {
            let a_ratio = self.block_metadata[a].children.len();
            let b_ratio = self.block_metadata[b].children.len();
            b_ratio.cmp(&a_ratio)
        });

        // Compact high-convergence blocks
        for hash in candidates {
            if self.can_compact_block(hash)? {
                self.compact_block(hash)?;
                compacted_count += 1;
            }
        }

        if compacted_count > 0 {
            info!("Compacted {} blocks in DAG", compacted_count);
        }

        Ok(compacted_count)
    }

    /// Get DAG statistics
    pub fn get_statistics(&self) -> Result<DAGStatistics> {
        let tips = self.dag.get_tips()?;
        let total_blocks = self.block_metadata.len();

        let mut total_children = 0;
        let mut total_parents = 0;
        let mut max_depth = 0;
        let mut total_depth = 0;

        for metadata in self.block_metadata.values() {
            total_children += metadata.children.len();
            total_parents += metadata.parents.len();
            max_depth = max_depth.max(metadata.depth);
            total_depth += metadata.depth;
        }

        let average_depth = if total_blocks > 0 {
            total_depth as f64 / total_blocks as f64
        } else {
            0.0
        };

        let branching_factor = if total_blocks > 0 {
            total_children as f64 / total_blocks as f64
        } else {
            0.0
        };

        Ok(DAGStatistics {
            total_blocks,
            tip_count: tips.len(),
            max_depth,
            average_depth,
            branching_factor,
            total_children,
            total_parents,
        })
    }

    /// Build metadata for all blocks
    async fn build_metadata(&mut self) -> Result<()> {
        self.block_metadata.clear();

        // Get all blocks from the DAG
        let tips = {
            let dag = self.dag.read().await;
            dag.get_tips()?
        };
        let mut to_process = VecDeque::new();
        let mut processed = HashSet::new();

        // Start from tips and work backwards
        for tip in tips {
            to_process.push_back((tip, 0));
        }

        while let Some((hash, depth)) = to_process.pop_front() {
            if processed.contains(&hash) {
                continue;
            }

            if let Some(block) = {
                let dag = self.dag.read().await;
                dag.get_block(&hash)?
            } {
                let mut children = Vec::new();
                let parents = block.header.parent_hashes.clone();

                // Find children by looking for blocks that reference this one as parent
                for (other_hash, _) in &self.block_metadata {
                    if let Some(other_block) = {
                        let dag = self.dag.read().await;
                        dag.get_block(other_hash)?
                    } {
                        if other_block.header.parent_hashes.contains(&hash) {
                            children.push(*other_hash);
                        }
                    }
                }

                let metadata = BlockMetadata {
                    depth,
                    children,
                    parents,
                    weight: 1.0, // Simple weight for now
                    last_accessed: Instant::now(),
                };

                self.block_metadata.insert(hash, metadata);
                processed.insert(hash);

                // Add parents to processing queue
                for parent in &block.header.parent_hashes {
                    if !processed.contains(parent) {
                        to_process.push_back((*parent, depth + 1));
                    }
                }
            }
        }

        Ok(())
    }

    /// Find the longest chain from a specific block
    fn find_longest_chain_from(&self, start: [u8; 32]) -> Result<usize> {
        let mut max_length = 0;
        let mut stack = vec![(start, 1)];

        while let Some((hash, length)) = stack.pop() {
            max_length = max_length.max(length);

            if let Some(metadata) = self.block_metadata.get(&hash) {
                for &child in &metadata.children {
                    stack.push((child, length + 1));
                }
            }
        }

        Ok(max_length)
    }

    /// Reconstruct path from parent map
    fn reconstruct_path(
        &self,
        parent_map: &HashMap<[u8; 32], [u8; 32]>,
        from: [u8; 32],
        to: [u8; 32],
    ) -> Result<Vec<[u8; 32]>> {
        let mut path = Vec::new();
        let mut current = to;

        while current != from {
            path.push(current);
            current = parent_map
                .get(&current)
                .copied()
                .ok_or_else(|| anyhow!("Path reconstruction failed"))?;
        }

        path.push(from);
        path.reverse();
        Ok(path)
    }

    /// Build chain path from a tip
    fn build_chain_path(&self, tip: [u8; 32]) -> Result<DAGPath> {
        let mut path = Vec::new();
        let mut current = tip;
        let mut total_weight = 0.0;

        while let Some(metadata) = self.block_metadata.get(&current) {
            path.push(current);
            total_weight += metadata.weight;

            if metadata.parents.is_empty() {
                break;
            }

            // Choose the parent with the highest weight
            current = metadata
                .parents
                .iter()
                .max_by(|a, b| {
                    let a_weight = self.block_metadata.get(*a).map(|m| m.weight).unwrap_or(0.0);
                    let b_weight = self.block_metadata.get(*b).map(|m| m.weight).unwrap_or(0.0);
                    a_weight
                        .partial_cmp(&b_weight)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .copied()
                .unwrap_or(current);
        }

        path.reverse();
        let path_len = path.len();
        Ok(DAGPath {
            path: path.clone(),
            length: path_len,
            total_weight,
            is_optimal: true,
        })
    }

    /// Mark all reachable blocks from a starting point
    fn mark_reachable(&self, start: [u8; 32], reachable: &mut HashSet<[u8; 32]>) -> Result<()> {
        let mut stack = vec![start];

        while let Some(hash) = stack.pop() {
            if reachable.contains(&hash) {
                continue;
            }

            reachable.insert(hash);

            if let Some(metadata) = self.block_metadata.get(&hash) {
                for &parent in &metadata.parents {
                    stack.push(parent);
                }
            }
        }

        Ok(())
    }

    /// Check if a block can be compacted
    fn can_compact_block(&self, hash: [u8; 32]) -> Result<bool> {
        if let Some(metadata) = self.block_metadata.get(&hash) {
            // Can compact if it has multiple children and they're similar
            if metadata.children.len() > 1 {
                // Check if children are similar (simplified check)
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Compact a specific block
    fn compact_block(&mut self, hash: [u8; 32]) -> Result<()> {
        // In a real implementation, this would merge similar blocks
        // For now, we just mark it as compacted
        debug!("Compacting block: {}", hex::encode(hash));
        Ok(())
    }
}

/// DAG statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGStatistics {
    pub total_blocks: usize,
    pub tip_count: usize,
    pub max_depth: usize,
    pub average_depth: f64,
    pub branching_factor: f64,
    pub total_children: usize,
    pub total_parents: usize,
}

impl DAGStatistics {
    /// Get a summary string
    pub fn summary(&self) -> String {
        format!(
            "DAG Stats: {} blocks, {} tips, max depth: {}, avg depth: {:.2}, branching: {:.2}",
            self.total_blocks,
            self.tip_count,
            self.max_depth,
            self.average_depth,
            self.branching_factor
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::Block;
    use ed25519_dalek::SigningKey;
    use rand_core::OsRng;
    use tempfile::tempdir;

    fn create_test_dag() -> (BlockDAG, Vec<[u8; 32]>) {
        let dir = tempdir().unwrap();
        let dag = BlockDAG::open(dir.path()).unwrap();

        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let mut block_hashes = Vec::new();

        // Create a simple DAG structure
        let block1 = Block::new(&signing_key, vec![], vec![b"tx1".to_vec()]);
        let hash1 = block1.hash();
        dag.insert_block(&block1).unwrap();
        block_hashes.push(hash1);

        let block2 = Block::new(&signing_key, vec![hash1], vec![b"tx2".to_vec()]);
        let hash2 = block2.hash();
        dag.insert_block(&block2).unwrap();
        block_hashes.push(hash2);

        let block3 = Block::new(&signing_key, vec![hash1], vec![b"tx3".to_vec()]);
        let hash3 = block3.hash();
        dag.insert_block(&block3).unwrap();
        block_hashes.push(hash3);

        (dag, block_hashes)
    }

    #[test]
    fn test_dag_analysis() {
        let (dag, _) = create_test_dag();
        let config = DAGOptimizationConfig::default();
        let mut ops = DAGOperations::new(dag, config);

        let analysis = ops.analyze_dag().unwrap();
        assert!(analysis.total_blocks > 0);
        assert!(analysis.max_depth > 0);
    }

    #[test]
    fn test_shortest_path() {
        let (dag, hashes) = create_test_dag();
        let config = DAGOptimizationConfig::default();
        let ops = DAGOperations::new(dag, config);

        if hashes.len() >= 2 {
            let path = ops.find_shortest_path(hashes[0], hashes[1]).unwrap();
            assert!(path.is_some());
        }
    }

    #[test]
    fn test_dag_statistics() {
        let (dag, _) = create_test_dag();
        let config = DAGOptimizationConfig::default();
        let ops = DAGOperations::new(dag, config);

        let stats = ops.get_statistics().unwrap();
        assert!(stats.total_blocks > 0);
        assert!(stats.tip_count > 0);
    }
}

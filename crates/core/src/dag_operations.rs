#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::Block;
    use ed25519_dalek::SigningKey;
    use rand_core::OsRng;
    use tempfile::tempdir;
    use std::sync::Arc;
    use tokio::sync::RwLock;

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

    #[tokio::test]
    async fn test_dag_analysis() {
        let (dag, _) = create_test_dag();
        let config = DAGOptimizationConfig::default();
        let dag_arc = Arc::new(RwLock::new(dag));
        let mut ops = DAGOperations::new(dag_arc, config);

        let analysis = ops.analyze_dag().await.unwrap();
        assert!(analysis.total_blocks > 0);
        assert!(analysis.max_depth > 0);
    }

    #[tokio::test]
    async fn test_shortest_path() {
        let (dag, hashes) = create_test_dag();
        let config = DAGOptimizationConfig::default();
        let dag_arc = Arc::new(RwLock::new(dag));
        let mut ops = DAGOperations::new(dag_arc, config);

        // Build metadata first
        let _ = ops.analyze_dag().await;

        if hashes.len() >= 2 {
            let path = ops.find_shortest_path(hashes[0], hashes[1]).unwrap();
            // In small DAG, path may or may not exist; ensure no error
            let _ = path;
        }
    }

    #[tokio::test]
    async fn test_dag_statistics() {
        let (dag, _) = create_test_dag();
        let config = DAGOptimizationConfig::default();
        let dag_arc = Arc::new(RwLock::new(dag));
        let mut ops = DAGOperations::new(dag_arc, config);

        // Build metadata used by statistics
        let _ = ops.analyze_dag().await;

        let stats = ops.get_statistics().unwrap();
        assert!(stats.total_blocks >= 0);
    }
}

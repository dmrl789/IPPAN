//! Tests for IPPAN BlockDAG consensus engine

use ippan::consensus::blockdag::{BlockDAG, Block, Transaction, TransactionType};
use std::collections::HashMap;

#[tokio::test]
async fn test_blockdag_creation() {
    let dag = BlockDAG::new();
    let stats = dag.get_dag_stats().await;
    
    assert_eq!(stats.total_blocks, 0);
    assert_eq!(stats.total_transactions, 0);
    assert_eq!(stats.tips_count, 0);
    assert_eq!(stats.finalized_blocks, 0);
    assert_eq!(stats.current_round, 0);
}

#[tokio::test]
async fn test_block_validation() {
    let dag = BlockDAG::new();
    
    // Create a valid block
    let block = Block {
        hash: "test_block_hash".to_string(),
        parent_hashes: vec![],
        round: 1,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        validator_id: "validator_1".to_string(),
        transactions: vec![],
        merkle_root: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        zk_proof_reference: "zk_proof_123".to_string(),
        hashtimer: "hashtimer_123".to_string(),
        signature: "signature_123".to_string(),
    };
    
    // Add validator to set
    {
        let mut validator_set = dag.validator_set.write().await;
        validator_set.insert("validator_1".to_string(), 1000);
    }
    
    let result = dag.add_block(block).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_transaction_validation() {
    let dag = BlockDAG::new();
    
    // Create a valid transaction
    let tx = Transaction {
        hash: "test_tx_hash".to_string(),
        signature: "signature_123".to_string(),
        public_key: "public_key_123".to_string(),
        tx_type: TransactionType::Payment,
        amount: 1000,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        hashtimer: "hashtimer_123".to_string(),
        data: None,
    };
    
    let result = dag.validate_transaction(&tx).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_block_with_transactions() {
    let dag = BlockDAG::new();
    
    // Create transactions
    let tx1 = Transaction {
        hash: "tx1_hash".to_string(),
        signature: "signature_1".to_string(),
        public_key: "public_key_1".to_string(),
        tx_type: TransactionType::Payment,
        amount: 1000,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        hashtimer: "hashtimer_1".to_string(),
        data: None,
    };
    
    let tx2 = Transaction {
        hash: "tx2_hash".to_string(),
        signature: "signature_2".to_string(),
        public_key: "public_key_2".to_string(),
        tx_type: TransactionType::Staking,
        amount: 10000,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        hashtimer: "hashtimer_2".to_string(),
        data: None,
    };
    
    // Create block with transactions
    let block = Block {
        hash: "block_with_txs".to_string(),
        parent_hashes: vec![],
        round: 1,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        validator_id: "validator_1".to_string(),
        transactions: vec![tx1, tx2],
        merkle_root: "merkle_root_123".to_string(),
        zk_proof_reference: "zk_proof_123".to_string(),
        hashtimer: "hashtimer_123".to_string(),
        signature: "signature_123".to_string(),
    };
    
    // Add validator to set
    {
        let mut validator_set = dag.validator_set.write().await;
        validator_set.insert("validator_1".to_string(), 1000);
    }
    
    let result = dag.add_block(block).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
    
    // Check that transactions were processed
    let stats = dag.get_dag_stats().await;
    assert_eq!(stats.total_transactions, 2);
}

#[tokio::test]
async fn test_fork_resolution() {
    let dag = BlockDAG::new();
    
    // Create multiple blocks to simulate forks
    let block1 = Block {
        hash: "block1".to_string(),
        parent_hashes: vec![],
        round: 1,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        validator_id: "validator_1".to_string(),
        transactions: vec![],
        merkle_root: "merkle_1".to_string(),
        zk_proof_reference: "zk_1".to_string(),
        hashtimer: "ht_1".to_string(),
        signature: "sig_1".to_string(),
    };
    
    let block2 = Block {
        hash: "block2".to_string(),
        parent_hashes: vec!["block1".to_string()],
        round: 2,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        validator_id: "validator_2".to_string(),
        transactions: vec![],
        merkle_root: "merkle_2".to_string(),
        zk_proof_reference: "zk_2".to_string(),
        hashtimer: "ht_2".to_string(),
        signature: "sig_2".to_string(),
    };
    
    // Add validators to set
    {
        let mut validator_set = dag.validator_set.write().await;
        validator_set.insert("validator_1".to_string(), 1000);
        validator_set.insert("validator_2".to_string(), 1000);
    }
    
    // Add blocks
    dag.add_block(block1).await.unwrap();
    dag.add_block(block2).await.unwrap();
    
    // Test fork resolution
    let result = dag.resolve_forks().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_block_retrieval() {
    let dag = BlockDAG::new();
    
    // Create and add a block
    let block = Block {
        hash: "test_block".to_string(),
        parent_hashes: vec![],
        round: 1,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        validator_id: "validator_1".to_string(),
        transactions: vec![],
        merkle_root: "merkle_123".to_string(),
        zk_proof_reference: "zk_123".to_string(),
        hashtimer: "ht_123".to_string(),
        signature: "sig_123".to_string(),
    };
    
    // Add validator to set
    {
        let mut validator_set = dag.validator_set.write().await;
        validator_set.insert("validator_1".to_string(), 1000);
    }
    
    dag.add_block(block.clone()).await.unwrap();
    
    // Retrieve the block
    let retrieved_block = dag.get_block("test_block").await;
    assert!(retrieved_block.is_some());
    assert_eq!(retrieved_block.unwrap().hash, "test_block");
}

#[tokio::test]
async fn test_transaction_retrieval() {
    let dag = BlockDAG::new();
    
    // Create a transaction
    let tx = Transaction {
        hash: "test_tx".to_string(),
        signature: "sig_123".to_string(),
        public_key: "pk_123".to_string(),
        tx_type: TransactionType::Payment,
        amount: 1000,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        hashtimer: "ht_123".to_string(),
        data: None,
    };
    
    // Add transaction directly
    {
        let mut transactions = dag.transactions.write().await;
        transactions.insert(tx.hash.clone(), tx.clone());
    }
    
    // Retrieve the transaction
    let retrieved_tx = dag.get_transaction("test_tx").await;
    assert!(retrieved_tx.is_some());
    assert_eq!(retrieved_tx.unwrap().hash, "test_tx");
}

#[tokio::test]
async fn test_merkle_root_computation() {
    let dag = BlockDAG::new();
    
    // Create transactions
    let tx1 = Transaction {
        hash: "tx1".to_string(),
        signature: "sig1".to_string(),
        public_key: "pk1".to_string(),
        tx_type: TransactionType::Payment,
        amount: 1000,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        hashtimer: "ht1".to_string(),
        data: None,
    };
    
    let tx2 = Transaction {
        hash: "tx2".to_string(),
        signature: "sig2".to_string(),
        public_key: "pk2".to_string(),
        tx_type: TransactionType::Payment,
        amount: 2000,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        hashtimer: "ht2".to_string(),
        data: None,
    };
    
    let transactions = vec![tx1, tx2];
    let merkle_root = dag.compute_merkle_root(&transactions);
    
    // Merkle root should not be empty
    assert!(!merkle_root.is_empty());
    assert_eq!(merkle_root.len(), 64); // SHA-256 hash length
}

#[tokio::test]
async fn test_empty_merkle_root() {
    let dag = BlockDAG::new();
    let transactions = vec![];
    let merkle_root = dag.compute_merkle_root(&transactions);
    
    // Empty merkle root should be all zeros
    assert_eq!(merkle_root, "0000000000000000000000000000000000000000000000000000000000000000");
}

#[tokio::test]
async fn test_block_propagation() {
    let dag = BlockDAG::new();
    
    let block = Block {
        hash: "propagation_test".to_string(),
        parent_hashes: vec![],
        round: 1,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        validator_id: "validator_1".to_string(),
        transactions: vec![],
        merkle_root: "merkle_123".to_string(),
        zk_proof_reference: "zk_123".to_string(),
        hashtimer: "ht_123".to_string(),
        signature: "sig_123".to_string(),
    };
    
    let result = dag.propagate_block(&block).await;
    assert!(result.is_ok());
} 
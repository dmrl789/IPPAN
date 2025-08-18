//! Simple test for zk-STARK roundchain integration

use crate::consensus::roundchain::*;
use crate::consensus::roundchain::RoundHeader;
use crate::consensus::roundchain::MerkleTree;
use crate::consensus::roundchain::ZkStarkProof;
use crate::consensus::roundchain::round_manager::RoundManagerConfig;
use crate::consensus::roundchain::zk_prover::ZkProverConfig;
use crate::consensus::roundchain::proof_broadcast::BroadcastConfig;
use crate::consensus::roundchain::tx_verifier::TxVerifierConfig;
use crate::consensus::roundchain::RoundAggregation;

#[test]
fn test_round_header_creation() {
    let header = RoundHeader::new(
        1,
        [1u8; 32],
        [2u8; 32],
        1234567890,
        [3u8; 32],
    );
    
    assert_eq!(header.round_number, 1);
    assert_eq!(header.merkle_root, [1u8; 32]);
    assert_eq!(header.state_root, [2u8; 32]);
    assert_eq!(header.hashtimer_timestamp, 1234567890);
    assert_eq!(header.validator_id, [3u8; 32]);
}

#[test]
fn test_merkle_tree_creation() {
    let hashes = vec![
        [1u8; 32],
        [2u8; 32],
        [3u8; 32],
        [4u8; 32],
    ];
    
    let tree = MerkleTree::new(hashes);
    assert_eq!(tree.height, 2);
    assert!(!tree.nodes.is_empty());
}

#[test]
fn test_merkle_inclusion_proof() {
    let hashes = vec![
        [1u8; 32],
        [2u8; 32],
        [3u8; 32],
        [4u8; 32],
    ];
    
    let tree = MerkleTree::new(hashes);
    let proof = tree.generate_inclusion_proof(0).unwrap();
    
    assert!(tree.verify_inclusion_proof([1u8; 32], &proof, 0));
}

#[test]
fn test_zk_stark_proof_creation() {
    let proof = ZkStarkProof {
        proof_data: vec![1, 2, 3, 4],
        proof_size: 4,
        proving_time_ms: 100,
        verification_time_ms: 10,
        round_number: 1,
        transaction_count: 100,
    };
    
    assert_eq!(proof.round_number, 1);
    assert_eq!(proof.proof_size, 4);
    assert_eq!(proof.transaction_count, 100);
}

#[test]
fn test_round_manager_config() {
    let config = RoundManagerConfig::default();
    assert_eq!(config.round_duration_ms, 200);
    assert_eq!(config.max_blocks_per_round, 1000);
    assert_eq!(config.max_transactions_per_round, 100_000);
}

#[test]
fn test_zk_prover_config() {
    let config = ZkProverConfig::default();
    assert_eq!(config.target_proof_size, 75_000);
    assert_eq!(config.max_proving_time_ms, 1500);
    assert_eq!(config.proving_threads, 4);
}

#[test]
fn test_broadcast_config() {
    let config = BroadcastConfig::default();
    assert_eq!(config.max_payload_size, 200_000);
    assert_eq!(config.broadcast_timeout_ms, 500);
    assert_eq!(config.target_propagation_latency_ms, 180);
}

#[test]
fn test_tx_verifier_config() {
    let config = TxVerifierConfig::default();
    assert_eq!(config.max_verification_time_ms, 100);
    assert_eq!(config.cache_size_limit, 10_000);
    assert!(config.enable_verification);
}

#[test]
fn test_round_aggregation_creation() {
    let header = RoundHeader::new(1, [1u8; 32], [2u8; 32], 1234567890, [3u8; 32]);
    let proof = ZkStarkProof {
        proof_data: vec![1, 2, 3, 4],
        proof_size: 4,
        proving_time_ms: 100,
        verification_time_ms: 10,
        round_number: 1,
        transaction_count: 100,
    };
    let merkle_tree = MerkleTree::new(vec![[1u8; 32], [2u8; 32]]);
    
    let aggregation = RoundAggregation {
        header,
        zk_proof: proof,
        transaction_hashes: vec![[1u8; 32], [2u8; 32]],
        merkle_tree,
    };
    
    assert_eq!(aggregation.header.round_number, 1);
    assert_eq!(aggregation.transaction_hashes.len(), 2);
    assert_eq!(aggregation.zk_proof.proof_size, 4);
} 
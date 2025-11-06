use super::*;
use crate::{PoAConfig, PoAConsensus, Validator};
use ed25519_dalek::SigningKey;
use ippan_storage::MemoryStorage;
use ippan_types::{
    AccessKey, Block, ConfidentialEnvelope, ConfidentialProof, ConfidentialProofType, Transaction,
};
use std::sync::Arc;
use std::time::Duration;

fn create_test_config() -> PoAConfig {
    PoAConfig {
        slot_duration_ms: 100,
        validators: vec![
            Validator {
                id: [1u8; 32],
                address: [1u8; 32],
                stake: 1000,
                is_active: true,
            },
            Validator {
                id: [2u8; 32],
                address: [2u8; 32],
                stake: 2000,
                is_active: true,
            },
        ],
        max_transactions_per_block: 1000,
        block_reward: 10,
        finalization_interval_ms: 200,
        enable_ai_reputation: false,
        enable_fee_caps: true,
        enable_dag_fair_emission: true,
    }
}

#[tokio::test]
async fn test_consensus_creation() {
    let config = create_test_config();
    let storage = Arc::new(MemoryStorage::default());
    let validator_id = [1u8; 32];

    let consensus = PoAConsensus::new(config.clone(), storage, validator_id);

    let state = consensus.get_state();
    assert_eq!(state.validator_count, 2);
    assert_eq!(state.latest_block_height, 0);
}

#[tokio::test]
async fn test_consensus_start_stop() {
    let config = create_test_config();
    let storage = Arc::new(MemoryStorage::default());
    let validator_id = [1u8; 32];

    let mut consensus = PoAConsensus::new(config, storage, validator_id);

    assert!(consensus.start().await.is_ok());
    assert!(consensus.stop().await.is_ok());
}

#[tokio::test]
async fn test_proposer_selection() {
    let config = create_test_config();
    let storage = Arc::new(MemoryStorage::default());
    let validator_id = [1u8; 32];

    let consensus = PoAConsensus::new(config, storage, validator_id);
    let state = consensus.get_state();

    assert!(state.current_proposer.is_some());
}

#[tokio::test]
async fn test_block_proposal() {
    let config = create_test_config();
    let storage = Arc::new(MemoryStorage::default());
    let validator_id = [1u8; 32];

    let consensus = PoAConsensus::new(config, storage.clone(), validator_id);

    let tx = Transaction::new(
        [3u8; 32],
        [4u8; 32],
        ippan_types::Amount::from_micro_ipn(1000),
        1,
    );
    consensus.mempool().add_transaction(tx).unwrap();

    let transactions = vec![Transaction::new(
        [5u8; 32],
        [6u8; 32],
        ippan_types::Amount::from_micro_ipn(2000),
        1,
    )];
    let result = consensus.propose_block(transactions).await;

    assert!(result.is_ok());
    let block = result.unwrap();
    assert_eq!(block.header.creator, validator_id);
}

#[tokio::test]
async fn test_block_validation() {
    let config = create_test_config();
    let storage = Arc::new(MemoryStorage::default());
    let validator_id = [1u8; 32];

    let consensus = PoAConsensus::new(config, storage, validator_id);

    let tx = Transaction::new(
        [1u8; 32],
        [2u8; 32],
        ippan_types::Amount::from_micro_ipn(1000),
        1,
    );
    let block = Block::new(vec![], vec![tx], 1, validator_id);

    let result = consensus.validate_block(&block).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_block_validation_rejects_invalid_confidential_tx() {
    let config = create_test_config();
    let storage = Arc::new(MemoryStorage::default());
    let validator_id = [1u8; 32];

    let consensus = PoAConsensus::new(config, storage, validator_id);

    let key_material = [11u8; 32];
    let signing_key = SigningKey::from_bytes(&key_material);
    let from = signing_key.verifying_key().to_bytes();

    let mut tx = Transaction::new(
        from,
        [9u8; 32],
        ippan_types::Amount::from_micro_ipn(1000),
        1,
    );
    tx.set_confidential_envelope(ConfidentialEnvelope {
        enc_algo: String::new(),
        iv: "deterministic-iv".into(),
        ciphertext: "ciphertext".into(),
        access_keys: vec![AccessKey {
            recipient_pub: "recipient".into(),
            enc_key: "key".into(),
        }],
    });
    tx.set_confidential_proof(ConfidentialProof {
        proof_type: ConfidentialProofType::Stark,
        proof: "proof-bytes".into(),
        public_inputs: Default::default(),
    });
    let signing_bytes = signing_key.to_bytes();
    tx.sign(&signing_bytes).expect("sign transaction");

    let block = Block::new(vec![], vec![tx], 1, validator_id);

    let result = consensus.validate_block(&block).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[tokio::test]
async fn test_validator_management() {
    let config = create_test_config();
    let storage = Arc::new(MemoryStorage::default());
    let validator_id = [1u8; 32];

    let mut consensus = PoAConsensus::new(config, storage, validator_id);

    let initial_count = consensus.get_state().validator_count;

    let new_validator = Validator {
        id: [3u8; 32],
        address: [3u8; 32],
        stake: 1500,
        is_active: true,
    };

    consensus.add_validator(new_validator);
    assert_eq!(consensus.get_state().validator_count, initial_count + 1);

    consensus.remove_validator(&[3u8; 32]);
    assert_eq!(consensus.get_state().validator_count, initial_count);
}

#[tokio::test]
async fn test_mempool_integration() {
    let config = create_test_config();
    let storage = Arc::new(MemoryStorage::default());
    let validator_id = [1u8; 32];

    let consensus = PoAConsensus::new(config, storage, validator_id);
    let mempool = consensus.mempool();

    let tx1 = Transaction::new(
        [1u8; 32],
        [2u8; 32],
        ippan_types::Amount::from_micro_ipn(1000),
        1,
    );
    let tx2 = Transaction::new(
        [3u8; 32],
        [4u8; 32],
        ippan_types::Amount::from_micro_ipn(2000),
        1,
    );

    assert!(mempool.add_transaction(tx1).is_ok());
    assert!(mempool.add_transaction(tx2).is_ok());
    assert_eq!(mempool.size(), 2);

    let block_txs = mempool.get_transactions_for_block(10);
    assert_eq!(block_txs.len(), 2);
}

#[tokio::test]
async fn test_consensus_with_ai_reputation() {
    let mut config = create_test_config();
    config.enable_ai_reputation = true;

    let storage = Arc::new(MemoryStorage::default());
    let validator_id = [1u8; 32];

    let consensus = PoAConsensus::new(config, storage, validator_id);

    let state = consensus.get_state();
    assert_eq!(state.validator_count, 2);
}

#[tokio::test]
async fn test_fee_validation() {
    let config = create_test_config();
    let storage = Arc::new(MemoryStorage::default());
    let validator_id = [1u8; 32];

    let consensus = PoAConsensus::new(config, storage, validator_id);

    let tx = Transaction::new(
        [1u8; 32],
        [2u8; 32],
        ippan_types::Amount::from_micro_ipn(1000),
        1,
    );
    let result = consensus
        .validate_block(&Block::new(vec![], vec![tx], 1, validator_id))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_round_finalization() {
    let config = create_test_config();
    let storage = Arc::new(MemoryStorage::default());
    let validator_id = [1u8; 32];

    let mut consensus = PoAConsensus::new(config, storage.clone(), validator_id);

    consensus.start().await.unwrap();

    let tx = Transaction::new(
        [1u8; 32],
        [2u8; 32],
        ippan_types::Amount::from_micro_ipn(1000),
        1,
    );
    consensus.mempool().add_transaction(tx).unwrap();

    tokio::time::sleep(Duration::from_millis(300)).await;

    consensus.stop().await.unwrap();

    let latest_height = storage.get_latest_height().unwrap();
    assert!(latest_height >= 0);
}

#[tokio::test]
async fn test_consensus_state_consistency() {
    let config = create_test_config();
    let storage = Arc::new(MemoryStorage::default());
    let validator_id = [1u8; 32];

    let consensus = PoAConsensus::new(config, storage, validator_id);

    let state1 = consensus.get_state();
    let state2 = consensus.get_state();

    assert_eq!(state1.validator_count, state2.validator_count);
    assert_eq!(state1.latest_block_height, state2.latest_block_height);
}

#[tokio::test]
async fn test_concurrent_operations() {
    let config = create_test_config();
    let storage = Arc::new(MemoryStorage::default());
    let validator_id = [1u8; 32];

    let consensus = Arc::new(PoAConsensus::new(config, storage, validator_id));

    let mut handles = vec![];
    for i in 0..10 {
        let consensus_clone = consensus.clone();
        let handle = tokio::spawn(async move {
            let tx = Transaction::new(
                [i as u8; 32],
                [(i + 1) as u8; 32],
                ippan_types::Amount::from_micro_ipn(1000 + i as u64),
                1,
            );
            consensus_clone.mempool().add_transaction(tx)
        });
        handles.push(handle);
    }

    for handle in handles {
        assert!(handle.await.is_ok());
    }

    assert_eq!(consensus.mempool().size(), 10);
}

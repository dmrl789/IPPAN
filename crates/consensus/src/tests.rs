use super::*;
use crate::{PoAConfig, PoAConsensus, Validator};
use ed25519_dalek::SigningKey;
use ippan_storage::MemoryStorage;
use ippan_types::{
    AccessKey, Block, ConfidentialEnvelope, ConfidentialProof, ConfidentialProofType,
    IppanTimeMicros, Transaction,
};
use std::sync::Arc;
use std::time::Instant;

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

fn signed_transaction(from_seed: u8, to_seed: u8, amount_micro: u64, nonce: u64) -> Transaction {
    let from_key = SigningKey::from_bytes(&[from_seed; 32]);
    let to_key = SigningKey::from_bytes(&[to_seed; 32]);
    let mut tx = Transaction::new(
        from_key.verifying_key().to_bytes(),
        to_key.verifying_key().to_bytes(),
        ippan_types::Amount::from_micro_ipn(amount_micro),
        nonce,
    );
    tx.sign(&from_key.to_bytes())
        .expect("sign test transaction");
    tx
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

    let tx = signed_transaction(3, 4, 1000, 1);
    consensus.mempool().add_transaction(tx).unwrap();

    let transactions = vec![signed_transaction(5, 6, 2000, 2)];
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

    let tx = signed_transaction(1, 2, 1000, 1);
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

    let tx1 = signed_transaction(1, 2, 1000, 1);
    let tx2 = signed_transaction(3, 4, 2000, 2);

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

    let tx = signed_transaction(1, 2, 1000, 1);
    let result = consensus
        .validate_block(&Block::new(vec![], vec![tx], 1, validator_id))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_round_finalization() {
    let mut config = create_test_config();
    let storage = Arc::new(MemoryStorage::default());
    let validator_id = [1u8; 32];

    config.validators = vec![Validator {
        id: validator_id,
        address: validator_id,
        stake: 1000,
        is_active: true,
    }];

    let consensus = PoAConsensus::new(config, storage.clone(), validator_id);

    // Create signed transaction
    let proposer_key = SigningKey::from_bytes(&[7u8; 32]);
    let recipient_key = SigningKey::from_bytes(&[8u8; 32]);
    let from = proposer_key.verifying_key().to_bytes();
    let to = recipient_key.verifying_key().to_bytes();
    let mut tx = Transaction::new(from, to, ippan_types::Amount::from_micro_ipn(1000), 1);
    let private_key = proposer_key.to_bytes();
    tx.sign(&private_key).expect("sign test transaction");
    consensus.mempool().add_transaction(tx).unwrap();

    // Propose block
    let slot = *consensus.current_slot.read();
    PoAConsensus::propose_block(
        &consensus.storage,
        &consensus.mempool,
        &consensus.config,
        &consensus.round_tracker,
        slot,
        validator_id,
        &consensus.telemetry_manager,
        &consensus.metrics,
    )
    .await
    .unwrap();

    // Simulate finalization
    {
        let mut tracker = consensus.round_tracker.write();
        tracker.round_start = Instant::now() - consensus.finalization_interval;
        let now = IppanTimeMicros::now();
        let elapsed = consensus.finalization_interval.as_micros();
        let elapsed_u64 = elapsed.min(u128::from(u64::MAX)) as u64;
        tracker.round_start_time = IppanTimeMicros(now.0.saturating_sub(elapsed_u64));
    }

    PoAConsensus::finalize_round_if_ready(
        &consensus.storage,
        &consensus.round_tracker,
        consensus.finalization_interval,
        &consensus.config,
        &consensus.fee_collector,
        &consensus.payment_engine,
        &consensus.metrics,
    )
    .unwrap();

    let latest_height = storage.get_latest_height().unwrap();
    let finalization = storage.get_latest_round_finalization().unwrap();

    assert!(
        latest_height > 0,
        "expected block to be stored before finalization"
    );
    assert!(finalization.is_some(), "expected finalization record");
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
            let from_seed = (i + 1) as u8;
            let to_seed = (i + 17) as u8;
            let tx = signed_transaction(from_seed, to_seed, 1000 + i as u64, 1);
            consensus_clone.mempool().add_transaction(tx)
        });
        handles.push(handle);
    }

    for handle in handles {
        assert!(handle.await.is_ok());
    }

    assert_eq!(consensus.mempool().size(), 10);
}

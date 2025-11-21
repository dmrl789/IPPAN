use ippan_ai_core::{model::Model, features::ValidatorTelemetry};
use ippan_consensus::{PoAConsensus, PoAConfig, Validator};
use ippan_storage::MemoryStorage;
use std::collections::HashMap;
use std::sync::Arc;

// V1-BLOCKER: add a long-duration soak test harness that exercises networking,
// storage, RPC, and consensus over N-day runs with log capture for mainnet gating.

#[tokio::test]
async fn test_ai_integration() {
    // Create a simple test model
    let model = Model::new(
        1,
        8,
        100,
        10000,
        vec![ippan_ai_core::model::Tree {
            nodes: vec![
                ippan_ai_core::model::Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(1000),
                },
            ],
        }],
    );

    // Create storage
    let storage = std::sync::Arc::new(MemoryStorage::new());

    // Create validators
    let validators = vec![
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
    ];

    // Create consensus config with AI enabled
    let config = PoAConfig {
        slot_duration_ms: 100,
        validators: validators.clone(),
        max_transactions_per_block: 1000,
        block_reward: 10,
        finalization_interval_ms: 200,
        enable_ai_reputation: true,
        enable_fee_caps: true,
        enable_dag_fair_emission: true,
    };

    // Create consensus engine
    let mut consensus = PoAConsensus::new(config, storage, [1u8; 32]);

    // Set AI model
    assert!(consensus.set_ai_model(model).is_ok());

    // Update validator telemetry
    let telemetry = ValidatorTelemetry {
        validator_id: [1u8; 32],
        block_production_rate: 12.5,
        avg_block_size: 1200.0,
        uptime: 0.98,
        network_latency: 80.0,
        validation_accuracy: 0.99,
        stake: 1500000,
        slashing_events: 0,
        last_activity: 300,
        custom_metrics: HashMap::new(),
    };

    consensus.update_validator_telemetry([1u8; 32], telemetry);

    // Test fee manager
    let fee_manager = consensus.get_fee_manager();
    assert!(fee_manager.read().validate_fee(ippan_consensus::fees::TransactionType::Transfer, 500).is_ok());

    // Test emission calculator
    let emission_calc = consensus.get_emission_calculator();
    let reward = emission_calc.read().calculate_round_reward(100);
    assert!(reward > 0);

    println!("AI integration test passed!");
}
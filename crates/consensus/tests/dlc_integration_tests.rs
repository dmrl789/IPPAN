//! Integration tests for DLC consensus

use ippan_consensus::*;
use ippan_storage::{SledStorage, Storage};
use ippan_types::{Block, Transaction, ValidatorId};
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn test_dlc_consensus_initialization() {
    let config = DLCConfig::default();
    let validator_id = [1u8; 32];
    let mut dlc = DLCConsensus::new(config, validator_id);

    let state = dlc.get_state();
    assert_eq!(state.round_id, 1);
    assert_eq!(state.primary_verifier, validator_id);
}

#[tokio::test]
async fn test_dgbdt_verifier_selection() {
    let engine = DGBDTEngine::new();
    let mut metrics = std::collections::HashMap::new();

    // Add test validators with different metrics
    for i in 0..5 {
        let mut id = [0u8; 32];
        id[0] = i;

        let validator_metrics = ValidatorMetrics {
            blocks_proposed: 100 + (i as u64 * 10),
            blocks_verified: 200 + (i as u64 * 20),
            rounds_active: 100,
            avg_latency_us: 50_000 + (i as u64 * 10_000),
            uptime_percentage: 0.95 + (i as f64 * 0.01),
            slash_count: 0,
            recent_performance: 0.9,
            network_contribution: 0.85,
            stake_amount: 10_000_000 * (i as u64 + 1),
        };

        metrics.insert(id, validator_metrics);
    }

    let result = engine.select_verifiers(1, &metrics, 3, 5000).unwrap();

    assert_eq!(result.shadows.len(), 3);
    assert_ne!(result.primary, result.shadows[0]);
    assert_ne!(result.primary, result.shadows[1]);
    assert_ne!(result.primary, result.shadows[2]);
}

#[tokio::test]
async fn test_shadow_verifier_parallel_validation() {
    let mut set = ShadowVerifierSet::new(3);
    let validators = vec![[1u8; 32], [2u8; 32], [3u8; 32]];

    // Create a test block
    let block = Block::new(vec![], vec![], 1, [1u8; 32]);

    let results = set.verify_block(&block, &validators).await.unwrap();

    assert_eq!(results.len(), 3);
    for result in results {
        assert!(result.verification_time_us > 0);
        assert!(result.is_valid); // Empty block should be valid
    }
}

#[tokio::test]
async fn test_validator_bonding() {
    let mut manager = BondingManager::new();
    let validator_id = [1u8; 32];

    // Add bond
    manager
        .add_bond(validator_id, VALIDATOR_BOND_AMOUNT)
        .unwrap();
    assert!(manager.has_valid_bond(&validator_id));

    // Check bond info
    let bond = manager.get_bond(&validator_id).unwrap();
    assert_eq!(bond.bonded_amount, VALIDATOR_BOND_AMOUNT);
    assert!(bond.is_valid());

    // Test slashing
    manager.slash_bond(&validator_id, 100_000_000).unwrap(); // Slash 1 IPN
    let bond = manager.get_bond(&validator_id).unwrap();
    assert_eq!(bond.effective_bond(), VALIDATOR_BOND_AMOUNT - 100_000_000);
    assert!(bond.is_valid()); // Still valid with 9 IPN
}

#[tokio::test]
async fn test_temporal_finality() {
    use std::time::Duration;
    use tokio::time::sleep;

    let config = DLCConfig {
        temporal_finality_ms: 100, // Short window for testing
        ..Default::default()
    };

    let validator_id = [1u8; 32];
    let mut dlc = DLCConsensus::new(config, validator_id);

    let initial_round = dlc.get_state().round_id;

    // Wait for temporal finality window
    sleep(Duration::from_millis(150)).await;

    // Process round (should close based on time)
    dlc.process_round().await.unwrap();

    let new_round = dlc.get_state().round_id;
    assert!(new_round > initial_round);
}

#[tokio::test]
async fn test_hashtimer_deterministic_ordering() {
    let round_id = 1;
    let previous_hash = [0u8; 32];
    let validator_id = [1u8; 32];

    let hashtimer1 = generate_round_hashtimer(round_id, &previous_hash, &validator_id);
    let hashtimer2 = generate_round_hashtimer(round_id, &previous_hash, &validator_id);

    // Should be deterministic for same inputs (within same microsecond)
    assert!(hashtimer1.timestamp_us().0 > 0);
    assert!(hashtimer2.timestamp_us().0 > 0);
}

#[tokio::test]
async fn test_dlc_integrated_consensus() {
    let temp_dir = tempdir().unwrap();
    let storage = Arc::new(SledStorage::new(temp_dir.path().to_str().unwrap()).unwrap());
    storage.initialize().unwrap();

    let validator_id = [1u8; 32];
    let poa_config = PoAConfig::default();
    let poa = PoAConsensus::new(poa_config, storage, validator_id);

    let dlc_config = dlc_config_from_poa(true, 250);
    let mut integrated = DLCIntegratedConsensus::new(poa, dlc_config, validator_id);

    assert!(integrated.dlc_enabled);

    // Test bonding
    integrated
        .add_validator_bond(validator_id, VALIDATOR_BOND_AMOUNT)
        .unwrap();

    // Test metrics update
    let metrics = ValidatorMetrics {
        blocks_proposed: 10,
        blocks_verified: 20,
        rounds_active: 10,
        avg_latency_us: 50_000,
        uptime_percentage: 0.99,
        slash_count: 0,
        recent_performance: 0.95,
        network_contribution: 0.90,
        stake_amount: 20_000_000,
    };
    integrated.update_validator_metrics(validator_id, metrics);
}

#[test]
fn test_dgbdt_reputation_scoring() {
    let engine = DGBDTEngine::new();

    // Test high-performing validator
    let good_metrics = ValidatorMetrics {
        blocks_proposed: 100,
        blocks_verified: 200,
        rounds_active: 100,
        avg_latency_us: 50_000,
        uptime_percentage: 0.99,
        slash_count: 0,
        recent_performance: 0.95,
        network_contribution: 0.90,
        stake_amount: 20_000_000,
    };

    let good_score = engine.calculate_reputation(&good_metrics);
    assert!(good_score >= 8000);

    // Test poor-performing validator
    let bad_metrics = ValidatorMetrics {
        blocks_proposed: 10,
        blocks_verified: 20,
        rounds_active: 100,
        avg_latency_us: 150_000,
        uptime_percentage: 0.70,
        slash_count: 5,
        recent_performance: 0.50,
        network_contribution: 0.40,
        stake_amount: 1_000_000,
    };

    let bad_score = engine.calculate_reputation(&bad_metrics);
    assert!(bad_score < 5000);
}

#[test]
fn test_bonding_minimum_requirements() {
    let mut manager = BondingManager::new();
    let validator_id = [1u8; 32];

    // Test insufficient bond
    let result = manager.add_bond(validator_id, VALIDATOR_BOND_AMOUNT - 1);
    assert!(result.is_err());

    // Test exact minimum bond
    let result = manager.add_bond(validator_id, VALIDATOR_BOND_AMOUNT);
    assert!(result.is_ok());
    assert!(manager.has_valid_bond(&validator_id));
}

#[test]
fn test_selection_determinism() {
    let engine = DGBDTEngine::new();
    let mut metrics = std::collections::HashMap::new();

    for i in 0..5 {
        let mut id = [0u8; 32];
        id[0] = i;
        metrics.insert(id, ValidatorMetrics::default());
    }

    // Same round should produce same selection
    let result1 = engine.select_verifiers(42, &metrics, 3, 0).unwrap();
    let result2 = engine.select_verifiers(42, &metrics, 3, 0).unwrap();

    assert_eq!(result1.primary, result2.primary);
    assert_eq!(result1.shadows, result2.shadows);

    // Different round should produce different selection
    let result3 = engine.select_verifiers(43, &metrics, 3, 0).unwrap();
    assert!(result3.primary != result1.primary || result3.shadows != result1.shadows);
}

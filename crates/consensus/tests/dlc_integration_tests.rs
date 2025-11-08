//! Integration tests for DLC consensus

use ippan_consensus::*;
use ippan_storage::SledStorage;
use ippan_types::Block;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn test_dlc_consensus_initialization() {
    let config = DLCConfig::default();
    let validator_id = [1u8; 32];
    let dlc = DLCConsensus::new(config, validator_id);

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
            uptime_percentage: 950_000 + (i as i64 * 10_000), // 95% + 1% per validator
            slash_count: 0,
            recent_performance: 900_000,   // 90% in fixed-point
            network_contribution: 850_000, // 85% in fixed-point
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
async fn test_hashtimer_generation() {
    let round_id = 1;
    let previous_hash = [0u8; 32];
    let validator_id = [1u8; 32];

    let hashtimer1 = generate_round_hashtimer(round_id, &previous_hash, &validator_id);
    let hashtimer2 = generate_round_hashtimer(round_id, &previous_hash, &validator_id);

    // HashTimers should be valid and have positive timestamps
    assert!(hashtimer1.timestamp_us > 0);
    assert!(hashtimer2.timestamp_us > 0);

    // Entropy is randomized for security, so they will differ
    // This is by design - each HashTimer should be unique
    assert_ne!(hashtimer1.entropy, hashtimer2.entropy);

    // But both should be the same validator
    // For the same round, different hashtimers can be generated
    let hashtimer3 = generate_round_hashtimer(round_id + 1, &previous_hash, &validator_id);
    assert!(hashtimer3.timestamp_us > 0);
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
    let integrated = DLCIntegratedConsensus::new(poa, dlc_config, validator_id);

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
        uptime_percentage: 990_000, // 99% in fixed-point
        slash_count: 0,
        recent_performance: 950_000,   // 95% in fixed-point
        network_contribution: 900_000, // 90% in fixed-point
        stake_amount: 20_000_000,
    };
    integrated.update_validator_metrics(validator_id, metrics);
}

#[tokio::test]
async fn test_dlc_verifier_selection_respects_min_reputation_threshold() {
    use std::time::Duration;
    use tokio::time::sleep;

    let mut config = DLCConfig::default();
    config.temporal_finality_ms = 5;
    config.min_reputation_score = 9_000;

    let validator_id = [9u8; 32];
    let mut dlc = DLCConsensus::new(config, validator_id);

    let low_validator = [1u8; 32];
    let low_metrics = ValidatorMetrics {
        blocks_proposed: 10,
        blocks_verified: 15,
        rounds_active: 100,
        avg_latency_us: 150_000,
        uptime_percentage: 700_000,
        slash_count: 5,
        recent_performance: 500_000,
        network_contribution: 400_000,
        stake_amount: 1_000_000,
    };

    dlc.update_validator_metrics(low_validator, low_metrics);

    sleep(Duration::from_millis(10)).await;
    dlc.process_round().await.unwrap();

    let state = dlc.get_state();
    assert_eq!(state.round_id, 2);
    assert_eq!(state.primary_verifier, validator_id);
    assert!(state.shadow_verifiers.is_empty());
}

#[tokio::test]
async fn test_dlc_verifier_selection_prefers_high_reputation_validator() {
    use std::time::Duration;
    use tokio::time::sleep;

    let mut config = DLCConfig::default();
    config.temporal_finality_ms = 5;
    config.min_reputation_score = 8_000;
    config.shadow_verifier_count = 2;

    let self_validator = [8u8; 32];
    let mut dlc = DLCConsensus::new(config, self_validator);

    let high_validator = [2u8; 32];
    let low_validator = [3u8; 32];

    let high_metrics = ValidatorMetrics {
        blocks_proposed: 120,
        blocks_verified: 240,
        rounds_active: 120,
        avg_latency_us: 40_000,
        uptime_percentage: 990_000,
        slash_count: 0,
        recent_performance: 970_000,
        network_contribution: 950_000,
        stake_amount: 25_000_000,
    };

    let low_metrics = ValidatorMetrics {
        blocks_proposed: 10,
        blocks_verified: 20,
        rounds_active: 100,
        avg_latency_us: 150_000,
        uptime_percentage: 700_000,
        slash_count: 5,
        recent_performance: 500_000,
        network_contribution: 400_000,
        stake_amount: 1_000_000,
    };

    dlc.update_validator_metrics(high_validator, high_metrics);
    dlc.update_validator_metrics(low_validator, low_metrics);

    sleep(Duration::from_millis(10)).await;
    dlc.process_round().await.unwrap();

    let state = dlc.get_state();
    assert_eq!(state.round_id, 2);
    assert_eq!(state.primary_verifier, high_validator);
    assert!(state.shadow_verifiers.is_empty());
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
        uptime_percentage: 990_000, // 99% in fixed-point
        slash_count: 0,
        recent_performance: 950_000,   // 95% in fixed-point
        network_contribution: 900_000, // 90% in fixed-point
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
        uptime_percentage: 700_000, // 70% in fixed-point
        slash_count: 5,
        recent_performance: 500_000,   // 50% in fixed-point
        network_contribution: 400_000, // 40% in fixed-point
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

#[test]
fn test_dgbdt_select_verifiers_filters_min_reputation() {
    let engine = DGBDTEngine::new();
    let mut metrics = std::collections::HashMap::new();

    let high_validator = [1u8; 32];
    let low_validator = [2u8; 32];

    let high_metrics = ValidatorMetrics {
        blocks_proposed: 100,
        blocks_verified: 200,
        rounds_active: 100,
        avg_latency_us: 50_000,
        uptime_percentage: 990_000,
        slash_count: 0,
        recent_performance: 950_000,
        network_contribution: 900_000,
        stake_amount: 20_000_000,
    };

    let low_metrics = ValidatorMetrics {
        blocks_proposed: 10,
        blocks_verified: 20,
        rounds_active: 100,
        avg_latency_us: 150_000,
        uptime_percentage: 700_000,
        slash_count: 5,
        recent_performance: 500_000,
        network_contribution: 400_000,
        stake_amount: 1_000_000,
    };

    metrics.insert(high_validator, high_metrics);
    metrics.insert(low_validator, low_metrics);

    let result = engine
        .select_verifiers(7, &metrics, 2, 8_000)
        .expect("high reputation validator should be selected");

    assert_eq!(result.primary, high_validator);
    assert!(result.shadows.is_empty());
    assert!(result.selection_scores.is_empty());
}

#[test]
fn test_dgbdt_select_verifiers_errors_without_candidates() {
    let engine = DGBDTEngine::new();
    let metrics = std::collections::HashMap::new();

    let err = engine.select_verifiers(1, &metrics, 1, 0);
    assert!(err.is_err());
}

#[test]
fn test_dgbdt_select_verifiers_errors_when_threshold_excludes_all() {
    let engine = DGBDTEngine::new();
    let mut metrics = std::collections::HashMap::new();

    let validator = [4u8; 32];
    let low_metrics = ValidatorMetrics {
        blocks_proposed: 10,
        blocks_verified: 20,
        rounds_active: 100,
        avg_latency_us: 150_000,
        uptime_percentage: 700_000,
        slash_count: 5,
        recent_performance: 500_000,
        network_contribution: 400_000,
        stake_amount: 1_000_000,
    };

    metrics.insert(validator, low_metrics);

    let err = engine.select_verifiers(3, &metrics, 1, 9_000);
    assert!(err.is_err());
}
